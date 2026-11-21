import { invoke } from "@tauri-apps/api/core";

export interface GitRepoInfo {
  path: string;
  name: string;
}

export interface GitWorktreeInfo {
  path: string;
  head: string | null;
  branch: string | null;
  bare: boolean;
  locked: string | null;
  prunable: string | null;
}

export interface GitBranchInfo {
  name: string;
  is_remote: boolean;
  is_current: boolean;
  commit: string;
}

export type GitStatusKind = "tracked" | "untracked" | "conflicted";

export interface GitStatusEntry {
  path: string;
  index_status: string;
  worktree_status: string;
  kind: GitStatusKind;
  original_path: string | null;
}

export interface GitStatusResult {
  entries: GitStatusEntry[];
}

export interface GitFileInfo {
  name: string;
  path: string;
  is_dir: boolean;
  is_symlink: boolean;
}

/**
 * Validates whether a directory is a Git repository, returning the canonical root and name.
 */
export async function gitValidateRepo(path: string): Promise<GitRepoInfo> {
  return await invoke<GitRepoInfo>("git_validate_repo", { path });
}

/**
 * Lists all worktrees of a Git repository using `git worktree list --porcelain`.
 */
export async function gitListWorktrees(repoPath: string): Promise<GitWorktreeInfo[]> {
  return await invoke<GitWorktreeInfo[]>("git_list_worktrees", { repoPath });
}

/**
 * Lists local and remote branches of a repository.
 */
export async function gitListBranches(repoPath: string): Promise<GitBranchInfo[]> {
  return await invoke<GitBranchInfo[]>("git_list_branches", { repoPath });
}

/**
 * Creates a new worktree at `worktreePath` checking out `branch` (or creating it if `newBranch` is true).
 */
export async function gitCreateWorktree(
  repoPath: string,
  worktreePath: string,
  branch: string,
  newBranch: boolean
): Promise<GitWorktreeInfo> {
  return await invoke<GitWorktreeInfo>("git_create_worktree", {
    repoPath,
    worktreePath,
    branch,
    newBranch,
  });
}

/**
 * Removes a worktree. The main worktree is refused by the backend.
 */
export async function gitRemoveWorktree(
  repoPath: string,
  worktreePath: string,
  force = false
): Promise<void> {
  return await invoke("git_remove_worktree", {
    repoPath,
    worktreePath,
    force,
  });
}

/**
 * Gets the porcelain status of a worktree.
 */
export async function gitGetStatus(worktreePath: string): Promise<GitStatusResult> {
  return await invoke<GitStatusResult>("git_get_status", { worktreePath });
}

/**
 * Gets unified diff of a worktree or specific file.
 */
export async function gitGetDiff(
  worktreePath: string,
  filePath?: string,
  cached = false
): Promise<string> {
  return await invoke<string>("git_get_diff", {
    worktreePath,
    filePath: filePath || null,
    cached,
  });
}

/**
 * Lists files in a directory inside a worktree (one level, protected against traversal).
 */
export async function gitListFiles(
  worktreePath: string,
  subpath?: string
): Promise<GitFileInfo[]> {
  return await invoke<GitFileInfo[]>("git_list_files", {
    worktreePath,
    subpath: subpath || null,
  });
}
