# AGENTS.md — Multi-Agent Collaboration Protocol

This repository (**Trunks / Terminal Workspace**) is developed collaboratively and asynchronously across multiple Artificial Intelligence agents (Junie, Codex, Claude, etc.) and human developers.

The goal of this document is to maximize collective efficiency, prevent duplicated effort, preserve context across sessions, and guarantee a smooth and immediate transition for any agent taking control of the project.

---

## 1. Session Start Protocol (Mandatory Reading)

When starting a new session, **every agent MUST perform the following steps in order before making any changes**:

1. **Read this file (`AGENTS.md`)** to review operating rules and constraints.
2. **Consult the current project status**:
   - Review the "Current Project Status and Roadmap" section in this file ([`AGENTS.md`](AGENTS.md)).
   - Review the active plan in [`docs/superpowers/plans/`](docs/superpowers/plans/) (currently [`docs/superpowers/plans/2026-11-22-m2-git-history-and-terminal-continuity.md`](docs/superpowers/plans/2026-11-22-m2-git-history-and-terminal-continuity.md)).
3. **Identify the immediate task**:
   - Locate the first pending item (`- [ ]`) in the active plan.
   - Contrast requirements with [`docs/spec.md`](docs/spec.md), [`docs/superpowers/specs/2026-11-22-m2-git-history-and-terminal-continuity-design.md`](docs/superpowers/specs/2026-11-22-m2-git-history-and-terminal-continuity-design.md), and the corresponding ADR in [`docs/adr/0001-stack.md`](docs/adr/0001-stack.md).

---

## 2. Task Hand-off Protocol (Documentation Updates)

To allow the next agent to resume without loss of context or ambiguity, **documenting all progress is mandatory before concluding a session**:

1. **Update the Active Plan**:
   - Mark completed tasks and subtasks as done (`- [x]`) in `docs/superpowers/plans/<active-plan>.md`.
   - If a task is partially completed or blocked, add a concise note beneath the subtask stating the reason and recommended next step.
2. **Update Project Status**:
   - Keep the "Current Project Status and Roadmap" section in `AGENTS.md` in sync with the completed milestone or task and next objective.
3. **Record Technical Evidence and Decisions**:
   - When milestone acceptance criteria are fulfilled, document results in `docs/milestones/<milestone>-results.md` (e.g., `docs/milestones/m0-results.md`, `docs/milestones/m1-results.md`).
   - If a stack component is proposed or ruled out, update `docs/adr/0001-stack.md`.
4. **Verification and Clean State**:
   - Ensure the codebase builds cleanly before wrapping up:
     - Frontend: `npm run check && npm run build`
     - Backend: `cargo check --manifest-path src-tauri/Cargo.toml` (or `cargo build`)
     - Unit tests: `npm run test:unit`
     - Backend tests: `cargo test --manifest-path src-tauri/Cargo.toml`
   - Ensure no leftover temporary files or detached processes remain.

---

## 3. Non-Negotiable Architectural Principles and Constraints

Every agent must respect the principles defined in [`docs/intent.md`](docs/intent.md) and [`docs/spec.md`](docs/spec.md):

- **Do not turn the project into an IDE**: Do not introduce LSPs, debuggers, or file editing capabilities within the initial scope. Trunks is a dedicated workspace for terminals and lightweight browsing/inspection.
- **No proprietary AI orchestration or integration**: Trunks runs locally and does not manage prompts, provider APIs, or agent sessions internally. CLI agents run like any other terminal command inside the PTY.
- **Git is the single source of truth**: Do not create secondary databases to shadow Git state. The application directly queries and reconciles against the system Git CLI.
- **Terminal Tab Identity vs. PTY Process**:
  - *Terminal tab*: Durable UI identity attached to a project/worktree and remembered working directory.
  - *PTY process*: Ephemeral live shell/CLI process instance assigned to that tab.
  - On application restart, tabs restore marked as "stopped" / `[restored]` (no phantom or runaway processes).
