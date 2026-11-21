<script lang="ts">
  import { onMount } from "svelte";
  import { gitGetDiff, gitReadFile } from "$lib/git";
  import DiffViewer from "./DiffViewer.svelte";

  let {
    worktreePath,
    path,
    mode,
    cached = false,
  }: {
    worktreePath: string;
    path: string;
    mode: "file" | "diff";
    cached?: boolean;
  } = $props();

  const POLL_MS = 2500;

  let content = $state("");
  let error = $state<string | null>(null);
  let loaded = $state(false);

  async function load() {
    const key = `${worktreePath}|${path}|${mode}|${cached}`;
    try {
      const text =
        mode === "diff"
          ? await gitGetDiff(worktreePath, path, cached)
          : await gitReadFile(worktreePath, path);
      if (key !== `${worktreePath}|${path}|${mode}|${cached}`) return;
      if (text !== content) content = text;
      error = null;
    } catch (e) {
      error = String(e);
      content = "";
    }
    loaded = true;
  }

  $effect(() => {
    void worktreePath;
    void path;
    void mode;
    void cached;
    loaded = false;
    content = "";
    load();
  });

  onMount(() => {
    const timer = setInterval(() => {
      if (!document.hidden) load();
    }, POLL_MS);
    return () => clearInterval(timer);
  });

  let lines = $derived(content.split("\n"));
</script>

<div class="viewer">
  <div class="title">
    <span class="p">{path}</span>
    <span class="tag">{mode === "diff" ? (cached ? "staged diff" : "diff") : "read-only"}</span>
  </div>
  {#if error}
    <div class="err">{error}</div>
  {:else if !loaded}
    <div class="hint">Loading…</div>
  {:else if mode === "diff"}
    {#if content}
      <DiffViewer diff={content} />
    {:else}
      <div class="hint">No textual changes.</div>
    {/if}
  {:else}
    <pre class="code">{#each lines as l, i}<span class="ln">{i + 1}</span><span class="tx">{l || " "}</span>
{/each}</pre>
  {/if}
</div>

<style>
  .viewer {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: #11111b;
    user-select: text;
  }
  .title {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 4px 12px;
    font-size: 11px;
    color: #8b949e;
    border-bottom: 1px solid #30363d;
    flex-shrink: 0;
  }
  .tag {
    margin-left: auto;
    font-size: 10px;
    text-transform: uppercase;
    color: #6e7681;
  }
  .viewer :global(.diff) { flex: 1; min-height: 0; }
  .code {
    flex: 1;
    min-height: 0;
    margin: 0;
    overflow: auto;
    font-family: "SF Mono", Menlo, Consolas, monospace;
    font-size: 12px;
    line-height: 1.5;
    color: #c9d1d9;
    padding: 6px 0;
  }
  .ln {
    display: inline-block;
    width: 48px;
    padding-right: 12px;
    text-align: right;
    color: #6e7681;
    user-select: none;
  }
  .tx { white-space: pre; }
  .hint { padding: 10px 12px; font-size: 12px; color: #8b949e; }
  .err { padding: 10px 12px; font-size: 12px; color: #ffa198; }
</style>
