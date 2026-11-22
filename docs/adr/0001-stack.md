# ADR 0001: Desktop, terminal, and persistence stack

**Status:** Accepted (validated via Milestone 0 Spike)  
**Date:** 2026-11-20 (Updated: 2026-11-21)

## Context

The product must be a lightweight, local-first desktop workspace for macOS, native Windows, and Linux. Real interactive terminals and workspace recovery are core requirements. Native Windows support is important; WSL is not a substitute for it. The implementation must keep terminal tab identity separate from the lifetime of a PTY process.

The candidate stack proposed before Milestone 0 consisted of Tauri 2, Svelte 5/TypeScript, Rust, xterm.js, portable-pty, system Git CLI, and SQLite. Milestone 0 implemented an end-to-end vertical slice (terminal lifecycle, full-screen TUIs, resize, multi-tab execution, crash recovery, and stale path reconciliation) to empirically validate this hypothesis.

## Stack Decision

Based on the results recorded in [`docs/milestones/m0-results.md`](../milestones/m0-results.md), the candidate stack is formally **Adopted**:

| Concern | Component | Validation & Status | Decision |
|---|---|---|---|
| Desktop shell | Tauri 2 | Lightweight footprint, fast startup, secure IPC bridges | **Adopted** |
| UI Framework | Svelte 5 + TypeScript | Reactive runes (`$state`, `$derived`, `$effect`), DOM retention (`display: none`) preserves background terminal emulators without recreation | **Adopted** |
| Native services | Rust | High-performance, memory-safe backend for PTY management, SQLite persistence, and Git operations | **Adopted** |
| Terminal emulator | `@xterm/xterm` (5.5) + `@xterm/addon-fit` | Handles ANSI colors, Unicode, mouse events, resize coordination, and interactive TUIs (`top`, `less`, `vim`) | **Adopted** |
| Terminal snapshots (M3) | `@xterm/addon-serialize` 0.13.0 (MIT, peer `@xterm/xterm ^5`) | Serializes the xterm buffer (scrollback and alternate screen) for opt-in visual history on restored tabs. Version 0.14+ targets xterm 6, so it is not used with xterm 5.5. Snapshot files are stored by a Rust module, never in SQLite | **Proposed** (adopted when Task 2 installs it) |
| PTY subsystem | `portable-pty` (0.8) | Cross-platform PTY management, async non-blocking output streaming via background threads | **Adopted** |
| Git operations | System Git CLI | Single source of truth for repository discovery and worktree management without shadow databases | **Adopted** |
| Workspace metadata | SQLite via `rusqlite` (0.32 bundled, WAL mode) | Transactional, crash-tolerant atomic persistence of tabs, layout, registered project references and the selection, with load-time path reconciliation. Amended in M1: worktrees are always read live from Git and are not persisted or cached | **Adopted** |
| Visual scrollback | DOM-retained buffer / Cold restart banner | Running tabs maintain active scrollback buffer; restored cold tabs display a clear restart banner without ghost shells | **Adopted baseline** (bounded historical snapshot evaluation deferred to P1) |

## Consequences and Principles

1. **Git remains authoritative:** Stored application data only stores presentation references and workspace layout, reconciling against actual filesystem and Git state on restore.
2. **Terminal tab vs. PTY process:** Durable UI tabs are decoupleable from live OS processes. Restored tabs start cold with explicit user intent required to launch a new shell.
3. **No daemon or multiplexer dependency:** No external runtime dependencies (tmux, Zellij, background services) required.
4. **Local-first & private:** Metadata is stored exclusively on the user's machine in SQLite (`workspace.db`), with no network telemetry or external API dependencies.

## Revisit when

Revisit stack components if cross-platform CI or native Windows/Linux hardware testing reveals unresolvable driver/PTY fidelity bugs that cannot be addressed within the current architecture.
