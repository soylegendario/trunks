# Dynamic Terminal Tab Titles

## Summary

Replace worktree branch based default tab titles with stable `Terminal N` names. When a running program sets the terminal title, show that title in the tab and persist it. Restore the tab's default title when its PTY exits.

## Implementation Changes

- Assign a generic `Terminal N` title to terminals opened from the global action and from worktrees; keep worktree identity and working directory unchanged.
- Observe xterm title change events and reflect non-empty titles in the terminal tab and persisted workspace state.
- Restore the tab's generic title when its PTY exits. Restored tabs without a process keep their persisted title.
- Programs and shells that do not emit terminal title sequences retain the generic title.

## Tests and Acceptance

- Confirm global and worktree terminals receive `Terminal N` titles rather than branch based titles.
- Confirm OSC title updates appear in the tab and persist, and PTY exit restores the generic title.
- Confirm worktree association and working directory remain unchanged.
- Run frontend checks and unit tests; manually verify an interactive program that sets an OSC terminal title.

## Assumptions

- `Terminal N` is the default title format.
- OSC title sequences are used as the available program title signal; no shell-specific hooks or command inference are installed.
- Shells or programs that do not set a terminal title use the generic fallback.
