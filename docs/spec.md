# Terminal Workspace — Functional & Technical Specification

**Version:** Draft 0.1  
**Status:** Initial requirements; architecture remains subject to Milestone 0  
**Related:** [Product Intent](intent.md), [Stack ADR](adr/0001-stack.md), [Milestone 0 spike](milestones/m0-terminal-workspace-spike.md)

## Scope and terminology

The initial product is a local desktop workspace for Git projects, worktrees, terminal sessions, and read-only inspection. A **terminal tab** is a durable UI identity attached to a project/worktree and remembered working directory. A **PTY process** is the live shell or command currently attached to that tab. They are separate: a tab can be restored even when its previous process no longer exists.

Requirements marked **P0** are foundational. **P1** means useful follow-on scope after the terminal and workspace foundation is viable. The release boundary is not a promise that every P1 item ships in v0.1.

## Functional requirements

### FR-01 — Register local Git projects (P0)

- Add a local Git repository to the workspace.
- Remember its display name and canonical location.
- Remove a project from the application without deleting repository files.
- On restore, report missing or inaccessible paths without dropping the saved entry.
- Detect invalid selections and explain that a Git repository is required.

### FR-02 — Discover and create worktrees (P0)

- Discover worktrees by querying Git; Git remains the source of truth.
- Show each worktree path and its branch when one is associated.
- Create a worktree from an existing or new branch, with destination and branch validation.
- Make destructive worktree removal an explicit, guarded action; it must never silently delete user files.
- Reflect worktree changes made outside the application when refreshed or revisited.

### FR-03 — Real terminal sessions (P0)

- Start the platform's configured shell in the selected worktree directory.
- Support interactive input/output, common ANSI behavior, resize events, scrollback, copy, and paste.
- Support multiple independent sessions and interactive CLI/TUI programs.
- Resizing the terminal area must update the PTY dimensions without restarting its process.
- Terminal output or a slow command must not block workspace navigation or the UI thread.

### FR-04 — Terminal identity and navigation (P0)

- Associate each terminal tab with a project, worktree, and remembered working directory.
- Keep live PTY sessions running while the user changes selected project, worktree, tab, or visible panel.
- Restore a live tab to the same process while that process remains alive in the running application.
- Closing a tab and terminating its process are distinct actions with clear behavior.

### FR-05 — Workspace persistence (P0)

Persist incrementally, without depending on orderly application shutdown:

- Registered project paths and display names.
- Selected project and worktree context.
- Terminal tab identities, association, working directory, order, and selected tab.
- Panel visibility and layout dimensions, subject to validation of the chosen UI approach.
- Enough recovery metadata to distinguish restored tabs from live PTY processes.

Use transactional, crash-tolerant storage. The proposed SQLite implementation is not final until the stack spike is accepted.

### FR-06 — Restore after application or system restart (P0)

- Rebuild the saved project/worktree navigation and window context on launch.
- Verify saved repository and worktree paths against the current filesystem and Git state.
- Restore terminal tabs as stopped/restored tabs; do not imply that their old processes survived.
- Allow the user to start a fresh shell in the remembered working directory.
- Do not automatically run arbitrary prior commands or resume agent sessions.

An abrupt termination must not lose previously committed workspace state. Very recent in-flight layout edits may be lost only within an explicitly documented persistence interval; the initial target is to persist workspace changes immediately or on each UI state change.

### FR-07 — File inspection (P1)

- Browse the selected project/worktree without eagerly traversing the entire repository.
- Open files read-only in the first inspection slice.
- Handle large directories and inaccessible paths without freezing the terminal.

### FR-08 — Git status and diffs (P1)

- Show added, modified, and deleted files, distinguishing staged and unstaged changes where Git reports them.
- Show a file diff and refresh state after external filesystem or Git changes.
- The first inspection slice is read-only: no staging, commits, or conflict resolution from the UI.

