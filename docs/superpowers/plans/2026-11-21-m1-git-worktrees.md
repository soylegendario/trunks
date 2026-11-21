# Milestone 1: Worktree-First Project Workflow & Git Integration Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Multi-Agent Protocol:** Every agent (Junie, Codex, Claude) must follow the protocol in [`AGENTS.md`](../../../AGENTS.md) when starting a session and after completing tasks.

**Goal:** Implement real Git repository registration, worktree discovery/creation/removal via system Git CLI, associate terminal tabs with real worktree directories, and provide a read-only sidecar inspector for Git status and diffs.

**Architecture:** Use system Git CLI as the single source of truth for repository and worktree state. Rust backend handles CLI execution safely and asynchronously. SQLite stores user project references and UI preferences. Frontend uses Svelte 5 runes for reactive state management.

**Tech Stack:** Tauri 2, Rust (tokio / process commands), Svelte 5 / TypeScript, `@xterm/xterm`, SQLite (`rusqlite`), system `git` CLI.

**Spec:** `docs/superpowers/specs/2026-11-21-m1-git-backend-design.md`, `docs/spec.md` (FR-01, FR-02, FR-03, FR-04, FR-07, FR-08), and `docs/milestones/m1-git-worktrees.md`.

---

## Global Constraints

- **Git as Single Source of Truth:** Never create a secondary shadow database for Git commits, branches, or worktrees. Always query Git CLI.
- **Cross-Platform Path Handling:** Paths must be normalized cleanly across macOS, Linux, and Windows (handling spaces, slashes, and Unicode characters).
- **Non-Blocking UI:** All Git commands in Rust must run without blocking the main/UI thread.
- **Safe Worktree Operations:** Deleting or removing worktrees must be guarded to prevent data loss.
- **Process Execution:** Invoke `git` with `tokio::process::Command`, pass arguments separately, set `GIT_TERMINAL_PROMPT=0`, and do not impose arbitrary command timeouts in M1.
- **Inspection Paths:** Normalize repository paths to the Git root. Inspection paths are relative to and contained within a worktree; file browsing is one level at a time and does not follow symlinks.
- **Removal Guard:** Normal worktree removal omits `--force`; pass it only after explicit UI confirmation. The backend rejects removal of the main worktree.

## Review Focus

- Paths containing spaces or Unicode must survive command invocation and output parsing — `parses_worktree_porcelain_records`, `parses_status_porcelain_v2_z_entries`, and relative-path guard tests in Task 1.
- Detached, bare, locked, and prunable worktrees must retain their states — `parses_worktree_porcelain_records` in Task 1.
- Rename/copy status records must preserve both NUL-delimited paths — `parses_status_porcelain_v2_z_entries` in Task 1.
- The main worktree and unlisted paths must be rejected for removal, while dirty worktrees remain protected without force — removal guard/argument tests in Task 1 and `AC-M1-06` in Task 5.
- Untracked text files must display all-added diffs; binary files must explain the limitation; symlinks must not be traversed — diff and path guard tests in Task 1.

---

### Task 1: Native Git Backend Service (Rust)

**Files:**
- Create: `src-tauri/src/git.rs` (Git CLI wrapper and parser)
- Modify: `src-tauri/src/lib.rs` (register new Tauri commands)
- Test: unit tests in `src-tauri/src/git.rs` (pure parsers and path guards)

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

- Serialized result types in `git.rs`:
  - Derive `serde::Serialize`, `Clone`, and `Debug` for IPC results; retain snake_case struct field names to match existing `db.rs` payloads.
  - `GitRepoInfo { path: String, name: String }`
  - `GitWorktreeInfo { path: String, head: Option<String>, branch: Option<String>, bare: bool, locked: Option<String>, prunable: Option<String> }`
  - `GitBranchInfo { name: String, is_remote: bool, is_current: bool, commit: String }`; `is_current` means the branch pointed to by `HEAD` in the registered repository root.
  - `GitStatusResult { entries: Vec<GitStatusEntry> }`
  - `GitStatusEntry { path: String, index_status: char, worktree_status: char, kind: GitStatusKind, original_path: Option<String> }`, with `GitStatusKind::{Tracked, Untracked, Conflicted}` serialized as `tracked`, `untracked`, or `conflicted`.
  - `GitFileInfo { name: String, path: String, is_dir: bool, is_symlink: bool }`

