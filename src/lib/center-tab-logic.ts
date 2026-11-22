/** Identity of read-only central tabs. Commit tabs never collide with file/diff tabs. */
export function fileTabId(
  worktreePath: string,
  mode: "file" | "diff",
  cached: boolean,
  path: string
): string {
  return `${worktreePath}|${mode}|${cached ? 1 : 0}|${path}`;
}

export function commitTabId(worktreePath: string, oid: string): string {
  return `${worktreePath}|commit|${oid}`;
}

export function commitTabTitle(shortHash: string, subject: string): string {
  return `${shortHash} ${subject}`.trim();
}
