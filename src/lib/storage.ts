import { invoke } from "@tauri-apps/api/core";

export interface WorktreeItem {
  id: string;
  project_id: string;
  name: string;
  branch: string | null;
  path: string;
  is_stale?: boolean;
  stale_reason?: string | null;
}

export interface ProjectItem {
  id: string;
  name: string;
  path: string;
  is_stale?: boolean;
  stale_reason?: string | null;
  worktrees: WorktreeItem[];
}

export interface TerminalTabItem {
  id: string;
  title: string;
  cwd: string;
  project_name: string | null;
  worktree_name: string | null;
  tab_order: number;
  is_stale?: boolean;
  stale_reason?: string | null;
}

export interface LayoutState {
  show_left: boolean;
  show_right: boolean;
  active_tab_id: string | null;
  // Legacy column names, kept to avoid a schema migration: `selected_project_name` holds the
  // selected project's id and `selected_worktree_name` the selected worktree's path.
  selected_project_name: string | null;
  selected_worktree_name: string | null;
}

export interface WorkspaceState {
  layout: LayoutState;
  projects: ProjectItem[];
  tabs: TerminalTabItem[];
}

export async function getWorkspaceState(): Promise<WorkspaceState> {
  return await invoke<WorkspaceState>("db_get_workspace_state");
}

export async function saveLayout(layout: LayoutState): Promise<void> {
  return await invoke("db_save_layout", { layout });
}

export async function saveProjects(projects: ProjectItem[]): Promise<void> {
  return await invoke("db_save_projects", { projects });
}

export async function saveTabs(tabs: TerminalTabItem[]): Promise<void> {
  return await invoke("db_save_tabs", { tabs });
}

export async function upsertTab(tab: TerminalTabItem): Promise<void> {
  return await invoke("db_upsert_tab", { tab });
}

export async function deleteTab(id: string): Promise<void> {
  return await invoke("db_delete_tab", { id });
}

export async function checkPathExists(path: string): Promise<boolean> {
  return await invoke<boolean>("db_check_path", { path });
}
