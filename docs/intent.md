# Terminal Workspace — Product Intent

**Status:** Draft 0.1  
**Product name:** Provisional  
**Target platforms:** macOS, Windows, Linux

## Vision

Build a lightweight desktop workspace for developers who prefer working in real terminals. It organizes local Git projects and worktrees, keeps multiple terminal sessions close at hand, and provides a compact place to inspect files and changes.

The application manages the working environment; it does not manage or replace the tools run inside it.

## Problem

Working across several repositories and worktrees often means juggling a terminal emulator, Git worktree commands, file browsing, and diff inspection. The context that ties those tools together is easy to lose, especially after an application crash or a forced operating-system restart.

The product should reduce that friction without growing into an IDE or an AI-agent client.

## Product principles

### Terminal first

The central surface runs native shells and interactive CLI programs. Terminal fidelity and predictable keyboard behavior are foundational requirements.

### Tool agnostic

Users may run any CLI available in their environment, including coding agents. The application does not require provider integrations, inspect private agent protocols, or automatically resume agent sessions.

### Local first

Core workspace state lives locally. Normal use does not require an account, cloud service, or network connection.

### Git native

Git is authoritative for repository, branch, worktree, and change state. Application state may remember selections and layout, but must not become a competing inventory of Git facts.

### Continuity by design

After an application or system restart, restore the workspace structure and its context. Do not promise to restore the old processes. A new shell can be started in the remembered directory; a user may manually resume a CLI tool when that tool supports it.

### Cross platform from the start

macOS, native Windows, and Linux are target platforms from the first technical milestone. Windows support must not depend on WSL. Advanced WSL support is outside the initial scope.

### Minimal by design

Each feature must support workspace navigation, terminal work, or inspection. Avoid adding capabilities because a full IDE or agent platform has them.

## Intended workspace

The primary window has three areas:

- **Left:** registered projects and their Git worktrees.
- **Center:** independent, real terminal sessions.
- **Right:** file navigation, Git status, and diffs.

The side panels can be hidden to give the terminal more room. Exact layout behavior remains subject to the terminal spike and subsequent interaction design.

## Non goals for the initial release

- Code editor, language server, debugger, or IDE refactoring tools.
- Built-in AI chat, agent orchestration, prompt management, or provider APIs.
- Automatic resume of Claude Code, Codex, or another tool.
- Keeping child processes alive after the application exits.
- Required tmux/Zellij integration, a background daemon, plugin system, or marketplace.
- Advanced WSL management.
- Cloud synchronization or a proprietary version-control model.
- File editing, Git staging, commits, or conflict resolution in the first inspection slice.

## What success looks like

A developer can register local Git repositories, move among their worktrees, run ordinary and interactive CLI tools in several terminals, inspect files and changes, and return after an unexpected restart to a recognizable workspace with useful terminal context.

The product succeeds by making terminal work more organized and recoverable with a small amount of UI.
