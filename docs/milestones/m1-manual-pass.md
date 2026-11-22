# M1 — Manual Verification Pass (Checklist)

Guide for manually validating `AC-M1-01` through `AC-M1-08` (final results are recorded in `docs/milestones/m1-results.md`, Task 5).

## Preparation

1. `scripts/m1-sandbox/setup.sh` creates the disposable repository and **prints its path** (default: `${TMPDIR:-/tmp}/trunks-m1-sandbox`). In this guide, `S` refers to that path. Re-running it resets the scenario (close the app beforehand). It only deletes a directory that the script itself created and refuses to build inside an existing Git repository.
2. Launch the app from the project root: `npm run tauri dev` (if `cargo` is not in PATH: `export PATH="$HOME/.cargo/bin:$PATH"`).
3. Main scenario repository: `S/repo ñ con espacio`.

> **Destructive steps (creating/deleting worktrees, steps 6–9 and 16–20) should ONLY be run against the sandbox.** Never execute them on a real repository registered in the app.

Record for each step: PASS / FAIL / observation. Steps labeled **OBSERVE** do not have a predetermined expected result.

## Prepared Scenario

Worktrees (8): main (`main`) · `wt-feature-login` (feature/login) · `wt con espacio ñ` (feature/ñandú) · `wt-detached` (detached) ·
`wt-locked` (bugfix/typo, locked) · `wt-prunable` (prunable/gone, deleted directory) · `wt-clean` (wip/clean) · `wt-dirty` (wip/dirty, with uncommitted changes).
Branches without a worktree: `release/1.0` (local and remote), `origin/remote-only` (remote-only); `feature/login` and `main` are also on `origin`.
`S/not-a-repo` is a regular folder. `S/external.sh add|remove|touch` triggers external changes.

## AC-M1-01 Registration

1. `+ Add Project` with `S/repo ñ con espacio` → appears with name "repo ñ con espacio" and its worktrees.
2. Add `S/not-a-repo` → clear error, NOTHING is registered.
3. Add `S/repo ñ con espacio/src/nested/deep` (nested subfolder) → normalizes to the repository root, does NOT duplicate the project.
4. Close and reopen the app → the project persists.

## AC-M1-02 Discovery (expected: 8 entries)

5. Each worktree displays its correct branch and path; `wt-detached` has no branch; `wt-locked` displays a lock icon (reason "sandbox: locked on purpose" in tooltip); `wt-prunable` marked `stale`; the main worktree is labeled `primary`; paths with `ñ` and spaces render properly (not escaped as `\303\261`).

## AC-M1-03 Creation

6. New branch `test/new`, destination `S/worktrees/wt-new` → appears; directory exists on disk; `git -C "S/repo ñ con espacio" worktree list` displays it.
7. Existing branch `release/1.0`, destination `S/worktrees/wt-release` → appears with that branch checked out.
8. New branch `main` (already exists) → visible Git error, nothing created.
9. Relative destination (`rel/dir`) → "must be absolute" error, nothing created in the repo.
10. OBSERVE: create a worktree in a project that is NOT currently selected (register a second disposable repo) → which project/worktree remains selected?
11. OBSERVE: choose `origin/remote-only` as an existing branch.

## AC-M1-04 Terminal (completed by Task 3)

12. Click `>_` on `wt-feature-login` → `pwd` = its worktree path. Repeat on `wt con espacio ñ`.
13. OBSERVE: click `>_` twice on the same worktree (Task 3 focuses the existing tab).

## AC-M1-05 External Synchronization

14. `S/external.sh add` → click the refresh button (or switch window focus back and forth) → `wt-external` appears.
15. `S/external.sh remove` → refresh → disappears. Restart the app → no corruption or ghost entries.

## AC-M1-06 Guarded Deletion

16. Remove `wt-clean` (🗑) → confirmation modal → directory deleted from disk.
17. Remove `wt-dirty` WITHOUT force → visible Git error; directory and `untracked text.txt` remain on disk (`ls`).
18. Remove `wt-dirty` WITH explicitly confirmed force → deleted.
19. The main worktree does not offer deletion (and backend rejects it if attempted).
20. OBSERVE: remove `wt-locked` (with and without force; Git requires double `--force`) and `wt-prunable`. Note the user-facing message. Click `>_` on `wt-prunable` (directory does not exist).
21. Remove the PROJECT (`✕`, not the worktree) → disappears from Trunks, while `S/repo ñ con espacio` remains intact on disk.

## Extra Cases

22. With 2 projects, close and reopen → each project displays "Querying git worktrees..." until Git answers, and worktrees from the previous session never flash before that response (no stale cache). The previously selected project and worktree are restored.
23. Stale: temporarily rename `S/repo ñ con espacio` → refresh → project marked stale with error; restore original name → refresh → clears stale status.
24. Repeated window focus changes (5–10 times) → no flickering or errors in devtools console.
25. Devtools console open throughout: note any errors or warnings.
26. Sidebar: with multiple repos, list scrolling, collapse/expand each project and verify state is remembered on reopen.
27. With a worktree selected, close app, remove it externally (`git worktree remove`) and reopen → on startup, selects the first worktree of the project without leaving an invalid selection.

## Prepared for Task 4 (UI Inspector Additions)

Verification points added after UI changes: `Cmd+B` and `Cmd+L` toggle side panels (even when focused inside a terminal); dragging the edge of each panel resizes it and width is remembered on reopen; clicking a file in Files or Git opens a read-only tab in the center; `new-dir/` lists its files grouped by collapsible folders; green/orange status dot appears on worktrees with running/restored terminals.

Expected state of `wt-dirty`: ` M README.md` · `M  src/app.txt` · `R  old-name.txt -> renamed ñ.txt` · `?? new-dir/` ·
`?? untracked text.txt` (3 lines, no trailing newline) · `?? untracked.bin` (binary) · `?? untracked-big.txt` (>1 MiB → "too large").
`S/external.sh touch` modifies `wt-clean/README.md` to test inspector refresh.
