# M2 Spike: terminal snapshot persistence (macOS)

Disposable spike. Nothing here is shipped; no product source, dependency, IPC or database change was made.

## Environment

- macOS 27.2.1 (build 26B131), MacBook Pro, Apple M5. Safari/WebKit 27.2.
- Measured inside the real Tauri WKWebView (`npm run tauri dev` from a disposable worktree), `devicePixelRatio` 2. Its user agent only reports `AppleWebKit/605.1.15`, so the exact WebView build is the system WebKit of macOS 27.2.1.
- `@xterm/xterm` 5.5.0, `@xterm/addon-serialize` ^0.13 (spike worktree only). Terminal 120x40, scrollback 10000. Dev build (unoptimised Rust, Vite dev server), so absolute times are indicative.

## Approaches compared

- **A. Serialize**: `SerializeAddon.serialize()` of the xterm buffer, restored by writing the string into a fresh terminal.
- **B. Raw tail**: keep the last N bytes of raw PTY output (64 KiB, 256 KiB, 1 MiB) and replay them into a fresh terminal. Here the tail is cut from the byte stream of the fixture; no Rust-side ring buffer was built.

Fidelity = per-cell comparison (character, fg/bg colour and mode, bold, italic, underline, inverse) of the whole active buffer, aligned from the bottom, against the original terminal.

## Results

Times are ms (serialize and restore are averages of 3 runs). "same/total" counts buffer lines.

| Fixture | Raw bytes | Serialized bytes | Serialize | Restore (A) | A fidelity | B 64K | B 256K | B 1M |
|---|---|---|---|---|---|---|---|---|
| plain 1k lines | 57,780 | 57,780 | 5.3 | 2.7 | 1001/1001 | full | full | full |
| ANSI styles 1k | 106,120 | 104,873 | 6.0 | 6.7 | 937/1001 (see note) | 617/618 lines kept, 383 lost | full | full |
| Unicode 1k | 96,890 | 96,890 | 3.0 | 21.7 | 1001/1001 | 676/677, 324 lost | full | full |
| mixed 10k lines | 874,445 | 870,287 | 29.0 | 18.0 | 9787/10000 (note) | 669/670, 9330 lost | 2678/2680, 7320 lost | full (25 ms) |
| mixed 50k lines | 4,394,994 | 990,489 | 24.0 | 20.0 | 10040/10040 | 662/663, 9377 lost | 2648/2649, 7391 lost | full (38 ms) |
| plain 100k lines | 5,977,890 | 601,236 | 25.0 | 24.7 | 10040/10040 | 1095/1096, 8944 lost | 4378/4379, 5661 lost | full (20 ms) |
| TUI, ends in alt screen | 669,698 | 2,278 | 0.3 | 7.0 | 40/40, alt screen restored | alt state lost | alt state lost | full |
| TUI, after leaving alt screen | 669,706 | 20 | 0.3 | 6.0 | 40/40 | 0/40 | 0/40 | full |

Notes:

- The ANSI "differences" in A (64 lines per 1000) are an encoding artefact: cells using `48;5;N` with N < 16 come back as 16-colour palette cells (colour mode `0x02000000` vs `0x01000000`). Same visible colour; my comparator counted it. Not a real loss.
- "lost" = scrollback lines that do not fit in the tail. Lines kept are essentially exact (1 mismatching line at the cut).
- The "TUI after exit" B row at 64K/256K fails; the likely cause (not isolated in this run) is that the tail starts in the middle of the alternate-screen session, so the leave-alt sequence has no matching enter. Only the 1 MiB tail, which includes the enter, matched.
- Restore finished in under 40 ms for every case, including a full 10k-line scrollback.

### Resize

Restoring at 80 and 160 columns gave results identical to restoring at 120 for both approaches, because the fixture lines are shorter than 80 columns and never wrap. **The resize behaviour of wrapped lines was therefore not exercised**; this is a gap. Known from xterm: serialize emits the text with wrapped-line markers, while raw replay reflows naturally since it re-runs the original writes at the new width.

## What each approach loses

- **A (serialize)**: the process state (it is a visual snapshot), mouse/mode flags, and anything outside the buffer (title, cursor shape). Scrollback beyond the configured limit. Colours are re-encoded equivalently. A running alt screen is restored.
- **B (raw tail)**: everything older than the tail. Alt-screen/mode state unless the tail contains the enter sequence. Replaying the bytes also re-emits terminal queries (title, device-attribute requests); in a real implementation these must not be answered into a dead PTY.

## Cost

- A: one dependency (`@xterm/addon-serialize`, same xterm release train, no native code). Snapshots are text of roughly raw size for short histories and bounded by scrollback (about 1 MB at 10k lines in these fixtures).
- B: no frontend dependency. Needs a Rust-side bounded ring buffer per PTY (writes on every output chunk) and a way to persist it. Size is exactly the cap chosen; fidelity depends on the cap.

## Privacy

Either approach persists terminal output to disk, which can contain secrets (tokens, `cat .env`, passwords echoed by tools, private code). Before any product persistence:

- Disclose it in the UI and make it opt-in or at least visible per tab.
- Provide "clear history" per tab and globally, and delete the stored snapshot when a tab is closed or its project removed.
- Store it only in the app data directory with user-only permissions; never sync or log it.
- Consider a size cap and an age limit. No cap is proposed here; the numbers above do not force one.

## Recommendation: revise (ship narrowly)

- Prefer **A (serialize)**: it preserves the whole scrollback and the alternate screen in a few KB to about 1 MB, serializes in under 30 ms and restores in under 25 ms in the real WKWebView, and the restored buffer can be marked historical by writing a dim banner. The cost is one small dependency.
- Keep **B** as the fallback if the dependency is rejected; it needs at least 1 MiB to keep a 10k-line scrollback and fails on alt-screen state unless the tail starts at the enter sequence.
- Before committing, still to do: test wrapped lines and live resize after restore, a TUI with mouse modes, snapshot-on-quit timing under a very long line, and Windows/Linux (WebView2 and WebKitGTK were not measured).
- The privacy items above are prerequisites to shipping, not options.

## Reproducing

The harness is committed on the disposable branch `spike/m2-snapshot` (commit `93cc0bd`, worktree `/private/tmp/trunks-spike`, may be removed): `src/routes/+page.svelte` (fixtures and measurements), `spike/server.mjs` (receives results on `127.0.0.1:7777`) and `spike/results-macos.json` (raw numbers from this run).

```
git worktree add /private/tmp/trunks-spike spike/m2-snapshot
cd /private/tmp/trunks-spike && npm install
node spike/server.mjs spike-results.json &
npm run tauri dev   # the window runs the measurements and posts the JSON
```

The first run with a cold cache also recompiles the Rust crate (set `CARGO_TARGET_DIR` to reuse an existing target).
