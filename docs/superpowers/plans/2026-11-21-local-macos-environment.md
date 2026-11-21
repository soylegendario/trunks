# Local macOS Development Environment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prepare this macOS host for the repository's Tauri 2, Svelte/TypeScript, and Rust development stack.

**Architecture:** Reuse the host's installed Node.js 22, npm, and Xcode. Install the missing stable Rust toolchain with rustup, install JavaScript dependencies from the committed npm lockfile, and verify both application layers without changing project dependency declarations.

**Tech Stack:** Node.js 22, npm, Rust stable via rustup, Tauri 2, Svelte 5, TypeScript.

**Spec:** `docs/superpowers/specs/2026-11-21-local-macos-environment-design.md`

## Global Constraints

- Keep setup scoped to the current macOS host.
- Reuse the existing Node.js, npm, and Xcode installations.
- Use `npm ci` and preserve the committed package lockfile.
- Do not change application code or dependency declarations.

## Review Focus

- Rust installer does not configure this shell's PATH; verify `rustc` and `cargo` can be run from a fresh shell.
- Native compilation exposes missing macOS prerequisites; capture the exact failing command and diagnostic before considering changes.
- npm install attempts to alter a tracked lockfile; inspect the diff and keep project dependency files unchanged.

---

### Task 1: Install the host Rust toolchain

**Files:**
- No repository files; install the stable Rust toolchain with rustup and configure the user's normal Cargo bin PATH if absent.

**Interfaces:**
- Consumes: existing macOS and Xcode installation.
- Produces: `rustc --version` and `cargo --version` available to the host shell.

- [x] **Step 1:** Install stable Rust via the official rustup installer using the host's default configuration.
- [x] **Step 2:** Ensure `$HOME/.cargo/bin` is available to zsh, then confirm `rustc --version` and `cargo --version` succeed in a fresh login shell.

### Task 2: Install locked frontend dependencies

**Files:**
- Runtime files under ignored `node_modules/`; do not modify tracked package manifests or lockfiles.

**Interfaces:**
- Consumes: Node.js 22, npm, and `package-lock.json`.
- Produces: installed dependencies matching the repository lockfile.

- [x] **Step 1:** Run `npm ci` from the repository root.
- [x] **Step 2:** Confirm `git status --short` shows no package manifest or lockfile changes.

### Task 3: Verify the full development stack

**Files:**
- No expected repository changes.

**Interfaces:**
- Consumes: installed frontend dependencies and Rust stable toolchain.
- Produces: successful frontend check/build and Rust backend check, or exact reproducible diagnostics for any host prerequisite gap.

- [x] **Step 1:** Run `npm run check` and `npm run build`.
- [x] **Step 2:** Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] **Step 3:** Recheck `git status --short` and report the installed versions and each verification result.
