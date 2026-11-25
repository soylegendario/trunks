<script lang="ts">
  import { projectStore } from "$lib/projects.svelte";

  let { isOpen = $bindable(false), onProjectAdded }: {
    isOpen: boolean;
    onProjectAdded?: (projectId: string) => void;
  } = $props();

  let pathInput = $state("");
  let isLoading = $state(false);
  let errorMessage = $state<string | null>(null);

  function handleClose() {
    if (isLoading) return;
    isOpen = false;
    errorMessage = null;
    pathInput = "";
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      handleClose();
    }
  }

  async function handleAdd() {
    const trimmed = pathInput.trim();
    if (!trimmed) {
      errorMessage = "Please enter a valid directory path.";
      return;
    }

    isLoading = true;
    errorMessage = null;

    try {
      const project = await projectStore.addProject(trimmed);
      isOpen = false;
      pathInput = "";
      onProjectAdded?.(project.id);
    } catch (err: any) {
      errorMessage = String(err);
    } finally {
      isLoading = false;
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-backdrop" onclick={handleClose}>
    <div class="modal-dialog" onclick={(e) => e.stopPropagation()}>
      <div class="modal-header">
        <h3>Add Project</h3>
        <button class="close-btn" onclick={handleClose} disabled={isLoading}>×</button>
      </div>

      <div class="modal-body">
        <p class="description">
          Enter the absolute path to an existing local Git project. Trunks will validate the project and discover all its worktrees.
        </p>

        <div class="form-group">
          <label for="repo-path">Project Path</label>
          <input
            id="repo-path"
            type="text"
            placeholder="/Users/username/workspace/my-repo"
            bind:value={pathInput}
            onkeydown={(e) => e.key === "Enter" && handleAdd()}
            disabled={isLoading}
          />
        </div>

        {#if errorMessage}
          <div class="error-banner">
            <span class="error-icon">⚠️</span>
            <span class="error-text">{errorMessage}</span>
          </div>
        {/if}
      </div>

      <div class="modal-footer">
        <button class="btn secondary" onclick={handleClose} disabled={isLoading}>
          Cancel
        </button>
        <button class="btn primary" onclick={handleAdd} disabled={isLoading || !pathInput.trim()}>
          {#if isLoading}
            Validating...
          {:else}
            Add Project
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
    max-width: 520px;
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
    color: #a6adc8;
    line-height: 1.4;
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
  }

  .form-group input {
    background: #11111b;
    border: 1px solid #45475a;
    border-radius: 5px;
    padding: 9px 12px;
    color: #cdd6f4;
    font-size: 13px;
    outline: none;
    transition: border-color 0.15s ease;
  }

  .form-group input:focus {
    border-color: var(--accent-primary, #9448f8);
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
