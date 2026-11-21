<script lang="ts">
  import FileTree from "./FileTree.svelte";
  import { gitListFiles, type GitFileInfo } from "$lib/git";

  let {
    worktreePath,
    subpath = "",
    depth = 0,
    refreshKey = 0,
    onOpenFile,
  }: {
    worktreePath: string;
    subpath?: string;
    depth?: number;
    refreshKey?: number;
    onOpenFile?: (path: string, mode: "file" | "diff", cached: boolean) => void;
  } = $props();

  let entries = $state<GitFileInfo[]>([]);
  let error = $state<string | null>(null);
  let expanded = $state<Record<string, boolean>>({});

  async function load() {
    try {
      entries = await gitListFiles(worktreePath, subpath || undefined);
      error = null;
    } catch (e) {
      error = String(e);
      entries = [];
    }
  }

  $effect(() => {
    // Reload when the worktree, folder or refresh key changes.
    void worktreePath;
    void subpath;
    void refreshKey;
    load();
  });

  function sorted(list: GitFileInfo[]) {
    return [...list].sort((a, b) => Number(b.is_dir) - Number(a.is_dir));
  }
</script>

{#if error}
  <div class="err">{error}</div>
{:else}
  <ul>
    {#each sorted(entries) as entry (entry.path)}
      <li>
        {#if entry.is_dir}
          <button
            class="row"
            style:padding-left={`${6 + depth * 12}px`}
            onclick={() => (expanded[entry.path] = !expanded[entry.path])}
          >
            <span class="chev">{expanded[entry.path] ? "▾" : "▸"}</span>
            <span class="name dir">{entry.name}</span>
          </button>
          {#if expanded[entry.path]}
            <FileTree {worktreePath} subpath={entry.path} depth={depth + 1} {refreshKey} {onOpenFile} />
          {/if}
        {:else}
          <button
            class="row file"
            style:padding-left={`${6 + depth * 12 + 12}px`}
            onclick={() => onOpenFile?.(entry.path, "file", false)}
          >
            <span class="name">{entry.name}{entry.is_symlink ? " →" : ""}</span>
          </button>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<style>
  ul { list-style: none; margin: 0; padding: 0; }
  .row {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    padding: 2px 6px;
    background: none;
    border: none;
    color: #c9d1d9;
    font: inherit;
    font-size: 12px;
    text-align: left;
    cursor: pointer;
  }
  .row.file { color: #8b949e; }
  .row.file:hover { color: #c9d1d9; }
  .row:hover { background: rgba(255, 255, 255, 0.05); }
  .chev { width: 10px; color: #8b949e; font-size: 10px; }
  .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dir { color: #9448f8; }
  .err { color: #ffa198; font-size: 11px; padding: 6px 10px; }
</style>
