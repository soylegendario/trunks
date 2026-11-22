import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { Unicode11Addon } from "@xterm/addon-unicode11";
import { SerializeAddon } from "@xterm/addon-serialize";
import {
  isSnapshotsEnabledLocal,
  snapshotLoad,
  snapshotSave,
} from "./snapshots";
import {
  buildRestoreSequence,
  captureDelayMs,
  newShellPrelude,
  shouldCapture,
  shouldRestoreSnapshot,
  snapshotHasAltScreen,
} from "./snapshot-logic";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import "@xterm/xterm/css/xterm.css";

export interface TerminalTab {
  id: string;
  title: string;
  cwd: string;
  projectName?: string;
  worktreeName?: string;
  status: "running" | "stopped";
  isRestored?: boolean;
  isStale?: boolean;
  staleReason?: string;
  tabOrder?: number;
}

export interface PtyOutputPayload {
  id: string;
  data: string;
}

export interface PtyExitPayload {
  id: string;
  exit_code: number | null;
}

export interface SystemInfo {
  os: string;
  default_shell: string;
  home_dir: string | null;
}

export class TerminalSession {
  public id: string;
  public tab: TerminalTab;
  public terminal: Terminal;
  public fitAddon: FitAddon;
  private unlistenOutput: UnlistenFn | null = null;
  private unlistenExit: UnlistenFn | null = null;
  private isDisposed = false;
  private onStatusChange?: (status: "running" | "stopped") => void;
  private fitTimer: ReturnType<typeof setTimeout> | null = null;
  private serializeAddon = new SerializeAddon();
  // Snapshot capture state. `dirty` is only set by live output, never by restoring a snapshot, so
  // an untouched restored tab is not re-saved with the old separator baked into its content.
  private dirty = false;
  private firstDirtyAt = 0;
  private lastOutputAt = 0;
  private captureTimer: ReturnType<typeof setTimeout> | null = null;
  private capturing = false;
  private restoredAltScreen = false;
  private showedSnapshot = false;

  constructor(
    tab: TerminalTab,
    onStatusChange?: (status: "running" | "stopped") => void
  ) {
    this.id = tab.id;
    this.tab = tab;
    this.onStatusChange = onStatusChange;

    this.terminal = new Terminal({
      cursorBlink: true,
      cursorStyle: "bar",
      fontSize: 13,
      fontFamily:
        'Menlo, Monaco, "SF Mono", "Apple Symbols", "Apple Color Emoji", "Courier New", monospace',
      theme: {
        background: "#11111b",
        foreground: "#cdd6f4",
        cursor: "#9448f8",
        selectionBackground: "#3a1d5e",
        black: "#11111b",
        red: "#f38ba8",
        green: "#a6e3a1",
        yellow: "#f9e2af",
        blue: "#89b4fa",
        magenta: "#9448f8",
        cyan: "#94e2d5",
        white: "#bac2de",
        brightBlack: "#585b70",
        brightRed: "#f38ba8",
        brightGreen: "#a6e3a1",
        brightYellow: "#f9e2af",
        brightBlue: "#89b4fa",
        brightMagenta: "#aa6dfb",
        brightCyan: "#94e2d5",
        brightWhite: "#a6adc8",
      },
      allowProposedApi: true,
    });

    this.fitAddon = new FitAddon();
    this.terminal.loadAddon(this.fitAddon);
    // Correct width for box-drawing/symbol/emoji glyphs used by TUIs (Claude Code statusline)
    this.terminal.loadAddon(new Unicode11Addon());
    this.terminal.unicode.activeVersion = "11";
    this.terminal.loadAddon(this.serializeAddon);

    // Forward user keystrokes to Rust PTY
    this.terminal.onData((data) => {
      if (this.tab.status === "running" && !this.isDisposed) {
        invoke("pty_write", { id: this.id, data }).catch((err) => {
          console.error(`Failed to write to PTY ${this.id}:`, err);
        });
      }
    });

    // Forward terminal resize to Rust PTY
    this.terminal.onResize(({ cols, rows }) => {
      if (this.tab.status === "running" && !this.isDisposed) {
        invoke("pty_resize", { id: this.id, cols, rows }).catch((err) => {
          console.error(`Failed to resize PTY ${this.id}:`, err);
        });
      }
    });
  }

  public async mount(container: HTMLElement) {
    this.terminal.open(container);
    this.fit();

    // Listen to PTY output
    this.unlistenOutput = await listen<PtyOutputPayload>("pty-output", (event) => {
      if (event.payload.id === this.id && !this.isDisposed) {
        this.terminal.write(event.payload.data);
        this.markDirty();
      }
    });

    // Listen to PTY exit
    this.unlistenExit = await listen<PtyExitPayload>("pty-exit", (event) => {
      if (event.payload.id === this.id && !this.isDisposed) {
        this.tab.status = "stopped";
        this.onStatusChange?.("stopped");
        // Capture once the parser has consumed the final output and the exit notice.
        this.terminal.write("\r\n\x1b[33m[Process completed]\x1b[0m\r\n", () => {
          this.dirty = true;
          if (this.firstDirtyAt === 0) this.firstDirtyAt = Date.now();
          void this.captureNow();
        });
      }
    });

    // Spawn the shell process if running
    if (this.tab.status === "running") {
      await this.startPty();
    } else if (this.tab.isRestored) {
      await this.restoreSnapshot();
      this.terminal.writeln("\x1b[36m[Workspace restored] This terminal tab was restored from a previous session.\x1b[0m");
      if (this.tab.isStale) {
        this.terminal.writeln(`\x1b[33m⚠️ Stale path: '${this.tab.cwd}' (${this.tab.staleReason || "directory not found"}).\x1b[0m`);
      } else {
        this.terminal.writeln(`\x1b[90mDirectory: ${this.tab.cwd}\x1b[0m`);
        this.terminal.writeln("\x1b[32mClick 'Start Shell' or press the Restart button to launch a new process.\x1b[0m\r\n");
      }
    }
  }

