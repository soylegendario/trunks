<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { TerminalSession, type TerminalTab, type SystemInfo } from "$lib/terminal";
  import {
    getWorkspaceState,
    saveLayout,
    saveProjects,
    saveTabs,
    upsertTab,
    deleteTab,
    type ProjectItem,
    type WorktreeItem,
    type TerminalTabItem,
    type WorkspaceState,
  } from "$lib/storage";
  import TerminalView from "$lib/TerminalView.svelte";

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

  // Acceptance Checklist State
  let checklist = $state({
    ac1: false, // Shell startup & expected PWD
    ac2: false, // Echo/input, Ctrl-C, ANSI colors, Unicode, scrollback
    ac3: false, // Full-screen interactive TUI (top/less/vim)
    ac4: false, // Resize redraw without restarting process
    ac5: false, // 2 simultaneous sessions in separate worktrees
    ac6: false, // Incremental persistence & crash recovery
    ac7: false, // Restored tabs marked as stopped (no ghost PTY)
    ac8: false, // Stale/removed worktree detection & labeling
  });

  // Example Project & Worktrees state synced with SQLite
  let projects = $state<ProjectItem[]>([
    {
      id: "proj-trunks-spike",
      name: "Trunks Workspace (M0 Spike)",
      path: "/tmp/trunks-spike",
      is_stale: false,
      stale_reason: null,
      worktrees: [
        {
          id: "wt-main",
          project_id: "proj-trunks-spike",
          name: "main",
          branch: "main",
          path: "/tmp/trunks-spike/wt-main",
          is_stale: false,
          stale_reason: null,
        },
        {
          id: "wt-feat",
          project_id: "proj-trunks-spike",
          name: "feature-recovery",
          branch: "feat/sqlite-recovery",
          path: "/tmp/trunks-spike/wt-feat",
          is_stale: false,
          stale_reason: null,
        },
        {
          id: "wt-unicode",
          project_id: "proj-trunks-spike",
          name: "unicode-path",
          branch: "feat/unicode-ñ",
          path: "/tmp/trunks-spike/wt-espacio y acento ñ",
          is_stale: false,
          stale_reason: null,
        },
      ],
    },
  ]);

  async function persistLayoutState() {
    if (!isLoaded) return;
    try {
      await saveLayout({
        show_left: showLeft,
        show_right: showRight,
        active_tab_id: activeTabId || null,
        selected_project_name: null,
        selected_worktree_name: null,
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

  async function persistProjectsState() {
    if (!isLoaded) return;
    try {
      await saveProjects(projects);
    } catch (err) {
      console.error("Failed to save projects:", err);
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

  async function createTab(cwd?: string, title?: string, worktreeName?: string) {
    const id = "tab-" + Date.now() + "-" + Math.random().toString(36).substring(2, 7);
    const tabTitle = title || `Terminal ${tabs.length + 1}`;
    const tabCwd = cwd || systemInfo?.home_dir || "";

    const newTab: TerminalTab = {
      id,
      title: tabTitle,
      cwd: tabCwd,
      projectName: "Trunks Workspace",
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

  // Acceptance Test Helpers
  async function reloadWorkspaceFromDatabase() {
    dbStatus = "Reloading state from SQLite...";
    try {
      const state = await getWorkspaceState();
      
      // Update layout
      showLeft = state.layout.show_left;
      showRight = state.layout.show_right;

      // Update projects with reconciled stale flags
      if (state.projects && state.projects.length > 0) {
        projects = state.projects;
      }

      // Dispose existing sessions
      for (const s of sessions.values()) {
        await s.destroy();
      }
      sessions.clear();

      // Restore tabs as stopped / restored tabs
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
      } else if (restoredTabs.length > 0) {
        activeTabId = restoredTabs[0].id;
      } else {
        activeTabId = "";
      }

      dbStatus = `Restored ${tabs.length} tabs, ${projects.length} projects (WAL mode)`;
    } catch (err) {
      dbStatus = `Failed to reload from DB: ${err}`;
      console.error(err);
    }
  }

  function addStaleWorktreeFixture() {
    const staleWt: WorktreeItem = {
      id: "wt-stale-" + Date.now(),
      project_id: projects[0]?.id || "proj-trunks-spike",
      name: "deleted-external-wt",
      branch: "feat/orphaned-branch",
      path: "/tmp/non_existent_deleted_worktree_" + Date.now(),
      is_stale: true,
      stale_reason: "Worktree directory was moved or removed outside Trunks",
    };

    if (projects.length > 0) {
      projects[0].worktrees.push(staleWt);
      projects = [...projects];
      persistProjectsState();
    }
  }

  let resizeObserver: ResizeObserver | null = null;

  onMount(() => {
    (async () => {
      try {
        systemInfo = await invoke<SystemInfo>("get_system_info");
      } catch (err) {
        console.error("Failed to get system info:", err);
      }

      // Initialize database and load persisted workspace state
      try {
        const state = await getWorkspaceState();
        showLeft = state.layout.show_left;
        showRight = state.layout.show_right;

        if (state.projects && state.projects.length > 0) {
          projects = state.projects;
        } else {
          // Save initial default project fixtures to SQLite
          await persistProjectsState();
        }

        if (state.tabs && state.tabs.length > 0) {
          // Restore tabs as stopped / restored tabs (AC-M0-07)
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
          dbStatus = `Restored ${tabs.length} tabs from SQLite (Crash Resilient)`;
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
      <button class="toggle-btn" class:active={showLeft} onclick={toggleLeft} title="Toggle Projects Sidebar">
        {showLeft ? "◀ Sidebar" : "▶ Sidebar"}
      </button>
      <span class="app-title">Trunks</span>
      <span class="badge">Milestone 0 Spike</span>
      <span class="db-badge" title="SQLite Transactional Store">SQLite: WAL</span>
    </div>
    <div class="header-center">
      <span class="dimension-indicator">Center Area: {centerWidth}px × {centerHeight}px</span>
      {#if systemInfo}
        <span class="os-indicator">OS: {systemInfo.os} ({systemInfo.default_shell})</span>
      {/if}
    </div>
    <div class="header-right">
      <button class="toggle-btn" class:active={showRight} onclick={toggleRight} title="Toggle Acceptance Checklist">
        {showRight ? "Checklist ▶" : "◀ Checklist"}
      </button>
    </div>
  </header>

  <div class="app-body">
    {#if showLeft}
      <aside class="panel left-panel">
        <div class="panel-header">
          <h3>Projects &amp; Worktrees</h3>
        </div>
        <div class="panel-content">
          {#each projects as project}
            <div class="project-card" class:stale={project.is_stale}>
              <div class="project-header">
                <span class="project-icon">📁</span>
                <span class="project-name">{project.name}</span>
                {#if project.is_stale}
                  <span class="stale-tag" title={project.stale_reason || "Directory missing"}>⚠️ Stale</span>
                {/if}
              </div>
              <div class="worktrees-list">
                <div class="section-label">Git Worktrees:</div>
                {#each project.worktrees as wt}
                  <div class="worktree-item" class:stale={wt.is_stale}>
                    <div class="worktree-info">
                      <div class="worktree-title-row">
                        <span class="branch-icon">⎇</span>
                        <span class="worktree-branch">{wt.branch || wt.name}</span>
                        {#if wt.is_stale}
                          <span class="stale-tag" title={wt.stale_reason || "Worktree path missing on disk"}>⚠️ Stale Path</span>
                        {/if}
                      </div>
                      <span class="worktree-path" title={wt.path}>{wt.path}</span>
                    </div>
                    <button
                      class="open-btn"
                      onclick={() => createTab(wt.path, `wt:${wt.name}`, wt.name)}
                      title="Open Terminal in this worktree"
                    >
                      + Terminal
                    </button>
                  </div>
                {/each}
              </div>
            </div>
          {/each}

          <div class="custom-action-box">
            <h4>Quick Launch</h4>
            <button
              class="action-btn"
              onclick={() => createTab("/tmp", "wt:temp", "temp")}
            >
              + Launch in /tmp
            </button>
            <button
              class="action-btn"
              onclick={() => createTab("/tmp/trunks-spike/wt-espacio y acento ñ", "wt:unicode", "unicode-ñ")}
            >
              + Launch with spaces &amp; ñ
            </button>
            <button
              class="action-btn warning"
              onclick={addStaleWorktreeFixture}
              title="Add a non-existent worktree path to test AC-M0-08 stale detection"
            >
              + Add Stale Worktree (AC-M0-08)
            </button>
          </div>
        </div>
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
              <span class="tab-title">{tab.title}</span>
              
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
                <span class="tab-badge" class:running={tab.status === "running"} class:stopped={tab.status === "stopped"}>
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
          <h3>M0 Acceptance Checklist</h3>
        </div>
        <div class="panel-content checklist-content">
          <div class="checklist-section">
            <h4>PTY &amp; Terminal Fidelity (Task 2)</h4>
            <div class="checklist-item">
              <label>
                <input type="checkbox" bind:checked={checklist.ac1} />
                <span><strong>AC-M0-01:</strong> Shell starts in expected worktree PWD</span>
              </label>
              <button class="test-helper-btn" onclick={() => sendQuickCommand("pwd && echo 'AC-M0-01 OK'")}>
                Test PWD
              </button>
            </div>

            <div class="checklist-item">
              <label>
                <input type="checkbox" bind:checked={checklist.ac2} />
                <span><strong>AC-M0-02:</strong> Input, Ctrl-C, ANSI &amp; Unicode fidelity</span>
              </label>
              <button
                class="test-helper-btn"
                onclick={() =>
                  sendQuickCommand(
                    "echo -e '\\033[32m[OK]\\033[0m ANSI Color test | \\033[34m[INFO]\\033[0m Unicode: 🚀 ñ á é í ó ú 🦀 中文'"
                  )}
              >
                Test ANSI/Unicode
              </button>
            </div>

            <div class="checklist-item">
              <label>
                <input type="checkbox" bind:checked={checklist.ac3} />
                <span><strong>AC-M0-03:</strong> Fullscreen TUI (top / less / vim)</span>
              </label>
              <button class="test-helper-btn" onclick={() => sendQuickCommand("top")}>
                Launch top
              </button>
            </div>

            <div class="checklist-item">
              <label>
                <input type="checkbox" bind:checked={checklist.ac4} />
                <span><strong>AC-M0-04:</strong> Resize redraws active process</span>
              </label>
              <span class="hint-text">Toggle sidebars to resize center area.</span>
            </div>

            <div class="checklist-item">
              <label>
                <input type="checkbox" bind:checked={checklist.ac5} />
                <span><strong>AC-M0-05:</strong> 2+ simultaneous sessions alive</span>
              </label>
              <button
                class="test-helper-btn"
                onclick={async () => {
                  await createTab("/tmp/trunks-spike/wt-main", "Tab A (wt-main)", "wt-main");
                  await createTab("/tmp/trunks-spike/wt-feat", "Tab B (wt-feat)", "wt-feat");
                }}
              >
                Spawn 2 Sessions
              </button>
            </div>
          </div>

          <div class="checklist-section">
            <h4>Persistence &amp; Recovery (Task 3)</h4>
            <div class="checklist-item">
              <label>
                <input type="checkbox" bind:checked={checklist.ac6} />
                <span><strong>AC-M0-06:</strong> Incremental SQLite persistence &amp; crash recovery</span>
              </label>
              <button class="test-helper-btn primary-action" onclick={reloadWorkspaceFromDatabase}>
                Simulate Relaunch
              </button>
            </div>

            <div class="checklist-item">
              <label>
                <input type="checkbox" bind:checked={checklist.ac7} />
                <span><strong>AC-M0-07:</strong> Restored tabs marked as stopped (no ghost PTYs)</span>
              </label>
              <span class="hint-text">Restored tabs show <code>restored</code> badge and require clicking ▶ Start.</span>
            </div>

            <div class="checklist-item">
              <label>
                <input type="checkbox" bind:checked={checklist.ac8} />
                <span><strong>AC-M0-08:</strong> Stale/removed path reconciliation</span>
              </label>
              <button class="test-helper-btn" onclick={addStaleWorktreeFixture}>
                Inject Stale WT
              </button>
            </div>
          </div>

          <div class="info-card">
            <h4>Workspace Persistence State</h4>
            <p>DB Status: <code>{dbStatus}</code></p>
            <p>Active Tab: <code>{activeTabId || "None"}</code></p>
            <p>Saved Tabs: <code>{tabs.length}</code> (Live: <code>{tabs.filter((t) => t.status === "running").length}</code>)</p>
            <p>Saved Projects: <code>{projects.length}</code></p>
          </div>
        </div>
      </aside>
    {/if}
  </div>
</div>

<style>
  :global(body, html) {
    margin: 0;
    padding: 0;
    width: 100%;
    height: 100%;
    overflow: hidden;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    background-color: #1a1b26;
    color: #c0caf5;
  }

  .app-container {
    display: flex;
    flex-direction: column;
    width: 100vw;
    height: 100vh;
    overflow: hidden;
  }

  .app-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 38px;
    background-color: #16161e;
    border-bottom: 1px solid #292e42;
    padding: 0 12px;
    font-size: 13px;
    user-select: none;
  }

  .header-left,
  .header-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .header-center {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .app-title {
    font-weight: 700;
    color: #7aa2f7;
    margin-left: 4px;
  }

  .badge {
    background-color: #24283b;
    color: #9ece6a;
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid #414868;
  }

  .db-badge {
    background-color: #1f2335;
    color: #7dcfff;
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid #2ac3de55;
    font-family: monospace;
  }

  .dimension-indicator {
    font-family: monospace;
    font-size: 11px;
    color: #7aa2f7;
    background-color: #1f2335;
    padding: 2px 6px;
    border-radius: 3px;
  }

  .os-indicator {
    font-size: 11px;
    color: #565f89;
  }

  .toggle-btn {
    background-color: #24283b;
    color: #a9b1d6;
    border: 1px solid #414868;
    padding: 4px 10px;
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
    transition: background-color 0.15s, color 0.15s;
  }

  .toggle-btn:hover {
    background-color: #414868;
    color: #c0caf5;
  }

  .toggle-btn.active {
    border-color: #7aa2f7;
    color: #7aa2f7;
  }

  .app-body {
    display: flex;
    flex: 1;
    overflow: hidden;
    position: relative;
  }

  .panel {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background-color: #1a1b26;
  }

  .left-panel {
    width: 280px;
    min-width: 240px;
    max-width: 380px;
    border-right: 1px solid #292e42;
    background-color: #16161e;
  }

  .right-panel {
    width: 320px;
    min-width: 260px;
    max-width: 420px;
    border-left: 1px solid #292e42;
    background-color: #16161e;
  }

  .center-panel {
    flex: 1;
    min-width: 300px;
    background-color: #1a1b26;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 36px;
    padding: 0 12px;
    border-bottom: 1px solid #24283b;
    background-color: #1f2335;
    user-select: none;
  }

  .panel-header h3 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    color: #a9b1d6;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .center-header {
    padding: 0;
    height: 36px;
  }

  .panel-content {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
  }

  .center-content {
    padding: 0;
    overflow: hidden;
    position: relative;
    background-color: #1a1b26;
  }

  /* Tabs Bar */
  .tabs-bar {
    display: flex;
    align-items: center;
    height: 100%;
    flex: 1;
    overflow-x: auto;
    overflow-y: hidden;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 100%;
    padding: 0 10px;
    background-color: #16161e;
    border-right: 1px solid #24283b;
    border-bottom: 2px solid transparent;
    font-size: 12px;
    color: #787c99;
    cursor: pointer;
    user-select: none;
    white-space: nowrap;
    transition: background-color 0.15s, color 0.15s;
  }

  .tab:hover {
    background-color: #1a1b26;
    color: #c0caf5;
  }

  .tab.active {
    background-color: #1a1b26;
    color: #7aa2f7;
    border-bottom: 2px solid #7aa2f7;
    font-weight: 500;
  }

  .tab.is-restored {
    border-left: 2px solid #bb9af7;
  }

  .tab.is-stale {
    border-left: 2px solid #f7768e;
  }

  .tab-status-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background-color: #565f89;
  }

  .tab-status-dot.running {
    background-color: #9ece6a;
    box-shadow: 0 0 4px #9ece6a;
  }

  .tab-status-dot.stopped {
    background-color: #e0af68;
  }

  .tab-status-dot.restored {
    background-color: #bb9af7;
  }

  .tab-title {
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tab-badge {
    font-size: 10px;
    padding: 1px 4px;
    border-radius: 3px;
    background-color: #24283b;
    color: #565f89;
  }

  .tab-badge.running {
    color: #9ece6a;
  }

  .tab-badge.stopped {
    color: #e0af68;
  }

  .tab-badge.restored {
    color: #bb9af7;
    border: 1px solid #bb9af755;
  }

  .tab-start-btn {
    background-color: #9ece6a22;
    color: #9ece6a;
    border: 1px solid #9ece6a66;
    border-radius: 3px;
    font-size: 10px;
    padding: 1px 5px;
    cursor: pointer;
    font-weight: 600;
  }

  .tab-start-btn:hover {
    background-color: #9ece6a44;
  }

  .tab-restart-btn {
    background: none;
    border: none;
    color: #e0af68;
    cursor: pointer;
    font-size: 12px;
    padding: 0 2px;
  }

  .tab-restart-btn:hover {
    color: #9ece6a;
  }

  .tab-stale-badge {
    font-size: 11px;
  }

  .tab-close-btn {
    background: none;
    border: none;
    color: #565f89;
    cursor: pointer;
    font-size: 14px;
    padding: 0 2px;
    line-height: 1;
    border-radius: 2px;
  }

  .tab-close-btn:hover {
    color: #f7768e;
    background-color: #24283b;
  }

  .new-tab-btn {
    background: none;
    border: none;
    color: #7aa2f7;
    font-size: 16px;
    padding: 0 10px;
    height: 100%;
    cursor: pointer;
  }

  .new-tab-btn:hover {
    background-color: #24283b;
  }

  .center-metrics {
    padding: 0 12px;
    font-size: 11px;
    font-family: monospace;
    color: #565f89;
  }

  /* Project Card & Worktree list */
  .project-card {
    background-color: #1f2335;
    border: 1px solid #292e42;
    border-radius: 6px;
    padding: 10px;
    margin-bottom: 12px;
  }

  .project-card.stale {
    border-color: #f7768e55;
    background-color: #2a1f28;
  }

  .project-header {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 8px;
    font-weight: 600;
    font-size: 13px;
    color: #c0caf5;
  }

  .stale-tag {
    font-size: 10px;
    background-color: #f7768e22;
    color: #f7768e;
    border: 1px solid #f7768e66;
    padding: 1px 4px;
    border-radius: 3px;
    font-weight: normal;
  }

  .worktrees-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .section-label {
    font-size: 11px;
    color: #565f89;
    text-transform: uppercase;
    font-weight: 600;
    margin-bottom: 2px;
  }

  .worktree-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background-color: #16161e;
    border: 1px solid #24283b;
    border-radius: 4px;
    padding: 6px 8px;
    gap: 8px;
  }

  .worktree-item.stale {
    border-color: #f7768e55;
    background-color: #221a22;
  }

  .worktree-info {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    gap: 2px;
  }

  .worktree-title-row {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .branch-icon {
    color: #bb9af7;
    font-size: 12px;
  }

  .worktree-branch {
    font-weight: 500;
    font-size: 12px;
    color: #7aa2f7;
  }

  .worktree-path {
    font-family: monospace;
    font-size: 10px;
    color: #565f89;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .open-btn {
    background-color: #24283b;
    color: #9ece6a;
    border: 1px solid #414868;
    border-radius: 3px;
    font-size: 11px;
    padding: 3px 6px;
    cursor: pointer;
    white-space: nowrap;
  }

  .open-btn:hover {
    background-color: #414868;
    color: #73daca;
  }

  .custom-action-box {
    margin-top: 16px;
    background-color: #1f2335;
    border: 1px dashed #414868;
    border-radius: 6px;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .custom-action-box h4 {
    margin: 0 0 4px 0;
    font-size: 11px;
    color: #a9b1d6;
    text-transform: uppercase;
  }

  .action-btn {
    background-color: #24283b;
    color: #c0caf5;
    border: 1px solid #414868;
    border-radius: 4px;
    padding: 5px 8px;
    font-size: 11px;
    cursor: pointer;
    text-align: left;
  }

  .action-btn:hover {
    background-color: #414868;
  }

  .action-btn.primary {
    background-color: #7aa2f7;
    color: #15161e;
    font-weight: 600;
    border: none;
    text-align: center;
  }

  .action-btn.warning {
    color: #e0af68;
    border-color: #e0af6855;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #565f89;
    gap: 12px;
  }

  /* Checklist Panel */
  .checklist-content {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .checklist-section {
    background-color: #1f2335;
    border: 1px solid #292e42;
    border-radius: 6px;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .checklist-section h4 {
    margin: 0;
    font-size: 11px;
    text-transform: uppercase;
    color: #7aa2f7;
    letter-spacing: 0.5px;
  }

  .checklist-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 12px;
    gap: 8px;
  }

  .checklist-item label {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    flex: 1;
  }

  .test-helper-btn {
    background-color: #24283b;
    color: #7dcfff;
    border: 1px solid #414868;
    border-radius: 3px;
    font-size: 10px;
    padding: 2px 6px;
    cursor: pointer;
    white-space: nowrap;
  }

  .test-helper-btn:hover {
    background-color: #414868;
    color: #bb9af7;
  }

  .test-helper-btn.primary-action {
    background-color: #7aa2f722;
    border-color: #7aa2f766;
    color: #7aa2f7;
    font-weight: 600;
  }

  .hint-text {
    font-size: 10px;
    color: #565f89;
    font-style: italic;
  }

  .info-card {
    background-color: #16161e;
    border: 1px solid #292e42;
    border-radius: 6px;
    padding: 10px;
    font-size: 11px;
  }

  .info-card h4 {
    margin: 0 0 6px 0;
    font-size: 11px;
    color: #a9b1d6;
    text-transform: uppercase;
  }

  .info-card p {
    margin: 3px 0;
    color: #787c99;
  }

  .info-card code {
    color: #7aa2f7;
    background-color: #1f2335;
    padding: 1px 4px;
    border-radius: 3px;
  }
</style>
