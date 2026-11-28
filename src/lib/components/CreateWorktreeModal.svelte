<script lang="ts">
  import { gitListBranches, type GitBranchInfo, type GitWorktreeInfo } from "$lib/git";
  import { projectStore, type ProjectState } from "$lib/projects.svelte";
  import { pickDirectory } from "$lib/dialog";
  import { checkPathExists } from "$lib/storage";
  import {
    suggestedWorktreePath,
    worktreePathInContainer,
    worktreesBrowseStart,
  } from "$lib/project-logic";

  let {
    isOpen = $bindable(false),
    project,
    onWorktreeCreated,
  }: {
    isOpen: boolean;
    project: ProjectState | null;
    onWorktreeCreated?: (wt: GitWorktreeInfo, openTerminal: boolean) => void;
  } = $props();

  let branchMode = $state<"existing" | "new">("new");
  let selectedBranch = $state("");
  let newBranchName = $state("");
  let customPath = $state("");
  let openTerminalChecked = $state(true);

  let branches = $state<GitBranchInfo[]>([]);
  let isLoadingBranches = $state(false);
  let isSubmitting = $state(false);
  let errorMessage = $state<string | null>(null);

  // Derived effective branch name
  let effectiveBranch = $derived(
    branchMode === "new" ? newBranchName.trim() : selectedBranch.trim()
  );

  // Default destination: <project>/.worktrees/<branch>
  let suggestedPath = $derived(
    project ? suggestedWorktreePath(project.path, effectiveBranch) : ""
  );

  // Track checked-out branches
  let checkedOutBranches = $derived.by(() => {
    if (!project) return new Set<string>();
    return new Set(
      project.worktrees
        .map((wt) => wt.branch)
        .filter((b): b is string => Boolean(b))
    );
  });

  // Reset and load branches whenever dialog opens
  $effect(() => {
    if (isOpen && project) {
      errorMessage = null;
      newBranchName = "";
      customPath = "";
      branchMode = "new";
      loadBranches();
    }
  });

  async function loadBranches() {
    if (!project) return;
    isLoadingBranches = true;
    try {
      const list = await gitListBranches(project.path);
      // Filter out remote branches if local exists with same name, or keep local first
      const local = list.filter((b) => !b.is_remote);
      const remote = list.filter((b) => b.is_remote && !local.some((l) => b.name.endsWith("/" + l.name)));
      branches = [...local, ...remote];

      const available = branches.find((b) => !checkedOutBranches.has(b.name));
      if (available) {
        selectedBranch = available.name;
      } else if (branches.length > 0) {
        selectedBranch = branches[0].name;
      }
    } catch (err: any) {
      console.warn("Could not load branches:", err);
      branches = [];
    } finally {
      isLoadingBranches = false;
    }
  }

  async function handleBrowse() {
    if (!project) return;
    if (!effectiveBranch) {
      errorMessage = "Enter a branch name first.";
      return;
    }
    const start = worktreesBrowseStart(project.path);
    const picked = await pickDirectory((await checkPathExists(start)) ? start : project.path);
    if (picked) {
      customPath = worktreePathInContainer(picked, effectiveBranch);
      errorMessage = null;
    }
  }

  function handleClose() {
    if (isSubmitting) return;
    isOpen = false;
    errorMessage = null;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      handleClose();
    }
  }

  async function handleCreate() {
    if (!project) return;
    const branchToUse = effectiveBranch;
    if (!branchToUse) {
      errorMessage = "Please enter or select a branch name.";
      return;
    }

    if (branchToUse.startsWith("-")) {
      errorMessage = "Branch name cannot start with a hyphen (-).";
      return;
    }

    const finalPath = (customPath.trim() || suggestedPath).trim();
    if (!finalPath) {
      errorMessage = "Please specify a destination worktree path.";
      return;
    }

    isSubmitting = true;
    errorMessage = null;

    try {
      const wt = await projectStore.createWorktree(
        project.id,
        finalPath,
        branchToUse,
        branchMode === "new"
      );
      isOpen = false;
      onWorktreeCreated?.(wt, openTerminalChecked);
    } catch (err: any) {
      errorMessage = String(err);
    } finally {
      isSubmitting = false;
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen && project}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-backdrop" onclick={handleClose}>
    <div class="modal-dialog" onclick={(e) => e.stopPropagation()}>
      <div class="modal-header">
        <h3>Create Git Worktree</h3>
        <button class="close-btn" onclick={handleClose} disabled={isSubmitting}>×</button>
      </div>

      <div class="modal-body">
        <div class="project-info-row">
          <span class="label">Project:</span>
          <span class="value">{project.name}</span>
          <span class="path-badge" title={project.path}>{project.path}</span>
        </div>

        <div class="mode-selector">
          <button
            class="mode-btn"
            class:active={branchMode === "new"}
            onclick={() => (branchMode = "new")}
            type="button"
          >
            Create New Branch
          </button>
          <button
            class="mode-btn"
            class:active={branchMode === "existing"}
            onclick={() => (branchMode = "existing")}
            type="button"
          >
            Checkout Existing Branch
          </button>
        </div>

        {#if branchMode === "new"}
          <div class="form-group">
            <label for="new-branch-input">New Branch Name</label>
            <input
              id="new-branch-input"
              type="text"
              placeholder="feat/my-new-feature"
              bind:value={newBranchName}
              disabled={isSubmitting}
            />
          </div>
        {:else}
          <div class="form-group">
            <label for="existing-branch-select">Select Branch</label>
            {#if isLoadingBranches}
              <div class="loading-hint">Loading project branches...</div>
            {:else if branches.length === 0}
              <div class="empty-hint">No branches found.</div>
            {:else}
              <select
                id="existing-branch-select"
                bind:value={selectedBranch}
                disabled={isSubmitting}
              >
                {#each branches as b}
                  {@const isCheckedOut = checkedOutBranches.has(b.name)}
                  <option value={b.name} disabled={isCheckedOut}>
                    {b.is_remote ? "🌐 " : "⎇ "} {b.name} {isCheckedOut ? "(already checked out)" : ""}
                  </option>
                {/each}
              </select>
            {/if}
          </div>
        {/if}

        <div class="form-group">
          <label for="worktree-path-input">
            Destination Directory
            <span class="sub-label">(Absolute path on disk)</span>
          </label>
          <div class="path-row">
            <input
              id="worktree-path-input"
              type="text"
              placeholder={suggestedPath || "<project>/.worktrees/<branch>"}
              bind:value={customPath}
              disabled={isSubmitting}
            />
            <button type="button" class="btn secondary" onclick={handleBrowse} disabled={isSubmitting}>
              Browse…
            </button>
          </div>
          {#if !customPath.trim() && suggestedPath}
            <span class="path-preview">
              Suggested: <code>{suggestedPath}</code>
            </span>
          {/if}
        </div>

        <div class="checkbox-group">
          <label>
            <input
              type="checkbox"
              bind:checked={openTerminalChecked}
              disabled={isSubmitting}
            />
            <span>Open new terminal tab in this worktree immediately</span>
          </label>
        </div>

        {#if errorMessage}
          <div class="error-banner">
            <span class="error-icon">⚠️</span>
            <span class="error-text">{errorMessage}</span>
          </div>
        {/if}
      </div>

      <div class="modal-footer">
        <button class="btn secondary" onclick={handleClose} disabled={isSubmitting}>
          Cancel
        </button>
        <button
          class="btn primary"
          onclick={handleCreate}
          disabled={isSubmitting || !effectiveBranch}
        >
          {#if isSubmitting}
            Creating Worktree...
          {:else}
            Create Worktree
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.65);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(2px);
  }

  .modal-dialog {
    background: #1e1e2e;
    border: 1px solid #313244;
    border-radius: 8px;
    width: 90%;
    max-width: 560px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 18px;
    background: #181825;
    border-bottom: 1px solid #313244;
  }

  .modal-header h3 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: #cdd6f4;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: #a6adc8;
    font-size: 20px;
    cursor: pointer;
    line-height: 1;
    padding: 0 4px;
  }

  .close-btn:hover:not(:disabled) {
    color: #f38ba8;
  }

  .modal-body {
    padding: 18px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .project-info-row {
    display: flex;
    align-items: center;
    gap: 8px;
    background: #11111b;
    padding: 8px 12px;
    border-radius: 6px;
    border: 1px solid #313244;
    font-size: 12px;
  }

  .project-info-row .label {
    color: #a6adc8;
  }

  .project-info-row .value {
    color: #cdd6f4;
    font-weight: 600;
  }

  .project-info-row .path-badge {
    margin-left: auto;
    color: #6c7086;
    font-family: monospace;
    font-size: 11px;
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mode-selector {
    display: flex;
    gap: 6px;
    background: #11111b;
    padding: 4px;
    border-radius: 6px;
    border: 1px solid #313244;
  }

  .mode-btn {
    flex: 1;
    background: transparent;
    border: none;
    color: #a6adc8;
    padding: 6px 12px;
    border-radius: 4px;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .mode-btn.active {
    background: #313244;
    color: var(--accent-primary, #9448f8);
    font-weight: 600;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-group label {
    font-size: 12px;
    font-weight: 500;
    color: #cdd6f4;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .sub-label {
    font-size: 11px;
    color: #a6adc8;
    font-weight: normal;
  }

  .path-row {
    display: flex;
    gap: 8px;
  }

  .path-row input {
    flex: 1;
    min-width: 0;
  }

  .form-group input,
  .form-group select {
    background: #11111b;
    border: 1px solid #45475a;
    border-radius: 5px;
    padding: 9px 12px;
    color: #cdd6f4;
    font-size: 13px;
    outline: none;
    transition: border-color 0.15s ease;
  }

  .form-group input:focus,
  .form-group select:focus {
    border-color: var(--accent-primary, #9448f8);
  }

  .loading-hint,
  .empty-hint {
    font-size: 12px;
    color: #a6adc8;
    padding: 8px 0;
  }

  .path-preview {
    font-size: 11px;
    color: #a6adc8;
  }

  .path-preview code {
    color: var(--accent-primary, #9448f8);
    background: #11111b;
    padding: 2px 5px;
    border-radius: 3px;
    font-family: monospace;
  }

  .checkbox-group label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: #cdd6f4;
    cursor: pointer;
  }

  .error-banner {
    background: rgba(243, 139, 168, 0.12);
    border: 1px solid #f38ba8;
    border-radius: 5px;
    padding: 10px 12px;
    display: flex;
    align-items: flex-start;
    gap: 8px;
    font-size: 12px;
    color: #f38ba8;
    word-break: break-word;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 14px 18px;
    background: #181825;
    border-top: 1px solid #313244;
  }

  .btn {
    padding: 7px 14px;
    border-radius: 5px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    border: none;
    transition: all 0.15s ease;
  }

  .btn.secondary {
    background: #313244;
    color: #cdd6f4;
  }

  .btn.secondary:hover:not(:disabled) {
    background: #45475a;
  }

  .btn.primary {
    background: var(--accent-primary, #9448f8);
    color: #ffffff;
  }

  .btn.primary:hover:not(:disabled) {
    background: var(--accent-hover, #aa6dfb);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
