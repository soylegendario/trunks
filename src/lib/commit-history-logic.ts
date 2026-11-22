import type { GitCommitCursor, GitCommitHistoryPage, GitCommitInfo } from "./git.ts";

/** Page metadata without the commit list. */
export type HistoryMeta = Omit<GitCommitHistoryPage, "commits" | "next_cursor" | "reset">;

export interface HistoryState {
  /** Identity of the query this state belongs to (worktree + base). */
  key: string;
  commits: GitCommitInfo[];
  cursor: GitCommitCursor | null;
  meta: HistoryMeta | null;
}

export function historyKey(worktreePath: string, baseRef: string | null): string {
  return JSON.stringify([worktreePath, baseRef]);
}

export function emptyHistory(key: string): HistoryState {
  return { key, commits: [], cursor: null, meta: null };
}

/** Re-targets the state: a different worktree or base discards everything accumulated. */
export function retarget(state: HistoryState, key: string): HistoryState {
  return state.key === key ? state : emptyHistory(key);
}

/**
 * Applies a response issued for `requestKey`. Stale responses (another worktree/base is now
 * selected) are ignored. A first page, or a page flagged `reset`, replaces what was
 * accumulated; otherwise the page is appended.
 */
export function applyPage(
  state: HistoryState,
  requestKey: string,
  page: GitCommitHistoryPage,
  isFirstPage: boolean
): HistoryState {
  if (requestKey !== state.key) return state;
  const { commits, next_cursor, reset, ...meta } = page;
  const replace = isFirstPage || reset;
  return {
    key: state.key,
    commits: replace ? commits : [...state.commits, ...commits],
    cursor: next_cursor,
    meta,
  };
}

/** True when the status poll shows a HEAD different from the one the history was read at. */
export function isHistoryStale(state: HistoryState, statusHeadOid: string | null | undefined): boolean {
  if (!state.meta || statusHeadOid === undefined) return false;
  return state.meta.head_oid !== statusHeadOid;
}
