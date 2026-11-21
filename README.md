![Trunks — Your workspace. Your flow.](docs/assets/trunks_concept.png)

# Terminal Workspace

A lightweight, local-first desktop workspace for Git projects, worktrees, and real terminal sessions. Product intent and scope are in [`docs/intent.md`](docs/intent.md); requirements and Milestone 0 acceptance criteria are in [`docs/spec.md`](docs/spec.md).

## Multi-Agent Collaboration

This repository is developed cooperatively across multiple AI agents (Junie, Codex, Claude) and humans. If you are an AI agent starting a session, read [`AGENTS.md`](AGENTS.md) first.

## Current stage

- **Milestone 0 (Terminal & Workspace Spike):** **COMPLETED.** The core architecture (Tauri 2, Svelte 5, Rust, `@xterm/xterm`, `portable-pty`, and SQLite in WAL mode) has been empirically validated on macOS and accepted in [`docs/adr/0001-stack.md`](docs/adr/0001-stack.md) with evaluation results recorded in [`docs/milestones/m0-results.md`](docs/milestones/m0-results.md).
- **Milestone 1 (Worktree-First Project Workflow & Real Git Integration):** **ACTIVE.**
  - **Goal:** Register real local Git repositories, discover and manage worktrees via the system Git CLI, associate interactive PTY terminals with actual worktree directories, and inspect read-only file status and diffs.
  - **Done:**
    - Task 1 — native Git backend in Rust (`src-tauri/src/git.rs`, eight Tauri commands registered, 43 tests passing).
    - Task 2 — project registration and worktree management UI (`src/lib/git.ts`, `src/lib/projects.svelte.ts`, modals and sidebar `WorktreeList.svelte`).
    - Task 3 — tabs bound to Git-reported worktree paths (open-or-focus, missing-directory guard). Code and unit tests done; manual `pwd` and cold-restart passes pending.
  - **Task 4 done (code):** read-only right-side inspector (Git status, diff viewer, file tree); pending a manual pass with the running app.
  - **Next Objective:** Task 5 — end-to-end validation of `AC-M1-01`..`AC-M1-08` and `docs/milestones/m1-results.md`.

Start with:

1. [`AGENTS.md`](AGENTS.md) — Multi-agent collaboration protocol & onboarding
2. Active Plan: [`docs/superpowers/plans/2026-11-21-m1-git-worktrees.md`](docs/superpowers/plans/2026-11-21-m1-git-worktrees.md)
3. Milestone 1 Spec: [`docs/milestones/m1-git-worktrees.md`](docs/milestones/m1-git-worktrees.md)
4. Accepted Architecture: [`docs/adr/0001-stack.md`](docs/adr/0001-stack.md)
5. Completed M0 Spike Results: [`docs/milestones/m0-results.md`](docs/milestones/m0-results.md)

### Development & Build Commands

- **Frontend build/check:** `npm run check` and `npm run build`
- **Native Tauri build:** `npm run tauri build` or `cargo build --manifest-path src-tauri/Cargo.toml`
- **Development mode:** `npm run tauri dev`
