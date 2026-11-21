# Milestone 1 — Results

Date: 2026-11-21 · Platform: macOS · Branch: `feat/m1-git-backend`

## Automated checks (run by the agent)

| Check | Result |
|---|---|
| `npm run check` | 0 errors, 0 warnings |
| `npm run build` | OK |
| `npm run test:unit` | 16 passed, 0 failed |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 43 passed, 0 failed (includes integration tests against real `git`) |

## Acceptance criteria

The manual pass was run by the user against the disposable repository (`scripts/m1-sandbox/setup.sh`, guide in `docs/milestones/m1-manual-pass.md`). The user reported **everything OK**; no per-step logs were recorded.

| ID | Criterion | Result |
|---|---|---|
| AC-M1-01 | Register a Git project; non-Git directory gives a clear error | Pass (manual) |
| AC-M1-02 | Worktree discovery (detached, locked, prunable, `ñ`/spaces) | Pass (manual) |
| AC-M1-03 | Create worktree from the UI | Pass (manual) |
| AC-M1-04 | Terminal starts in the worktree path (`pwd`) | Pass (manual) |
| AC-M1-05 | External worktree changes reflected on refresh, SQLite intact | Pass (manual) |
| AC-M1-06 | Guarded worktree removal (dirty needs force) | Pass (manual) |
| AC-M1-07 | Git status shown in the inspector | Pass (manual) |
| AC-M1-08 | Unified diff of a modified file | Pass (manual) |

## Delivered beyond the original scope

- Multiple terminals per worktree, filtered by the selected worktree.
- Green/orange dots for worktrees with running/restored terminals.
- Read-only file and diff tabs in the centre area (`git_read_file`, ninth Tauri command).
- Resizable side panels, VS Code-style toggle icons, Cmd+B / Cmd+L.
- Git status grouped as a collapsible folder tree.

## Known debt

- No Rust test for `git_read_file`.
- No frontend tests for Svelte components, the project store or IPC wrappers.
- `addProject` de-duplicates by lower-cased path (merges distinct repos on case-sensitive filesystems).
- Unused leftovers in `+page.svelte` (`sendQuickCommand`, `checklist` state).
- File/diff tabs are not persisted across restarts; no syntax highlighting.
