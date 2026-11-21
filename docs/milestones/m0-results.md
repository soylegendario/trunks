# Milestone 0 (M0 Spike) — Evaluation Results

**Date:** 2026-11-21  
**Status:** In Progress (Tasks 1, 2 & 3 completed; Task 4 next)  
**App Revision:** Milestone 0 Spike (Tauri 2 + Svelte 5 + xterm.js + portable-pty + rusqlite)

---

## 1. Acceptance Criteria Evaluation Matrix

| Criterion | Description | macOS (ARM64) | Windows (Native) | Linux (x86_64) | Notes / Evidence |
|---|---|---|---|---|---|
| **AC-M0-01** | Shell starts in expected worktree PWD | **PASS** | Pending | Pending | Native shell (`/bin/zsh`) starts in specified worktree/directory; verified with paths containing spaces and Unicode (`test_pty_cwd_with_spaces_and_unicode`). |
| **AC-M0-02** | Echo/input, Ctrl-C, ANSI colors & Unicode | **PASS** | Pending | Pending | UTF-8, ANSI color escape sequences, and standard key events handled cleanly through `@xterm/xterm` + `portable-pty`. |
| **AC-M0-03** | Full-screen interactive TUI program | **PASS** | Pending | Pending | Interactive full-screen apps (`top`, `less`, `vim`) run correctly without rendering corruption. |
| **AC-M0-04** | Resize terminal while interactive process is live | **PASS** | Pending | Pending | Panel toggling and window resizing trigger `FitAddon.fit()` and `pty_resize` without restarting or dropping child processes. |
| **AC-M0-05** | Multiple simultaneous tabs in separate worktrees | **PASS** | Pending | Pending | Independent PTY instances run concurrently; hidden tabs stay mounted in DOM (`display: none`), retaining emulator buffer, cursor and state. |
| **AC-M0-06** | Incremental persistence & crash recovery | **PASS** | Pending | Pending | Transactional storage in `SQLite` (`rusqlite`) in WAL mode (`workspace_layout`, `projects`, `worktrees`, `terminal_tabs`). Verified recovery of tab order, active tab, panel state, and directories (`test_tabs_ordering_and_incremental_upsert_delete`, `test_layout_persistence`). |
| **AC-M0-07** | Restored tabs marked as stopped/detached | **PASS** | Pending | Pending | Tabs restored from SQLite are instantiated with `status: "stopped"` and labeled `[restored]`, displaying an informative banner in the terminal emulator. No ghost shell or PTY processes are spawned until explicit user action (`▶ Start Shell` / restart). |
| **AC-M0-08** | Stale/removed worktree detection | **PASS** | Pending | Pending | Startup path reconciliation identifies moved/deleted worktree or project directories, marking them as `is_stale: true` with explanatory reasons while preserving workspace metadata without data loss (`test_projects_and_worktree_persistence_and_stale_reconciliation`). |
| **AC-M0-09** | Cross-platform fidelity matrix consolidation | *In Progress* | *Pending* | *Pending* | macOS validation complete for Tasks 1-3; Windows and Linux runs pending. |

---

## 2. Technical Evidence & Implementation Details (Task 2: PTY & Terminal Lifecycle)

### Rust Native Backend (`src-tauri/src/pty.rs`)
- Employs `portable-pty` (0.8) `native_pty_system()`.
- Threaded non-blocking reader loop streaming output chunks via Tauri events (`pty-output`).
- Explicit process lifecycle commands: `pty_create`, `pty_write`, `pty_resize`, `pty_kill`, `pty_list`.
- Graceful exit detection and cleanup emitting `pty-exit`.

### Frontend Terminal Adapter (`src/lib/terminal.ts`, `src/lib/TerminalView.svelte`)
- Uses `@xterm/xterm` (5.5.0) and `@xterm/addon-fit` (0.10.0).
- DOM retention strategy: inactive tabs remain in DOM with `display: none`, preserving active terminal buffer, history, and scrollback.
- Dynamic resizing: `ResizeObserver` on center panel calls `fitAddon.fit()` and emits resize coordinates to Rust PTY.

---

## 3. Technical Evidence & Implementation Details (Task 3: Persistence & Recovery)

### Rust Native Persistence Backend (`src-tauri/src/db.rs`)
- Embedded SQLite with `rusqlite` (0.32 bundled), configured with `PRAGMA journal_mode = WAL;` and `PRAGMA synchronous = NORMAL;` for atomic, crash-resilient local updates.
- Normalized relational schema:
  - `workspace_layout`: single-row singleton preserving sidebar visibility (`show_left`, `show_right`), active tab ID, and selection.
  - `projects`: registered project IDs, names, and root filesystem paths.
  - `worktrees`: child worktrees foreign-keyed to projects with `ON DELETE CASCADE`.
  - `terminal_tabs`: durable tab identity, tab ordering (`tab_order`), working directory (`cwd`), and project/worktree affiliations.
- Non-destructive path reconciliation at load time: verifies filesystem paths against `std::path::Path::exists()`, populating `is_stale` and `stale_reason` without dropping rows.
- Transactional sync methods: `save_projects`, `save_tabs`, `save_layout`, `upsert_tab`, `delete_tab`.

### Frontend State Recovery (`src/lib/storage.ts`, `src/routes/+page.svelte`)
- Asynchronous workspace hydration on launch (`onMount` -> `getWorkspaceState()`).
- Restored tabs are mounted in cold state (`status: "stopped"`, `isRestored: true`), displaying a clean terminal banner and requiring user intent (`▶ Start`) to spawn a new PTY.
- Stale project and worktree paths are surfaced visually with warning badges (`⚠️ Stale Path`), preventing silent failure.
- Auto-persistence on reactive user interactions (tab switching, reordering, creation, closure, sidebar toggles).

### Automated Tests Passing
- `db::tests::test_schema_init_and_default_layout`: validates default schema creation and initial singleton layout state.
- `db::tests::test_layout_persistence`: validates updating and retrieving sidebar visibility and active tab ID.
- `db::tests::test_projects_and_worktree_persistence_and_stale_reconciliation`: validates persistent projects, worktrees, and non-destructive reconciliation of missing/deleted paths.
- `db::tests::test_tabs_ordering_and_incremental_upsert_delete`: validates tab order retention, incremental updates, and deletions.
- `pty::tests::test_system_info`: verifies host OS and shell resolution.
- `pty::tests::test_pty_lifecycle_direct`: verifies spawn, input write, execution, output capture, and dynamic resize.
- `pty::tests::test_pty_cwd_with_spaces_and_unicode`: verifies spawning in temporary directories containing spaces and non-ASCII characters (`ñ`, accents).

---

## 4. Next Steps (Task 4)
- Consolidate evidence and finalize cross-platform acceptance evaluation matrix (`AC-M0-01` to `AC-M0-09`).
- Update `docs/adr/0001-stack.md` reflecting candidate stack decisions supported by empirical evidence.
