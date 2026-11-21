<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { TerminalSession, type TerminalTab, type SystemInfo } from "$lib/terminal";
  import {
    getWorkspaceState,
    saveLayout,
    saveTabs,
    deleteTab,
    type TerminalTabItem,
  } from "$lib/storage";
  import { projectStore } from "$lib/projects.svelte";
  import TerminalView from "$lib/TerminalView.svelte";
  import WorktreeList from "$lib/components/WorktreeList.svelte";

  let showLeft = $state(true);
  let showRight = $state(true);
  let centerWidth = $state(0);
  let centerHeight = $state(0);
  let centerElement = $state<HTMLElement | null>(null);

  let systemInfo = $state<SystemInfo | null>(null);
  let tabs = $state<TerminalTab[]>([]);
  let activeTabId = $state<string>("");
  let sessions = $state<Map<string, TerminalSession>>(new Map());
  let dbStatus = $state<string>("Connecting to SQLite...");
  let isLoaded = $state(false);

  // M1 Acceptance Checklist State
  let checklist = $state({
    ac1: false, // AC-M1-01: Local Git project registration
    ac2: false, // AC-M1-02: Worktree discovery & branch parsing
    ac3: false, // AC-M1-03: Worktree creation (git worktree add)
    ac4: false, // AC-M1-04: Terminal tab spawning in worktree PWD
    ac5: false, // AC-M1-05: External CLI synchronization & refresh
    ac6: false, // AC-M1-06: Guarded worktree removal & force flag
  });

  async function persistLayoutState() {
    if (!isLoaded) return;
    try {
      await saveLayout({
        show_left: showLeft,
        show_right: showRight,
        active_tab_id: activeTabId || null,
        selected_project_name: projectStore.selectedProjectId,
        selected_worktree_name: projectStore.selectedWorktreePath,
      });
    } catch (err) {
      console.error("Failed to save layout:", err);
    }
  }

  async function persistTabsState() {
    if (!isLoaded) return;
    try {
      const tabItems: TerminalTabItem[] = tabs.map((t, idx) => ({
        id: t.id,
        title: t.title,
        cwd: t.cwd,
        project_name: t.projectName || null,
        worktree_name: t.worktreeName || null,
        tab_order: idx,
        is_stale: t.isStale,
        stale_reason: t.staleReason || null,
      }));
      await saveTabs(tabItems);
    } catch (err) {
      console.error("Failed to save tabs:", err);
    }
  }

  function toggleLeft() {
    showLeft = !showLeft;
    persistLayoutState();
  }

  function toggleRight() {
    showRight = !showRight;
    persistLayoutState();
  }

  async function createTab(
    cwd?: string,
    title?: string,
    worktreeName?: string,
    projectName?: string
  ) {
    const id = "tab-" + Date.now() + "-" + Math.random().toString(36).substring(2, 7);
    const tabTitle = title || `Terminal ${tabs.length + 1}`;
    const tabCwd = cwd || systemInfo?.home_dir || "";

    const newTab: TerminalTab = {
      id,
      title: tabTitle,
      cwd: tabCwd,
      projectName: projectName || "Trunks Workspace",
      worktreeName: worktreeName || "default",
      status: "running",
      isRestored: false,
    };

    const session = new TerminalSession(newTab, (newStatus) => {
      const idx = tabs.findIndex((t) => t.id === id);
      if (idx !== -1) {
        tabs[idx].status = newStatus;
      }
    });

    sessions.set(id, session);
    tabs = [...tabs, newTab];
    activeTabId = id;

    await persistTabsState();
    await persistLayoutState();

    // Trigger fit after DOM update
    requestAnimationFrame(() => {
      session.fit();
    });
  }

  async function closeTab(id: string, event?: Event) {
    if (event) event.stopPropagation();
    const session = sessions.get(id);
    if (session) {
      await session.destroy();
      sessions.delete(id);
    }

    const remaining = tabs.filter((t) => t.id !== id);
    tabs = remaining;

    if (activeTabId === id) {
      if (remaining.length > 0) {
        activeTabId = remaining[remaining.length - 1].id;
      } else {
        activeTabId = "";
      }
    }

    await deleteTab(id).catch(console.error);
    await persistTabsState();
    await persistLayoutState();
  }

  async function startShellOnTab(id: string) {
    const session = sessions.get(id);
    if (session) {
      await session.startPty();
      const idx = tabs.findIndex((t) => t.id === id);
      if (idx !== -1) {
        tabs[idx].status = "running";
        tabs[idx].isRestored = false;
      }
      await persistTabsState();
    }
  }

  async function restartTab(id: string) {
    const session = sessions.get(id);
    if (session) {
      await session.restart();
      const idx = tabs.findIndex((t) => t.id === id);
      if (idx !== -1) {
        tabs[idx].status = "running";
        tabs[idx].isRestored = false;
      }
      await persistTabsState();
    }
  }

  function selectTab(id: string) {
    activeTabId = id;
    persistLayoutState();
    requestAnimationFrame(() => {
      const currentSession = sessions.get(id);
      currentSession?.fit();
      currentSession?.terminal.focus();
    });
  }

  function sendQuickCommand(cmd: string) {
    if (!activeTabId) return;
    const session = sessions.get(activeTabId);
    if (session && session.tab.status === "running") {
      invoke("pty_write", { id: activeTabId, data: cmd + "\r" });
    }
  }

  let resizeObserver: ResizeObserver | null = null;

  function handleWindowFocus() {
    // On-focus automatic refresh of worktrees from system Git CLI
    projectStore.refreshAll();
  }

  onMount(() => {
    window.addEventListener("focus", handleWindowFocus);

    (async () => {
      try {
        systemInfo = await invoke<SystemInfo>("get_system_info");
      } catch (err) {
        console.error("Failed to get system info:", err);
      }

      // Initialize project store (SQLite + Git discovery)
      await projectStore.init();

      // Initialize database and load persisted workspace layout & tabs
      try {
        const state = await getWorkspaceState();
        showLeft = state.layout.show_left;
        showRight = state.layout.show_right;

        if (state.tabs && state.tabs.length > 0) {
          // Restore tabs as stopped / restored tabs (AC-M0-07 / AC-M1-04)
          const restoredTabs: TerminalTab[] = [];
          for (const t of state.tabs) {
            const tabData: TerminalTab = {
              id: t.id,
              title: t.title,
              cwd: t.cwd,
              projectName: t.project_name || undefined,
              worktreeName: t.worktree_name || undefined,
              status: "stopped",
              isRestored: true,
              isStale: t.is_stale,
              staleReason: t.stale_reason || undefined,
              tabOrder: t.tab_order,
            };

            const session = new TerminalSession(tabData, (newStatus) => {
              const idx = tabs.findIndex((tab) => tab.id === t.id);
              if (idx !== -1) {
                tabs[idx].status = newStatus;
              }
            });

            sessions.set(t.id, session);
            restoredTabs.push(tabData);
          }

          tabs = restoredTabs;
          if (state.layout.active_tab_id && sessions.has(state.layout.active_tab_id)) {
            activeTabId = state.layout.active_tab_id;
          } else {
            activeTabId = restoredTabs[0]?.id || "";
          }
          dbStatus = `Restored ${tabs.length} tabs from SQLite`;
        } else {
          // First time launch: create default tab
          await createTab(undefined, "Terminal 1", "main");
          dbStatus = "Initialized SQLite persistence store";
        }

        isLoaded = true;
      } catch (err) {
        console.error("Database initialization failed:", err);
        dbStatus = `SQLite error: ${err}`;
        if (tabs.length === 0) {
          await createTab(undefined, "Terminal 1", "main");
        }
        isLoaded = true;
      }
    })();

    // Setup ResizeObserver for center terminal area
    if (centerElement) {
      resizeObserver = new ResizeObserver((entries) => {
        for (const entry of entries) {
          centerWidth = Math.round(entry.contentRect.width);
          centerHeight = Math.round(entry.contentRect.height);
          if (activeTabId) {
            const currentSession = sessions.get(activeTabId);
            currentSession?.fit();
          }
        }
      });
      resizeObserver.observe(centerElement);
    }

    return () => {
      window.removeEventListener("focus", handleWindowFocus);
      if (resizeObserver) {
        resizeObserver.disconnect();
      }
    };
  });

  onDestroy(async () => {
    for (const session of sessions.values()) {
      await session.destroy();
    }
    sessions.clear();
  });
