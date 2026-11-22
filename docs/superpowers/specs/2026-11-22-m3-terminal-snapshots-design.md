# M3: Terminal Snapshot Persistence — Design

Status: draft, pending review. Date: 2026-11-22.

Inputs: [`docs/milestones/m2-snapshot-spike-results.md`](../../milestones/m2-snapshot-spike-results.md), [`docs/spec.md`](../../spec.md), [`AGENTS.md`](../../../AGENTS.md).

## Goal

When Trunks restarts, a restored (stopped) terminal tab shows the visual history it had when the app last ran, clearly marked as a read-only snapshot. The old process is **not** restored.

## Decisions (confirmed by the user)

- **What is stored:** the xterm buffer serialized with `@xterm/addon-serialize` (scrollback and alternate screen included).
- **Privacy:** opt-in. Disabled by default. A global setting explains the risk before it can be enabled.
- **Storage:** one file per tab in the app data directory with restricted permissions. SQLite stores no snapshot content.

## Non-goals

- No process, shell state, environment, TUI or mouse-mode restoration.
- No raw PTY byte tail (kept only as a documented alternative).
- No snapshot sync, export, search or encryption in this milestone.
- No Windows/Linux validation claimed (deferred). Code must not use Unix-only APIs without a fallback; permission hardening on Windows is best effort and reported as untested.

## Behavior

1. **Setting:** `Settings → Save terminal history`, off by default. Enabling shows a confirmation dialog: terminal output can contain secrets (tokens, `.env`, passwords echoed by tools, private code) and is stored unencrypted on this machine. Disabling the setting deletes all stored snapshots after confirmation.
2. **Capture:** with the setting on, a tab's buffer is serialized (a) when the tab's process exits, (b) periodically while output changed (debounced, interval chosen from measurements), and (c) on app close. Capture never blocks input; failures are logged and do not affect the terminal.
3. **Limits:** scrollback is capped by the existing xterm limit. A per-snapshot size cap applies (provisional, to be set from measurements; a larger buffer is trimmed from the oldest lines, never refused silently). Total store size cap with oldest-first eviction.
4. **Restore:** on restart, a restored tab writes the snapshot into xterm, then a visible separator line: `[Snapshot from <date> — process not restored]`. The existing "Start Shell" flow is unchanged. Starting a new shell appends below the separator; the old snapshot stays until the next capture overwrites it.
5. **Alternate screen:** a snapshot taken inside a TUI is restored as plain screen content and must not leave the terminal stuck in alternate-screen mode when a new shell starts (reset terminal modes on separator).
6. **Deletion:** snapshot is deleted when its tab is closed, when its project/worktree is removed, and via `Clear all history` in the settings. Orphan files (no matching tab) are removed at startup.
7. **Stale paths:** a tab with a stale cwd still shows its snapshot.

## Storage

- Location: `<app_data_dir>/snapshots/<tab_id>.snap`. `tab_id` is validated (UUID-like charset) before use; never build a path from unvalidated input.
- Directory `0700`, files `0600` on Unix. Written atomically (temp file + rename).
- Format: small header (version, created-at, cols, rows) + serialized text. Unknown version is ignored, not an error.
- SQLite: only a boolean setting key in layout/settings; no content, no snapshot index. The filesystem is the index; orphans are reconciled at startup.
- Snapshot content must never be logged.

## IPC

- `snapshot_save(tab_id, cols, rows, data)`, `snapshot_load(tab_id) -> Option<{data, created_at, cols, rows}>`, `snapshot_delete(tab_id)`, `snapshot_clear_all()`, `snapshot_reconcile(live_tab_ids)`.
- All commands reject invalid ids, oversize payloads and, when the setting is off, `snapshot_save` (the backend also enforces the opt-in; the frontend flag is not the only guard).

## Open questions / deferred

- Capture interval and size caps: decide from measurements of the real app, not from the spike's fixtures.
- Whether snapshots should also be redacted (e.g. known token patterns): out of scope; the notice must not promise redaction.
- Resize with wrapped long lines, TUI mouse modes: unmeasured in the spike; verify during Task 3 manual pass.

## Acceptance

- With the setting off, no file is ever written (verified by test and manual check of the directory).
- With it on, restart shows the previous content plus the separator; new shell starts normally.
- Closing a tab, removing a worktree, disabling the setting and `Clear all history` delete the corresponding files.
- File permissions are `0700`/`0600` on macOS.
- Input latency in an active terminal is unaffected during capture (manual check with heavy output).
- No claim of process restoration appears in UI or docs.