  public async startPty() {
    try {
      if (this.showedSnapshot) {
        // Leave a restored TUI's alternate screen and reset leftover modes before the new shell.
        this.terminal.write(newShellPrelude(this.restoredAltScreen));
        this.showedSnapshot = false;
        this.restoredAltScreen = false;
      }
      this.tab.isRestored = false;
      this.tab.status = "running";
      this.onStatusChange?.("running");
      const cols = this.terminal.cols || 80;
      const rows = this.terminal.rows || 24;

      await invoke("pty_create", {
        id: this.id,
        cwd: this.tab.cwd || undefined,
        cols,
        rows,
      });
    } catch (err) {
      this.tab.status = "stopped";
      this.onStatusChange?.("stopped");
      this.terminal.writeln(`\r\n\x1b[31mFailed to start PTY process: ${err}\x1b[0m`);
    }
  }

  /** Shows the stored snapshot (if any) above the restored-tab notice. Never throws. */
  private async restoreSnapshot() {
    if (
      !shouldRestoreSnapshot({
        enabled: isSnapshotsEnabledLocal(),
        isRestored: !!this.tab.isRestored,
        status: this.tab.status,
      })
    ) {
      return;
    }
    try {
      const snap = await snapshotLoad(this.id);
      if (!snap || this.isDisposed) return;
      this.restoredAltScreen = snapshotHasAltScreen(snap.data);
      this.showedSnapshot = true;
      await new Promise<void>((resolve) =>
        this.terminal.write(buildRestoreSequence(snap.data, snap.created_at), resolve)
      );
    } catch (err) {
      console.warn(`Failed to restore snapshot for tab ${this.id}:`, String(err));
    }
  }

  private markDirty() {
    if (!isSnapshotsEnabledLocal()) return;
    const now = Date.now();
    if (!this.dirty) {
      this.dirty = true;
      this.firstDirtyAt = now;
    }
    this.lastOutputAt = now;
    if (this.captureTimer) clearTimeout(this.captureTimer);
    const delay = captureDelayMs({
      now,
      firstDirtyAt: this.firstDirtyAt,
      lastOutputAt: this.lastOutputAt,
    });
    this.captureTimer = setTimeout(() => {
      this.captureTimer = null;
      void this.captureNow();
    }, delay);
  }

  /**
   * Serializes the buffer and stores it. Fire-and-forget safe: failures are logged without content
   * and never affect the terminal. Also used to flush on app close.
   */
  public async captureNow(): Promise<void> {
    if (this.captureTimer) {
      clearTimeout(this.captureTimer);
      this.captureTimer = null;
    }
    if (
      this.capturing ||
      !shouldCapture({
        enabled: isSnapshotsEnabledLocal(),
        disposed: this.isDisposed,
        dirty: this.dirty,
      })
    ) {
      return;
    }
    this.capturing = true;
    try {
      this.dirty = false;
      this.firstDirtyAt = 0;
      const t0 = performance.now();
      const data = this.serializeAddon.serialize();
      const t1 = performance.now();
      await snapshotSave(this.id, this.terminal.cols, this.terminal.rows, data);
      // Content-free timing for the manual measurements (docs/milestones/m3-results.md).
      console.debug(
        `[snapshot] tab=${this.id} serialize=${(t1 - t0).toFixed(1)}ms chars=${data.length} save=${(performance.now() - t1).toFixed(1)}ms`
      );
    } catch (err) {
      this.dirty = true; // keep it pending so a later trigger retries
      console.warn(`Failed to capture snapshot for tab ${this.id}:`, String(err));
    } finally {
      this.capturing = false;
    }
  }

  public fit() {
    if (this.isDisposed) return;
    try {
      // Skip when the grid would not change: avoids redundant SIGWINCH/reflow.
      const dims = this.fitAddon.proposeDimensions();
      if (
        !dims ||
        Number.isNaN(dims.cols) ||
        Number.isNaN(dims.rows) ||
        (dims.cols === this.terminal.cols && dims.rows === this.terminal.rows)
      ) {
        return;
      }
      this.fitAddon.fit();
    } catch (err) {
      // Element might be hidden (display: none)
    }
  }

  /**
   * Debounced fit for continuous resize sources (panel toggle/drag, window resize).
   * Only the settled size reaches the PTY, so TUIs repaint once at the final width.
   */
  public scheduleFit(delay = 100) {
    if (this.isDisposed) return;
    if (this.fitTimer) clearTimeout(this.fitTimer);
    this.fitTimer = setTimeout(() => {
      this.fitTimer = null;
      this.fit();
    }, delay);
  }

  public async restart() {
    this.terminal.clear();
    await this.startPty();
  }

  public async destroy() {
    this.isDisposed = true;
    if (this.captureTimer) {
      clearTimeout(this.captureTimer);
      this.captureTimer = null;
    }
    if (this.fitTimer) {
      clearTimeout(this.fitTimer);
      this.fitTimer = null;
    }
    if (this.unlistenOutput) {
      this.unlistenOutput();
      this.unlistenOutput = null;
    }
    if (this.unlistenExit) {
      this.unlistenExit();
      this.unlistenExit = null;
    }
    try {
      await invoke("pty_kill", { id: this.id });
    } catch {
      // Ignore if already dead
    }
    this.terminal.dispose();
  }
}
