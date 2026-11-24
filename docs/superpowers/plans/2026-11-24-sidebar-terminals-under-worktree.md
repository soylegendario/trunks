# Sidebar terminals under each worktree — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show each worktree's terminals nested beneath it in the left sidebar (Project → Worktree → Terminals), collapsible, with one status dot per terminal and an aggregate dot when collapsed.

**Architecture:** Pure helpers in `src/lib/project-logic.ts` compute dot state, aggregate state and collapsed-set pruning (unit-tested). `+page.svelte` passes `terminalsFor(path)` and `onSelectTerminal(...)` callbacks into `WorktreeList.svelte`, replacing `runningTerminals`/`restoredTerminals`. No backend, persistence or tab-model changes.

**Tech Stack:** Svelte 5 (runes), TypeScript, Node built-in test runner (`npm run test:unit`).

**Spec:** `docs/superpowers/specs/2026-11-24-sidebar-terminals-under-worktree-design.md` (issue soylegendario/trunks#17)

## Global Constraints

- Tabs and terminal lifecycle unchanged; terminals stay bound to a worktree by `cwd` via `tabsForWorktree`.
- No close/rename/create controls on sidebar terminal rows.
- Collapse state per worktree in `localStorage` (key `trunks.sidebar.collapsedWorktrees`, keyed by Git-reported worktree path), expanded by default, storage failures ignored, dead paths pruned.
- Dot colours: green = running, orange = restored (stopped + `isRestored`), dim = stopped (non-restored). Collapsed aggregate: running if any running; else restored if any restored; else no dot. Count shown only when more than one terminal.
- Clicking a terminal row must select its project + worktree AND activate its tab (central area only shows tabs of the selected worktree).
- Unit tests live in `unit-tests/*.test.ts` and import with `.ts` extensions.
- Do not commit unless the user asks (AGENTS.md §6). Verification before finishing: `npm run check && npm run build && npm run test:unit`; then update the AGENTS.md status section.

## Review Focus

- Worktree with zero terminals: no dot, no count, no empty list chrome.
- Only non-restored stopped terminals while collapsed: no aggregate dot (per spec), but rows still visible when expanded.
- Clicking a terminal of a worktree in another project switches project, worktree and tab together.
- Collapsed-set containing paths of removed worktrees is pruned on write, not accumulated.
- Corrupt/unavailable `localStorage` value: sidebar renders expanded without throwing.

---

### Task 1: Pure helpers in project-logic

**Files:**
- Modify: `src/lib/project-logic.ts`
- Test: `unit-tests/project-logic.test.ts`

**Interfaces:**
- Produces:
  - `type TerminalDotState = "running" | "restored" | "stopped"`
  - `terminalDotState(tab: { status: string; isRestored?: boolean }): TerminalDotState`
  - `aggregateTerminalState(tabs: { status: string; isRestored?: boolean }[]): { state: "running" | "restored" | null; count: number }` (`count` = `tabs.length`)
  - `pruneCollapsedWorktrees(collapsed: string[], livePaths: string[]): string[]` (keeps only entries present in `livePaths`, preserving order)

- [ ] **Step 1: Write failing tests** in `unit-tests/project-logic.test.ts` (new `describe` blocks): `terminalDotState` maps running→"running", stopped+isRestored→"restored", stopped without isRestored→"stopped"; `aggregateTerminalState` returns `{state:null,count:0}` for `[]`, `"running"` when any running (even with restored present), `"restored"` when no running but a restored, `null` for only plain stopped, with `count` equal to the length; `pruneCollapsedWorktrees(["/a","/gone"],["/a","/b"])` equals `["/a"]`.
- [ ] **Step 2: Run** `npm run test:unit` — Expected: FAIL (exports missing).
- [ ] **Step 3: Implement** the three exports in `src/lib/project-logic.ts` per the signatures above.
- [ ] **Step 4: Run** `npm run test:unit` — Expected: PASS (all prior tests too).

### Task 2: Sidebar UI and wiring

**Files:**
- Modify: `src/lib/components/WorktreeList.svelte`
- Modify: `src/routes/+page.svelte` (the `<WorktreeList ...>` usage, ~line 539, plus a new handler near `openOrFocusWorktreeTab`)

**Interfaces:**
- Consumes: Task 1 helpers; `TerminalTab` from `src/lib/terminal.ts`; `tabsForWorktree`; `selectTab(id)` in `+page.svelte`.
- Produces: `WorktreeList` props `terminalsFor?: (worktreePath: string) => TerminalTab[]` (default `() => []`), `activeTerminalId?: string`, `onSelectTerminal?: (projectId: string, worktreePath: string, tabId: string) => void`. Removes props `runningTerminals` and `restoredTerminals`.

- [ ] **Step 1: In `+page.svelte`**, replace the `runningTerminals`/`restoredTerminals` props with `terminalsFor={(path) => tabsForWorktree(tabs, path)}`, `activeTerminalId={activeFile ? "" : activeTabId}` (an open file tab means no terminal is active; reuse the existing `activeFile` expression used for `.active` at the tab bar), and `onSelectTerminal` calling a new `selectTerminalFromSidebar(projectId, worktreePath, tabId)`: sets `projectStore.selectedProjectId` and `selectedWorktreePath`, then `selectTab(tabId)` (which also persists layout).
- [ ] **Step 2: In `WorktreeList.svelte`**, add per-worktree collapse state mirroring `loadCollapsed`/`setCollapsed` (key `trunks.sidebar.collapsedWorktrees`, array of paths, try/catch around all storage access, pruned with `pruneCollapsedWorktrees` against all current worktree paths of `projectStore.projects`).
- [ ] **Step 3: Worktree row**: remove the old worktree-level `term-dot` block; add a chevron button (`aria-expanded`) shown only when the worktree has ≥1 terminal, toggling collapse without triggering `selectWorktree`. When collapsed, show the aggregate dot (`aggregateTerminalState`, classes `term-dot` / `term-dot restored`) and a count badge when `count > 1`.
- [ ] **Step 4: Terminal list**: when expanded and terminals exist, render one row per terminal beneath the worktree item: dot by `terminalDotState` (new `.term-dot.stopped` dim style), title, `class:active` when `tab.id === activeTerminalId`, `onclick` stops propagation and calls `onSelectTerminal(project.id, wt.path, tab.id)`. Rows are focusable `button`s (keyboard accessible). Follow the existing Catppuccin palette; nested/indented under the worktree item.
- [ ] **Step 5: Verify** `npm run check && npm run build && npm run test:unit` — Expected: all pass.
- [ ] **Step 6: Manual check** (`npm run tauri dev`): create 2 terminals in a worktree and 1 in another; confirm dots per terminal, collapse persists across reload, collapsed worktree shows aggregate dot + count, clicking a terminal in another worktree switches worktree and tab, old worktree-level dot is gone, tab bar behaves as before.
- [ ] **Step 7: Update `AGENTS.md`** "Current Project Status" with a one-line entry for this feature (and the unit-test count).

## Self-Review

- Spec coverage: nested list (T2.4), collapsible + persistence + pruning (T1, T2.2), per-terminal dot (T2.4), aggregate when collapsed (T1, T2.3), click selects worktree+tab (T2.1), tabs/lifecycle untouched, AGENTS.md update (T2.7).
- Type consistency: `terminalsFor`, `onSelectTerminal`, `activeTerminalId`, `terminalDotState`, `aggregateTerminalState`, `pruneCollapsedWorktrees` named identically throughout.
