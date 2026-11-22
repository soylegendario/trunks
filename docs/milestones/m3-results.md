# M3 results: opt-in terminal snapshots

Status: **implemented, automated checks pass, manual macOS acceptance pending** (see `docs/milestones/m3-manual-pass.md`). Windows/Linux are not validated (deferred until external testers exist).

## What ships

- Opt-in setting "Save terminal history" (Settings ⚙ in the sidebar header), off by default, with a risk notice before enabling, a deletion confirmation before disabling, and "Clear all history".
- Backend store `src-tauri/src/snapshots.rs`: one file per tab in `<app_data>/snapshots/<tab_id>.snap`, directory `0700`, files `0600`, atomic writes, validated ids, per-snapshot / payload / total caps, oldest-first eviction, orphan reconcile at startup. The opt-in flag lives in SQLite (`app_settings`), is read by the backend under the store lock, and `snapshot_save` is refused when it is off.
- Capture (frontend, `CaptureController` in `src/lib/snapshot-logic.ts`): on process exit, debounced while output changes (3 s debounce, 15 s max wait), and on window close (bounded 2 s). Failures back off exponentially (5 s doubling to 5 min) and never affect the terminal.
- Restore: a restored tab writes the snapshot, resets terminal modes, then `[Snapshot from <date> — process not restored]`. A TUI snapshot stays in the alternate screen until Start Shell, which leaves it and resets modes first. Serialized with `excludeModes` so modes are never replayed.
- Deletion: tab close (waits for an in-flight save), worktree/project removal (tabs bound to the removed paths), disable (atomic with the flag), Clear all history, orphans at startup.
- Dependency: `@xterm/addon-serialize` 0.13.0 (MIT; 0.14+ targets xterm 6). ADR and `DEPENDENCY-LICENSE-REVIEW.md` updated.

## Automated evidence

| Check | Result |
|---|---|
| `cargo test --manifest-path src-tauri/Cargo.toml` | 102 passed |
| `npm run test:unit` | 68 passed |
| `npm run check` | 0 errors, 0 warnings |
| `npm run build` | OK |

A real headless xterm confirmed the mode handling: after restoring a snapshot taken with focus, mouse, paste, insert, origin, no-wrap, keypad and reverse-wrap modes on, `MODE_RESET` returns `terminal.modes` to defaults; without the reset they all stay on. (One-off script, not committed: it needs `@xterm/headless`, which is not a project dependency.)

## Measurements (proxy, not WKWebView)

Headless xterm in Node (V8), scrollback 1000, median of 7 `serialize()` runs:

| Scenario | Size | serialize median / max |
|---|---|---|
| Plain text, 120 cols | 122 KiB | 3.8 / 16.4 ms |
| Plain text, 250 cols | 259 KiB | 6.9 / 22.8 ms |
| Box-drawing + wide glyphs | 276 KiB | 4.2 / 12.6 ms |
| Alt-screen colour TUI | 214 KiB | 5.0 / 12.1 ms |
| One very long wrapped line | 122 KiB | 2.5 / 4.6 ms |
| Worst case: random colour per cell, 120 cols | 2.4 MiB | 20.6 / 41.4 ms |
| Worst case: random colour per cell, 250 cols | 5.0 MiB | 54.3 / 88.8 ms |

Limits kept: 2 MiB per snapshot (about 7x typical, pathological buffers are trimmed from the oldest lines), 16 MiB payload, 64 MiB total. **Not yet measured:** WKWebView timings and input latency during capture. The app logs `[snapshot] tab=… serialize=…ms chars=… save=…ms` (no content) to the console to fill this in.

## Review

A whole-branch review (fresh reviewer) found no critical issue and seven important ones. Fixed with tests that failed first: check-then-act race on disable/close (#1), incomplete mode reset (#2), failure storm (#3), restore racing Start (#4), final output dropped during an in-flight capture (#5). Not fixed, on purpose:

- **#6 Cmd+Q may skip the window-close flush.** Quitting through the macOS app menu may not emit the window close event. Bounded by the 15 s max-wait capture; AC-M3-07 measures it. If it loses data in practice, add a `RunEvent::ExitRequested` flush handshake with a timeout.
- **#7 Eviction re-reads every snapshot on each save** while holding the store lock (tokio worker, not the UI thread). Follow-up: read only headers, skip when under the cap, use `spawn_blocking`.

Deferred minors: clear-all notice should mention that open terminals save again on their next output; separator can overwrite TUI rows when the restored cursor is mid-screen; startup reconcile keeps live-tab files while the setting is off; temp-dir fallback for the snapshot directory; rustfmt on `snapshots.rs`; no DOM-level test for `TerminalSession`; serialization runs on the UI thread (idle scheduling).

## Manual pass

Pending. Run `docs/milestones/m3-manual-pass.md` (AC-M3-01 … AC-M3-08) and record results here. Until then, the claims about restart behaviour, Cmd+Q, WKWebView timings and input latency are **unverified**.
