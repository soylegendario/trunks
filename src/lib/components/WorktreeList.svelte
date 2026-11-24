<script lang="ts">
  import {
    projectStore,
    type ProjectState,
  } from "$lib/projects.svelte";
  import type { GitWorktreeInfo } from "$lib/git";
  import type { TerminalTab } from "$lib/terminal";
  import {
    aggregateTerminalState,
    pruneCollapsedWorktrees,
    terminalDotState,
  } from "$lib/project-logic";
  import AddProjectModal from "./AddProjectModal.svelte";
  import CreateWorktreeModal from "./CreateWorktreeModal.svelte";
  import RemoveWorktreeModal from "./RemoveWorktreeModal.svelte";
  import RemoveProjectModal from "./RemoveProjectModal.svelte";
  import SettingsModal from "./SettingsModal.svelte";

  let {
    onOpenTerminal,
    onSelectWorktree,
    terminalsFor = () => [],
    activeTerminalId = "",
    onSelectTerminal,
    onWorktreeRemoved,
    onProjectRemoved,
  }: {
    onOpenTerminal: (
      cwd: string,
      title: string,
      worktreeName: string,
      projectName: string
    ) => void;
    onSelectWorktree?: (projectId: string, worktreePath: string) => void;
    /** Terminals bound to a worktree path, in tab order. */
    terminalsFor?: (worktreePath: string) => TerminalTab[];
    /** Id of the terminal tab currently shown in the center area ("" if none). */
    activeTerminalId?: string;
    /** A terminal row was clicked: select its project/worktree and activate the tab. */
    onSelectTerminal?: (projectId: string, worktreePath: string, tabId: string) => void;
    /** A worktree was removed: its terminal snapshots must be deleted. */
    onWorktreeRemoved?: (worktreePath: string) => void;
    /** A project was removed: snapshots of terminals bound to any of these paths must be deleted. */
    onProjectRemoved?: (removedPaths: string[]) => void;
  } = $props();

  let addProjectOpen = $state(false);
  let createWorktreeOpen = $state(false);
  let removeWorktreeOpen = $state(false);
  let removeProjectOpen = $state(false);
  let settingsOpen = $state(false);

  let targetProject = $state<ProjectState | null>(null);
  let targetWorktree = $state<GitWorktreeInfo | null>(null);

  // Collapsed projects are a per-user UI preference (not Git state), kept in localStorage.
  const COLLAPSED_KEY = "trunks.sidebar.collapsedProjects";

  function loadCollapsed(): Record<string, boolean> {
    try {
      const parsed = JSON.parse(localStorage.getItem(COLLAPSED_KEY) ?? "[]");
      const ids: string[] = Array.isArray(parsed)
        ? parsed.filter((i) => typeof i === "string")
        : [];
      return Object.fromEntries(ids.map((id) => [id, true]));
    } catch {
      return {}; // storage unavailable (prerender, private mode): start expanded
    }
  }

  let collapsed = $state<Record<string, boolean>>(loadCollapsed());

  function setCollapsed(projectId: string, value: boolean) {
    collapsed[projectId] = value;
    try {
      const live = new Set(projectStore.projects.map((p) => p.id));
      const ids = Object.keys(collapsed).filter((id) => collapsed[id] && live.has(id));
      localStorage.setItem(COLLAPSED_KEY, JSON.stringify(ids));
    } catch {
      // preference only: ignore storage failures
    }
  }

  // Collapsed worktrees (terminal list hidden), keyed by worktree path. Same UI-preference
  // treatment as collapsed projects: localStorage only, failures ignored, dead paths pruned.
  const COLLAPSED_WT_KEY = "trunks.sidebar.collapsedWorktrees";

  function loadCollapsedWorktrees(): Record<string, boolean> {
    try {
      const parsed = JSON.parse(localStorage.getItem(COLLAPSED_WT_KEY) ?? "[]");
      const paths: string[] = Array.isArray(parsed)
        ? parsed.filter((i) => typeof i === "string")
        : [];
      return Object.fromEntries(paths.map((p) => [p, true]));
    } catch {
      return {};
    }
  }

  let collapsedWorktrees = $state<Record<string, boolean>>(loadCollapsedWorktrees());

  function setWorktreeCollapsed(path: string, value: boolean) {
    collapsedWorktrees[path] = value;
    try {
      const live = projectStore.projects.flatMap((p) => p.worktrees.map((w) => w.path));
      const kept = pruneCollapsedWorktrees(
        Object.keys(collapsedWorktrees).filter((k) => collapsedWorktrees[k]),
        live
      );
      localStorage.setItem(COLLAPSED_WT_KEY, JSON.stringify(kept));
    } catch {
      // preference only: ignore storage failures
    }
  }

  /** Middle ellipsis: the tail of a path is what tells worktrees apart. */
  function shortPath(path: string, max = 40): string {
    if (path.length <= max) return path;
    const head = Math.ceil((max - 1) * 0.35);
    const tail = max - 1 - head;
    return `${path.slice(0, head)}…${path.slice(path.length - tail)}`;
  }

  function openCreateWorktree(project: ProjectState) {
    targetProject = project;
    createWorktreeOpen = true;
  }

  function openRemoveWorktree(project: ProjectState, worktree: GitWorktreeInfo) {
    targetProject = project;
    targetWorktree = worktree;
    removeWorktreeOpen = true;
  }

  function openRemoveProject(project: ProjectState) {
    targetProject = project;
    removeProjectOpen = true;
  }

  function handleWorktreeCreated(wt: GitWorktreeInfo, openTerminal: boolean) {
    if (targetProject) setCollapsed(targetProject.id, false); // make the new worktree visible
    if (openTerminal && targetProject) {
      const wtName = wt.branch || wt.path.split(/[\\/]/).pop() || "worktree";
      onOpenTerminal(wt.path, `wt:${wtName}`, wtName, targetProject.name);
    }
  }

  function selectWorktree(project: ProjectState, worktree: GitWorktreeInfo) {
    projectStore.selectedProjectId = project.id;
    projectStore.selectedWorktreePath = worktree.path;
    onSelectWorktree?.(project.id, worktree.path);
  }
