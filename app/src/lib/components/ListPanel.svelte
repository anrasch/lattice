<script lang="ts">
  import { onMount } from "svelte";
  import { api, type Node, type Edge } from "$lib/api";
  import { previewNote, pinNote } from "$lib/stores";

  let { kind }: { kind: "orphans" | "broken" } = $props();
  let nodes = $state<Node[]>([]);
  let edges = $state<Edge[]>([]);

  onMount(async () => {
    if (kind === "orphans") nodes = await api.orphans();
    else edges = await api.brokenLinks();
  });

  let count = $derived(kind === "orphans" ? nodes.length : edges.length);
</script>

<div class="list-panel">
  <div class="meta">{count} {kind === "orphans" ? "orphans" : "broken links"}</div>

  {#if kind === "orphans"}
    {#each nodes as n}
      <button
        class="hit"
        onclick={() => previewNote(n.path)}
        ondblclick={() => pinNote(n.path)}
        title={n.path}
      >
        {n.path}
      </button>
    {/each}
  {:else}
    {#each edges as e}
      <button
        class="hit broken"
        onclick={() => previewNote(e.src)}
        ondblclick={() => pinNote(e.src)}
        title={e.src}
      >
        <span class="src">{e.src.split("/").pop()}</span>
        <span class="arrow">→</span>
        <span class="tgt">{e.raw_target}</span>
      </button>
    {/each}
  {/if}
</div>

<style>
  .list-panel {
    padding: 10px;
  }
  .meta {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-faint);
    margin: 2px 4px 8px;
  }
  .hit {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: 0;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    color: var(--text-dim);
    font-family: var(--font-mono);
    font-size: 11px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .hit:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .arrow {
    color: var(--text-faint);
    margin: 0 5px;
  }
  .tgt {
    color: var(--danger);
  }
</style>
