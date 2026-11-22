![Trunks — Your workspace. Your flow.](docs/assets/trunks_concept.png)

# Trunks — Terminal Workspace

**A lightweight, local-first desktop workspace for Git projects, worktrees, and interactive terminal sessions.**

Trunks is built for developers who live in the terminal and frequently juggle multiple branches, worktrees, and long-running processes. Instead of fighting heavy, slow IDEs or getting lost in disconnected terminal windows, Trunks organizes real interactive terminal sessions around Git worktrees with crash-resilient persistence and instant read-only inspection.

---

## Key Features

- 🌿 **Worktree-First Organization**: Register local Git repositories and effortlessly navigate across multiple worktrees. Create and delete worktrees directly from the UI with safety guards for uncommitted changes.
- 💻 **Native Interactive Terminals**: High-fidelity terminal emulation powered by `portable-pty` and `@xterm/xterm`. Full support for full-screen interactive TUIs (`vim`, `top`, `htop`, `less`, `lazygit`), dynamic resizing, ANSI 256/true color, and Unicode.
- 📑 **Durable Tab Identity**: Tab identity is decoupled from OS processes. Each worktree maintains its own set of terminal tabs. Hidden terminals continue running in the background without UI overhead.
- 🛡️ **Crash Resilience & Zero Ghost Shells**: Workspace layout and tab metadata persist transactionally in SQLite in Write-Ahead Logging (WAL) mode. When reopened after a reboot or crash, tabs restore gracefully labeled `[restored]`—preventing runaway background processes.
- 🔍 **Integrated Read-Only Inspector**:
  - Live Git status with collapsible directory trees (staged, modified, and untracked changes).
  - Unified diff viewer with clear syntax coloring.
  - Lazy file tree browser for quick directory navigation.
  - Open files and diffs side-by-side with terminals in central read-only tabs.
- ⚡ **Streamlined Ergonomics**: Resizable side panels with quick keyboard shortcuts (`Cmd+B` / `Ctrl+B` for Projects, `Cmd+L` / `Ctrl+L` for Inspector).
- 🤖 **Agent & CLI-Friendly by Design**: Run any CLI tool or autonomous AI agent (such as Claude Code, Codex, Aider) directly in standard PTY shells. Zero proprietary AI lock-in, zero cloud dependencies, and zero prompt interception.

---

## Technology Stack