</script>

<div class="worktree-list-container">
  <div class="sidebar-header">
    <div class="header-title">
      <h3>Projects</h3>
    </div>
    <div class="header-actions">
      <button
        class="icon-btn"
        class:spinning={projectStore.isRefreshing}
        onclick={() => projectStore.refreshAll()}
        title="Refresh all projects & worktrees"
      >
        ↻
      </button>
      <button
        class="icon-btn"
        onclick={() => (settingsOpen = true)}
        title="Settings"
        aria-label="Settings"
      >
        ⚙
      </button>
      <button
        class="action-btn-sm primary"
        onclick={() => (addProjectOpen = true)}
        title="Add a local Git project"
      >
        + Add Project
      </button>
    </div>
  </div>

  <div class="sidebar-content">
    {#if projectStore.projects.length === 0}
      <div class="empty-state">
        <p class="empty-text">No projects registered yet.</p>
        <p class="empty-subtext">
          Add an existing Git project to discover and manage its worktrees and launch terminal workspaces.
        </p>
        <button
          class="add-repo-btn"
          onclick={() => (addProjectOpen = true)}
        >
          + Add Project
        </button>
      </div>
    {:else}
      {#each projectStore.projects as project (project.id)}
        {@const isCollapsed = Boolean(collapsed[project.id])}
        <div
          class="project-card"
          class:stale={project.is_stale}
          class:collapsed={isCollapsed}
          class:active={projectStore.selectedProjectId === project.id}
        >
          <div class="project-header">
            <button
              type="button"
              class="project-info"
              title={project.path}
              aria-expanded={!isCollapsed}
              onclick={() => setCollapsed(project.id, !isCollapsed)}
            >
              <span class="chevron" aria-hidden="true">{isCollapsed ? "▸" : "▾"}</span>
              <span class="project-name">{project.name}</span>
              <span class="worktree-count" title="Worktrees">{project.worktrees.length}</span>
              {#if project.is_stale}
                <span class="stale-badge" title={project.stale_reason || "Directory not found"}>
                  ⚠️ Stale
                </span>
              {/if}
            </button>

            <div class="project-actions">
              <button
                class="icon-btn-sm"
                onclick={() => openCreateWorktree(project)}
                title="Create new worktree in this project"
              >
                + wt
              </button>
              <button
                class="icon-btn-sm"
                class:spinning={project.isLoading}
                onclick={() => projectStore.refreshProject(project.id)}
                title="Refresh worktrees from Git"
              >
                ↻
              </button>
              <button
                class="icon-btn-sm danger"
                onclick={() => openRemoveProject(project)}
                title="Remove project from workspace"
              >
                ✕
              </button>
            </div>
          </div>

          {#if !isCollapsed}
          <div class="worktree-items-list">
            {#if project.isLoading && project.worktrees.length === 0}
              <div class="loading-state">Querying git worktrees...</div>
            {:else if project.worktrees.length === 0}
              <div class="no-worktrees">No worktrees discovered.</div>
            {:else}
              {#each project.worktrees as wt, idx (wt.path)}
                {@const isMain = idx === 0}
                {@const isSelected = projectStore.selectedWorktreePath === wt.path}
                {@const wtName = wt.branch || wt.path.split(/[\\/]/).pop() || "worktree"}
                {@const terminals = terminalsFor(wt.path)}
                {@const wtCollapsed = Boolean(collapsedWorktrees[wt.path])}
                {@const summary = aggregateTerminalState(terminals)}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div
                  class="worktree-item"
                  class:selected={isSelected}
                  class:stale={Boolean(wt.prunable)}
                  title={`${wt.path}\n${wt.branch ?? "detached"}${wt.head ? ` @ ${wt.head}` : ""}`}
                  onclick={() => selectWorktree(project, wt)}
                >
                  <div class="worktree-main-row">
                    <div class="worktree-branch-box">
                      {#if terminals.length > 0}
                        <button
                          type="button"
                          class="wt-chevron"
                          aria-expanded={!wtCollapsed}
                          aria-label={wtCollapsed ? "Show terminals" : "Hide terminals"}
                          title={wtCollapsed ? "Show terminals" : "Hide terminals"}
                          onclick={(e) => {
                            e.stopPropagation();
                            setWorktreeCollapsed(wt.path, !wtCollapsed);
                          }}
                        >
                          {wtCollapsed ? "▸" : "▾"}
                        </button>
                      {/if}
                      <span class="branch-icon">⎇</span>
                      {#if wtCollapsed && summary.state}
                        <span
                          class="term-dot"
                          class:restored={summary.state === "restored"}
                          title={summary.state === "running"
                            ? `${summary.count} terminal(s), at least one running`
                            : `${summary.count} terminal(s), restored and not running`}
                        ></span>
                      {/if}
                      {#if wtCollapsed && summary.count > 1}
                        <span class="term-count" title="Terminals">{summary.count}</span>
                      {/if}
                      <span class="branch-name" title={wt.branch || "detached"}>
                        {wt.branch || (wt.head ? `detached (${wt.head.substring(0, 7)})` : "detached")}
                      </span>
                    </div>

                    <div class="worktree-tags">
                      {#if isMain}
                        <span class="tag primary" title="Primary worktree of the project">primary</span>
                      {/if}
                      {#if wt.head && wt.branch}
                        <span class="tag commit" title={`HEAD: ${wt.head}`}>
                          {wt.head.substring(0, 7)}
                        </span>
                      {/if}
                      {#if wt.locked}
                        <span class="tag locked" title={wt.locked || "Locked"}>🔒</span>
                      {/if}
                      {#if wt.prunable}
                        <span class="tag stale" title={wt.prunable}>⚠️ stale</span>
                      {/if}
                    </div>

                    <div class="worktree-actions">
                      <button
                        class="wt-btn terminal-btn"
                        onclick={(e) => {
                          e.stopPropagation();
                          onOpenTerminal(wt.path, `wt:${wtName}`, wtName, project.name);
                        }}
                        disabled={Boolean(wt.prunable)}
                        title={wt.prunable
                          ? "Worktree directory is missing; prune it or restore it first"
                          : "Open or focus the terminal of this worktree"}
                        aria-label="Open terminal in this worktree"
                      >
                        &gt;_
                      </button>
                      {#if !isMain}
                        <button
                          class="wt-btn remove-btn"
                          onclick={(e) => {
                            e.stopPropagation();
                            openRemoveWorktree(project, wt);
                          }}
                          title="Remove worktree"
                          aria-label="Remove worktree"
                        >
                          🗑
                        </button>
                      {/if}
                    </div>
                  </div>

                  {#if isSelected}
                    <div class="worktree-path-row">
                      <span class="worktree-path" title={wt.path}>{shortPath(wt.path)}</span>
                    </div>
                  {/if}
                </div>
                {#if !wtCollapsed && terminals.length > 0}
                  <div class="terminal-list">
                    {#each terminals as tab (tab.id)}
                      {@const dot = terminalDotState(tab)}
                      <button
                        type="button"
                        class="terminal-row"
                        class:active={tab.id === activeTerminalId}
                        title={`${tab.title}\n${dot}`}
                        aria-label={`${tab.title} (${dot})`}
                        onclick={() => onSelectTerminal?.(project.id, wt.path, tab.id)}
                      >
                        <span
                          class="term-dot"
                          class:restored={dot === "restored"}
                          class:stopped={dot === "stopped"}
                        ></span>
                        <span class="terminal-title">{tab.title}</span>
                      </button>
                    {/each}
                  </div>
                {/if}
              {/each}
            {/if}
          </div>
          {/if}
        </div>
      {/each}
    {/if}
  </div>
</div>

<AddProjectModal
  bind:isOpen={addProjectOpen}
  onProjectAdded={(id) => {
    projectStore.selectedProjectId = id;
  }}
/>

<CreateWorktreeModal
  bind:isOpen={createWorktreeOpen}
  project={targetProject}
  onWorktreeCreated={handleWorktreeCreated}
/>

<RemoveWorktreeModal
  bind:isOpen={removeWorktreeOpen}
  project={targetProject}
  worktree={targetWorktree}
  onRemoved={(path) => onWorktreeRemoved?.(path)}
/>

<SettingsModal bind:isOpen={settingsOpen} />

<RemoveProjectModal
  bind:isOpen={removeProjectOpen}
  project={targetProject}
  onRemoved={(paths) => onProjectRemoved?.(paths)}
/>

<style>
  .worktree-list-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #181825;
    color: #cdd6f4;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    background: #11111b;
    border-bottom: 1px solid #313244;
  }

  .header-title {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .header-title h3 {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #a6adc8;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .icon-btn {
    background: #313244;
    border: none;
    color: #cdd6f4;
    width: 26px;
    height: 26px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-size: 14px;
    transition: all 0.15s ease;
  }

  .icon-btn:hover {
    background: #45475a;
    color: #9448f8;
  }

  .icon-btn.spinning {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .action-btn-sm {
    background: #9448f8;
    color: #ffffff;
    border: none;
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .action-btn-sm:hover {
    background: #aa6dfb;
  }

  .sidebar-content {
    flex: 1;
    overflow-y: auto;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .empty-state {
    padding: 24px 12px;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }

  .empty-text {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: #cdd6f4;
  }

  .empty-subtext {
    margin: 0;
    font-size: 11px;
    color: #a6adc8;
    line-height: 1.4;
    max-width: 220px;
  }

  .add-repo-btn {
    margin-top: 8px;
    background: #313244;
    border: 1px solid #45475a;
    color: #9448f8;
    padding: 6px 12px;
    border-radius: 5px;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .add-repo-btn:hover {
    background: #45475a;
    border-color: #9448f8;
  }

  .project-card {
    background: #1e1e2e;
    border: 1px solid #313244;
    border-radius: 6px;
    overflow: hidden;
    /* A flex column would otherwise squeeze cards (overflow:hidden lets them shrink to
       zero height), so nothing overflowed and the sidebar never scrolled. */
    flex-shrink: 0;
    transition: border-color 0.15s ease;
  }

  .project-card.collapsed .project-header {
    border-bottom: none;
  }

  .project-card.active {
    border-color: #9448f8;
  }

  .project-card.stale {
    border-color: #f9e2af;
  }

  .project-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
    background: #181825;
    border-bottom: 1px solid #313244;
  }

  .project-info {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    flex: 1;
    background: none;
    border: none;
    padding: 0;
    margin: 0;
    font: inherit;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }

  .project-info:hover .project-name {
    color: #9448f8;
  }

  .chevron {
    width: 10px;
    flex-shrink: 0;
    font-size: 10px;
    color: #a6adc8;
  }

  .worktree-count {
    flex-shrink: 0;
    font-size: 9px;
    padding: 1px 5px;
    border-radius: 8px;
    background: #313244;
    color: #a6adc8;
  }

  .project-name {
    font-size: 12px;
    font-weight: 600;
    color: #cdd6f4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .stale-badge {
    background: rgba(249, 226, 175, 0.15);
    color: #f9e2af;
    border: 1px solid #f9e2af;
    font-size: 9px;
    font-weight: bold;
    padding: 1px 4px;
    border-radius: 3px;
  }

  .project-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .icon-btn-sm {
    background: transparent;
    border: none;
    color: #a6adc8;
    padding: 2px 5px;
    border-radius: 3px;
    font-size: 11px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .icon-btn-sm:hover {
    background: #313244;
    color: #9448f8;
  }

  .icon-btn-sm.danger:hover {
    background: rgba(243, 139, 168, 0.2);
    color: #f38ba8;
  }

  .icon-btn-sm.spinning {
    animation: spin 1s linear infinite;
  }

  .worktree-items-list {
    display: flex;
    flex-direction: column;
    padding: 4px;
    gap: 3px;
  }

  .loading-state,
  .no-worktrees {
    padding: 8px 6px;
    font-size: 11px;
    color: #a6adc8;
    font-style: italic;
  }

  .worktree-item {
    background: #11111b;
    border: 1px solid #313244;
    border-radius: 5px;
    padding: 5px 8px;
    display: flex;
    flex-direction: column;
    gap: 3px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .worktree-item:hover {
    border-color: #45475a;
    background: #141420;
  }

  .worktree-item.selected {
    border-color: #9448f8;
    background: rgba(148, 72, 248, 0.12);
  }

  .worktree-item.stale {
    border-color: #f9e2af;
    background: rgba(249, 226, 175, 0.05);
  }

  .worktree-main-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
  }

  .worktree-branch-box {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    flex: 1;
  }

  .branch-icon {
    color: #9448f8;
    font-size: 12px;
  }

  .branch-name {
    font-size: 12px;
    font-weight: 600;
    color: #cdd6f4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .worktree-tags {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .tag {
    font-size: 9px;
    padding: 1px 4px;
    border-radius: 3px;
    font-weight: 500;
  }

  .tag.primary {
    background: rgba(166, 227, 161, 0.15);
    color: #a6e3a1;
    border: 1px solid #a6e3a1;
  }

  .tag.commit {
    background: #313244;
    color: #a6adc8;
    font-family: monospace;
  }

  .tag.locked {
    background: rgba(249, 226, 175, 0.15);
    color: #f9e2af;
  }

  .tag.stale {
    background: rgba(249, 226, 175, 0.15);
    color: #f9e2af;
    border: 1px solid #f9e2af;
  }

  .worktree-path-row {
    display: flex;
    min-width: 0;
  }

  .worktree-path {
    font-family: monospace;
    font-size: 10px;
    color: #6c7086;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .worktree-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
  }

  .term-dot {
    flex-shrink: 0;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #3fb950;
    box-shadow: 0 0 5px rgba(63, 185, 80, 0.7);
  }

  .term-dot.restored {
    background: #f0883e;
    box-shadow: 0 0 5px rgba(240, 136, 62, 0.7);
  }

  .term-dot.stopped {
    background: #585b70;
    box-shadow: none;
  }

  .wt-chevron {
    flex-shrink: 0;
    width: 12px;
    padding: 0;
    background: none;
    border: none;
    color: #a6adc8;
    font-size: 10px;
    cursor: pointer;
  }

  .wt-chevron:hover {
    color: #9448f8;
  }

  .term-count {
    flex-shrink: 0;
    font-size: 9px;
    padding: 1px 5px;
    border-radius: 8px;
    background: #313244;
    color: #a6adc8;
  }

  .terminal-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 0 0 2px 18px;
  }

  .terminal-row {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding: 3px 8px;
    background: transparent;
    border: none;
    /* Reserved so the active marker does not shift the row. */
    border-left: 2px solid transparent;
    border-radius: 0 4px 4px 0;
    color: #a6adc8;
    font: inherit;
    font-size: 11px;
    text-align: left;
    cursor: pointer;
  }

  .terminal-row:hover {
    background: #141420;
    color: #cdd6f4;
  }

  .terminal-row.active {
    border-left-color: #9448f8;
    color: #cdd6f4;
  }

  .terminal-title {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .wt-btn {
    border: none;
    border-radius: 3px;
    font-size: 10px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .wt-btn.terminal-btn {
    background: #313244;
    color: #9448f8;
    padding: 3px 6px;
    font-family: monospace;
    font-weight: 700;
  }

  .wt-btn.terminal-btn:hover {
    background: #9448f8;
    color: #ffffff;
  }

  .wt-btn.terminal-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .wt-btn.remove-btn {
    background: transparent;
    color: #a6adc8;
    padding: 2px 4px;
  }

  .wt-btn.remove-btn:hover {
    background: rgba(243, 139, 168, 0.2);
    color: #f38ba8;
  }
</style>
