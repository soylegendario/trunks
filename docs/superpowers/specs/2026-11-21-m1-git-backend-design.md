# M1 Native Git Backend Design

**Status:** Approved by product owner
**Scope:** Milestone 1, Task 1 in `docs/superpowers/plans/2026-11-21-m1-git-worktrees.md`

## Intent

Provide a native Rust service that uses the installed Git CLI for repository validation, worktree and branch discovery, guarded worktree operations, status, diffs, and shallow file browsing. Git remains the source of truth; SQLite will not mirror Git state. The service must work on macOS, Windows, and Linux without blocking the UI thread.

## Goals and boundaries

- Register eight Tauri commands for the Git operations already listed in the M1 plan.
- Normalize a selected directory to Git's repository root, including when the selected directory is nested inside a repository.
- Return structured, serializable data for repositories, worktrees, branches, status entries, and file entries.
- Keep file inspection read-only and scoped to the selected worktree.
- Preserve Git's protection against removing dirty worktrees and require explicit confirmation before force removal.

The service will not implement Git operations through a library, maintain a shadow Git database, stage files, commit, resolve conflicts, or recursively scan a worktree. Frontend registration and confirmation UI remain in Task 2.

## Approaches considered

1. **Tokio process commands (chosen):** Run Git directly with `tokio::process::Command`, passing arguments separately. This fits the existing Tokio dependency and keeps command waits off the UI thread.
2. **Synchronous process commands on a worker thread:** Avoids async command bodies but adds thread-pool coordination and more complex cancellation without a project need.
3. **Shell command strings:** Rejected because quoting and escaping differ across platforms and are unnecessary.

## Behavior and interfaces

- All Git invocations use the executable `git`, never a shell. Disable terminal prompts (`GIT_TERMINAL_PROMPT=0`); do not impose arbitrary timeouts in M1. Return non-zero exit errors with operation context and Git's diagnostic output. Missing Git must return a clear error. If the installed Git does not support porcelain v2, return a clear upgrade error; do not add a legacy parser.
- `git_validate_repo` runs `git rev-parse --show-toplevel` and returns the canonical root and display name.
- `git_list_worktrees` parses `git worktree list --porcelain`, including detached, bare, locked, and prunable states.
- `git_list_branches` reports local and remote branch names, current-branch state, and commit IDs using a machine-readable Git format.
- `git_create_worktree` invokes `git worktree add`; `new_branch=true` creates the requested branch, otherwise it checks out the requested existing branch. Destination paths may be outside the repository.
- `git_remove_worktree` only accepts a path reported by Git as a worktree and rejects the main worktree. Normal removal omits `--force` and lets Git reject dirty worktrees. Force removal passes `--force` only when the caller explicitly requests it after UI confirmation.
- `git_get_status` parses `git status --porcelain=v2 -z` into entries with the path, index and worktree status codes, change kind, and prior path for rename/copy records.
- `git_get_diff` returns unified diff text, disables external diff and color, and treats an optional relative file path as a literal pathspec. For an untracked text file, synthesize a new-file diff with every line added. For an untracked binary file, return a clear message that a text diff is unavailable.
- `git_list_files` lists one directory level at a time, hides `.git`, includes ordinary hidden files, and does not follow symlinks. Inspection paths are relative to and contained within the worktree. If the requested directory is missing or unreadable, fail the whole request with a contextual error rather than returning partial results.
- Path handling must preserve spaces and Unicode. Repository input may be nested and is normalized to the root. Worktree destinations may be external; remove targets must be Git-listed worktrees.

## Testing and acceptance

Add Rust unit tests for porcelain worktree and status parsing, including detached/locked/prunable worktrees, staged/unstaged/untracked entries, rename records, and paths containing spaces or Unicode. Keep parsers separated from process execution so fixture tests do not require a real repository. The milestone's end-to-end acceptance checks remain in Task 5.

## Builder decisions

The builder may choose internal module layout, exact Rust enum representation, and the concrete machine-readable branch format, provided the public behavior above is preserved. No latency budget is set before measurement.