- [x] Add failing unit test `parses_worktree_porcelain_records` for the main worktree, a branch worktree, a detached worktree, a bare record, locked reason, prunable reason, and paths containing spaces/Unicode.
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml parses_worktree_porcelain_records`; confirm it fails because the parser is not implemented.
- [x] Define the serialized result types above and implement pure `parse_worktree_list(&str) -> Result<Vec<GitWorktreeInfo>, String>` in `git.rs`; parse blank-line-delimited records and preserve optional fields.
- [x] Run the focused worktree parser test; confirm all fields and edge cases pass.
- [x] Add failing unit test `parses_status_porcelain_v2_z_entries` covering ordinary staged/unstaged entries, untracked entries, conflicts, a rename/copy with its second NUL-delimited path, and spaces/Unicode.
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml parses_status_porcelain_v2_z_entries`; confirm it fails before implementing the parser.
- [x] Implement `parse_status_porcelain_v2_z(&[u8]) -> Result<GitStatusResult, String>` and preserve the raw index/worktree status characters, entry kind, new path, and optional original path.
- [x] Run the focused status parser test; confirm all fixture entries and both rename paths pass.
- [x] Add failing unit tests for worktree-relative path resolution: accept a contained relative directory, reject absolute paths and `..` traversal, and refuse to traverse a symlink.
- [x] Run the focused path guard tests; confirm rejection cases fail before implementation.
- [x] Implement shared async Git execution helpers using `tokio::process::Command`; pass args separately, set `GIT_TERMINAL_PROMPT=0`, capture stdout/stderr, add operation context to non-zero errors, and do not add timeouts.
- [x] Implement async `git_validate_repo(path: String) -> Result<GitRepoInfo, String>` using `git rev-parse --show-toplevel`; canonicalize and return the Git root even when `path` is nested inside it.
- [x] Implement async `git_list_worktrees(repo_path: String) -> Result<Vec<GitWorktreeInfo>, String>` using `git worktree list --porcelain` and the tested parser.
- [x] Add failing unit test `parses_branch_ref_records` for local, remote, current, and symbolic remote HEAD refs.
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml parses_branch_ref_records`; confirm it fails because the parser is not implemented.
- [x] Implement pure `parse_branch_refs(&str) -> Result<Vec<GitBranchInfo>, String>` for records from `git for-each-ref --format=%(refname)%00%(objectname)%00%(HEAD)%00%(symref) refs/heads refs/remotes`; strip `refs/heads/` or `refs/remotes/`, set `is_remote`, set `is_current` from `%(HEAD)` for the registered repository root, and omit records with a non-empty `%(symref)`.
- [x] Implement async `git_list_branches(repo_path: String) -> Result<Vec<GitBranchInfo>, String>` by invoking the exact `for-each-ref` format above and applying the tested parser.
- [x] Run the focused branch parser test; confirm local/remote/current fields are parsed and symbolic remote HEAD aliases are omitted.
- [x] Implement async `git_create_worktree(repo_path: String, worktree_path: String, branch: String, new_branch: bool) -> Result<GitWorktreeInfo, String>`; use `git worktree add -b <branch> <path>` for new branches and `git worktree add <path> <branch>` for existing branches, then return the matching refreshed worktree record.
- [x] Add failing unit tests `rejects_main_and_unlisted_worktree_removal` and `removal_args_respect_force_confirmation`: reject the main worktree and paths not present in Git's worktree list; omit `--force` for false and include it for true.
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml removal_`; confirm both tests fail before the guard and argument helper exist.
- [x] Implement async `git_remove_worktree(repo_path: String, worktree_path: String, force: bool) -> Result<(), String>`; require a listed non-main worktree, omit `--force` when false, and append `--force` only when true. Leave dirty-tree refusal to Git.
- [x] Implement async `git_get_status(worktree_path: String) -> Result<GitStatusResult, String>` using `git status --porcelain=v2 -z`; if the installed Git rejects porcelain v2, return a clear upgrade error without a fallback parser.
- [x] Add failing unit tests for untracked diff generation; assert a UTF-8 text file produces a unified diff with all lines added, and a binary file returns a clear no-text-diff error.
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml untracked_diff`; confirm both cases fail before diff generation is implemented.
- [x] Implement async `git_get_diff(worktree_path: String, file_path: Option<String>, cached: bool) -> Result<String, String>` by invoking `git --literal-pathspecs diff --no-ext-diff --no-color [--cached] -- <relative-path>` (omit `--cached` when false and omit the path when `file_path` is `None`). For an untracked text file in the non-cached view, synthesize a unified new-file diff with all lines added; for a binary file, return a clear no-text-diff message.
- [x] Run the focused untracked diff tests; confirm all text lines are added and binary content is not decoded as text.
- [x] Add failing unit test `lists_one_level_and_marks_symlinks_without_following` for one-level results, hidden files, `.git` exclusion, and symlink entries; add a missing-directory error assertion.
- [x] Run `cargo test --manifest-path src-tauri/Cargo.toml lists_one_level`; confirm the file-list test fails before implementation.
- [x] Implement async `git_list_files(worktree_path: String, subpath: Option<String>) -> Result<Vec<GitFileInfo>, String>` using one-level `read_dir`; hide `.git`, include other hidden entries, list symlinks without following them, sort by name, and fail the whole request with context on missing/unreadable directories.
- [x] Run the focused file-list and path-guard tests; confirm directory results are shallow, sorted, and symlinks are never traversed.
- [x] Register all eight commands in `src-tauri/src/lib.rs` and confirm command names and signatures match the interfaces above.
- [x] Run focused Rust tests for all parser and path-guard cases; confirm they pass.
- [x] Run `cargo check --manifest-path src-tauri/Cargo.toml`; confirm the native backend compiles.

**Task 1 implementation notes (completed 2026-11-21; 43 Rust tests pass, `cargo check` clean):**
- Deviations from the plan text, all covered by tests:
  - Added integration tests that drive the real `git` binary against temp repositories (validate, list, create, remove, status, diff, files). Git must be on `PATH` to run `cargo test`.
  - `git_create_worktree` requires an absolute destination and rejects empty or `-`-prefixed branch names; `--` precedes the path.
  - `git_remove_worktree` compares canonicalised paths (macOS `/var` vs `/private/var`) and removes using the path Git reports, not the caller's spelling.
  - Status and diff run with `--no-optional-locks` so the inspector's polling does not contend for the index lock.
  - `git_get_diff`: untracked files above 1 MiB or with a NUL in the first 8 KiB return `Err` ("too large" / "binary file"); synthetic headers always say mode `100644`.
  - `git_list_files` sorts by name in byte order (not case-insensitive) and hides `.git` at any depth.
  - Status omits ignored (`!`) records and headers (`#`).
