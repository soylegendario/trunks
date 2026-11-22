# M3: Terminal Snapshot Persistence Implementation Plan

> **Multi-Agent Protocol:** Every agent (Junie, Codex, Claude) must follow the protocol in [`AGENTS.md`](../../../AGENTS.md) when starting a session and after completing tasks.

**Goal:** Opt-in persistence of serialized xterm buffers so restored terminal tabs show their previous visual history, clearly marked as a snapshot.

**Architecture:** A Rust `snapshots` module owns a directory of per-tab files (atomic write, restricted permissions, validated ids, size caps, opt-in enforced in the backend). The frontend serializes with `@xterm/addon-serialize` from `TerminalSession`, and writes the snapshot back on restore. SQLite stores only the opt-in flag.

**Tech Stack:** Tauri 2, Rust, Svelte 5 / TypeScript, `@xterm/xterm` 5.5.0, `@xterm/addon-serialize` (version compatible with xterm 5.5.0; verify before installing).

**Spec:** [`docs/superpowers/specs/2026-11-22-m3-terminal-snapshots-design.md`](../specs/2026-11-22-m3-terminal-snapshots-design.md). Spike: [`docs/milestones/m2-snapshot-spike-results.md`](../../milestones/m2-snapshot-spike-results.md).

## Global Constraints

- Opt-in, off by default; the backend refuses `snapshot_save` when the setting is off.
- Snapshot content never goes into SQLite, logs or error messages.
- Never build a file path from an unvalidated `tab_id`.
- Snapshots restore visuals only; never imply process restoration. Preserve cold-tab semantics.
- Capture must not block input or break the terminal on failure.
- Do not add dependencies beyond `@xterm/addon-serialize` without updating `docs/adr/0001-stack.md` and `DEPENDENCY-LICENSE-REVIEW.md`.
- Windows/Linux validation remains deferred and must not be reported as passed.

## Review Focus

- Opt-in enforced in backend, not only in UI.
- Path traversal via `tab_id`; permissions; atomic writes; orphan cleanup.
- Alternate-screen snapshots must not leave the terminal in a broken mode.
- Deleting on tab close, worktree removal, disable and clear-all.

## Task 1: Backend snapshot store

- [x] Add dependency review note for `@xterm/addon-serialize` (license, version) in `DEPENDENCY-LICENSE-REVIEW.md` and ADR.
- [x] Create `src-tauri/src/snapshots.rs` with types and stubs that compile; write failing tests first.
- [x] Tests: id validation (rejects `..`, `/`, empty, long); save/load roundtrip; atomic overwrite; size cap trims oldest lines; unknown header version returns `None`; delete idempotent; clear_all; reconcile removes orphans only; permissions `0700`/`0600` (unix cfg).
- [x] Implement store, with the opt-in flag read from the database; test that save is refused when off and writes nothing.
- [x] Register commands in `lib.rs`; add the settings flag to `db.rs` (migration-safe default off) with tests.
- [x] `cargo test` passes; commit `feat: terminal snapshot store`.

## Task 2: Frontend capture and restore

- [x] Install `@xterm/addon-serialize`; wrapper in `src/lib/snapshots.ts`.
- [x] Pure logic in `src/lib/snapshot-logic.ts` (debounce decision, separator text, capture triggers) with unit tests.
- [x] `TerminalSession`: load addon; capture on process exit, debounced while dirty, and on app close; restore on mount of a restored tab with separator and mode reset.
- [x] Delete snapshot in `closeTab` and on worktree/project removal; run reconcile at startup with live tab ids.
- [x] `npm run check`, `npm run build`, `npm run test:unit`; commit `feat: capture and restore terminal snapshots`.

## Task 3: Settings and privacy notice

- [x] Settings entry with off-by-default toggle, risk confirmation dialog, deletion confirmation on disable, `Clear all history` button.
- [x] Persist flag via db commands; frontend state reflects backend as source of truth.
- [x] Check, build, unit tests; commit `feat: terminal history setting with privacy notice`.

## Task 4: Measurement and limits

- [ ] Measure in the real app (macOS WKWebView): serialize time and size with heavy output, wrapped long lines, TUI with mouse modes.
  - Partial: serialize cost/size measured with a headless xterm in Node (V8, proxy), not in WKWebView. In-app timing is logged with `console.debug("[snapshot] ...")` for the manual pass. Pending: real-app measurement.
- [x] Set capture interval, per-snapshot cap and total cap from the data; record in `docs/milestones/m3-results.md`.
  - Values kept (3 s debounce, 15 s max wait, 2 MiB/snapshot, 16 MiB payload, 64 MiB total) with data from the proxy; re-check against the in-app log.
- [ ] Verify input latency during capture with heavy output.
  - Pending: needs a human with the real app (worst proxy stall 55 ms for a pathological 250-column per-cell-colour buffer; typical 4-7 ms).

## Task 5: Acceptance and handoff

- [x] Full checks: `npm run check && npm run build`, `npm run test:unit`, `cargo test --manifest-path src-tauri/Cargo.toml`.
- [ ] Manual checklist on macOS: setting off writes nothing; on restores after restart with separator; TUI snapshot then new shell works; deletion paths; permissions.
  - Pending: needs a human with the real app. Guide: `docs/milestones/m3-manual-pass.md`.
- [x] Write `docs/milestones/m3-results.md`; update `AGENTS.md`, this plan and ADR.
- [x] `git status` shows only intended files.
