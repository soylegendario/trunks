# Milestone 0 — Terminal and workspace recovery spike

**Status:** Ready to implement as a bounded technical spike  
**Goal:** Prove the candidate terminal path and workspace recovery on macOS, native Windows, and Linux before building broader product UI.

## Deliverable

A throwaway-quality but runnable desktop thin slice with:

1. A basic three-region layout or equivalent panel controls.
2. A project/worktree selector backed by Git discovery for a local test repository.
3. Two independent terminal tabs in different worktree directories, backed by real PTYs.
4. Tab/panel switching that leaves live PTYs attached and responsive.
5. Incremental persistence of project path, selected worktree, terminal-tab identities/order, working directories, active tab, and basic layout.
6. Launch-time reconciliation with Git and the filesystem; after app restart, tabs restore as stopped and can start new shells in their remembered directories.
7. A short result record per operating system, including failures and deviations.

Use the stack in ADR 0001 as a candidate for the experiment; keep implementation choices easy to replace until the evidence is reviewed. Do not add agents, cloud behavior, daemon, multiplexer, editor, or full Git UI to the spike.

## Test setup

For each OS, prepare one disposable local Git repository and two worktrees with distinct directories. Record OS release, shell and version, Git version, app revision, and terminal dimensions. Include a full-screen interactive program that is available on that system; record its name rather than assuming identical tools are installed everywhere.

Do not use a real working repository for destructive worktree operations. The spike should only create or remove its disposable fixtures.

## Acceptance checklist

The spike is accepted only when the result is recorded separately for macOS, Windows, and Linux against [AC-M0-01 through AC-M0-09](../spec.md#milestone-0-acceptance-criteria).

In particular, verify:

- Expected working directory, interactive input, Ctrl-C, ANSI, Unicode, and scrollback.
- A full-screen interactive process redraws after repeated resizes.
- Two PTYs remain alive while switching tabs and panels.
- Forced application termination and relaunch reconstruct the workspace without claiming old processes survived.
- Missing or externally removed worktree paths are surfaced while other workspace state survives.

Visual scrollback restoration is an optional experiment during M0. If attempted, distinguish a serialized emulator screen from raw output replay and label recovered output as historical. It does not block the PTY viability decision.

## Required result note

Create `docs/milestones/m0-results.md` after running the spike. Include:

- One pass/fail/blocked row for every acceptance criterion on every OS.
- Exact reproduction notes for failures, including shell and interactive program.
- Measured input/resize behavior and basic resource observations; do not set budgets from one run.
- A recommendation to adopt, revise, or reject each ADR candidate component.
- Explicit remaining risks and whether the project proceeds to product implementation.

## Stop conditions

Stop expanding the spike if:

- A target OS cannot create a usable native PTY through the candidate path.
- Interactive input or resize corrupts a live process in a reproducible way.
- Panel/tab navigation requires terminating and recreating PTYs.
- Forced restart loses previously committed workspace state.

When a stop condition occurs, capture evidence and revise the ADR before adding features.
