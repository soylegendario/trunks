# M2 Design — Git History and Terminal Continuity

**Status:** Approved for planning  
**Date:** 2026-11-22  
**Target validation platform:** macOS; Windows and Linux validation is deferred until external testers are available.

## Intent

M2 adds useful commit history to the existing read-only worktree inspector, then investigates whether Trunks can preserve bounded visual terminal history across an application restart. The first part is a product feature. The second is a disposable technical spike whose results determine whether snapshot persistence deserves a later implementation proposal.

## Current state

- The right inspector has mutually exclusive **Git** and **Files** tabs. Git status refreshes every 2.5 seconds.
- The Rust service queries Git directly and already lists local and remote branches, but it has no commit-history command.
- Terminal tabs and workspace layout are persisted in SQLite. PTY processes are not restored after restart; restored tabs are cold and clearly marked.
- A live terminal emulator remains mounted while the app runs. Its visual buffer is not persisted across restarts.
- Dependencies include `@xterm/xterm` and `@xterm/addon-fit`; no serialization addon is currently installed.

## Goals

1. Add a **Commits** tab to the right inspector for the selected worktree's current branch.
2. Show the complete commit ancestry reachable from that branch, partitioned into commits reachable only from that branch and history shared with an automatically detected base branch.
3. Preserve the existing local-first model: query Git for history; do not maintain a shadow commit database.
4. Run a throwaway macOS spike to evaluate bounded terminal-buffer serialization and restoration. Measure the result before proposing persistent snapshots or storage limits.
5. Keep cold terminal restoration honest: visual history is historical output, not a revived process or TUI state.

## Non-goals

- Showing repository-wide commits from branches that are not reachable from the selected branch.
- Authoring, checking out, cherry-picking, reverting, or otherwise mutating commits.
- Persisting commit history in SQLite.
- Shipping terminal snapshot persistence as part of this design. The spike produces evidence and a recommendation only.
- Changing terminal process lifecycle or automatically resuming commands/agents.
- Treating Windows/Linux validation as an M2 blocker; record it as deferred platform evidence.
- General workspace enhancement without a concrete user outcome.

## Approaches considered

1. **Terminal snapshots first, plus commits.** Delivers both visible features together, but combines a privacy- and storage-sensitive persistence change with a relatively bounded Git inspection feature.
2. **Git history only.** Smallest milestone, but leaves the planned terminal-history question uninvestigated.
3. **Two stages: commits, then a snapshot spike — selected.** Delivers the low-risk inspector feature first; gathers macOS evidence for snapshots before deciding whether they are worth shipping. This is the user's selected approach.

## Commit history behavior

### Scope and presentation

- Add **Commits** as a peer tab beside **Git** and **Files** in `InspectorPanel`.
- Use the selected worktree's `HEAD`; do not mix histories from other worktrees.
- For an attached branch, show commits reachable from its tip. Group them as:
  - **Only reachable from `<branch>`**: commits reachable from the selected branch but not from the detected base.
  - **Shared history with `<base>`**: commits reachable from both tips.
- Keep a clear separator between the groups. Use a topological ordering within each group so parent/merge relationships remain readable.
- Include a short hash, subject, author, and commit date in each row. Detailed commit diffs are outside the first slice.
- Bound the initial query and load older entries incrementally; do not invent a numeric history or latency budget before measuring representative repositories.

### Base detection

- If the selected local branch has a configured non-local upstream, first use that remote's symbolic `HEAD` reference (for example, `origin/HEAD`) when it resolves to a commit.
- If there is no usable upstream remote, use a remote default only when exactly one configured remote has a resolvable symbolic `HEAD` reference.
- If the remote candidate is absent, stale, or ambiguous, fall back to a local `main` or `master` only when exactly one of those refs exists and resolves to a commit.
- If no unambiguous base can be detected, show the reachable history as one flat list with a clear “base unavailable” state. Do not silently guess or require the user to pick a base in the normal flow.
- The user can override detection with an explicit base selector listing existing local and remote branches, with an “Automatic” entry to return to detection. This covers gitflow repositories where `origin/HEAD` points to `main` but branches are cut from `develop`. An explicit base that is missing or unresolvable shows an error state and never falls back to another base. The choice is kept in memory for the session only; persisting it is out of scope for M2.

