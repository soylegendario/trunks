<script lang="ts">
  import { onMount } from "svelte";
  import { gitGetStatus, type GitStatusResult } from "$lib/git";
  import FileTree from "./FileTree.svelte";
  import GitStatusView from "./GitStatusView.svelte";
  import CommitHistoryView from "./CommitHistoryView.svelte";

  let {
    worktreePath,
    onOpenFile,
  }: {
    worktreePath: string | null;
    onOpenFile?: (path: string, mode: "file" | "diff", cached: boolean) => void;
  } = $props();

  const POLL_MS = 2500;

  let tab = $state<"files" | "git" | "commits">("git");
  let status = $state<GitStatusResult | null>(null);
  let error = $state<string | null>(null);
  let refreshKey = $state(0);

  async function refresh() {
    if (!worktreePath) return;
    const path = worktreePath;
    try {
      const s = await gitGetStatus(path);
      if (path === worktreePath) {
        status = s;
        error = null;
      }
    } catch (e) {
      if (path === worktreePath) error = String(e);
    }
  }

  $effect(() => {
    void worktreePath;
    status = null;
    error = null;
    refresh();
  });

  onMount(() => {
    const timer = setInterval(() => {
      if (document.hidden) return;
      refresh();
      refreshKey++;
    }, POLL_MS);
    const onFocus = () => {
      refresh();
      refreshKey++;
    };
    window.addEventListener("focus", onFocus);
    return () => {
      clearInterval(timer);
      window.removeEventListener("focus", onFocus);
    };
  });

  let changeCount = $derived(status?.entries.length ?? 0);
</script>

<div class="inspector">
  <div class="tabs">
    <button class:active={tab === "git"} onclick={() => (tab = "git")}>
      Git{changeCount ? ` (${changeCount})` : ""}
    </button>
    <button class:active={tab === "commits"} onclick={() => (tab = "commits")}>Commits</button>
    <button class:active={tab === "files"} onclick={() => (tab = "files")}>Files</button>
  </div>
  <div class="body">
    {#if !worktreePath}
      <div class="hint">Select a worktree to inspect it.</div>
    {:else if tab === "git"}
      <GitStatusView {worktreePath} {status} {error} {onOpenFile} />
    {:else if tab === "commits"}
      <CommitHistoryView {worktreePath} headOid={status ? status.head_oid : undefined} />
    {:else}
      <div class="tree">
        <FileTree {worktreePath} {refreshKey} {onOpenFile} />
      </div>
    {/if}
  </div>
</div>

<style>
  .inspector { display: flex; flex-direction: column; height: 100%; min-height: 0; }
  .tabs { display: flex; border-bottom: 1px solid #30363d; flex-shrink: 0; }
  .tabs button {
    flex: 1;
    padding: 6px;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: #8b949e;
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .tabs button.active { color: #c9d1d9; border-bottom-color: #9448f8; }
  .body { flex: 1; min-height: 0; overflow: hidden; }
  .tree { height: 100%; overflow: auto; padding: 4px 0; }
  .hint { padding: 10px; font-size: 12px; color: #8b949e; }
</style>