### FR-09 — Visual terminal history (P1 / desirable)

- Evaluate bounded terminal emulator snapshots for restoring useful visual scrollback.
- Label saved output as historical and separate it from a new PTY session.
- Enforce size/retention limits and provide a clear way to clear saved output.
- Do not claim that replaying output restores the internal state of an old TUI or agent.

## Non-functional requirements

### NFR-01 — Platform coverage

macOS, native Windows, and Linux are first-class targets. Validate the terminal thin slice on each OS before committing to the proposed PTY stack. WSL management is not required for v0.1.

### NFR-02 — Responsiveness

Directory scans and Git inspection must not block terminal input/output or the UI. Establish numeric latency and resource budgets after measuring a representative prototype; do not invent budgets before collecting measurements.

### NFR-03 — Recovery and filesystem drift

Handle missing projects, removed worktrees, changed branch state, and inaccessible directories without corrupting saved workspace state. Git and the filesystem may change while the application is closed.

### NFR-04 — Privacy

Keep core workspace metadata local. Terminal output may contain secrets; if scrollback snapshots are implemented, bound retention and expose deletion controls. Do not add telemetry or network dependence by default.

## Proposed architecture boundaries (not yet decisions)

- **Workspace state:** durable project references, tabs, selection, and layout.
- **Git service:** queries and explicit worktree operations through Git; no shadow Git database.
- **Terminal service:** PTY lifecycle, input/output, and resize; independent from tab identity.
- **Terminal renderer:** emulates terminal behavior and keeps each live session mounted or otherwise attached while hidden.
- **Inspection service:** bounded filesystem and Git status/diff reads.
- **Persistence service:** atomic workspace updates and, only if validated, bounded terminal snapshots.

Candidate technologies and their validation gates are recorded in the [ADR](adr/0001-stack.md).

## Milestone 0 acceptance criteria

Run the same acceptance checklist on macOS, Windows, and Linux. Record OS version, shell, app commit, commands, and observed failures for each run.

- **AC-M0-01:** Launch the thin-slice app and start a native shell in a chosen temporary Git worktree; the shell reports the expected working directory.
- **AC-M0-02:** Verify echo/input, Ctrl-C, ANSI colors/styles, Unicode text, and normal scrollback without corruption.
- **AC-M0-03:** Run at least one full-screen interactive program available on the platform; keyboard input, redraw, and exit work. Record any program-specific limitation.
- **AC-M0-04:** Resize the terminal region repeatedly while an interactive process is live; the process remains alive and redraws to the new dimensions.
- **AC-M0-05:** Open two terminal tabs in separate worktrees, switch tabs and panels repeatedly, and confirm both original processes continue receiving input/output.
- **AC-M0-06:** Persist project, selected worktree, tab order, active tab, working directories, and panel/layout state. Force-terminate the app, relaunch, and confirm the saved workspace is reconstructed.
- **AC-M0-07:** After relaunch, each restored tab is clearly marked as having no live prior process; a fresh shell can be started in its remembered directory.
- **AC-M0-08:** Remove or rename a saved worktree outside the app, relaunch, and confirm recovery reports the stale path without losing the rest of the workspace.
- **AC-M0-09:** Record terminal fidelity gaps and platform-specific blockers. Do not claim cross-platform viability until all three OS runs pass or the ADR is revised with explicit platform scope.

Visual scrollback restoration is measured separately and is not a gate for proving PTY viability. It remains desirable and must not be represented as restoration of a live TUI process.

## Open decisions

- Which shell discovery/default-shell behavior is acceptable per platform?
- Which terminal protocols are required beyond common ANSI behavior (for example, hyperlinks, image protocols, or advanced keyboard reporting)?
- What persistence mechanism and schema best support immediate workspace recovery?
- Can the candidate terminal renderer and PTY backend meet native Windows fidelity requirements?
- What are acceptable measured startup, input latency, memory, and scrollback limits?