“Only reachable” describes graph reachability, not who authored a commit or which branch originally created it. Commits brought in by a merge can appear in the upper group when they are not reachable from the base. Commits from branches never merged into the selected branch are not part of this history and remain out of scope.

For detached `HEAD`, show a flat reachable history and indicate that no branch is checked out. If the selected branch is itself the detected base, the branch-only group is empty and the shared-history group contains its ancestry.

### Data and failure handling

- Add a typed Git-service operation that returns commit metadata and enough reachability classification to render the groups. Resolve references and pass them as separate process arguments; do not interpolate them into a shell command.
- Keep the query read-only and asynchronous. Handle an unborn branch, missing/stale remote base, shallow history, and Git errors with explicit UI states rather than fabricated entries.
- In a shallow repository, show the commits currently reachable from the selected tip with a visible partial-history notice and a flat list. Do not draw the branch/base separator because the missing ancestry can make that grouping incomplete.
- Do not persist commit results; refresh when the selected worktree changes and on an explicit refresh/focus path consistent with the inspector.
- First-slice acceptance does not include opening commit diffs. Add that only after the list and grouping behavior is reviewed.

### Acceptance criteria

- The Commits tab is available for a selected worktree without changing Git or workspace state.
- A branch with linear commits above a shared base displays those groups in the correct order and labels the base used.
- A merge fixture demonstrates that commits reachable through a merged side branch are classified by reachability, not by author or presumed ownership.
- A commit already reachable from the base appears in shared history, not in the branch-only group.
- Detached `HEAD`, missing base, unborn branch, and empty history render understandable states.
- A shallow repository displays its available commits with a partial-history notice and no branch/base separator.
- Large history loads incrementally without blocking terminal input or navigation.
- Existing Git and Files inspector behavior remains available in its own tabs.

## Terminal snapshot spike

### Question

Can the current xterm.js terminal renderer serialize and restore a useful, bounded visual scrollback snapshot on macOS without implying that the prior process or TUI state has resumed?

### Spike boundaries

- Use a disposable branch or prototype and label all output clearly as historical.
- Evaluate the available xterm serialization path, including the official serialization addon if needed; do not commit a dependency or persistence schema as part of the spike.
- Exercise plain text, ANSI styling, Unicode, long scrollback, resize, and a full-screen TUI transcript. Record what survives and what does not.
- Measure serialized size and restore time across representative output sizes. Do not set a product cap until these measurements exist.
- Assess how snapshots would be cleared and how the UI would disclose that terminal output may contain secrets.
- Do not write snapshots into the user's normal workspace database during the spike.

### Spike exit criteria

Produce a short result note with:

- fidelity findings, including known loss of terminal/TUI internal state;
- serialized-size and restore-time measurements;
- candidate bounds and retention behavior based on measurements;
- privacy and clear/delete UX implications;
- a recommendation to ship, revise, or drop persisted visual snapshots.

No snapshot persistence feature is approved until that evidence is reviewed and a separate implementation scope is accepted.

## Risks and mitigations

- **Wrong base classification:** use a documented detection order and fall back to a flat list instead of guessing.
- **Merge history looks like ownership:** use “only reachable” labels and include a merge fixture in acceptance.
- **Very large histories:** paginate and keep Git queries off the UI thread.
- **Snapshot contents may contain secrets:** keep the spike out of normal persistence and require bounds plus explicit deletion controls before any product implementation.
- **Snapshots may look like process restoration:** mark them as historical and retain the existing stopped/restored state for the PTY tab.
- **Platform evidence is incomplete:** validate this milestone on macOS and record Windows/Linux as deferred, not passed.

## Calls left to the implementation plan

- Exact IPC payload shape and Git command format for history records.
- Pagination size, selected after measuring ordinary repositories.
- Visual treatment of the base label and empty/loading/error states within the existing inspector design.
- Spike fixture sizes and instrumentation method.
