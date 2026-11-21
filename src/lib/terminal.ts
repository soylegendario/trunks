import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
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
      fontFamily: 'Menlo, Monaco, "Courier New", monospace',
      theme: {
        background: "#1a1b26",
        foreground: "#c0caf5",
        cursor: "#7aa2f7",
        selectionBackground: "#33467c",
        black: "#15161e",
        red: "#f7768e",
        green: "#9ece6a",
        yellow: "#e0af68",
        blue: "#7aa2f7",
        magenta: "#bb9af7",
        cyan: "#7dcfff",
        white: "#a9b1d6",
        brightBlack: "#414868",
        brightRed: "#f7768e",
        brightGreen: "#9ece6a",
        brightYellow: "#e0af68",
        brightBlue: "#7aa2f7",
        brightMagenta: "#bb9af7",
        brightCyan: "#7dcfff",
        brightWhite: "#c0caf5",
      },
      allowProposedApi: true,
    });

    this.fitAddon = new FitAddon();
    this.terminal.loadAddon(this.fitAddon);

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
      }
    });

    // Listen to PTY exit
    this.unlistenExit = await listen<PtyExitPayload>("pty-exit", (event) => {
      if (event.payload.id === this.id && !this.isDisposed) {
        this.tab.status = "stopped";
        this.onStatusChange?.("stopped");
        this.terminal.writeln("\r\n\x1b[33m[Process completed]\x1b[0m");
      }
    });

    // Spawn the shell process if running
    if (this.tab.status === "running") {
      await this.startPty();
    } else if (this.tab.isRestored) {
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

  public fit() {
    if (this.isDisposed) return;
    try {
      this.fitAddon.fit();
    } catch (err) {
      // Element might be hidden (display: none)
    }
  }

  public async restart() {
    this.terminal.clear();
    await this.startPty();
  }

  public async destroy() {
    this.isDisposed = true;
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
