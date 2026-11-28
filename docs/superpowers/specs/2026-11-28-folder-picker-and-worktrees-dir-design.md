# Folder picker and default `.worktrees/` directory — Design

Date: 2026-11-28 · Size: small change · Status: approved by user

## Intent
Stop typing absolute paths by hand in the Add Project and Create Worktree dialogs, and keep worktrees
inside the project folder by default.

## Goals
- "Browse…" button in `AddProjectModal` and `CreateWorktreeModal` using the native folder picker.
- Suggested worktree path becomes `<project>/.worktrees/<sanitized-branch>`.
- `.worktrees/` does not pollute `git status` of the main worktree.

## Non-goals / anti-goals
- Non-git folders as workspaces (explicitly dropped by the user: an idea, not a need).
- Migrating existing worktrees created at the old location (git keeps reporting them).
- A setting to change the default directory (`.worktrees/` is fixed).
- A custom, hand-made folder browser.
- Modifying the user's versioned `.gitignore`.

## Approaches considered
1. **`tauri-plugin-dialog` + `open({ directory: true })` from the frontend** — chosen. Native UX, minimal code.
2. Custom Rust command + in-app folder browser — rejected: more code, worse UX.
3. Ignoring via versioned `.gitignore` — rejected: touches user-owned tracked files.
   Chosen: local `.git/info/exclude` (resolved with `git rev-parse --git-path info/exclude`,
   because `.git` may be a file in worktrees/submodules).

## Design
1. **Dependency:** add `tauri-plugin-dialog` (Cargo crate, npm package, plugin init in `lib.rs`,
   `dialog:allow-open` in `src-tauri/capabilities`). Helper `pickDirectory(defaultPath?)` in `src/lib/`
   shared by both modals.
2. **Add Project:** "Browse…" next to the path input fills it; field stays editable; validation
   unchanged (`git_validate_repo`).
3. **Create Worktree:**
   - Suggested path logic moves to `project-logic.ts` (`<project>/.worktrees/<sanitized-branch>`),
     unit-tested incl. Windows separators.
   - "Browse…" opens at `<project>/.worktrees` if it exists, else at the project root. The chosen
     folder is the container; the field becomes `<chosen>/<sanitized-branch>` (git rejects existing
     non-empty destinations, so we never use the chosen folder as the final path).
4. **Exclude:** when the final destination lies under `<project>/.worktrees/`, ensure `.worktrees/` is
   listed in the repo's `info/exclude` (idempotent, backend-side, after/before `git worktree add`;
   failure to write the exclude must not fail the worktree creation — surface as a warning).
5. **Docs:** README changelog entry; `AGENTS.md` status update.

## Left to the builder
Exact module/function names, whether the exclude step is a new Tauri command or part of
`git_create_worktree`, Windows path-normalisation details, test layout.

## Verification
`npm run check && npm run build`, `npm run test:unit`, `cargo test --manifest-path src-tauri/Cargo.toml`;
manual: browse in both dialogs, create a worktree and confirm `git status` of the main worktree is clean.
