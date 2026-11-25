<script lang="ts">
  import { untrack } from "svelte";
  import {
    gitCancelCommitHistory,
    gitGetCommitHistory,
    gitListBranches,
    type GitBranchInfo,
    type GitCommitInfo,
  } from "$lib/git";
  import {
    applyPage,
    emptyHistory,
    historyKey,
    isHistoryStale,
    retarget,
    type HistoryState,
  } from "$lib/commit-history-logic";

  let {
    worktreePath,
    headOid,
    onOpenCommit,
  }: {
    worktreePath: string | null;
    /** HEAD reported by the status poll; undefined until the first poll arrives. */
    headOid?: string | null;
    onOpenCommit?: (oid: string, shortHash: string, subject: string) => void;
  } = $props();

  // Explicit base per worktree, kept in memory for the session only.
  let baseByWorktree = $state<Record<string, string>>({});
  let branches = $state<GitBranchInfo[]>([]);
  let history = $state<HistoryState>(emptyHistory(""));
  let loading = $state(false);
  let error = $state<string | null>(null);
  let activeRequest: string | null = null;
  let counter = 0;

  let baseRef = $derived(worktreePath ? (baseByWorktree[worktreePath] ?? null) : null);

  function newRequestId(): string {
    counter += 1;
    return `ch-${Date.now()}-${counter}`;
  }

  function cancelActive() {
    if (activeRequest) {
      void gitCancelCommitHistory(activeRequest).catch(() => {});
      activeRequest = null;
    }
  }

  async function load(more: boolean) {
    if (!worktreePath) return;
    const path = worktreePath;
    const base = baseRef;
    const key = historyKey(path, base);
    history = retarget(history, key);
    if (more && !history.cursor) return;
    cancelActive();
    const id = newRequestId();
    activeRequest = id;
    loading = true;
    error = null;
    try {
      const page = await gitGetCommitHistory(path, id, base ?? undefined, more ? (history.cursor ?? undefined) : undefined);
      if (activeRequest !== id) return;
      history = applyPage(history, key, page, !more);
    } catch (e) {
      if (activeRequest !== id) return; // superseded or cancelled
      if (key === history.key) error = String(e);
    } finally {
      if (activeRequest === id) {
        activeRequest = null;
        loading = false;
      }
    }
  }

  // Worktree or base change: drop accumulated pages and fetch the first page again.
  $effect(() => {
    void worktreePath;
    void baseRef;
    const path = worktreePath;
    const base = baseRef;
    untrack(() => {
      history = emptyHistory(path ? historyKey(path, base) : "");
      error = null;
      loading = false;
    });
    if (path) {
      untrack(() => void load(false));
      void gitListBranches(worktreePath)
        .then((b) => {
          if (b) branches = b;
        })
        .catch(() => (branches = []));
    } else {
      cancelActive();
    }
    return cancelActive;
  });

  // HEAD moved according to the status poll: refetch the first page. The history query
  // itself is never polled.
  $effect(() => {
    if (worktreePath && !loading && isHistoryStale(history, headOid)) untrack(() => void load(false));
  });

  function setBase(value: string) {
    if (!worktreePath) return;
    const next = { ...baseByWorktree };
    if (value) next[worktreePath] = value;
    else delete next[worktreePath];
    baseByWorktree = next;
  }

  const fmtDate = (t: number) => new Date(t * 1000).toLocaleDateString();

  type Section = { title: string | null; commits: GitCommitInfo[] };
  let sections = $derived.by<Section[]>(() => {
    const m = history.meta;
    const out: Section[] = [];
    const only = history.commits.filter((c) => c.group === "branch_only");
    const shared = history.commits.filter((c) => c.group === "shared");
    const flat = history.commits.filter((c) => c.group === "flat");
    if (flat.length) out.push({ title: null, commits: flat });
    if (only.length) out.push({ title: `Only reachable from ${m?.branch ?? "this branch"}`, commits: only });
    if (shared.length) out.push({ title: `Shared history with ${m?.base ?? "base"}`, commits: shared });
    return out;
  });

  let meta = $derived(history.meta);
