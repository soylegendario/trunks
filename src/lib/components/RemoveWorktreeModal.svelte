<script lang="ts">
  import type { GitWorktreeInfo } from "$lib/git";
  import { projectStore, type ProjectState } from "$lib/projects.svelte";

  let {
    isOpen = $bindable(false),
    project,
    worktree,
    onRemoved,
  }: {
    isOpen: boolean;
    project: ProjectState | null;
    worktree: GitWorktreeInfo | null;
    onRemoved?: () => void;
  } = $props();

  let isForceChecked = $state(false);
  let isSubmitting = $state(false);
  let errorMessage = $state<string | null>(null);

  let isMainWorktree = $derived.by(() => {
    if (!project || !worktree || project.worktrees.length === 0) return false;
    return project.worktrees[0].path === worktree.path;
  });

  $effect(() => {
    if (isOpen) {
      isForceChecked = false;
      errorMessage = null;
    }
  });

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

  async function handleRemove() {
    if (!project || !worktree || isMainWorktree) return;

    isSubmitting = true;
    errorMessage = null;

    try {
      await projectStore.removeWorktree(project.id, worktree.path, isForceChecked);
      isOpen = false;
      onRemoved?.();
    } catch (err: any) {
      errorMessage = String(err);
    } finally {
      isSubmitting = false;
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen && project && worktree}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-backdrop" onclick={handleClose}>
    <div class="modal-dialog" onclick={(e) => e.stopPropagation()}>
      <div class="modal-header">
        <h3>{isMainWorktree ? "Cannot Remove Main Worktree" : "Remove Worktree"}</h3>
        <button class="close-btn" onclick={handleClose} disabled={isSubmitting}>×</button>
      </div>

      <div class="modal-body">
        {#if isMainWorktree}
          <div class="warning-banner">
            <span class="warning-icon">ℹ️</span>
            <div>
              <strong>Primary Worktree Protected</strong>
              <p>
                The primary worktree (<code>{worktree.path}</code>) is the core project root and cannot be removed by Git.
              </p>
            </div>
          </div>
        {:else}
          <p class="description">
            Are you sure you want to remove the worktree for branch
            <strong>{worktree.branch || "detached"}</strong>?
          </p>

          <div class="worktree-details">
            <div class="detail-row">
              <span class="label">Branch:</span>
              <span class="val">⎇ {worktree.branch || "detached"}</span>
            </div>
            <div class="detail-row">
              <span class="label">Directory:</span>
              <span class="val path" title={worktree.path}>{worktree.path}</span>
            </div>
          </div>

          <div class="checkbox-group">
            <label>
              <input
                type="checkbox"
                bind:checked={isForceChecked}
                disabled={isSubmitting}
              />
              <span>
                <strong>Force removal (--force)</strong>
                <span class="sub">Allow removal even if uncommitted or untracked changes exist</span>
              </span>
            </label>
          </div>

          {#if errorMessage}
            <div class="error-banner">
              <span class="error-icon">⚠️</span>
              <span class="error-text">{errorMessage}</span>
            </div>
          {/if}
        {/if}
      </div>

      <div class="modal-footer">
        <button class="btn secondary" onclick={handleClose} disabled={isSubmitting}>
          {isMainWorktree ? "OK" : "Cancel"}
        </button>
        {#if !isMainWorktree}
          <button
            class="btn danger"
            onclick={handleRemove}
            disabled={isSubmitting}
          >
            {#if isSubmitting}
              Removing...
            {:else}
              Remove Worktree
            {/if}
          </button>
        {/if}
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
    max-width: 500px;
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

  .description {
    margin: 0;
    font-size: 13px;
    color: #cdd6f4;
    line-height: 1.4;
  }

  .worktree-details {
    background: #11111b;
    border: 1px solid #313244;
    border-radius: 6px;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
  }

  .detail-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .detail-row .label {
    color: #a6adc8;
    min-width: 60px;
  }

  .detail-row .val {
    color: #cdd6f4;
    font-weight: 500;
  }

  .detail-row .val.path {
    font-family: monospace;
    font-size: 11px;
    color: #9448f8;
    word-break: break-all;
  }

  .warning-banner {
    background: rgba(148, 72, 248, 0.1);
    border: 1px solid #9448f8;
    border-radius: 6px;
    padding: 12px;
    display: flex;
    gap: 10px;
    font-size: 12px;
    color: #cdd6f4;
  }

  .warning-banner p {
    margin: 4px 0 0 0;
    color: #a6adc8;
  }

  .checkbox-group label {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    font-size: 12px;
    color: #cdd6f4;
    cursor: pointer;
  }

  .checkbox-group .sub {
    display: block;
    font-size: 11px;
    color: #a6adc8;
    margin-top: 2px;
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

  .btn.danger {
    background: #f38ba8;
    color: #11111b;
  }

  .btn.danger:hover:not(:disabled) {
    background: #eba0ac;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
