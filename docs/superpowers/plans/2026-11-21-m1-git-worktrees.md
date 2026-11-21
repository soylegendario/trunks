# Milestone 1: Worktree-First Project Workflow & Git Integration Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Multi-Agent Protocol:** Every agent (Junie, Codex, Claude) must follow the protocol in [`AGENTS.md`](../../../AGENTS.md) when starting a session and after completing tasks.

**Goal:** Implement real Git repository registration, worktree discovery/creation/removal via system Git CLI, associate terminal tabs with real worktree directories, and provide a read-only sidecar inspector for Git status and diffs.

**Architecture:** Use system Git CLI as the single source of truth for repository and worktree state. Rust backend handles CLI execution safely and asynchronously. SQLite stores user project references and UI preferences. Frontend uses Svelte 5 runes for reactive state management.

**Tech Stack:** Tauri 2, Rust (tokio / process commands), Svelte 5 / TypeScript, `@xterm/xterm`, SQLite (`rusqlite`), system `git` CLI.

**Spec:** `docs/spec.md` (FR-01, FR-02, FR-03, FR-04, FR-07, FR-08) and `docs/milestones/m1-git-worktrees.md`.

---

## Global Constraints

- **Git as Single Source of Truth:** Never create a secondary shadow database for Git commits, branches, or worktrees. Always query Git CLI.
- **Cross-Platform Path Handling:** Paths must be normalized cleanly across macOS, Linux, and Windows (handling spaces, slashes, and Unicode characters).
- **Non-Blocking UI:** All Git commands in Rust must run without blocking the main/UI thread.
- **Safe Worktree Operations:** Deleting or removing worktrees must be guarded to prevent data loss.

---

### Task 1: Native Git Backend Service (Rust)

**Files:**
- Create: `src-tauri/src/git.rs` (Git CLI wrapper and parser)
- Modify: `src-tauri/src/lib.rs` (register new Tauri commands)
- Create: `src-tauri/tests/git_tests.rs` or unit tests in `git.rs`

**Interfaces:**
- Tauri Commands:
  - `git_validate_repo(path: String) -> Result<GitRepoInfo, String>`
  - `git_list_worktrees(repo_path: String) -> Result<Vec<GitWorktreeInfo>, String>`
  - `git_list_branches(repo_path: String) -> Result<Vec<GitBranchInfo>, String>`
  - `git_create_worktree(repo_path: String, worktree_path: String, branch: String, new_branch: bool) -> Result<GitWorktreeInfo, String>`
  - `git_remove_worktree(repo_path: String, worktree_path: String, force: bool) -> Result<(), String>`
  - `git_get_status(worktree_path: String) -> Result<GitStatusResult, String>`
  - `git_get_diff(worktree_path: String, file_path: Option<String>, cached: bool) -> Result<String, String>`
  - `git_list_files(worktree_path: String, subpath: Option<String>) -> Result<Vec<GitFileInfo>, String>`

- [ ] Implement `git_validate_repo` to check if a directory is a valid Git repository root using `git rev-parse --show-toplevel`.
- [ ] Implement `git_list_worktrees` parsing `git worktree list --porcelain` into structured worktree structs (path, HEAD commit, branch, bare, locked/prunable).
- [ ] Implement `git_list_branches` using `git branch --list --all --format=...`.
- [ ] Implement `git_create_worktree` wrapping `git worktree add` with path and branch parameters.
- [ ] Implement `git_remove_worktree` wrapping `git worktree remove` with safety checks.
- [ ] Implement `git_get_status` using `git status --porcelain=v2` to retrieve staged, unstaged, untracked changes.
- [ ] Implement `git_get_diff` and `git_list_files` for read-only inspection.
- [ ] Add unit tests in Rust covering parsing of `git worktree list --porcelain` and `git status --porcelain=v2`.

---

### Task 2: Project Registration & Worktree Management UI (Frontend)

