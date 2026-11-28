# Cross-platform GitHub Actions CI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add pull request CI that produces temporary native bundles for Linux, macOS, and Windows, plus tag-triggered GitHub Releases with unsigned bundles for all three systems.

**Architecture:** Keep verification and pull request builds read-only in a CI workflow. Use a separate version-tag workflow for release publication, with write permission limited to its publishing job. Both workflows build on native operating-system runners; no application code or bundle configuration changes.

**Tech Stack:** GitHub Actions, Node.js/npm, stable Rust, Tauri CLI 2, `tauri-apps/tauri-action` for releases, `actions/upload-artifact` for temporary CI bundles.

**Spec:** [`docs/superpowers/specs/2026-11-24-github-actions-cross-platform-ci-design.md`](../specs/2026-11-24-github-actions-cross-platform-ci-design.md)

## Global Constraints

- Pull request CI runs with read-only repository permissions and no secrets.
- Use standard hosted native runners: `ubuntu-24.04` x64, `macos-15` arm64, and `windows-2022` x64.
- Temporary CI artifacts have a 7-day retention period.
- Publish a GitHub Release only for tags matching `v*` and only when the tag equals the app version prefixed with `v`.
- Release assets have no Developer ID, Authenticode, ad-hoc, or notarization setup; document the unsigned macOS Gatekeeper limitation.
- Keep `src-tauri/tauri.conf.json` bundle targets unchanged (`all`); do not add dependencies or modify application code.
- Never commit or expose signing credentials. The tag publishing job alone may have `contents: write`.

## Review Focus

- Fork pull requests cannot access secrets or publish releases; verify `pull_request` uses read-only permissions.
- A mismatched version tag fails before the publishing job starts; test matching and mismatching tag values.
- Concurrent matrix jobs for one tag all upload assets to the same release without overwriting assets from another OS.
- Each platform produces at least one bundle; missing output fails instead of uploading an empty artifact.
- macOS release assets remain unsigned and unnotarized, and user-facing notes explain Gatekeeper may block or flag downloaded builds.

---

## File map

- Create `.github/workflows/ci.yml` for PR and `main` checks, native bundle builds, and short-lived workflow artifacts.
- Create `.github/workflows/release.yml` for tag validation and GitHub Release publication.
- Modify `docs/superpowers/specs/2026-11-24-github-actions-cross-platform-ci-design.md` only if implementation uncovers a necessary design change; do not silently broaden release/signing scope.

### Task 1: Add pull request CI and temporary OS artifacts

**Files:**
- Create: `.github/workflows/ci.yml`

**Interfaces:**
- Triggers: `pull_request` and `push` to `main`.
- Verification job: Ubuntu, Node 22, stable Rust; runs the repository's frontend and Rust checks/tests.
- Build job: native runner matrix, `npm run tauri build`, then uploads bundle output as one artifact per OS.

- [ ] Define workflow triggers and `permissions: contents: read`; set matrix `fail-fast: false`.
- [ ] Add an Ubuntu verification job with checkout, Node 22, stable Rust, and required Linux system dependencies; run `npm ci`, `npm run check`, `npm run test:unit`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `cargo test --manifest-path src-tauri/Cargo.toml`.
- [ ] Add native matrix builds for `ubuntu-24.04`, `macos-15`, and `windows-2022`; install each runner's Tauri prerequisites and Node/Rust toolchains, then run `npm ci` and `npm run tauri build`.
- [ ] Upload `src-tauri/target/release/bundle/**` as distinctly named OS artifacts with `retention-days: 7` and `if-no-files-found: error`.
- [ ] Review the workflow for valid YAML, least-privilege permissions, correct paths on Windows, and behavior that requires no secrets.

### Task 2: Publish unsigned bundles from version tags

**Files:**
- Create: `.github/workflows/release.yml`

**Interfaces:**
- Trigger: pushes of tags matching `v*`.
- Version gate: compare the tag to `v` plus the shared version in `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml`.
- Publish: native three-OS build matrix using the official Tauri GitHub Action; each job uploads its bundle to the release for the triggering tag.

- [ ] Define tag-only trigger and workflow-level `contents: read`; grant `contents: write` only to the publish job.
- [ ] Add a validation job that checks the tag syntax and verifies all three project version declarations match it; fail before any release is created if they differ.
- [ ] Add the native OS matrix and platform prerequisites. Use a reviewed, immutable commit SHA for `tauri-apps/tauri-action`; provide the triggering tag and release name so matrix jobs update the same release.
- [ ] Do not pass signing or notarization secrets or set signing identity variables. Add release notes stating that macOS downloads are unsigned and may be blocked or flagged by Gatekeeper.
- [ ] Ensure the release job runs only after version validation, and that its assets are the OS bundle outputs rather than temporary workflow artifacts.
- [ ] Review re-run behavior so a failed tag build can be re-run without duplicating or deleting assets from other matrix jobs.

### Task 3: Validate workflow behavior and handoff

**Files:**
- Modify: `AGENTS.md` current project status after CI is merged or accepted, following repository handoff protocol.
- Modify: this plan as tasks are completed.

- [ ] Run a workflow linter (`actionlint`) against both YAML files; fix all reported errors.
- [ ] Validate both matching and mismatching version-tag cases without publishing a release.
- [ ] Push a test branch/PR and confirm verification runs, all three native jobs build, and each job exposes a non-empty artifact with 7-day retention.
- [ ] Do not create a disposable public release for testing. Validate the publish flow on the next intended version tag, or leave release execution pending until the user chooses to publish a version.
- [ ] Record actual check names and the unsigned macOS download limitation; do not record Windows/Linux manual runtime acceptance as passed.
- [ ] Update project status and mark completed plan items; report any platform runner/build failures with logs and next steps.
