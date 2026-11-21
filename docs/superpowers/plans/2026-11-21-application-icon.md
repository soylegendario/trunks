# Trunks Application Icon Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Use the supplied Trunks artwork as the desktop application icon on supported platforms.

**Architecture:** Preserve the square transparent PNG as the source asset in `src-tauri/icons/app-icon.png`. Use the existing Tauri CLI to generate platform-specific assets in the directory already referenced by `tauri.conf.json`; do not change bundle configuration.

**Tech Stack:** Tauri CLI v2, PNG, ICNS, ICO.

**Spec:** `docs/superpowers/specs/2026-11-21-application-icon-design.md`.

## Global Constraints

- Keep the supplied artwork unchanged.
- Preserve the existing icon paths in `src-tauri/tauri.conf.json`.
- Keep the source artwork in the repository so generated icons can be reproduced.

## Review Focus

- The source remains a square PNG with alpha — inspect `app-icon.png` dimensions and alpha.
- Every icon path referenced by Tauri still exists — compare generated assets against `tauri.conf.json`.
- Native platform formats are regenerated from the same source — inspect PNG, ICNS, and ICO outputs.
- The generator writes only to `src-tauri/icons` — review changed paths before finishing.
- The artwork is not altered or cropped — compare the retained source with the Downloads original.

---

### Task 1: Generate platform application icons

**Files:**
- Create: `src-tauri/icons/app-icon.png` (source artwork)
- Modify: generated icon assets in `src-tauri/icons/`
- Read: `src-tauri/tauri.conf.json` (verify existing bundle references)

**Interfaces:**
- Consumes: `/Users/rleon/Downloads/trunks_icon.png`.
- Produces: standard Tauri PNG, ICNS, ICO, and platform tile assets in `src-tauri/icons/`.

- [x] Copy `/Users/rleon/Downloads/trunks_icon.png` unchanged to `src-tauri/icons/app-icon.png`.
- [x] Run `npm run tauri -- icon src-tauri/icons/app-icon.png --output src-tauri/icons`; confirm Tauri generates platform assets in that directory.
- [x] Run `cmp -s /Users/rleon/Downloads/trunks_icon.png src-tauri/icons/app-icon.png`; confirm the retained source is byte-for-byte unchanged.
- [x] Inspect `sips -g pixelWidth -g pixelHeight -g hasAlpha src-tauri/icons/app-icon.png` and confirm the retained source is square and has alpha.
- [x] Confirm all paths listed under `bundle.icon` in `src-tauri/tauri.conf.json` exist and were generated from the new artwork.
- [x] Run `npm run check`; confirm the frontend check passes.
- [x] Run `npm run build`; confirm the frontend production build passes.
- [x] Run `cargo check --manifest-path src-tauri/Cargo.toml`; confirm the native backend still compiles.
