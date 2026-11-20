# ADR 0001: Candidate desktop and terminal stack

**Status:** Proposed — validate in Milestone 0 before adoption  
**Date:** 2026-11-20

## Context

The product must be a lightweight, local-first desktop application for macOS, native Windows, and Linux. Real interactive terminals and workspace recovery are core requirements. Native Windows support is important; WSL is not a substitute for it. The implementation must keep terminal tab identity separate from the lifetime of a PTY process.

The earlier product discussion proposed Tauri 2, Svelte/TypeScript, Rust, xterm.js, portable-pty, Git CLI, and SQLite. This proposal is a starting hypothesis, not an accepted architecture.

## Candidate

| Concern | Candidate | Validation required |
|---|---|---|
| Desktop shell | Tauri 2 | Packaging and native behavior on all three platforms |
| UI | Svelte + TypeScript | Hidden tabs retain their live terminal renderer/session correctly |
| Native services | Rust | PTY, filesystem, Git, and persistence integration |
| Terminal emulator | xterm.js | Interactive TUI fidelity, resize, input, Unicode, scrollback, copy/paste |
| PTY | portable-pty | Native behavior and process lifecycle on macOS, Windows, Linux |
| Git operations | System Git CLI | Worktree discovery/creation and status/diff behavior |
| Workspace metadata | SQLite | Crash-tolerant incremental updates and straightforward recovery |
| Visual scrollback | Bounded snapshots, implementation TBD | Feasibility, data sensitivity, limits, and historical/live distinction |

## Decision

No stack component is adopted by this ADR yet. Milestone 0 must implement a small end-to-end vertical slice and record results for all three operating systems. The terminal and PTY combination is the primary gate. Workspace persistence must be included in the slice so that architectural choices do not make later recovery an afterthought.

Git remains authoritative regardless of the final UI or persistence technology. Stored application data may cache presentation state but must be reconciled with current Git/filesystem state on restore.

## Alternatives considered

- **Use tmux/Zellij as a required session backend:** rejected for the initial milestone because it adds an external runtime and does not provide a consistent native Windows baseline. Optional integration may be reconsidered later.
- **Build separate native terminal renderers per OS:** not selected for the initial experiment because it multiplies implementation paths before terminal fidelity has been measured.
- **Adopt a daemon to preserve processes:** deferred. The product requires workspace reconstruction after restart, not process survival after application exit. A daemon adds lifecycle, installation, and recovery complexity.
- **Persist terminal output directly in the workspace database:** not selected as an assumption. If visual history is pursued, compare bounded snapshots separately so terminal output does not dominate metadata persistence.

## Consequences and risks

- The first implementation should remain disposable until the platform matrix provides evidence.
- Cross-platform packaging and interactive behavior may expose platform-specific gaps; passing on one developer machine does not prove support.
- xterm.js and a PTY backend may not by themselves guarantee identical terminal behavior. M0 must report concrete gaps rather than hide them behind a generic “terminal supported” claim.
- SQLite is a candidate, but crash consistency, migrations, and recovery semantics still need a small prototype.
- No process persistence, agent auto-resume, multiplexer dependency, plugin system, or daemon is implied by this proposal.

## Revisit when

Any required M0 acceptance criterion fails on a target OS, or the prototype shows that the candidate stack cannot meet terminal fidelity, recovery, packaging, or lightweight operation requirements.
