# M3 manual pass: terminal snapshots (macOS)

Run with `npm run tauri dev`. Open the WebView inspector (right click → Inspect Element → Console) to see the `[snapshot] ...` timing lines (they never contain terminal content).

Snapshot directory: `~/Library/Application Support/<app identifier>/snapshots/` (find it with `find ~/Library/Application\ Support -name snapshots -maxdepth 3`).

Record results in `docs/milestones/m3-results.md` (section "Manual pass").

## AC-M3-01 Setting off writes nothing
1. Fresh state or setting off (Settings ⚙ in the sidebar header). Run `echo hello` and `seq 1 2000` in a terminal; exit the shell (`exit`).
2. `ls` the snapshot directory: it must not exist or be empty. Restart the app: restored tab shows only the old "[Workspace restored]" notice.

## AC-M3-02 Enable shows the risk notice
1. Settings → toggle "Save terminal history": a notice mentioning secrets, unencrypted storage and "not redacted" appears. Cancel leaves it off. Confirm turns it on.

## AC-M3-03 Restore with separator
1. With it on, run `echo MARKER-1`, `seq 1 300`, wait 4 s (debounce). `ls -l` the directory: one `.snap` file, mode `-rw-------`; directory mode `drwx------`.
2. Quit with the window close button, relaunch. The tab shows the previous output, then `[Snapshot from <date> — process not restored]`, then the "[Workspace restored]" notice. Click Start Shell: a new prompt appears below the separator.

## AC-M3-04 Process exit captures
1. Run `exit` in a tab. Without waiting, force-quit is not needed: check the directory has the snapshot immediately (the exit triggers a capture).

## AC-M3-05 TUI then new shell
1. Run `vim` (or `less /etc/hosts`), leave it open, wait 4 s, close the window.
2. Relaunch: the TUI screen is visible as plain content with the separator below. Mouse clicks must not print escape sequences. Click Start Shell: the terminal leaves the alternate screen, shows the old scrollback and a working prompt; typing and `vim` work again.

## AC-M3-06 Deletion paths
- Close a tab (×): its `.snap` file disappears.
- Remove a worktree/project (modals): files of tabs bound to it disappear.
- Settings → Clear all history: directory empty, setting unchanged.
- Settings → toggle off → confirm: setting off, directory empty.
- Create a stray file `snapshots/zzz.snap` by copying a real one under another valid name, restart: it is removed as an orphan; a file `notes.txt` placed there is left alone.

## AC-M3-07 Quit paths (known risk)
- Close the window with the red button after output, relaunch: last output present.
- Quit with Cmd+Q after output stopped less than 3 s earlier: record whether the last lines are present. If not, the loss is bounded by the 15 s max-wait capture; report it.

## AC-M3-08 Performance (Task 4 pending items)
1. Heavy output: `yes | head -c 50000000 | cat` (and `cat` a large colored log) in one tab while typing in another. Note the `[snapshot] ... serialize=..ms chars=..` lines, and whether typing in the active terminal stalls while a capture runs.
2. Wrapped long lines: `printf 'y%.0s' $(seq 1 100000)`, then resize the window; check the snapshot restores sensibly.
3. TUI with mouse modes (`htop`, or `vim` with `:set mouse=a`), close and relaunch; no stuck mouse mode.
4. Record serialize ms, chars and save ms per scenario and any perceived input lag in `m3-results.md`; retune `CAPTURE_*` in `src/lib/snapshot-logic.ts` and `Limits::default` in `src-tauri/src/snapshots.rs` if needed.
