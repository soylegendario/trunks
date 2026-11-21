import {
  gitValidateRepo,
  gitListWorktrees,
  gitCreateWorktree,
  gitRemoveWorktree,
  type GitWorktreeInfo,
  type GitRepoInfo,
} from "./git";
import {
  getWorkspaceState,
  saveProjects,
  type ProjectItem,
  type WorktreeItem,
} from "./storage";

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

class ProjectStore {
  public projects = $state<ProjectState[]>([]);
  public isInitialized = $state(false);
  public isRefreshing = $state(false);
  public selectedProjectId = $state<string | null>(null);
  public selectedWorktreePath = $state<string | null>(null);

  /**
   * Initializes store by loading cached projects from SQLite,
   * then reconciles with real Git worktree state for each project.
   */
  public async init() {
    try {
      const workspace = await getWorkspaceState();
      if (workspace.projects && workspace.projects.length > 0) {
        this.projects = workspace.projects.map((p) => ({
          id: p.id,
          name: p.name,
          path: p.path,
          is_stale: Boolean(p.is_stale),
          stale_reason: p.stale_reason || null,
          worktrees: p.worktrees.map((wt) => ({
            path: wt.path,
            head: null,
            branch: wt.branch,
            bare: false,
            locked: null,
            prunable: wt.is_stale ? wt.stale_reason || "Directory missing" : null,
          })),
          isLoading: false,
          error: null,
        }));
      }

      this.isInitialized = true;
      // Immediately reconcile with real Git CLI in background
      await this.refreshAll();
    } catch (err) {
      console.error("Failed to initialize projects store:", err);
      this.isInitialized = true;
    }
  }

  /**
   * Adds a new Git repository by local path.
   */
  public async addProject(path: string): Promise<ProjectState> {
    const repoInfo: GitRepoInfo = await gitValidateRepo(path);

    // Check if project already exists
    const existing = this.projects.find(
      (p) => p.path.toLowerCase() === repoInfo.path.toLowerCase()
    );
    if (existing) {
      this.selectedProjectId = existing.id;
      await this.refreshProject(existing.id);
      return existing;
    }

    const id = "proj-" + Date.now() + "-" + Math.random().toString(36).substring(2, 7);
    const newProject: ProjectState = {
      id,
      name: repoInfo.name,
      path: repoInfo.path,
      is_stale: false,
      stale_reason: null,
      worktrees: [],
      isLoading: true,
      error: null,
    };

    this.projects = [...this.projects, newProject];
    this.selectedProjectId = id;

    // Fetch worktrees
    await this.refreshProject(id);
    await this.persist();
    return newProject;
  }

  /**
   * Removes a project reference from Trunks workspace (does not delete repository on disk).
   */
  public async removeProject(id: string): Promise<void> {
    this.projects = this.projects.filter((p) => p.id !== id);
    if (this.selectedProjectId === id) {
      this.selectedProjectId = this.projects[0]?.id || null;
      this.selectedWorktreePath = this.projects[0]?.worktrees[0]?.path || null;
    }
    await this.persist();
  }

  /**
   * Refreshes worktree list for a specific project via Git CLI.
   */
  public async refreshProject(id: string): Promise<void> {
    const project = this.projects.find((p) => p.id === id);
    if (!project) return;

    project.isLoading = true;
    project.error = null;

    try {
      const worktrees = await gitListWorktrees(project.path);
      project.worktrees = worktrees;
      project.is_stale = false;
      project.stale_reason = null;

      // Select first worktree if none selected
      if (this.selectedProjectId === id && !this.selectedWorktreePath && worktrees.length > 0) {
        this.selectedWorktreePath = worktrees[0].path;
      }
    } catch (err: any) {
      console.warn(`Failed to list worktrees for ${project.name} (${project.path}):`, err);
      project.is_stale = true;
      project.stale_reason = String(err);
      project.error = String(err);
    } finally {
      project.isLoading = false;
      await this.persist();
    }
  }

  /**
   * Refreshes all registered projects against system Git CLI.
   */
  public async refreshAll(): Promise<void> {
    if (this.isRefreshing) return;
    this.isRefreshing = true;

    try {
      await Promise.all(this.projects.map((p) => this.refreshProject(p.id)));
    } finally {
      this.isRefreshing = false;
    }
  }

  /**
   * Creates a new worktree in the given project.
   */
  public async createWorktree(
    projectId: string,
    worktreePath: string,
    branch: string,
    newBranch: boolean
  ): Promise<GitWorktreeInfo> {
    const project = this.projects.find((p) => p.id === projectId);
    if (!project) {
      throw new Error(`Project with ID ${projectId} not found.`);
    }

    const wt = await gitCreateWorktree(project.path, worktreePath, branch, newBranch);
    await this.refreshProject(projectId);
    this.selectedWorktreePath = wt.path;
    return wt;
  }

  /**
   * Removes a worktree from a project.
   */
  public async removeWorktree(
    projectId: string,
    worktreePath: string,
    force = false
  ): Promise<void> {
    const project = this.projects.find((p) => p.id === projectId);
    if (!project) {
      throw new Error(`Project with ID ${projectId} not found.`);
    }

    await gitRemoveWorktree(project.path, worktreePath, force);
    await this.refreshProject(projectId);

    if (this.selectedWorktreePath === worktreePath) {
      this.selectedWorktreePath = project.worktrees[0]?.path || null;
    }
  }

  /**
   * Persists current projects and worktrees to SQLite store.
   */
  private async persist(): Promise<void> {
    try {
      const items: ProjectItem[] = this.projects.map((p) => ({
        id: p.id,
        name: p.name,
        path: p.path,
        is_stale: p.is_stale,
        stale_reason: p.stale_reason,
        worktrees: p.worktrees.map((wt, idx) => ({
          id: `${p.id}-wt-${idx}`,
          project_id: p.id,
          name: wt.branch || wt.path.split(/[/\\]/).pop() || `wt-${idx}`,
          branch: wt.branch,
          path: wt.path,
          is_stale: Boolean(wt.prunable),
          stale_reason: wt.prunable,
        })),
      }));

      await saveProjects(items);
    } catch (err) {
      console.error("Failed to persist projects to SQLite:", err);
    }
  }
}

export const projectStore = new ProjectStore();
