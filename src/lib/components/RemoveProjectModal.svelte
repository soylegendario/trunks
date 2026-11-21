<script lang="ts">
  import { projectStore, type ProjectState } from "$lib/projects.svelte";

  let {
    isOpen = $bindable(false),
    project,
    onRemoved,
  }: {
    isOpen: boolean;
    project: ProjectState | null;
    onRemoved?: () => void;
  } = $props();

  let isSubmitting = $state(false);

  function handleClose() {
    if (isSubmitting) return;
    isOpen = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      handleClose();
    }
  }

  async function handleRemove() {
    if (!project) return;
    isSubmitting = true;
    try {
      await projectStore.removeProject(project.id);
      isOpen = false;
      onRemoved?.();
    } catch (err) {
      console.error("Failed to remove project:", err);
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
        <h3>Remove Project from Workspace</h3>
        <button class="close-btn" onclick={handleClose} disabled={isSubmitting}>×</button>
      </div>

      <div class="modal-body">
        <p class="description">
          Are you sure you want to remove <strong>{project.name}</strong> from your Trunks workspace?
        </p>

        <div class="info-box">
          <span class="info-icon">ℹ️</span>
          <span>
            This only removes the project reference from Trunks. <strong>Your project files and worktrees on disk will not be deleted or modified.</strong>
          </span>
        </div>

        <div class="path-display">
          <span class="label">Project Path:</span>
          <code>{project.path}</code>
        </div>
      </div>

      <div class="modal-footer">
        <button class="btn secondary" onclick={handleClose} disabled={isSubmitting}>
          Cancel
        </button>
        <button class="btn danger" onclick={handleRemove} disabled={isSubmitting}>
          {#if isSubmitting}
            Removing...
          {:else}
            Remove from Workspace
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
    max-width: 480px;
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

  .info-box {
    background: rgba(137, 180, 250, 0.1);
    border: 1px solid rgba(137, 180, 250, 0.3);
    border-radius: 6px;
    padding: 10px 12px;
    display: flex;
    gap: 8px;
    font-size: 12px;
    color: #a6adc8;
    line-height: 1.4;
  }

  .info-box strong {
    color: #cdd6f4;
  }

  .path-display {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
  }

  .path-display .label {
    color: #a6adc8;
  }

  .path-display code {
    background: #11111b;
    border: 1px solid #313244;
    padding: 6px 8px;
    border-radius: 4px;
    color: #89b4fa;
    font-family: monospace;
    font-size: 11px;
    word-break: break-all;
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
