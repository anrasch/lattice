<script lang="ts">
  import { api, type Node, type Edge } from "$lib/api";
  import { currentNote } from "$lib/stores";

  let view = $state<"none" | "orphans" | "broken" | "search">("none");
  let nodes = $state<Node[]>([]);
  let edges = $state<Edge[]>([]);
  let term = $state("");

  async function showOrphans() {
    view = "orphans";
    nodes = await api.orphans();
  }
  async function showBroken() {
    view = "broken";
    edges = await api.brokenLinks();
  }
  async function runSearch() {
    if (!term.trim()) return;
    view = "search";
    nodes = await api.search(term);
  }
</script>

<div class="queries">
  <div class="row">
    <button onclick={showOrphans}>Orphans</button>
    <button onclick={showBroken}>Broken</button>
  </div>
  <input
    placeholder="search…"
    bind:value={term}
    onkeydown={(e) => e.key === "Enter" && runSearch()}
  />

  {#if view === "broken"}
    <div class="count">{edges.length} broken</div>
    {#each edges as e}
      <button class="hit" onclick={() => currentNote.set(e.src)}
        >{e.src} → {e.raw_target}</button
      >
    {/each}
  {:else if view !== "none"}
    <div class="count">{nodes.length} {view}</div>
    {#each nodes as n}
      <button class="hit" onclick={() => currentNote.set(n.path)}>{n.path}</button>
    {/each}
  {/if}
</div>

<style>
  .queries {
    padding: 8px;
    border-bottom: 1px solid rgba(127, 127, 127, 0.25);
    font-size: 12px;
  }
  .row {
    display: flex;
    gap: 6px;
    margin-bottom: 6px;
  }
  input {
    width: 100%;
    box-sizing: border-box;
    margin-bottom: 6px;
  }
  .count {
    opacity: 0.5;
    margin: 4px 0;
  }
  .hit {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: 0;
    padding: 2px 4px;
    cursor: pointer;
    color: #4a8fe0;
    font: inherit;
  }
  .hit:hover {
    text-decoration: underline;
  }
</style>
