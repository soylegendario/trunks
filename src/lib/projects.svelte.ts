import {
  gitValidateRepo,
  gitListWorktrees,
  gitCreateWorktree,
  gitRemoveWorktree,
  type GitWorktreeInfo,
  type GitRepoInfo,
} from "./git";
import { getWorkspaceState, saveProjects } from "./storage";
import {
  hydrateProjects,
  toPersistedProjects,
  resolveSelectedWorktree,
  restoreSelection,
  type ProjectState,
} from "./project-logic";

export type { ProjectState };

class ProjectStore {
  public projects = $state<ProjectState[]>([]);
  public isInitialized = $state(false);
  public isRefreshing = $state(false);
  public selectedProjectId = $state<string | null>(null);
  public selectedWorktreePath = $state<string | null>(null);

  /**
   * Initializes the store: loads the registered projects (name and path) and the saved
   * selection from SQLite, then asks Git for each project's worktrees. Worktrees are never
   * persisted or painted from a cache; they stay empty and loading until Git answers.
   */
  public async init() {
    try {
      const workspace = await getWorkspaceState();
      this.projects = hydrateProjects(workspace.projects ?? []);

      // The layout columns keep their legacy names: `selected_project_name` holds the project
      // id and `selected_worktree_name` the worktree path (see LayoutState in storage.ts).
      const restored = restoreSelection(
        workspace.layout?.selected_project_name ?? null,
        workspace.layout?.selected_worktree_name ?? null,
        this.projects
      );
      this.selectedProjectId = restored.projectId;
      this.selectedWorktreePath = restored.worktreePath;

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
    await this.persist(); // the registration must survive even if the refresh below fails

    // Fetch worktrees (queried live from Git; nothing about them is persisted)
    await this.refreshProject(id);
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

      // Keep the selection only while Git still lists it; otherwise fall back to the first.
      if (this.selectedProjectId === id) {
        this.selectedWorktreePath = resolveSelectedWorktree(this.selectedWorktreePath, worktrees);
      }
    } catch (err: any) {
      console.warn(`Failed to list worktrees for ${project.name} (${project.path}):`, err);
      project.is_stale = true;
      project.stale_reason = String(err);
      project.error = String(err);
    } finally {
      project.isLoading = false;
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
    this.selectedProjectId = projectId; // select the project too, or the next refresh would undo it
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
   * Persists the registered projects (id, name, path) to SQLite. Called only when a project is
   * added or removed, never on refresh, and never with worktree or Git state.
   */
  private async persist(): Promise<void> {
    try {
      await saveProjects(toPersistedProjects(this.projects));
    } catch (err) {
      console.error("Failed to persist projects to SQLite:", err);
    }
  }
}

export const projectStore = new ProjectStore();
