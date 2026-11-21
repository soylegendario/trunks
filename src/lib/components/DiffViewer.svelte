<script lang="ts">
  let { diff }: { diff: string } = $props();

  type Kind = "add" | "del" | "hunk" | "meta" | "ctx";

  function classify(line: string): Kind {
    if (line.startsWith("+++") || line.startsWith("---")) return "meta";
    if (line.startsWith("diff ") || line.startsWith("index ") || line.startsWith("new file") ||
        line.startsWith("deleted file") || line.startsWith("similarity") || line.startsWith("rename ")) {
      return "meta";
    }
    if (line.startsWith("@@")) return "hunk";
    if (line.startsWith("+")) return "add";
    if (line.startsWith("-")) return "del";
    return "ctx";
  }

  let lines = $derived(diff.split("\n").map((text) => ({ text, kind: classify(text) })));
</script>

<pre class="diff">{#each lines as l}<span class="line {l.kind}">{l.text || " "}</span>{/each}</pre>

<style>
  .diff {
    margin: 0;
    font-family: "SF Mono", Menlo, Consolas, monospace;
    font-size: 11px;
    line-height: 1.45;
    overflow: auto;
    background: #11111b;
    color: #c9d1d9;
    height: 100%;
  }
  .line {
    display: block;
    padding: 0 8px;
    white-space: pre;
    min-width: max-content;
  }
  .add { background: rgba(63, 185, 80, 0.15); color: #7ee787; }
  .del { background: rgba(248, 81, 73, 0.15); color: #ffa198; }
  .hunk { background: rgba(148, 72, 248, 0.15); color: #9448f8; }
  .meta { color: #8b949e; }
</style>