**Files:**
- Create: `src/lib/git.ts` (TypeScript client calling Tauri Git commands)
- Modify: `src/routes/+page.svelte` (replace mock projects with real Git project management)
- Create: `src/lib/components/AddProjectModal.svelte` (or dialog for adding repo path / folder picker)
- Create: `src/lib/components/CreateWorktreeModal.svelte` (dialog for branch selection, new branch name, destination directory)
- Create: `src/lib/components/WorktreeList.svelte` (left sidebar tree view)

**Interfaces:**
- Consumes: Tauri Git commands and SQLite project store.
- Produces: Dynamic sidebar displaying real repositories, active worktrees, branches, and quick action buttons.

- [ ] Implement `src/lib/git.ts` wrapping Tauri IPC calls with TypeScript interfaces.
- [ ] Replace static project fixture in `+page.svelte` with dynamic loading from SQLite + live Git discovery.
- [ ] Implement "Add Repository" UI allowing the user to select or enter a local Git folder, validating via `git_validate_repo`, saving to SQLite and refreshing worktrees.
- [ ] Implement "Remove Project" UI (removes reference from Trunks workspace without deleting repository files on disk).
- [ ] Implement "Add Worktree" modal allowing selection of existing branches or creating a new branch and specifying destination path.
- [ ] Implement "Remove Worktree" with confirmation modal and error surfacing if uncommitted changes exist.
- [ ] Add manual "Refresh" button and automatic refresh when window regains focus.

---

### Task 3: Terminal Spawning & Worktree Context Synchronization

**Files:**
- Modify: `src/lib/terminal.ts` and `src/routes/+page.svelte`
- Modify: `src-tauri/src/db.rs`

**Interfaces:**
- Consumes: Real worktree paths from Task 2.
- Produces: Terminal tabs whose PTY shells launch in the exact worktree directory and retain association in SQLite.

- [ ] Connect worktree selection to terminal tabs: clicking a worktree opens or focuses a terminal tab in that worktree's path.
- [ ] Update tab creation to automatically populate tab metadata (`projectName`, `worktreeName`, `cwd`) from the active worktree.
- [ ] Verify that spawning new tabs launches the shell directly in the worktree PWD (`pwd` verified).
- [ ] Test cold restart: verify saved tabs retain their real project/worktree paths and restart cleanly on user action.

---

### Task 4: Read-Only Sidecar Inspector (File Tree, Git Status & Diff Viewer)

**Files:**
- Create: `src/lib/components/InspectorPanel.svelte`
- Create: `src/lib/components/FileTree.svelte`
- Create: `src/lib/components/GitStatusView.svelte`
- Create: `src/lib/components/DiffViewer.svelte`
- Modify: `src/routes/+page.svelte` (Right sidebar integration)

**Interfaces:**
- Consumes: `git_get_status`, `git_get_diff`, `git_list_files` from active worktree.
- Produces: Interactive inspector showing changed files and syntax-highlighted diffs.

- [ ] Build the inspector tab switcher in the right panel (File Explorer / Git Status).
- [ ] Implement `GitStatusView` displaying modified, staged, and untracked files with status indicators.
- [ ] Implement `DiffViewer` rendering unified diffs with line additions/deletions highlighting for selected files.
- [ ] Implement lightweight `FileTree` browsing the active worktree directory structure.
- [ ] Ensure polling / refresh updates the status when commands run in the terminal.

---

### Task 5: End-to-End Validation & Documentation Handoff

**Files:**
- Create: `docs/milestones/m1-results.md`
- Modify: `README.md` and `AGENTS.md`

**Interfaces:**
- Consumes: Fully functional Milestone 1 workflow.
- Produces: Validated criteria (`AC-M1-01` to `AC-M1-08`) and clean documentation handoff.

- [ ] Create a disposable test Git repository with multiple branches and worktrees to execute manual & automated checks.
- [ ] Validate `AC-M1-01` through `AC-M1-08` on macOS.
- [ ] Run full build and checks: `npm run check`, `npm run build`, `cargo test`.
- [ ] Record results in `docs/milestones/m1-results.md`.
- [ ] Update `README.md` and `AGENTS.md` to reflect Milestone 1 completion.
