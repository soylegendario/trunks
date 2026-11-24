# Terminals nested under each worktree in the sidebar — Design (short)

Issue: soylegendario/trunks#17. Size: small change (one component, one pure function, no backend/persistence changes).

## Intent and why
Show the terminals of each worktree nested in the left sidebar (Project → Worktree → Terminals) so the user can identify and switch between terminals without going through the tab bar.

## Goals
- Each worktree lists its terminals beneath it, one row per terminal (title + status dot).
- Terminal lists are collapsible per worktree.
- Clicking a terminal row selects its worktree and activates that terminal's tab.
- The per-worktree green/orange dot becomes one dot per terminal.

## Non-goals / anti-goals
- No change to the tab bar (tabs stay) or to terminal lifecycle (create/close/restore/snapshots).
- No close, rename or create-terminal controls in the sidebar rows (my call, to keep scope small).
- No new data model: terminals are still bound to a worktree via `cwd` (`tabsForWorktree`).
- Must not hide activity: a collapsed worktree must still show an aggregate indicator.

## Constraints
- Respect AGENTS.md: tab identity vs PTY process; Git remains source of truth; no extra DB.
- Visible tabs in the central area are filtered by the selected worktree, so selecting a terminal in another worktree must also select that worktree.

## Approaches considered
- **A (chosen):** callbacks from `+page.svelte`. `WorktreeList` receives `terminalsFor(path)` (replaces `runningTerminals`/`restoredTerminals`) and `onSelectTerminal(projectId, worktreePath, tabId)`. Dot state and aggregate computed by a pure function in `project-logic.ts`, unit-tested. Minimal, matches current structure.
- **B (rejected):** extract tabs into a shared store so the sidebar reads them directly. Large refactor of state tied to PTY sessions, persistence and `visibleTabs`, unjustified for a presentation change.

## Design
- `project-logic.ts`: pure helper(s) returning, for a list of tabs, each terminal's dot state (`running` | `restored` | `stopped`) and an aggregate `{state, count}` (running if any running; else restored if any restored; else none).
- `WorktreeList.svelte`:
  - Under each worktree item, a terminal list (`▾/▸` chevron on the worktree row toggles it). Default expanded.
  - Collapsed worktree state persisted in `localStorage` (own key, e.g. `trunks.sidebar.collapsedWorktrees`, keyed by worktree path, pruned of dead paths like the project collapse), failures ignored.
  - Collapsed worktree shows the aggregate dot plus a count when more than one terminal.
  - Terminal row: dot (green running, orange restored, dim stopped), title, highlighted if it is the active tab.
  - Old worktree-level dot removed.
- `+page.svelte`: provide `terminalsFor` and `onSelectTerminal` (select project/worktree, `selectTab(id)`, persist layout). Existing "open terminal" button behaviour unchanged.

## Left to the builder
- Exact styling (follow existing Catppuccin-style palette in the component).
- Keyboard/aria details for the new rows and chevron.
- Unit tests for the helper; verification per AGENTS.md (`npm run check && npm run build`, `npm run test:unit`), plus update the AGENTS.md status section.
