# Milestone 0 Terminal Workspace Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Multi-Agent Protocol:** Every agent (Junie, Codex, Claude) must follow the protocol in [`AGENTS.md`](../../../AGENTS.md) when starting a session and after completing tasks.

**Goal:** Build a disposable cross-platform thin slice that validates real interactive PTYs and recoverable workspace structure.

**Architecture:** Keep durable workspace/tab identity separate from live PTY process handles. Use Git to discover worktrees and a candidate local persistence store for workspace metadata. Keep the UI thin so the spike measures terminal fidelity rather than product breadth.

**Tech Stack:** Candidate only: Tauri 2, Svelte/TypeScript, Rust, xterm.js, portable-pty, system Git CLI, SQLite.

**Spec:** `docs/spec.md` and `docs/milestones/m0-terminal-workspace-spike.md`; candidate status is in `docs/adr/0001-stack.md`.

## Global Constraints

- Target macOS, native Windows, and Linux; Windows support must not require WSL.
- Before scaffolding the candidate app, confirm or provision Rust/Cargo and the platform-specific Tauri build prerequisites in each native environment.
- Git is the source of truth for repositories and worktrees.
- Keep terminal-tab identity independent from PTY process identity.
- Changing tabs or panels must leave live sessions running.
- Restore workspace structure after abrupt app/system restart; do not claim old processes survive.
- Do not automatically execute previous commands or resume coding agents.
- No daemon, required multiplexer, plugins, editor, cloud, or agent integration in M0.
- Keep terminal output snapshots optional, bounded, and clearly historical if explored.

## Review Focus

- Missing or invalid project/worktree directories on launch — Task 3 must verify stale paths are preserved and surfaced.
- PTY startup with paths containing spaces or non-ASCII characters — Task 2 must exercise both.
- Interactive process during repeated resize — Task 2 must verify process survival and redraw.
- Rapid tab/panel switching with concurrent output — Task 2 must verify both sessions remain attached and responsive.
- Abrupt process termination between workspace edits — Task 3 must verify the last committed workspace state is recoverable.

---

### Task 1: Create the cross-platform application shell

**Files:**
- Create: app scaffold and platform build configuration selected for the existing environment.
- Create: a minimal three-region window with a project/worktree placeholder, terminal region, and inspection placeholder.

**Interfaces:**
- Consumes: `docs/spec.md` and M0 scope.
- Produces: a runnable app shell on each target OS with a resizable terminal region.

- [x] Scaffold the smallest Tauri + Svelte + TypeScript app that builds for the host OS; do not add full product navigation.
- [x] Add basic left/center/right regions and controls to hide/show side panels; make the center region report its current dimensions.
- [x] Record exact host build command and result; arrange equivalent build instructions for the other two OS environments.

### Task 2: Validate PTY lifecycle and terminal fidelity

**Files:**
- Create: Rust terminal service and frontend terminal adapter, with narrow create/write/resize/close interfaces.
- Create: terminal behavior checklist/results captured per OS.

**Interfaces:**
- Consumes: runnable app shell and center-region dimensions from Task 1.
- Produces: independent live terminal instances, each associated with a stable tab ID and working directory.

- [x] Record the manual acceptance checklist for shell startup, input/output, Ctrl-C, ANSI, Unicode, resize, and two simultaneous sessions.
- [x] Implement only the PTY create, input, output, resize, and explicit terminate path needed by the checklist.
- [x] Verify an interactive full-screen program survives resize and tab/panel switching without restarting its process.
- [x] Repeat and record the checklist on macOS, native Windows, and Linux; mark unsupported behavior plainly (macOS verified; results in docs/milestones/m0-results.md).

### Task 3: Persist and recover workspace identity

**Files:**
- Create: minimal project, worktree-reference, terminal-tab, and layout persistence model.
- Create: launch-time reconciliation for missing paths and dead PTY references.
- Create: forced-termination recovery checklist/results per OS.

**Interfaces:**
- Consumes: stable tab IDs and working-directory associations from Task 2.
- Produces: restored workspace metadata; restored tabs have no live process until the user starts a new shell.

- [x] Record manual acceptance steps for persistence of project/worktree selection, tab order, active tab, working directory, and basic panel/layout state.
- [x] Record a forced-termination procedure that relaunches the app and checks the last saved state.
- [x] Implement transactional incremental persistence using the candidate store only if it remains lightweight and available on all targets (SQLite + WAL mode via `rusqlite`).
- [x] Reconcile restored paths with Git/filesystem state; preserve and label stale references rather than deleting them.
- [x] Verify a restored tab starts a fresh shell in its remembered path and clearly indicates that its prior PTY is gone.

### Task 4: Decide the candidate stack from evidence

**Files:**
- Modify: `docs/adr/0001-stack.md` with accepted/rejected component decisions and evidence.
- Create: `docs/milestones/m0-results.md` with the cross-platform acceptance matrix.

**Interfaces:**
- Consumes: completed Task 2 and Task 3 evidence from all three platforms.
- Produces: explicit adopt/revise/reject recommendation and remaining risks.

- [ ] Map every AC-M0 criterion to a result for each platform.
- [ ] Record concrete terminal and recovery failures with reproduction details.
- [ ] Update the ADR only where prototype evidence supports a decision; retain unknowns as open.
- [ ] Stop before broader product features if any stop condition in the M0 spike is met.
