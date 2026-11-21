# Milestone 1 — Worktree-First Project Workflow & Real Git Integration

**Status:** Ready to Implement  
**Goal:** Transition from simulated fixtures to a full local-first Git workflow: register real repositories, discover and manage worktrees via the system Git CLI, associate interactive terminals with actual worktree directories, and inspect read-only file/diff status.

---

## 1. Scope & Deliverables

Milestone 1 implements the foundational **P0** functional requirements and the first **P1** inspection slice from [`docs/spec.md`](../spec.md):

1. **Local Git Project Registration (`FR-01`):**
   - Add local Git repositories using native directory picker or path input.
   - Validate repository validity via `git rev-parse --is-inside-work-tree` / `git rev-parse --show-toplevel`.
   - Store canonical repository paths and display names in SQLite.
   - Remove projects from workspace without touching repository files on disk.

2. **Worktree Discovery & Management (`FR-02`):**
   - Query worktrees directly via `git worktree list --porcelain`.
   - Display worktree paths, active branches, HEAD commits, and locked/detached states.
   - Create new worktrees (`git worktree add`) with branch selection or new branch creation.
   - Explicit, guarded worktree removal (`git worktree remove`) preventing accidental data loss.
   - Reconcile external filesystem and Git changes upon manual or automatic refresh.

3. **Terminal Association with Live Worktrees (`FR-03`, `FR-04`):**
   - Spawning terminal tabs directly initializes the PTY working directory to the target worktree's filesystem path.
   - Restored tabs remember their worktree binding and allow starting a fresh shell in the verified directory.

4. **Read-only Inspector Panel (`FR-07`, `FR-08`):**
   - Inspect files within the active worktree (directory tree navigation without blocking the UI thread).
   - Display staged, unstaged, and untracked file status via `git status --porcelain=v2`.
   - Provide a lightweight, read-only diff viewer (`git diff` and `git diff --cached`).

---

## 2. Acceptance Criteria (AC-M1)

- **AC-M1-01 (Project Registration):** Adding a valid Git repository displays it in the left sidebar with its name and worktrees; attempting to add a non-Git directory surfaces a clear error.
- **AC-M1-02 (Worktree Discovery):** All existing worktrees of a registered project are discovered and listed with their correct branch names and filesystem paths.
- **AC-M1-03 (Worktree Creation):** Creating a worktree from the UI executes `git worktree add`, creates the directory, checks out/creates the branch, and automatically appears in the sidebar.
- **AC-M1-04 (Terminal Spawning in Worktree):** Opening a terminal tab for a worktree starts the shell process with `cwd` set to that worktree path (`pwd` matches).
- **AC-M1-05 (External Worktree Synchronization):** Adding or removing worktrees outside Trunks via CLI is reflected in the UI upon refresh without corrupting SQLite metadata.
- **AC-M1-06 (Guarded Worktree Removal):** Removing a worktree requires confirmation; if uncommitted changes exist, Git blocks removal unless explicitly confirmed or clean.
- **AC-M1-07 (Git Status & Inspection):** Modifying a file in a worktree terminal tab shows up in the right inspector panel under Git Status (modified/untracked).
- **AC-M1-08 (Diff Viewing):** Selecting a modified file in the right inspector renders the unified diff cleanly and without lag.

---

## 3. Architecture & Boundaries

- **Rust Git Backend (`src-tauri/src/git.rs`):** Interacts directly with the system `git` CLI asynchronously using `tokio::process::Command` or `std::process::Command`. No shadow Git database.
- **SQLite Persistence (`src-tauri/src/db.rs`):** Stores registered repository locations, display preferences, tab identities, and layout. Reconciles with live Git state on launch.
- **Frontend Svelte Store / State (`src/lib/git.ts`, `src/lib/projects.svelte.ts`):** Reactive runes managing project lists, selected worktree, inspection tree, and diff viewing.
- **Terminal Adapter (`src/lib/terminal.ts`):** Spawns PTYs linked to live worktree paths.