- Known gaps: the "upgrade Git" message for `--porcelain=v2` is only unit-tested against an assumed Git wording; symlink tests are `#[cfg(unix)]`; Windows `\\?\` canonical paths and worktrees with quoted (newline) paths are untested; untracked directories appear as a single `dir/` entry (Git default).

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
- **IPC argument naming (partially verified):** the Rust commands take snake_case parameters (e.g. `repo_path`, `worktree_path`, `new_branch`) and `src/lib/git.ts` passes camelCase keys (`repoPath`, `worktreePath`, `newBranch`), the Tauri 2 convention. Confirmed through the real IPC in the running app for `git_validate_repo` and `git_list_worktrees`. `git_list_branches` (called when the create-worktree modal opens) and `git_create_worktree` were exercised by the manual pass, reported OK without detail. `git_remove_worktree`, `git_get_status`, `git_get_diff` and `git_list_files` have not been checked through the IPC (Task 4 will exercise the last three). Task 1 tests call the commands as plain Rust functions. Returned structs keep snake_case fields (`is_dir`, `index_status`, ...); `GitStatusKind` serializes as `tracked` / `untracked` / `conflicted`.

- [x] Implement `src/lib/git.ts` wrapping Tauri IPC calls with TypeScript interfaces.
- [x] Replace static project fixture in `+page.svelte` with dynamic loading from SQLite + live Git discovery.
- [x] Implement "Add Repository" UI allowing the user to select or enter a local Git folder, validating via `git_validate_repo`, saving to SQLite and refreshing worktrees.
- [x] Implement "Remove Project" UI (removes reference from Trunks workspace without deleting repository files on disk).
- [x] Implement "Add Worktree" modal allowing selection of existing branches or creating a new branch and specifying destination path.
- [x] Implement "Remove Worktree" with confirmation modal and error surfacing if uncommitted changes exist.
- [x] Add manual "Refresh" button and automatic refresh when window regains focus.

**Task 2 implementation notes (completed 2026-11-21; `npm run check` clean, `npm run build` succeeds):**
- Created `src/lib/git.ts` with complete typed wrappers for all 8 Tauri Git backend commands.
- Created `src/lib/projects.svelte.ts` managing reactive project state, SQLite caching, and live Git reconciliation.
- Created modal components: `AddProjectModal.svelte`, `CreateWorktreeModal.svelte`, `RemoveWorktreeModal.svelte`, and `RemoveProjectModal.svelte`.
- Created `WorktreeList.svelte` rendering the tree of registered repositories and their worktrees in the left sidebar with branch tags, status indicators, and quick action buttons.
- Updated `src/routes/+page.svelte` to initialize `projectStore`, listen to `window` focus for auto-refresh, and updated the interactive M1 checklist.

**Task 2 review and manual pass (2026-11-21):**
- Verified statically: `npm run check` (0 errors, 0 warnings), `npm run build`, `cargo test` (43 passed; no Rust changed in this task).
- Verified by hand in `npm run tauri dev` against a disposable repo with 8 worktrees (main, linked, detached, locked, prunable, clean, dirty, Unicode/space path): repository registration, worktree discovery with correct states and `ñ`/space paths, the sidebar scrolling with several repositories, per-project collapse, and a terminal opened from a worktree starting in a directory with spaces and `ñ`. Create worktree and the not-a-repo error were reported OK without detail. **Not yet verified:** guarded removal (`AC-M1-06`: dirty without/with force, locked, prunable, main), external synchronisation (`AC-M1-05`), restart persistence, and behaviour of "open terminal" on a prunable worktree. Task 5 must still run these.
- Fixed after the manual pass (`f68aa15`): the sidebar did not scroll with more than one repository (`.project-card` had no `flex-shrink: 0`); added per-project collapse (kept in `localStorage`) and compact worktree rows; the primary worktree tag now reads `primary` instead of `main`.
- **Open decision before Task 3 — SQLite worktree cache:** `projects.svelte.ts` writes every project's worktree list (branch, path, stale flag) to SQLite on each refresh and paints the sidebar from it on startup (with invented `head: null`, `bare: false`). This conflicts with the Global Constraint "never create a secondary shadow database for ... worktrees" and with "Git as single source of truth". Recommended: persist only project name and path; always read worktrees from Git. Decide before Task 3 builds on it.
- Known issues found by code review, not yet fixed or reproduced:
  - `persist()` rewrites the full project list on every `refreshProject`, so `refreshAll` (and every window focus) fires N concurrent full writes even when nothing changed.
  - Worktree ids are index-based (`<projectId>-wt-<idx>`) and unstable across refreshes; do not bind tabs or selection to them (Task 3).
  - `addProject` de-duplicates by lower-cased path, which merges distinct repos on case-sensitive filesystems.
  - `createWorktree` sets `selectedWorktreePath` without `selectedProjectId`, so creating in a non-selected project may leave the selection inconsistent (checklist step 10, unverified).
  - The layout columns `selected_project_name` / `selected_worktree_name` are filled with a project id and a path, and no restore of the selection on startup was found.
  - "Open terminal" always creates a new tab (Task 3 asks to focus an existing one) and is enabled on prunable worktrees.
  - No native directory picker (no dialog plugin installed): repositories are added by typing a path.
  - There are no frontend tests; the store's reconciliation logic is untested.

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