- **Native Cross-Platform**:
  - macOS, native Windows (without requiring WSL), and Linux.
  - Avoid dependencies that only function on Unix or require external multiplexer tools such as `tmux` or `zellij`.

---

## 4. Current Project Status and Roadmap

### Current Status
- **Milestone:** Milestone 1 (M1 — Worktree-First Project Workflow & Git Integration) **completed** and merged into `main`; M2 design approved and implementation plan drafted, pending plan review.
- **Progress:**
  - [x] **Milestone 0 Completed:** Scaffold Tauri 2 + Svelte 5, native PTY and terminal fidelity (`portable-pty` + xterm.js), transactional SQLite persistence (WAL mode) with path reconciliation and cold restart recovery without ghost shells (`docs/milestones/m0-results.md`, `docs/adr/0001-stack.md`).
  - [x] **M1 - Task 1 Completed:** Native Git service in Rust (`src-tauri/src/git.rs`; 8 Tauri commands registered in this task, 9 with `git_read_file` added in Task 4; 43 tests including integration against real `git`).
  - [x] **M1 - Task 2 Completed:** UI integration for Git repository registration and worktree management in left sidebar (`src/lib/git.ts`, `src/lib/projects.svelte.ts`, modal components, and `WorktreeList.svelte`).
  - ✅ **Architectural Decision (Worktree Caching):** SQLite stores only the project reference (id, name, path) and active selection; worktrees are always queried live from Git with no caching. Logic resides in `src/lib/project-logic.ts` and is tested via `npm run test:unit`.
  - [x] **M1 - Task 3 Completed:** Terminal tabs bound to Git-reported worktree paths (`findTabForWorktree` in `project-logic.ts`; opening terminal focuses existing tab; button disabled on prunable worktrees; `pty_create` validates cwd existence).
  - [x] **M1 - Task 4 Completed:** Read-only right inspector panel (`InspectorPanel`, `GitStatusView`, `DiffViewer`, `FileTree`) with polling; Git status grouped by collapsible folders; files and diffs opened as central read-only tabs (`FileViewer`, `git_read_file`); resizable panels with shortcuts `Cmd+B` / `Cmd+L`.
  - [x] **M1 - Task 5 Completed:** Acceptance criteria `AC-M1-01` through `AC-M1-08` validated manually by user; `npm run check`, `build`, `test:unit` (16), and `cargo test` (43) all pass. Results documented in `docs/milestones/m1-results.md`.
- **Next Objective:** Review and approve [`docs/superpowers/plans/2026-11-22-m2-git-history-and-terminal-continuity.md`](docs/superpowers/plans/2026-11-22-m2-git-history-and-terminal-continuity.md), then implement the Git commit history tab and run the macOS snapshot spike. Windows/Linux validation remains deferred until external testers are available.

---

## 5. Development and Verification Commands

> **Subshell note:** If the `cargo` binary is not in PATH, run `export PATH="$HOME/.cargo/bin:$PATH"`. Also ensure system `git` (v2.28+) is available.

| Action | Command |
|---|---|
| Install frontend dependencies | `npm install` |
| Type checking and linting | `npm run check` |
| Frontend compilation | `npm run build` |
| Frontend unit tests | `npm run test:unit` (Built-in Node test runner; runs native TypeScript) |
| Rust fast check | `cargo check --manifest-path src-tauri/Cargo.toml` |
| Rust debug compilation | `cargo build --manifest-path src-tauri/Cargo.toml` |
| Run full test suite | `npm run test:unit && cargo test --manifest-path src-tauri/Cargo.toml` |
| Run in development mode | `npm run tauri dev` |
| Build release desktop app | `npm run tauri build` |
| Disposable repo for M1 manual pass | `scripts/m1-sandbox/setup.sh` (guide: `docs/milestones/m1-manual-pass.md`) |

---

## 6. Commit Message Conventions (If applicable)

When an agent commits code (only upon explicit user request), include the appropriate co-authorship trailer:
- Conventional prefixes: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`.