</script>

<div class="history">
  <div class="toolbar">
    <label>
      Base
      <select value={baseRef ?? ""} onchange={(e) => setBase(e.currentTarget.value)} disabled={!worktreePath}>
        <option value="">Automatic</option>
        {#each branches as b (b.name + b.is_remote)}
          <option value={b.name}>{b.name}{b.is_remote ? " (remote)" : ""}</option>
        {/each}
      </select>
    </label>
    <button onclick={() => load(false)} disabled={loading || !worktreePath} title="Refresh">↻</button>
  </div>

  <div class="list">
    {#if error}
      <div class="hint error">{error}</div>
    {/if}

    {#if meta}
      {#if meta.head_state === "detached"}
        <div class="hint">Detached HEAD: no branch is checked out. Showing reachable history as a flat list.</div>
      {:else if meta.head_state === "unborn"}
        <div class="hint">This branch has no commits yet.</div>
      {/if}
      {#if meta.partial_history}
        <div class="hint warn">Shallow repository: this is a partial history.</div>
      {/if}
      {#if meta.head_state === "branch" && !meta.partial_history}
        {#if meta.base}
          <div class="hint">
            Base: <strong>{meta.base}</strong> ({meta.base_source === "explicit" ? "chosen" : "automatic"}).
            “Only reachable” is graph reachability: squash-merged or rebased commits can still appear there.
          </div>
        {:else}
          <div class="hint">Base unavailable: showing a flat history.</div>
        {/if}
      {/if}
    {/if}

    {#each sections as section}
      {#if section.title}<div class="group">{section.title}</div>{/if}
      {#each section.commits as c (c.hash)}
        <button
          type="button"
          class="row"
          title={`${c.hash}\n${c.author}`}
          onclick={() => onOpenCommit?.(c.hash, c.short_hash, c.subject)}
        >
          <span class="hash">{c.short_hash}</span>
          <span class="subject">{c.subject}</span>
          <span class="meta">{c.author} · {fmtDate(c.commit_time)}</span>
        </button>
      {/each}
    {/each}

    {#if loading}
      <div class="hint">Loading…</div>
    {:else if meta && history.commits.length === 0 && meta.head_state !== "unborn" && !error}
      <div class="hint">No commits to show.</div>
    {/if}

    {#if history.cursor && !loading}
      <button class="more" onclick={() => load(true)}>Load more</button>
    {/if}
  </div>
</div>

<style>
  .history { display: flex; flex-direction: column; height: 100%; min-height: 0; font-size: 12px; }
  .toolbar { display: flex; gap: 6px; align-items: center; padding: 4px 8px; border-bottom: 1px solid #30363d; }
  .toolbar label { display: flex; gap: 6px; align-items: center; flex: 1; min-width: 0; color: #8b949e; }
  .toolbar select { flex: 1; min-width: 0; background: #0d1117; color: #c9d1d9; border: 1px solid #30363d; font: inherit; }
  .toolbar button { background: none; border: 1px solid #30363d; color: #c9d1d9; cursor: pointer; }
  .list { flex: 1; overflow: auto; }
  .hint { padding: 6px 10px; color: #8b949e; }
  .hint.error { color: #f85149; }
  .hint.warn { color: #d29922; }
  .group { padding: 4px 10px; background: #161b22; color: #8b949e; border-top: 1px solid #30363d; border-bottom: 1px solid #30363d; position: sticky; top: 0; }
  .row { display: grid; grid-template-columns: auto 1fr; gap: 0 8px; padding: 3px 10px; width: 100%; background: none; border: 0; font: inherit; text-align: left; cursor: pointer; color: inherit; }
  .row:hover, .row:focus-visible { background: #161b22; outline: none; }
  .hash { font-family: monospace; color: var(--accent-primary, #9448f8); }
  .subject { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: #c9d1d9; }
  .meta { grid-column: 2; color: #6e7681; font-size: 11px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .more { margin: 8px 10px; background: none; border: 1px solid #30363d; color: #c9d1d9; cursor: pointer; padding: 4px 8px; }
</style>
