// Pure project/selection logic, kept free of Svelte runes and Tauri so it can be unit tested
// with `node --test`. Only erasable TypeScript syntax is allowed here (Node strips types).
//
// Principle (docs/adr/0001-stack.md, AGENTS.md): Git is the single source of truth. SQLite keeps
// only the project reference (id, name, path) and the user's selection; worktrees, branches and
// their states are always read from Git and never persisted or painted from a cache.
import type { GitWorktreeInfo } from "./git";
import type { ProjectItem } from "./storage";

export interface ProjectState {
  id: string;
  name: string;
  path: string;
  is_stale: boolean;
  stale_reason: string | null;
  worktrees: GitWorktreeInfo[];
  isLoading: boolean;
  error: string | null;
}

/**
 * Turns what SQLite returned into UI state. Worktree rows left over from earlier versions are
 * ignored: the list starts empty and loading until Git answers. `is_stale` here comes from the
 * backend's filesystem check of the project directory, not from Git.
 */
export function hydrateProjects(items: ProjectItem[]): ProjectState[] {
  return items.map((p) => ({
    id: p.id,
    name: p.name,
    path: p.path,
    is_stale: Boolean(p.is_stale),
    stale_reason: p.stale_reason || null,
    worktrees: [],
    isLoading: true,
    error: null,
  }));
}

/** What gets written to SQLite: the project reference only, never Git-derived state. */
export function toPersistedProjects(projects: ProjectState[]): ProjectItem[] {
  return projects.map((p) => ({
    id: p.id,
    name: p.name,
    path: p.path,
    is_stale: false,
    stale_reason: null,
    worktrees: [],
  }));
}

/**
 * Which worktree should be selected once Git has answered: the current selection if Git still
 * lists it, otherwise the first listed worktree (the primary one), otherwise nothing.
 */
export function resolveSelectedWorktree(
  selected: string | null,
  worktrees: GitWorktreeInfo[]
): string | null {
  if (selected && worktrees.some((w) => w.path === selected)) {
    return selected;
  }
  return worktrees[0]?.path ?? null;
}

/**
 * Validates a persisted selection against the registered projects. A selection whose project
 * is gone is dropped entirely; a worktree path without a project is meaningless.
 */
export function restoreSelection(
  projectId: string | null,
  worktreePath: string | null,
  projects: { id: string }[]
): { projectId: string | null; worktreePath: string | null } {
  if (projectId && projects.some((p) => p.id === projectId)) {
    return { projectId, worktreePath };
  }
  return { projectId: null, worktreePath: null };
}
