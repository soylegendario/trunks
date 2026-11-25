<script lang="ts">
  import {
    getSnapshotsEnabled,
    setSnapshotsEnabled,
    snapshotClearAll,
    snapshotsDisableAndClear,
  } from "$lib/snapshots";
  import {
    CLEAR_NOTICE,
    DISABLE_NOTICE,
    ENABLE_NOTICE,
    applyClearAll,
    applyDisable,
    applyEnable,
    stepForToggle,
    type SettingsStep,
  } from "$lib/snapshot-settings-logic";
  import { themeStore } from "$lib/theme.svelte";
  import { THEME_LIST, type ThemeId } from "$lib/theme-logic";

  let { isOpen = $bindable(false) }: { isOpen: boolean } = $props();

  let enabled = $state(false);
  let step = $state<SettingsStep>("idle");
  let busy = $state(false);
  let message = $state<string | null>(null);
  let errorMessage = $state<string | null>(null);
  let themeMenuOpen = $state(false);

  function selectTheme(id: ThemeId) {
    themeStore.setTheme(id);
    themeMenuOpen = false;
  }

  const deps = {
    setEnabled: setSnapshotsEnabled,
    clearAll: snapshotClearAll,
    disableAndClear: snapshotsDisableAndClear,
    readEnabled: getSnapshotsEnabled,
  };

  // The backend is the source of truth: re-read the flag every time the dialog opens.
  $effect(() => {
    if (!isOpen) {
      themeMenuOpen = false;
      return;
    }
    step = "idle";
    message = null;
    errorMessage = null;
    themeMenuOpen = false;
    getSnapshotsEnabled()
      .then((v) => (enabled = v))
      .catch((err) => (errorMessage = `Could not read the setting: ${err}`));
  });

  function handleClose() {
    if (busy) return;
    themeMenuOpen = false;
    isOpen = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      if (themeMenuOpen) {
        themeMenuOpen = false;
        e.stopPropagation();
        return;
      }
      if (isOpen) handleClose();
    }
  }

  function requestToggle() {
    message = null;
    errorMessage = null;
    step = stepForToggle(enabled, !enabled);
  }

  async function confirm() {
    busy = true;
    errorMessage = null;
    message = null;
    try {
      if (step === "confirm-enable") {
        const r = await applyEnable(deps);
        enabled = r.enabled;
        if (!r.ok) errorMessage = `Could not enable terminal history: ${r.error}`;
        else message = "Terminal history is on. New output will be saved.";
      } else if (step === "confirm-disable") {
        const r = await applyDisable(deps);
        enabled = r.enabled;
        if (!r.ok) errorMessage = `Could not fully disable terminal history: ${r.error}`;
        else message = "Terminal history is off and the saved history was deleted.";
      } else if (step === "confirm-clear") {
        const r = await applyClearAll(deps);
        if (!r.ok) errorMessage = `Could not delete the saved history: ${r.error}`;
        else message = "Saved terminal history was deleted.";
      }
    } finally {
      step = "idle";
      busy = false;
    }
  }

  let notice = $derived(
    step === "confirm-enable"
      ? ENABLE_NOTICE
      : step === "confirm-disable"
        ? DISABLE_NOTICE
        : step === "confirm-clear"
          ? CLEAR_NOTICE
          : ""
  );
  let confirmLabel = $derived(
    step === "confirm-enable"
      ? "Save terminal history"
      : step === "confirm-disable"
        ? "Turn off and delete"
        : "Delete history"
  );
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-backdrop" onclick={handleClose}>
    <div class="modal-dialog" onclick={(e) => e.stopPropagation()}>
      <div class="modal-header">
        <h3>Settings</h3>
        <button class="close-btn" onclick={handleClose} disabled={busy} aria-label="Close">×</button>
      </div>

      <div class="modal-body">
        <div class="setting-row">
          <div class="setting-text">
            <strong>Appearance</strong>
            <span class="hint">Choose an accent color theme inspired by Dragon Ball.</span>
          </div>
          <div class="theme-dropdown-container">
            <button
              type="button"
              class="theme-select-btn"
              onclick={() => (themeMenuOpen = !themeMenuOpen)}
              aria-haspopup="listbox"
              aria-expanded={themeMenuOpen}
              disabled={busy}
            >
              <span class="theme-dot" style="background-color: {themeStore.currentTheme.dotColor}"></span>
              <span class="theme-name">{themeStore.currentTheme.name}</span>
              <span class="theme-arrow" class:open={themeMenuOpen}>▾</span>
            </button>

            {#if themeMenuOpen}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div class="theme-dropdown-backdrop" onclick={() => (themeMenuOpen = false)}></div>
              <div class="theme-menu" role="listbox" aria-label="Appearance — Theme">
                <div class="theme-menu-header">APPEARANCE — THEME</div>
                {#each THEME_LIST as theme (theme.id)}
                  <button
                    type="button"
                    class="theme-menu-item"
                    class:selected={theme.id === themeStore.currentThemeId}
                    role="option"
                    aria-selected={theme.id === themeStore.currentThemeId}
                    onclick={() => selectTheme(theme.id)}
                  >
                    <span class="theme-dot" style="background-color: {theme.dotColor}"></span>
                    <span class="theme-item-name">{theme.name}</span>
                    {#if theme.id === themeStore.currentThemeId}
                      <span class="theme-checkmark">✓</span>
                    {/if}
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        </div>

        <div class="divider"></div>

        <div class="setting-row">
          <div class="setting-text">
            <strong>Save terminal history</strong>
            <span class="hint">
              Off by default. Shows the previous text of a terminal after a restart. The process is
              not restored.
            </span>
          </div>
          <button
            class="switch"
            class:on={enabled}
            role="switch"
            aria-checked={enabled}
            aria-label="Save terminal history"
            onclick={requestToggle}
            disabled={busy || step !== "idle"}
          >
            <span class="knob"></span>
          </button>
        </div>

        {#if step !== "idle"}
          <div class="notice" role="alertdialog" aria-live="polite">
            <p>{notice}</p>
            <div class="notice-actions">
              <button class="btn secondary" onclick={() => (step = "idle")} disabled={busy}>
                Cancel
              </button>
              <button
                class="btn"
                class:danger={step !== "confirm-enable"}
                class:primary={step === "confirm-enable"}
                onclick={confirm}
                disabled={busy}
              >
                {busy ? "Working..." : confirmLabel}
              </button>
            </div>
          </div>
        {/if}

        <div class="setting-row">
          <div class="setting-text">
            <strong>Clear all history</strong>
            <span class="hint">Deletes every saved terminal snapshot from this computer.</span>
          </div>
          <button
            class="btn secondary"
            onclick={() => {
              message = null;
              errorMessage = null;
              step = "confirm-clear";
            }}
            disabled={busy || step !== "idle"}
          >
            Clear all history
          </button>
        </div>

        {#if message}<p class="status ok">{message}</p>{/if}
        {#if errorMessage}<p class="status err">{errorMessage}</p>{/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
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
    overflow: visible;
  }
  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 18px;
    background: #181825;
    border-bottom: 1px solid #313244;
    border-radius: 8px 8px 0 0;
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
    gap: 16px;
    color: #cdd6f4;
    font-size: 13px;
  }
  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .setting-text {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .divider {
    height: 1px;
    background: #313244;
    margin: -4px 0;
  }
  .theme-dropdown-container {
    position: relative;
  }
  .theme-select-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    background: #181825;
    border: 1px solid #313244;
    border-radius: 6px;
    padding: 7px 12px;
    color: #cdd6f4;
    font-size: 13px;
    cursor: pointer;
    min-width: 140px;
    justify-content: space-between;
    transition: all 0.15s ease;
  }
  .theme-select-btn:hover:not(:disabled) {
    border-color: #45475a;
    background: #1e1e2e;
  }
  .theme-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    flex-shrink: 0;
    box-shadow: 0 0 4px rgba(0, 0, 0, 0.4);
  }
  .theme-name {
    flex: 1;
    text-align: left;
    margin-left: 2px;
  }
  .theme-arrow {
    font-size: 11px;
    color: #a6adc8;
    transition: transform 0.15s ease;
  }
  .theme-arrow.open {
    transform: rotate(180deg);
  }
  .theme-dropdown-backdrop {
    position: fixed;
    inset: 0;
    z-index: 10;
  }
  .theme-menu {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    width: 230px;
    background: #181825;
    border: 1px solid #313244;
    border-radius: 8px;
    padding: 8px 0;
    box-shadow: 0 10px 25px rgba(0, 0, 0, 0.6);
    z-index: 20;
    display: flex;
    flex-direction: column;
  }
  .theme-menu-header {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.8px;
    text-transform: uppercase;
    color: #6c7086;
    padding: 6px 14px 8px;
    user-select: none;
  }
  .theme-menu-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 14px;
    background: transparent;
    border: none;
    color: #cdd6f4;
    font-size: 13px;
    cursor: pointer;
    text-align: left;
    width: 100%;
    transition: background 0.1s ease;
  }
  .theme-menu-item:hover,
  .theme-menu-item:focus-visible {
    background: #313244;
    outline: none;
  }
  .theme-menu-item.selected {
    color: #ffffff;
  }
  .theme-item-name {
    flex: 1;
  }
  .theme-checkmark {
    font-size: 14px;
    font-weight: bold;
    color: #cdd6f4;
  }
  .hint {
    color: #a6adc8;
    font-size: 12px;
    line-height: 1.4;
  }
  .switch {
    flex: none;
    width: 38px;
    height: 22px;
    border-radius: 11px;
    border: 1px solid #45475a;
    background: #313244;
    padding: 0;
    cursor: pointer;
    position: relative;
  }
  .switch.on {
    background: var(--accent-primary, #9448f8);
    border-color: var(--accent-primary, #9448f8);
  }
  .switch:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #cdd6f4;
    transition: transform 0.12s;
  }
  .switch.on .knob {
    transform: translateX(16px);
  }
  .notice {
    background: rgba(249, 226, 175, 0.08);
    border: 1px solid rgba(249, 226, 175, 0.35);
    border-radius: 6px;
    padding: 12px;
  }
  .notice p {
    margin: 0 0 12px;
    line-height: 1.5;
    color: #cdd6f4;
  }
  .notice-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }
  .btn {
    padding: 7px 14px;
    border-radius: 5px;
    border: 1px solid transparent;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
  }
  .btn:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .btn.secondary {
    background: transparent;
    border-color: #45475a;
    color: #cdd6f4;
  }
  .btn.primary {
    background: var(--accent-primary, #9448f8);
    color: #fff;
  }
  .btn.danger {
    background: #f38ba8;
    color: #11111b;
  }
  .status {
    margin: 0;
    font-size: 12px;
  }
  .status.ok {
    color: #a6e3a1;
  }
  .status.err {
    color: #f38ba8;
  }
</style>
