import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { Unicode11Addon } from "@xterm/addon-unicode11";
import { SerializeAddon } from "@xterm/addon-serialize";
import { CanvasAddon } from "@xterm/addon-canvas";
import {
  isSnapshotsEnabledLocal,
  snapshotLoad,
  snapshotSave,
} from "./snapshots";
import {
  CaptureController,
  newShellPrelude,
  restoreSnapshotGuarded,
  shouldRestoreSnapshot,
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
  private webgl: CanvasAddon | null = null;
  private repaintTimer: ReturnType<typeof setTimeout> | null = null;
  private repaintUntil = 0;
  private serializeAddon = new SerializeAddon();
  // Capture scheduling lives in a pure controller. Its `dirty` flag is only set by live output,
  // never by restoring a snapshot, so an untouched restored tab is not re-saved with the old
  // separator baked into its content.
  private capture: CaptureController;
  private restorePromise: Promise<void> | null = null;
  private restoredAltScreen = false;
  private showedSnapshot = false;

  constructor(
    tab: TerminalTab,
    onStatusChange?: (status: "running" | "stopped") => void
  ) {
    this.id = tab.id;
    this.tab = tab;
    this.capture = new CaptureController({
      isEnabled: isSnapshotsEnabledLocal,
      capture: () => this.performCapture(),
      now: () => Date.now(),
      schedule: (fn, ms) => setTimeout(fn, ms),
      cancel: (h) => clearTimeout(h as ReturnType<typeof setTimeout>),
    });
    this.onStatusChange = onStatusChange;

    this.terminal = new Terminal({
      cursorBlink: true,
      cursorStyle: "bar",
      fontSize: 13,
      fontFamily:
        '"SF Mono", Monaco, "Courier New", monospace, "Apple Symbols", "Apple Color Emoji"',
      customGlyphs: true,
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

    // Shift+Enter inserts a newline (LF, like Ctrl+J) instead of submitting with CR
    this.terminal.attachCustomKeyEventHandler((e) => {
      if (e.key === "Enter" && e.shiftKey && !e.ctrlKey && !e.altKey && !e.metaKey) {
        if (e.type === "keydown" && this.tab.status === "running" && !this.isDisposed) {
          invoke("pty_write", { id: this.id, data: "\n" }).catch((err) => {
            console.error(`Failed to write to PTY ${this.id}:`, err);
          });
        }
        return false;
      }
      return true;
    });

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
    // Canvas renderer draws box-drawing/block glyphs itself (customGlyphs) so k9s lines join,
    // without the WebGL glyph-atlas corruption seen on resize with Claude Code.
    try {
      const canvas = new CanvasAddon();
      this.terminal.loadAddon(canvas);
      this.webgl = canvas;
    } catch (err) {
      console.warn("Canvas renderer unavailable, using DOM renderer:", err);
    }
    this.fit();

    // Listen to PTY output
    this.unlistenOutput = await listen<PtyOutputPayload>("pty-output", (event) => {
      if (event.payload.id === this.id && !this.isDisposed) {
        this.terminal.write(event.payload.data, () => this.scheduleRepaintAfterResize());
        this.capture.markOutput();
      }
    });

    // Listen to PTY exit
    this.unlistenExit = await listen<PtyExitPayload>("pty-exit", (event) => {
      if (event.payload.id === this.id && !this.isDisposed) {
        this.tab.status = "stopped";
        this.onStatusChange?.("stopped");
        // Capture once the parser has consumed the final output and the exit notice.
        this.terminal.write("\r\n\x1b[33m[Process completed]\x1b[0m\r\n", () => {
          this.capture.markDirtyNow();
          void this.capture.capture();
        });
      }
    });

    // Spawn the shell process if running
    if (this.tab.status === "running") {
      await this.startPty();
    } else if (this.tab.isRestored) {
      this.restorePromise = this.restoreSnapshot();
      await this.restorePromise;
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
      // A snapshot still loading must land (or be skipped) before the shell starts, never after.
      if (this.restorePromise) await this.restorePromise;
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
    const restorable = () =>
      !this.isDisposed &&
      shouldRestoreSnapshot({
        enabled: isSnapshotsEnabledLocal(),
        isRestored: !!this.tab.isRestored,
        status: this.tab.status,
      });
    if (!restorable()) return;
    const result = await restoreSnapshotGuarded({
      load: () => snapshotLoad(this.id),
      // Re-checked after the load: the user may have clicked Start meanwhile.
      stillRestorable: restorable,
      write: (seq) => new Promise<void>((resolve) => this.terminal.write(seq, resolve)),
    });
    if (result.restored) {
      this.showedSnapshot = true;
      this.restoredAltScreen = result.hasAltScreen;
    }
  }

  /** Serializes the buffer and stores it. Rejects on failure; the controller backs off. */
  private async performCapture(): Promise<void> {
    const t0 = performance.now();
    // Modes are meaningless for a dead snapshot and would be replayed into the next shell.
    const data = this.serializeAddon.serialize({ excludeModes: true });
    const t1 = performance.now();
    try {
      await snapshotSave(this.id, this.terminal.cols, this.terminal.rows, data);
    } catch (err) {
      console.warn(`Failed to capture snapshot for tab ${this.id}:`, String(err));
      throw err;
    }
    // Content-free timing for the manual measurements (docs/milestones/m3-results.md).
    console.debug(
      `[snapshot] tab=${this.id} serialize=${(t1 - t0).toFixed(1)}ms chars=${data.length} save=${(performance.now() - t1).toFixed(1)}ms`
    );
  }

  /** Waits for any in-flight capture and stores the newest state (window close). */
  public flushSnapshot(): Promise<void> {
    return this.capture.flush();
  }

  public fit() {
    if (this.isDisposed) return;
    try {
      // Skip when the grid would not change: avoids redundant SIGWINCH/reflow.
      const dims = this.fitAddon.proposeDimensions();
      if (!dims || Number.isNaN(dims.cols) || Number.isNaN(dims.rows)) {
        return;
      }
      if (dims.cols !== this.terminal.cols || dims.rows !== this.terminal.rows) {
        this.fitAddon.fit();
        // The TUI redraws asynchronously after SIGWINCH; repaint again once that output settles.
        this.repaintUntil = Date.now() + 2000;
      }
      // Also on unchanged size: fit() runs when a tab becomes visible again, and the WebGL
      // canvas/glyph atlas can be corrupt after being hidden (garbled text until selection).
      this.repaint();
    } catch (err) {
      // Element might be hidden (display: none)
    }
  }

  /** Debounced repaint for output that arrives shortly after a resize (TUI redraw). */
  private scheduleRepaintAfterResize() {
    if (this.isDisposed || Date.now() > this.repaintUntil) return;
    if (this.repaintTimer) clearTimeout(this.repaintTimer);
    this.repaintTimer = setTimeout(() => {
      this.repaintTimer = null;
      if (!this.isDisposed) this.repaint();
    }, 120);
  }

  /** Rebuilds the WebGL glyph atlas and repaints every row. */
  private repaint() {
    this.webgl?.clearTextureAtlas();
    this.terminal.refresh(0, this.terminal.rows - 1);
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
    // Wait for an in-flight save so the caller's snapshot delete cannot be overtaken by it.
    await this.capture.dispose();
    if (this.fitTimer) {
      clearTimeout(this.fitTimer);
      this.fitTimer = null;
    }
    if (this.repaintTimer) {
      clearTimeout(this.repaintTimer);
      this.repaintTimer = null;
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
