<script lang="ts">
  import { onMount } from "svelte";
  import { gitGetDiff, gitReadFile } from "$lib/git";
  import DiffViewer from "./DiffViewer.svelte";
  import { isMarkdownPath, renderMarkdown } from "$lib/markdown-logic";

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

  let view = $state<"preview" | "code" | "split">("preview");
  let isMd = $derived(mode === "file" && isMarkdownPath(path));
  let html = $derived(isMd && view !== "code" ? renderMarkdown(content) : "");

  $effect(() => {
    void path;
    view = "preview";
  });

  function onMdClick(e: MouseEvent) {
    if ((e.target as HTMLElement | null)?.closest("a")) e.preventDefault();
  }
</script>

<div class="viewer">
  <div class="title">
    <span class="p">{path}</span>
    {#if isMd}
      <div class="seg" role="group" aria-label="Vista">
        <button class:on={view === "code"} onclick={() => (view = "code")} title="Código" aria-label="Código" aria-pressed={view === "code"}>
          <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"><path d="M2 3.5h12M2 6.5h12M2 9.5h12M2 12.5h12" /></svg>
        </button>
        <button class:on={view === "split"} onclick={() => (view = "split")} title="Vista dividida" aria-label="Vista dividida" aria-pressed={view === "split"}>
          <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round" stroke-linecap="round"><rect x="2" y="2.5" width="12" height="11" rx="1.5" /><path d="M8 2.5v11" /></svg>
        </button>
        <button class:on={view === "preview"} onclick={() => (view = "preview")} title="Vista previa" aria-label="Vista previa" aria-pressed={view === "preview"}>
          <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round" stroke-linecap="round"><rect x="2" y="2.5" width="12" height="11" rx="1.5" /><path d="M3 12l3.5-4 2.5 2.5 2-2L13 12" /></svg>
        </button>
      </div>
    {/if}
    <span class="tag" class:first={!isMd}>{mode === "diff" ? (cached ? "staged diff" : "diff") : "read-only"}</span>
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
  {:else if isMd && view === "split"}
    <div class="split">
      <pre class="code">{#each lines as l, i}<span class="ln">{i + 1}</span><span class="tx">{l || " "}</span>
{/each}</pre>
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div class="md" onclick={onMdClick}>{@html html}</div>
    </div>
  {:else if isMd && view === "preview"}
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div class="md" onclick={onMdClick}>{@html html}</div>
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
  .seg { display: flex; margin-left: auto; border: 1px solid #30363d; border-radius: 4px; overflow: hidden; }
  .seg button { display: flex; align-items: center; background: none; border: none; color: #8b949e; padding: 3px 8px; cursor: pointer; }
  .seg button:hover { color: #c9d1d9; }
  .seg button.on { background: #30363d; color: #c9d1d9; }
  .seg + .tag { margin-left: 0; }
  .split { flex: 1; min-height: 0; display: flex; }
  .split > .code, .split > .md { flex: 1 1 0; min-width: 0; }
  .split > .md { border-left: 1px solid #30363d; }
  .md { flex: 1; min-height: 0; overflow: auto; padding: 12px 20px; color: #c9d1d9; font-size: 13px; line-height: 1.6; }
  .md :global(h1), .md :global(h2) { border-bottom: 1px solid #30363d; padding-bottom: 4px; }
  .md :global(h1), .md :global(h2), .md :global(h3), .md :global(h4) { color: #e6edf3; margin: 16px 0 8px; }
  .md :global(a) { color: #58a6ff; }
  .md :global(code) { background: #1e1e2e; padding: 1px 4px; border-radius: 3px; font-family: "SF Mono", Menlo, Consolas, monospace; font-size: 12px; }
  .md :global(pre) { background: #1e1e2e; padding: 10px; border-radius: 4px; overflow: auto; }
  .md :global(pre code) { background: none; padding: 0; }
  .md :global(blockquote) { margin: 8px 0; padding: 0 12px; border-left: 3px solid #30363d; color: #8b949e; }
  .md :global(table) { border-collapse: collapse; }
  .md :global(th), .md :global(td) { border: 1px solid #30363d; padding: 4px 8px; }
  .md :global(img) { max-width: 100%; }
  .md :global(hr) { border: none; border-top: 1px solid #30363d; }
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