| Component | Technology | Description |
|---|---|---|
| **Desktop Shell** | [Tauri 2](https://tauri.app/) (Rust) | Minimal-overhead native windowing, security sandbox, and system integration |
| **Frontend UI** | [Svelte 5](https://svelte.dev/) (Runes) + TypeScript | Reactive, zero-virtual-DOM user interface with compile-time reactivity |
| **Bundler & Tooling** | [Vite](https://vite.dev/) | Instant hot module replacement and optimized production builds |
| **Terminal Emulator** | [`@xterm/xterm`](https://xtermjs.org/) + `@xterm/addon-fit` | Canvas-accelerated terminal emulator with dynamic geometry fitting |
| **PTY Engine** | [`portable-pty`](https://crates.io/crates/portable-pty) | Cross-platform pseudo-terminal management (Unix PTYs and Windows ConPTY) |
| **Persistence** | [SQLite](https://sqlite.org/) via [`rusqlite`](https://crates.io/crates/rusqlite) | Transactional local storage in WAL mode for workspace layout and tab state |
| **Git Integration** | System Git CLI | Live discovery and operations directly against the Git CLI (no shadow database) |

---

## Architecture & Core Principles

Trunks adheres to four non-negotiable architectural principles:

1. **Not an IDE**: Trunks does not include LSPs, debuggers, or code editors. It is intentionally designed as an uncluttered workspace for terminals and fast inspection.
2. **Git is the Single Source of Truth**: The application maintains no secondary shadow database for Git state. Worktree lists, branches, status, and diffs are queried live from the system Git CLI.
3. **Tab Identity Decoupled from Process Lifecycle**: A terminal tab represents a persistent UI entity attached to a project and directory. The underlying OS process is ephemeral and clean restarts never produce orphaned background processes.
4. **Tool and AI Agnostic**: The application executes locally and does not orchestrate AI models or vendor APIs. External CLI agents run inside native PTY shells like any standard developer tool.

---

## Getting Started

### Prerequisites

Ensure you have the following installed on your machine:

- **Git CLI**: Version 2.28+ recommended (must support `git status --porcelain=v2`).
- **Node.js**: Version 20+ (recommended: Node 22 LTS) and `npm`.
- **Rust Toolchain**: Stable Rust installed via [`rustup`](https://rustup.rs/).
- **Platform Build Tools**:
  - **macOS**: Xcode Command Line Tools (`xcode-select --install`).
  - **Linux**: Standard build dependencies (`build-essential`, `libwebkit2gtk-4.1-dev`, `libssl-dev`, `curl`, `wget`).
  - **Windows**: Microsoft Visual Studio C++ Build Tools.

### Installation & Development

1. Clone the repository:
   ```bash
   git clone https://github.com/rafaelleonpastor/trunks.git
   cd trunks
   ```

2. Install frontend dependencies:
   ```bash
   npm install
   ```

3. Launch Trunks in development mode:
   ```bash
   npm run tauri dev
   ```

### Building for Production

To compile a native desktop binary for your current operating system:

```bash
npm run tauri build
```

Bundled installers and executables will be output to `src-tauri/target/release/bundle/`.

---

## Testing & Verification

Trunks maintains comprehensive verification across both frontend and native layers:

```bash
# Frontend static analysis and type checking
npm run check

# Frontend production compilation test
npm run build

# Frontend unit tests (Node.js native test runner)
npm run test:unit

# Rust backend checks
cargo check --manifest-path src-tauri/Cargo.toml

# Rust unit and integration tests (drives live Git commands)
cargo test --manifest-path src-tauri/Cargo.toml
```

---

## Usage Guide

### 1. Registering a Project
- Click **+ Add Project** in the left sidebar.
- Enter the absolute path to any local Git repository (or a subfolder within it).
- Trunks normalizes the path to the repository root and queries all existing worktrees.

### 2. Managing Worktrees
- Worktrees associated with the active repository appear listed in the sidebar with branch names and status badges (e.g., `primary`, `detached`, `locked`, `stale`).
- Click **+ New Worktree** to checkout an existing branch or create a new branch in a dedicated directory.
- Click the **Trash** icon to remove a worktree. Trunks prevents accidental deletion: dirty worktrees with uncommitted files require explicit force confirmation.

### 3. Running Terminals
- Click the terminal icon (`>_`) next to any worktree to open or focus a terminal session initialized in that worktree's directory.
- Worktrees with active terminal sessions display a green status dot; worktrees with restored (stopped) sessions display an orange dot.
- Open multiple terminal tabs within the same worktree using the `+` button in the terminal header.

### 4. Inspecting Git Status & Diffs
- The right-side **Inspector** updates periodically with the current worktree's status.
- Files with changes are organized in collapsible directory trees.
- Click any file to open a read-only tab in the center area:
  - Modified/staged files open in the **Diff Viewer** showing unified diffs.
  - Regular files open in the **File Viewer** for instant reference.

### Keyboard Shortcuts

| Shortcut (macOS) | Shortcut (Win/Linux) | Action |
|---|---|---|
| `Cmd + B` | `Ctrl + B` | Toggle left sidebar (Projects & Worktrees) |
| `Cmd + L` | `Ctrl + L` | Toggle right sidebar (Inspector: Git & Files) |

---

## Project Roadmap & Documentation

- **[Milestone 0: Terminal & Workspace Spike](docs/milestones/m0-terminal-workspace-spike.md)**: **Completed.** Core architecture validated (`docs/milestones/m0-results.md`).
- **[Milestone 1: Worktree-First Workflow & Git Integration](docs/milestones/m1-git-worktrees.md)**: **Completed.** Native Git service, worktree UI, terminal tab binding, and read-only inspector delivered (`docs/milestones/m1-results.md`).
- **Milestone 2: Terminal Visual History & Extended Workspaces**: Upcoming.

For deeper technical insight, consult:
- [`docs/intent.md`](docs/intent.md): Core vision, user scenarios, and product boundaries.
- [`docs/spec.md`](docs/spec.md): Comprehensive functional and non-functional specifications.
- [`docs/adr/0001-stack.md`](docs/adr/0001-stack.md): Architectural decision record for the core stack.

---

## Multi-Agent Collaboration

This project is developed asynchronously and collaboratively across human developers and AI coding agents. All agents and contributors follow the operating protocol defined in:

👉 **[`AGENTS.md`](AGENTS.md)** — *Operating guidelines, session start/end protocols, and roadmap state.*

---

## License

Distributed under the MIT License. See [`package.json`](package.json) for details.