</script>

<div class="app-container">
  <header class="app-header">
    <div class="header-left">
      <button
        class="toggle-btn"
        class:active={showLeft}
        onclick={toggleLeft}
        title="Toggle Projects & Worktrees Sidebar"
      >
        {showLeft ? "◀ Worktrees" : "▶ Worktrees"}
      </button>
      <span class="app-title">Trunks</span>
      <span class="badge">Milestone 1 — Worktrees</span>
      <span class="db-badge" title="SQLite Transactional Store">SQLite: WAL</span>
    </div>
    <div class="header-center">
      <span class="dimension-indicator">Center Area: {centerWidth}px × {centerHeight}px</span>
      {#if systemInfo}
        <span class="os-indicator">OS: {systemInfo.os} ({systemInfo.default_shell})</span>
      {/if}
    </div>
    <div class="header-right">
      <button
        class="toggle-btn"
        class:active={showRight}
        onclick={toggleRight}
        title="Toggle Acceptance Checklist"
      >
        {showRight ? "Checklist ▶" : "◀ Checklist"}
      </button>
    </div>
  </header>

  <div class="app-body">
    {#if showLeft}
      <aside class="panel left-panel">
        <WorktreeList
          onOpenTerminal={(cwd, title, worktreeName, projectName) =>
            createTab(cwd, title, worktreeName, projectName)}
          onSelectWorktree={() => persistLayoutState()}
        />
      </aside>
    {/if}

    <main class="panel center-panel" bind:this={centerElement}>
      <div class="panel-header center-header">
        <div class="tabs-bar">
          {#each tabs as tab (tab.id)}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="tab"
              class:active={activeTabId === tab.id}
              class:is-restored={tab.isRestored}
              class:is-stale={tab.isStale}
              onclick={() => selectTab(tab.id)}
            >
              <span
                class="tab-status-dot"
                class:running={tab.status === "running"}
                class:stopped={tab.status === "stopped"}
                class:restored={tab.isRestored}
              ></span>
              <span class="tab-title" title={`${tab.projectName || ""} › ${tab.title} (${tab.cwd})`}>
                {tab.title}
              </span>

              {#if tab.isRestored && tab.status === "stopped"}
                <span class="tab-badge restored" title="Restored workspace identity (No ghost process)">
                  restored
                </span>
                <button
                  class="tab-start-btn"
                  onclick={(e) => {
                    e.stopPropagation();
                    startShellOnTab(tab.id);
                  }}
                  title="Spawn fresh shell in remembered path"
                >
                  ▶ Start
                </button>
              {:else}
                <span
                  class="tab-badge"
                  class:running={tab.status === "running"}
                  class:stopped={tab.status === "stopped"}
                >
                  {tab.status}
                </span>
                {#if tab.status === "stopped"}
                  <button
                    class="tab-restart-btn"
                    onclick={(e) => {
                      e.stopPropagation();
                      restartTab(tab.id);
                    }}
                    title="Restart shell in remembered directory"
                  >
                    ↻
                  </button>
                {/if}
              {/if}

              {#if tab.isStale}
                <span class="tab-stale-badge" title={tab.staleReason || "Directory not found"}>⚠️</span>
              {/if}

              <button
                class="tab-close-btn"
                onclick={(e) => closeTab(tab.id, e)}
                title="Close terminal"
              >
                ×
              </button>
            </div>
          {/each}
          <button class="new-tab-btn" onclick={() => createTab()} title="Open new terminal tab">
            +
          </button>
        </div>
        <div class="center-metrics">
          <span>{centerWidth} × {centerHeight} px</span>
        </div>
      </div>

      <div class="panel-content center-content">
        {#if tabs.length === 0}
          <div class="empty-state">
            <p>No active terminal sessions.</p>
            <button class="action-btn primary" onclick={() => createTab()}>
              + Open Terminal
            </button>
          </div>
        {:else}
          {#each tabs as tab (tab.id)}
            {@const session = sessions.get(tab.id)}
            {#if session}
              <TerminalView {session} isActive={activeTabId === tab.id} />
            {/if}
          {/each}
        {/if}
      </div>
    </main>

    {#if showRight}
      <aside class="panel right-panel">
        <div class="panel-header">
          <h3>M1 Acceptance Checklist</h3>
        </div>
        <div class="panel-content checklist-content">
          <div class="checklist-section">
            <h4>Worktree Management (Task 2)</h4>

            <div class="checklist-item">
              <label>
                <input type="checkbox" bind:checked={checklist.ac1} />
                <span><strong>AC-M1-01:</strong> Local Git Project Registration</span>
              </label>
              <span class="hint-text">Add local repo via "+ Add Repo" in the left sidebar.</span>
            </div>

            <div class="checklist-item">
              <label>
                <input type="checkbox" bind:checked={checklist.ac2} />
                <span><strong>AC-M1-02:</strong> Real Git Worktree Discovery</span>
              </label>
              <span class="hint-text">Discovers all worktrees, branches &amp; HEAD hashes directly via Git CLI.</span>
            </div>

            <div class="checklist-item">
              <label>
                <input type="checkbox" bind:checked={checklist.ac3} />
                <span><strong>AC-M1-03:</strong> UI Worktree Creation (git worktree add)</span>
              </label>
              <span class="hint-text">Creates worktree on new branch or checks out existing branch.</span>
            </div>

            <div class="checklist-item">
              <label>
                <input type="checkbox" bind:checked={checklist.ac4} />
                <span><strong>AC-M1-04:</strong> Terminal Spawning in Worktree PWD</span>
              </label>
              <button
                class="test-helper-btn"
                onclick={() => sendQuickCommand("pwd && git status -s")}
              >
                Test PWD &amp; Git
              </button>
            </div>

            <div class="checklist-item">
              <label>
                <input type="checkbox" bind:checked={checklist.ac5} />
                <span><strong>AC-M1-05:</strong> External CLI Synchronization</span>
              </label>
              <button
                class="test-helper-btn"
                onclick={() => projectStore.refreshAll()}
              >
                Refresh Worktrees (↻)
              </button>
            </div>

            <div class="checklist-item">
              <label>
                <input type="checkbox" bind:checked={checklist.ac6} />
                <span><strong>AC-M1-06:</strong> Guarded Worktree Removal</span>
              </label>
              <span class="hint-text">Refuses main worktree; supports `--force` for dirty worktrees.</span>
            </div>
          </div>

          <div class="info-card">
            <h4>Live Workspace State</h4>
            <p>DB Status: <code>{dbStatus}</code></p>
            <p>Active Tab: <code>{activeTabId || "None"}</code></p>
            <p>Active Tabs: <code>{tabs.length}</code> (Running: <code>{tabs.filter((t) => t.status === "running").length}</code>)</p>
            <p>Registered Repos: <code>{projectStore.projects.length}</code></p>
          </div>
        </div>
      </aside>
    {/if}
  </div>
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    background-color: #11111b;
    color: #cdd6f4;
    overflow: hidden;
    user-select: none;
  }

  :global(*), :global(*::before), :global(*::after) {
    box-sizing: border-box;
  }

  .app-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
  }

  .app-header {
    height: 38px;
    background-color: #181825;
    border-bottom: 1px solid #313244;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 12px;
    font-size: 12px;
    z-index: 10;
  }

  .header-left, .header-center, .header-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .app-title {
    font-weight: 700;
    font-size: 14px;
    color: #cdd6f4;
    letter-spacing: 0.5px;
  }

  .badge {
    background: #45475a;
    color: #89b4fa;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 600;
  }

  .db-badge {
    background: #313244;
    color: #a6e3a1;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 500;
    border: 1px solid #45475a;
  }

  .toggle-btn {
    background: #313244;
    border: 1px solid #45475a;
    color: #cdd6f4;
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .toggle-btn:hover {
    background: #45475a;
    color: #89b4fa;
  }

  .toggle-btn.active {
    background: #45475a;
    color: #89b4fa;
    border-color: #89b4fa;
  }

  .dimension-indicator, .os-indicator {
    color: #a6adc8;
    font-family: monospace;
    font-size: 11px;
    background: #11111b;
    padding: 2px 6px;
    border-radius: 3px;
    border: 1px solid #313244;
  }

  .app-body {
    display: flex;
    flex: 1;
    height: calc(100vh - 38px);
    overflow: hidden;
  }

  .panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .left-panel {
    width: 300px;
    min-width: 260px;
    background-color: #181825;
    border-right: 1px solid #313244;
  }

  .center-panel {
    flex: 1;
    background-color: #11111b;
    min-width: 320px;
  }

  .right-panel {
    width: 320px;
    min-width: 280px;
    background-color: #181825;
    border-left: 1px solid #313244;
  }

  .panel-header {
    height: 38px;
    background-color: #181825;
    border-bottom: 1px solid #313244;
    display: flex;
    align-items: center;
    padding: 0 12px;
  }

  .panel-header h3 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #a6adc8;
  }

  .center-header {
    padding: 0;
    justify-content: space-between;
    background-color: #11111b;
  }

  .tabs-bar {
    display: flex;
    align-items: center;
    height: 100%;
    overflow-x: auto;
    scrollbar-width: none;
    flex: 1;
  }

  .tabs-bar::-webkit-scrollbar {
    display: none;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 12px;
    height: 100%;
    background-color: #181825;
    border-right: 1px solid #313244;
    color: #a6adc8;
    font-size: 12px;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.15s ease;
    border-top: 2px solid transparent;
  }

  .tab:hover {
    background-color: #1e1e2e;
    color: #cdd6f4;
  }

  .tab.active {
    background-color: #11111b;
    color: #89b4fa;
    border-top: 2px solid #89b4fa;
    font-weight: 500;
  }

  .tab.is-restored {
    border-left: 2px dashed #f9e2af;
  }

  .tab.is-stale {
    border-top: 2px solid #f9e2af;
  }

  .tab-status-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background-color: #6c7086;
  }

  .tab-status-dot.running {
    background-color: #a6e3a1;
    box-shadow: 0 0 6px #a6e3a1;
  }

  .tab-status-dot.stopped {
    background-color: #f38ba8;
  }

  .tab-status-dot.restored {
    background-color: #f9e2af;
  }

  .tab-badge {
    font-size: 9px;
    padding: 1px 4px;
    border-radius: 3px;
    background: #313244;
    color: #a6adc8;
    text-transform: uppercase;
  }

  .tab-badge.running {
    background: rgba(166, 227, 161, 0.15);
    color: #a6e3a1;
  }

  .tab-badge.stopped {
    background: rgba(243, 139, 168, 0.15);
    color: #f38ba8;
  }

  .tab-badge.restored {
    background: rgba(249, 226, 175, 0.2);
    color: #f9e2af;
    border: 1px solid #f9e2af;
  }

  .tab-start-btn, .tab-restart-btn {
    background: #89b4fa;
    color: #11111b;
    border: none;
    padding: 1px 5px;
    border-radius: 3px;
    font-size: 10px;
    font-weight: bold;
    cursor: pointer;
  }

  .tab-restart-btn {
    background: #45475a;
    color: #cdd6f4;
  }

  .tab-start-btn:hover {
    background: #b4befe;
  }

  .tab-restart-btn:hover {
    background: #585b70;
  }

  .tab-stale-badge {
    font-size: 10px;
  }

  .tab-close-btn {
    background: transparent;
    border: none;
    color: #6c7086;
    font-size: 14px;
    cursor: pointer;
    padding: 0 2px;
    border-radius: 3px;
    line-height: 1;
  }

  .tab-close-btn:hover {
    background: rgba(243, 139, 168, 0.2);
    color: #f38ba8;
  }

  .new-tab-btn {
    background: transparent;
    border: none;
    color: #a6adc8;
    font-size: 16px;
    padding: 0 10px;
    height: 100%;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .new-tab-btn:hover {
    background-color: #181825;
    color: #89b4fa;
  }

  .center-metrics {
    padding: 0 12px;
    font-family: monospace;
    font-size: 11px;
    color: #6c7086;
  }

  .panel-content {
    flex: 1;
    overflow-y: auto;
    position: relative;
  }

  .center-content {
    background-color: #11111b;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #6c7086;
    gap: 12px;
  }

  .action-btn {
    background: #313244;
    border: 1px solid #45475a;
    color: #cdd6f4;
    padding: 6px 12px;
    border-radius: 4px;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-btn.primary {
    background: #89b4fa;
    color: #11111b;
    border-color: #89b4fa;
    font-weight: 600;
  }

  .action-btn:hover {
    background: #45475a;
    color: #89b4fa;
  }

  .action-btn.primary:hover {
    background: #b4befe;
    border-color: #b4befe;
  }

  /* Right Panel / Checklist Styles */
  .checklist-content {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .checklist-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .checklist-section h4 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    color: #89b4fa;
    border-bottom: 1px solid #313244;
    padding-bottom: 4px;
  }

  .checklist-item {
    background: #11111b;
    border: 1px solid #313244;
    border-radius: 6px;
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .checklist-item label {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    font-size: 11px;
    color: #cdd6f4;
    cursor: pointer;
    line-height: 1.3;
  }

  .checklist-item input[type="checkbox"] {
    margin-top: 2px;
    accent-color: #89b4fa;
  }

  .hint-text {
    font-size: 10px;
    color: #6c7086;
    padding-left: 20px;
  }

  .test-helper-btn {
    align-self: flex-start;
    margin-left: 20px;
    background: #313244;
    border: 1px solid #45475a;
    color: #89b4fa;
    font-size: 10px;
    padding: 3px 8px;
    border-radius: 4px;
    cursor: pointer;
    font-family: monospace;
    transition: all 0.15s ease;
  }

  .test-helper-btn:hover {
    background: #45475a;
    border-color: #89b4fa;
  }

  .info-card {
    background: #11111b;
    border: 1px solid #313244;
    border-radius: 6px;
    padding: 10px;
    font-size: 11px;
    color: #a6adc8;
  }

  .info-card h4 {
    margin: 0 0 8px 0;
    color: #89b4fa;
    font-size: 11px;
    font-weight: 600;
  }

  .info-card p {
    margin: 4px 0;
  }

  .info-card code {
    background: #181825;
    padding: 1px 4px;
    border-radius: 3px;
    color: #cdd6f4;
  }
</style>
