# Dragon Ball Color Themes — Implementation Plan

> **Multi-Agent Protocol:** Every agent (Junie, Codex, Claude) must follow the protocol in [`AGENTS.md`](../../../AGENTS.md) when starting a session and after completing tasks.

**Goal:** Incorporate optional accent color themes inspired by Dragon Ball characters (Trunks, Goku, Vegeta, Bulma, Piccolo) selectable from Settings, persistent in `localStorage`, without FOUC, and dynamically synchronized with UI and live terminal sessions.

**Architecture:** Pure logic and palette definitions in `src/lib/theme-logic.ts`, reactive store in `src/lib/theme.svelte.ts`, CSS custom properties in document root, early preloading script in `app.html` to avoid FOUC, and dynamic palette updates on active `TerminalSession` instances.

**Tech Stack:** Svelte 5 (Runes), TypeScript, Node test runner (`node:test`), `@xterm/xterm`.

**Spec:** [`docs/superpowers/specs/2026-11-25-dragon-ball-themes-design.md`](../specs/2026-11-25-dragon-ball-themes-design.md)

---

## Global Constraints

- Do not alter SQLite schema for purely cosmetic UI preferences; theme selection resides in `localStorage` under key `trunks.theme` (with fallback to default `"trunks"`).
- Dark base interface background (`#11111b` / `#181825`) remains untouched; only accent tones and associated transparencias are themed.
- Zero flash of unstyled color (FOUC): theme must be resolved and CSS variables set synchronously before DOM render in `src/app.html`.
- Live terminal sessions (`xterm.js`) must reflect theme changes immediately without restarting PTY processes.
- Verification commands per `AGENTS.md`: `npm run check && npm run build && npm run test:unit && cargo test --manifest-path src-tauri/Cargo.toml`.

---

## Review Focus

- Corrupt or missing `localStorage` key safely falls back to `"trunks"`.
- Active terminal cursor and selection colors match the active character palette.
- UI elements (tabs, primary buttons, worktree indicator, diff hunks, directory icons, commit hashes, modal switches, resizers) react instantly without page refresh.
- Dropdown in `SettingsModal.svelte` accurately renders color dots and checkmark `✓` matching design reference `trunks_themes.png`.

---

## Delivery Tasks

### Task 1: Data model and pure logic unit tests
- [x] Create `src/lib/theme-logic.ts` defining `ThemeId`, `ThemeColors`, and `DRAGON_BALL_THEMES` dictionary for Trunks (`#9161D4`), Goku (`#E44E4B`), Vegeta (`#3D59BE`), Bulma (`#45B4D6`), and Piccolo (`#57A461`).
- [x] Implement pure helper functions: `resolveTheme(raw)`, `getThemeDefinition(id)`, `getThemeCssVariables(id)`, and `getXtermTheme(id)`.
- [x] Create unit tests in `unit-tests/theme-logic.test.ts` covering valid/invalid theme resolution, palette completeness, CSS variable mapping, and terminal color extraction.
- [x] Verify tests pass with `npm run test:unit`.

### Task 2: Reactive store and global CSS variables
- [x] Create reactive store `src/lib/theme.svelte.ts` with Svelte 5 runes (`$state`) exposing `currentTheme`, `setTheme(id)`, and DOM CSS synchronization.
- [x] Add synchronous early preloading script in `src/app.html` reading `trunks.theme` and applying CSS variables to `:root` to eliminate FOUC.
- [x] Verify store updates `localStorage` and document styles upon theme transition.

### Task 3: Settings modal theme dropdown
- [x] Add `APPEARANCE — THEME` section to `src/lib/components/SettingsModal.svelte`.
- [x] Implement custom dropdown matching `trunks_themes.png` with color dot, character name, chevron, and checkmark `✓` for the active selection.
- [x] Add keyboard navigation (Escape to dismiss, Enter/Space to select) and click-outside dismissal.

### Task 4: UI components migration and live terminal sync
- [x] Replace hardcoded purple accents (`#9448f8`, `#aa6dfb`, `rgba(148, 72, 248, ...)`) with semantic CSS custom properties (`--accent-primary`, `--accent-hover`, `--accent-muted`, `--accent-border`, `--accent-subtle`) across:
  - `src/routes/+page.svelte` (tabs, buttons, resizer hover)
  - `src/lib/components/WorktreeList.svelte` (active worktree border & dot, action buttons)
  - `src/lib/components/InspectorPanel.svelte` (active tab)
  - `src/lib/components/CommitHistoryView.svelte` (commit hashes)
  - `src/lib/components/DiffViewer.svelte` (diff hunk headers)
  - `src/lib/components/FileTree.svelte` (directory icons)
  - `src/lib/components/GitStatusView.svelte` (branch badges)
  - Modals: `AddProjectModal`, `CreateWorktreeModal`, `RemoveProjectModal`, `RemoveWorktreeModal`, `SettingsModal` (switches)
- [x] Update `TerminalSession` in `src/lib/terminal.ts` with `updateTheme(themeId)` and initialize with active theme colors.
- [x] Wire live terminal session updates in `+page.svelte` when theme changes.

### Task 5: Testing, documentation, and handoff
- [x] Run full test suite: `npm run test:unit` (95 unit tests pass) and `cargo test --manifest-path src-tauri/Cargo.toml` (105 tests pass).
- [x] Verify type checking and compilation: `npm run check && npm run build`.
- [x] Document feature in `README.md` Changelog under `2026-11-25`.
- [x] Update protocol in `AGENTS.md` and record feature status.
