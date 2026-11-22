# Milestone 2 — Results

Date: 2026-11-22 · Platform: macOS · Branch: `feat/m2-git-history`

## Automated checks (run by the agent)

| Check | Result |
|---|---|
| `npm run check` | 0 errors, 0 warnings |
| `npm run build` | OK |
| `npm run test:unit` | 26 passed, 0 failed |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 73 passed, 0 failed (includes integration tests against real `git`) |

## Shipped: Git commit history (Tasks 1a, 1b, 2, 2b)

- Backend (`git.rs`): paginated history with OID-anchored cursor and `reset`, automatic base detection (upstream, single remote, single `main`/`master`) or explicit base, branch-only/shared/flat grouping, detached / unborn / shallow handling, per-worktree cancellation by `request_id`, `head_oid` in `GitStatusResult`.
- Commit diff (`git_get_commit_diff`): always against the first parent, 40-hex OID validated, output capped at 1 MiB (provisional) with a truncation notice.
- Frontend: **Commits** tab in the inspector, base selector (in memory), clickable rows that open a read-only central commit-diff tab.
- Extra (outside the plan): the inspector remembers its active tab per worktree (in memory, session only).

## Manual acceptance

Run by the user in the app against a disposable repository (`/tmp/trunks-m2-sandbox/lin`, created outside the project): `feat` worktree with linear commits, a `--no-ff` merge and a 30,000-line "big diff" commit; `main` worktree; detached worktree.

The user reported that **it works perfectly**; no per-step logs were recorded. Not individually confirmed: the large-diff truncation notice wording, and the live refresh after an external commit (reported only as overall OK).

## Snapshot spike (not shipped)

Only a spike was done. **No snapshot persistence exists in the product.** Details in `m2-snapshot-spike-results.md`. Recommendation: revisit; prefer serialising the xterm buffer with privacy controls (notice, deletion, permissions) as a prerequisite; raw PTY byte tail is the lighter alternative. The prototype harness lives only in branch `spike/m2-snapshot`.

## Known debt / open decisions

- Page size (200) and `--topo-order` cost on large merge-heavy repositories are unmeasured.
- Cancellation killing the real `git` process has no automatic test (registry only).
- Commit-diff size limit (1 MiB) is provisional.
- Spike gaps: resize with wrapped long lines, TUI mouse modes, Windows and Linux.
- Windows/Linux validation remains deferred.
- Inspector tab choice is not persisted across restarts.
