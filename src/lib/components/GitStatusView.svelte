<script lang="ts">
  import type { GitStatusEntry, GitStatusResult } from "$lib/git";

  let {
    worktreePath,
    status,
    error,
    onOpenFile,
  }: {
    worktreePath: string;
    status: GitStatusResult | null;
    error: string | null;
    onOpenFile?: (path: string, mode: "file" | "diff", cached: boolean) => void;
  } = $props();

  let selected = $state<{ path: string; cached: boolean } | null>(null);

  const isNone = (c: string) => c === "." || c === " " || c === "";

  function badge(e: GitStatusEntry): string {
    if (e.kind === "untracked") return "?";
    if (e.kind === "conflicted") return "U";
    const c = isNone(e.worktree_status) ? e.index_status : e.worktree_status;
    return c;
  }

  function badgeClass(e: GitStatusEntry): string {
    if (e.kind === "untracked") return "untracked";
    if (e.kind === "conflicted") return "conflicted";
    return badge(e) === "D" ? "deleted" : badge(e) === "A" ? "added" : "modified";
  }

  let staged = $derived(
    (status?.entries ?? []).filter((e) => e.kind === "tracked" && !isNone(e.index_status))
  );
  let unstaged = $derived(
    (status?.entries ?? []).filter(
      (e) => e.kind === "untracked" || e.kind === "conflicted" || !isNone(e.worktree_status)
    )
  );

  type Row =
    | { type: "dir"; key: string; name: string; depth: number; count: number; open: boolean }
    | { type: "file"; key: string; name: string; depth: number; entry: GitStatusEntry };
  type DirNode = { dirs: Map<string, DirNode>; files: GitStatusEntry[]; count: number };

  // Directories the user collapsed, keyed by "section:dir/path".
  let collapsed = $state(new Set<string>());

  function toggleDir(key: string) {
    const next = new Set(collapsed);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    collapsed = next;
  }

  function buildRows(entries: GitStatusEntry[], section: string, hidden: Set<string>): Row[] {
    const root: DirNode = { dirs: new Map(), files: [], count: 0 };
    for (const e of entries) {
      const parts = e.path.split("/");
      let node = root;
      node.count++;
      for (const p of parts.slice(0, -1)) {
        let child = node.dirs.get(p);
        if (!child) {
          child = { dirs: new Map(), files: [], count: 0 };
          node.dirs.set(p, child);
        }
        child.count++;
        node = child;
      }
      node.files.push(e);
    }
    const rows: Row[] = [];
    const walk = (node: DirNode, prefix: string, depth: number) => {
      const names = [...node.dirs.keys()].sort();
      for (const name of names) {
        const child = node.dirs.get(name)!;
        const key = `${section}:${prefix}${name}`;
        const open = !hidden.has(key);
        rows.push({ type: "dir", key, name, depth, count: child.count, open });
        if (open) walk(child, `${prefix}${name}/`, depth + 1);
      }
      for (const e of [...node.files].sort((a, b) => a.path.localeCompare(b.path))) {
        rows.push({
          type: "file",
          key: `${section}:${e.path}`,
          name: e.path.split("/").pop() ?? e.path,
          depth,
          entry: e,
        });
      }
    };
    walk(root, "", 0);
    return rows;
  }

  let stagedRows = $derived(buildRows(staged, "s", collapsed));
  let unstagedRows = $derived(buildRows(unstaged, "u", collapsed));

  function select(e: GitStatusEntry, cached: boolean) {
    selected = { path: e.path, cached };
    onOpenFile?.(e.path, "diff", cached);
  }

  // Drop the selection when the worktree changes.
  $effect(() => {
    void worktreePath;
    selected = null;
  });
</script>

<div class="git-status">
  {#if error}
    <div class="err">{error}</div>
  {:else if !status}
    <div class="hint">Loading status…</div>
  {:else if status.entries.length === 0}
    <div class="hint">Working tree clean.</div>
  {:else}
    <div class="lists">
      {#if staged.length}
        <h4>Staged ({staged.length})</h4>
        {#each stagedRows as r (r.key)}
          {#if r.type === "dir"}
            <button class="item dir" style:padding-left="{10 + r.depth * 12}px" onclick={() => toggleDir(r.key)}>
              <span class="chev">{r.open ? "▾" : "▸"}</span>
              <span class="path">{r.name}</span>
              <span class="cnt">{r.count}</span>
            </button>
          {:else}
            <button
              class="item"
              style:padding-left="{10 + r.depth * 12}px"
              class:active={selected?.path === r.entry.path && selected?.cached}
              onclick={() => select(r.entry, true)}
            >
              <span class="badge added">{r.entry.index_status}</span>
              <span class="path" title={r.entry.path}>{r.name}</span>
            </button>
          {/if}
        {/each}
      {/if}
      {#if unstaged.length}
        <h4>Changes ({unstaged.length})</h4>
        {#each unstagedRows as r (r.key)}
          {#if r.type === "dir"}
            <button class="item dir" style:padding-left="{10 + r.depth * 12}px" onclick={() => toggleDir(r.key)}>
              <span class="chev">{r.open ? "▾" : "▸"}</span>
              <span class="path">{r.name}</span>
              <span class="cnt">{r.count}</span>
            </button>
          {:else}
            <button
              class="item"
              style:padding-left="{10 + r.depth * 12}px"
              class:active={selected?.path === r.entry.path && !selected?.cached}
              onclick={() => select(r.entry, false)}
            >
              <span class="badge {badgeClass(r.entry)}">{badge(r.entry)}</span>
              <span class="path" title={r.entry.path}>{r.name}</span>
            </button>
          {/if}
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .git-status { display: flex; flex-direction: column; height: 100%; min-height: 0; }
  .lists { overflow-y: auto; flex: 1; min-height: 0; }
  h4 {
    margin: 0;
    padding: 6px 10px 2px;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: #8b949e;
  }
  .item {
    display: flex;
    gap: 8px;
    align-items: center;
    width: 100%;
    padding: 2px 10px;
    background: none;
    border: none;
    color: #c9d1d9;
    font: inherit;
    font-size: 12px;
    text-align: left;
    cursor: pointer;
  }
  .item:hover { background: rgba(255, 255, 255, 0.05); }
  .item.active { background: var(--accent-muted, rgba(148, 72, 248, 0.18)); }
  .path { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .badge {
    width: 14px;
    text-align: center;
    font-family: monospace;
    font-weight: 700;
    flex-shrink: 0;
  }
  .dir { color: #8b949e; }
  .chev { width: 14px; text-align: center; flex-shrink: 0; font-size: 10px; }
  .cnt { margin-left: auto; font-size: 10px; color: #6e7681; }
  .modified { color: #d29922; }
  .added { color: #3fb950; }
  .deleted { color: #f85149; }
  .untracked { color: #8b949e; }
  .conflicted { color: #f0883e; }
  .hint { padding: 8px 10px; font-size: 12px; color: #8b949e; }
  .err { padding: 8px 10px; font-size: 11px; color: #ffa198; }
</style>
