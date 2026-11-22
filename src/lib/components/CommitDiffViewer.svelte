<script lang="ts">
  import { gitGetCommitDiff } from "$lib/git";
  import DiffViewer from "./DiffViewer.svelte";

  let {
    worktreePath,
    oid,
    title,
  }: {
    worktreePath: string;
    oid: string;
    title: string;
  } = $props();

  let diff = $state("");
  let truncated = $state(false);
  let error = $state<string | null>(null);
  let loaded = $state(false);

  // A commit is immutable, so it is loaded once per (worktree, oid) and never polled.
  $effect(() => {
    const key = `${worktreePath}|${oid}`;
    loaded = false;
    diff = "";
    truncated = false;
    error = null;
    gitGetCommitDiff(worktreePath, oid)
      .then((r) => {
        if (key !== `${worktreePath}|${oid}`) return;
        diff = r.diff;
        truncated = r.truncated;
      })
      .catch((e) => {
        if (key !== `${worktreePath}|${oid}`) return;
        error = String(e);
      })
      .finally(() => {
        if (key === `${worktreePath}|${oid}`) loaded = true;
      });
  });
</script>

<div class="viewer">
  <div class="title">
    <span class="p" title={oid}>{title}</span>
    <span class="tag">commit · first parent</span>
  </div>
  {#if error}
    <div class="err">{error}</div>
  {:else if !loaded}
    <div class="hint">Loading…</div>
  {:else}
    {#if truncated}
      <div class="warn">This diff is too large and was truncated. Showing only the first part.</div>
    {/if}
    {#if diff}
      <DiffViewer {diff} />
    {:else}
      <div class="hint">No textual changes.</div>
    {/if}
  {/if}
</div>

<style>
  .viewer { display: flex; flex-direction: column; height: 100%; min-height: 0; background: #11111b; user-select: text; }
  .title { display: flex; gap: 8px; align-items: center; padding: 4px 12px; font-size: 11px; color: #8b949e; border-bottom: 1px solid #30363d; flex-shrink: 0; }
  .tag { margin-left: auto; font-size: 10px; text-transform: uppercase; color: #6e7681; }
  .viewer :global(.diff) { flex: 1; min-height: 0; }
  .hint { padding: 10px 12px; font-size: 12px; color: #8b949e; }
  .warn { padding: 6px 12px; font-size: 12px; color: #d29922; border-bottom: 1px solid #30363d; }
  .err { padding: 10px 12px; font-size: 12px; color: #ffa198; }
</style>
