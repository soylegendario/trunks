# Folder picker and default `.worktrees/` directory Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add native "Browse…" folder pickers to the Add Project and Create Worktree dialogs, default new worktrees to `<project>/.worktrees/<branch>`, and keep `.worktrees/` out of `git status`.

**Architecture:** Pure path logic lives in `src/lib/project-logic.ts` (unit-tested). The native picker is `tauri-plugin-dialog` wrapped by a tiny `src/lib/dialog.ts`. The git-exclude step lives in the backend inside `git_create_worktree` (`src-tauri/src/git.rs`), so no new Tauri command and no frontend coordination.

**Tech Stack:** Tauri 2, Rust (tokio, real-`git` integration tests), Svelte 5, TypeScript, Node test runner (`npm run test:unit`).

**Spec:** `docs/superpowers/specs/2026-11-28-folder-picker-and-worktrees-dir-design.md`

## Global Constraints

- Git is the single source of truth: no new persistence, no DB changes.
- Cross-platform (macOS, native Windows, Linux): path logic must handle `\` and `/` in the project path.
- `.worktrees/` is a fixed directory name; no setting.
- Never modify the versioned `.gitignore`; use the repo's `info/exclude`, resolved via `git rev-parse --git-path info/exclude` (`.git` may be a file).
- Failure to write the exclude must NOT fail worktree creation (best effort; log with `eprintln!`).
- Worktree destination passed to git must remain absolute and must not already exist as a non-empty dir.
- Before finishing: `npm run check && npm run build`, `npm run test:unit`, `cargo test --manifest-path src-tauri/Cargo.toml` all pass; README Changelog entry and `AGENTS.md` status updated (repo rule).

## Review Focus

- Branch names with `/`, spaces, unicode (`feat/ñandú x`) → sanitized to a single safe folder name, never nested or empty.
- Project path with trailing separator or Windows `C:\repo` → no doubled separators, separator style preserved.
- Project that is itself a linked worktree/submodule (`.git` is a file) → exclude still written to the right place.
- Exclude file missing trailing newline or already containing `.worktrees/` / `/.worktrees/` → no corrupted or duplicate lines.
- User cancels the native dialog (returns `null`) or the picker errors → fields unchanged, no error banner for cancel.

## File Structure

- Modify `src/lib/project-logic.ts` — add `sanitizeBranchForPath`, `suggestedWorktreePath`, `worktreePathInContainer`.
- Modify `unit-tests/project-logic.test.ts` — tests for the above.
- Create `src/lib/dialog.ts` — `pickDirectory`.
- Modify `AddProjectModal.svelte`, `CreateWorktreeModal.svelte` — Browse buttons, new suggested path.
- Modify `src-tauri/src/git.rs` — exclude helpers + call from `git_create_worktree`, tests.
- Modify `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`, `src-tauri/capabilities/default.json`, `package.json` — plugin wiring.
- Modify `README.md` (Changelog 2026-11-28), `AGENTS.md` (status).

---

### Task 1: Backend — keep `.worktrees/` out of git status

**Files:**
- Modify: `src-tauri/src/git.rs` (`git_create_worktree` ~line 784; tests module ~line 1469+)

**Interfaces:**
- Produces (private, in `git.rs`):
  - `fn is_under_worktrees_dir(repo: &Path, dest: &Path) -> bool` — true when `canonical_or_raw(dest)` starts with `canonical_or_raw(repo).join(".worktrees")`. Note `dest` does not exist yet: canonicalize its nearest existing ancestor (or compare against `repo.join(".worktrees")` lexically after canonicalizing `repo`).
  - `fn exclude_content_has_worktrees(content: &str) -> bool` — true if any trimmed line equals `.worktrees`, `.worktrees/`, `/.worktrees` or `/.worktrees/`.
  - `async fn ensure_worktrees_excluded(repo: &Path) -> Result<(), String>` — runs `git rev-parse --git-path info/exclude` (via `run_git`/`stdout_text`), resolves a relative result against `repo`, creates the parent dir, appends `.worktrees/\n` (adding a leading `\n` if the file is non-empty and lacks a trailing newline) unless `exclude_content_has_worktrees`.
- `git_create_worktree` calls `ensure_worktrees_excluded` before `git worktree add` only when `is_under_worktrees_dir`; on `Err` it `eprintln!`s and continues.

- [ ] **Step 1: Write failing unit tests** for `exclude_content_has_worktrees` (table: `""`→false, `".worktrees/\n"`→true, `"/.worktrees"`→true, `"  .worktrees/  "`→true, `"foo\n.worktrees-old/\n"`→false) and `is_under_worktrees_dir` (inside→true, sibling `repo-feat`→false, `repo/.worktreesX/a`→false).
- [ ] **Step 2: Run** `cargo test --manifest-path src-tauri/Cargo.toml exclude_content` → FAIL (not defined).
- [ ] **Step 3: Implement** the two pure helpers. Run the tests → PASS.
- [ ] **Step 4: Write failing integration tests** (using `TempDir`, `init_repo`, `sync_git_command`):
  - `create_worktree_under_dot_worktrees_keeps_status_clean`: create `<repo>/.worktrees/feat-a` → `git status --porcelain` in repo has empty stdout, and `<git-path info/exclude>` contains exactly one `.worktrees/` line.
  - `create_second_worktree_does_not_duplicate_exclude_entry`.
  - `create_worktree_outside_dot_worktrees_leaves_exclude_untouched`.
  - `exclude_without_trailing_newline_is_preserved`: pre-write `"foo"` to `info/exclude`, create worktree → file is `"foo\n.worktrees/\n"`.
  - `exclude_works_when_project_is_a_linked_worktree`: make a linked worktree W of the repo, call `git_create_worktree` with `repo_path = W`, dest `W/.worktrees/x` → `git status --porcelain` in W is clean.
- [ ] **Step 5: Run** them → FAIL. **Implement** `ensure_worktrees_excluded` and the call in `git_create_worktree`. Run `cargo test --manifest-path src-tauri/Cargo.toml` → all PASS (existing 100+ tests included).
- [ ] **Step 6: Commit** `feat: exclude .worktrees/ from git status when creating worktrees there`.

---

### Task 2: Frontend path logic (suggested path + container join)

**Files:**
- Modify: `src/lib/project-logic.ts`
- Test: `unit-tests/project-logic.test.ts`

**Interfaces:**
- Produces:
  - `sanitizeBranchForPath(branch: string): string` — same rule as today: `[\\/]`→`-`, then non `[a-zA-Z0-9._-]`→`_`.
  - `suggestedWorktreePath(projectPath: string, branch: string): string` — `<projectPath>/.worktrees/<sanitized>`; `""` if `branch` is blank. Separator = `\` if `projectPath` contains `\` and no `/`, else `/`; trailing separators on `projectPath` are stripped first.
  - `worktreePathInContainer(containerPath: string, branch: string): string` — `<container>/<sanitized>` with same separator/trailing rules; `""` if branch blank.
  - `worktreesBrowseStart(projectPath: string): string` — `<projectPath>/.worktrees` (existence check is the caller's job, see Task 3).

- [ ] **Step 1: Write failing tests** in `project-logic.test.ts`: `suggestedWorktreePath("/Users/me/repo","feat/x y")==="/Users/me/repo/.worktrees/feat-x_y"`; trailing slash `"/Users/me/repo/"` → same; `"C:\\work\\repo"` → `"C:\\work\\repo\\.worktrees\\feat-x"`; blank branch → `""`; `worktreePathInContainer("/tmp/c/","feat/ñ")==="/tmp/c/feat-_"`; `worktreesBrowseStart("/r")==="/r/.worktrees"`.
- [ ] **Step 2: Run** `npm run test:unit` → FAIL.
- [ ] **Step 3: Implement** the four functions in `project-logic.ts`. Run `npm run test:unit` → PASS.
- [ ] **Step 4: Commit** `feat: add worktree path helpers for .worktrees default`.

---

### Task 3: Native picker + UI wiring + docs

**Files:**
- Modify: `src-tauri/Cargo.toml` (`tauri-plugin-dialog = "2"`), `src-tauri/src/lib.rs` (`.plugin(tauri_plugin_dialog::init())` next to opener), `src-tauri/capabilities/default.json` (+`"dialog:allow-open"`), `package.json` (`@tauri-apps/plugin-dialog` `^2`; run `npm install`)
- Create: `src/lib/dialog.ts`
- Modify: `src/lib/components/AddProjectModal.svelte`, `src/lib/components/CreateWorktreeModal.svelte`, `README.md`, `AGENTS.md`

**Interfaces:**
- Consumes: `suggestedWorktreePath`, `worktreePathInContainer`, `worktreesBrowseStart` (Task 2); `gitCheckPath`-style existence check — use the existing `db_check_path` wrapper if one exists in `src/lib/`, otherwise just fall back to the project path when `.worktrees` is missing by trying the picker with `.worktrees` and accepting the OS fallback.
- Produces: `pickDirectory(defaultPath?: string): Promise<string | null>` in `src/lib/dialog.ts` — wraps `open({ directory: true, multiple: false, defaultPath })`, returns `null` on cancel, never throws (catches and `console.warn`s, returns `null`).

- [ ] **Step 1: Wire the plugin** (Cargo, `lib.rs`, capability, npm). Run `cargo check --manifest-path src-tauri/Cargo.toml` → OK.
- [ ] **Step 2: Create `pickDirectory`** in `src/lib/dialog.ts` as specified.
- [ ] **Step 3: AddProjectModal** — "Browse…" button beside the input (secondary style, disabled while loading); on pick, set `pathInput` when non-null. Validation unchanged.
- [ ] **Step 4: CreateWorktreeModal** — replace the inline `suggestedPath` derivation with `suggestedWorktreePath(project.path, effectiveBranch)`. Add "Browse…" beside Destination: opens at `worktreesBrowseStart(project.path)` (fallback `project.path` if that dir does not exist); on pick, set `customPath = worktreePathInContainer(picked, effectiveBranch)`; if no branch yet, show the error "Enter a branch name first." instead of opening. Update the placeholder/hint text to mention `.worktrees/`.
- [ ] **Step 5: Docs** — README Changelog `### 2026-11-28` entry (folder picker; new default `.worktrees/`; auto-exclude), `AGENTS.md` status line.
- [ ] **Step 6: Verify** — `npm run check && npm run build`, `npm run test:unit`, `cargo test --manifest-path src-tauri/Cargo.toml` all PASS. Manual (`npm run tauri dev`): Browse in both dialogs; cancel leaves fields untouched; create a worktree with the default path → it lands in `<project>/.worktrees/<branch>` and the main worktree's git status stays clean.
- [ ] **Step 7: Commit** `feat: native folder picker in project/worktree dialogs`.

---

## Self-review notes

- Spec coverage: picker (T3), Add Project (T3), Create Worktree suggested path + browse (T2/T3), exclude via `rev-parse --git-path` (T1), docs (T3), non-goals untouched.
- One deliberate refinement of the spec: exclude failures are logged (`eprintln!`), not surfaced in the UI, because `git_create_worktree` returns `GitWorktreeInfo` and changing that type isn't worth it for a cosmetic failure.
