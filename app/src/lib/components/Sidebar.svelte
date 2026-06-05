<script lang="ts">
  import { onMount } from "svelte";
  import { api, type TreeEntry } from "$lib/api";
  import { currentNote, treeEntries } from "$lib/stores";
  import QueryViews from "./QueryViews.svelte";

  let entries = $state<TreeEntry[]>([]);

  onMount(async () => {
    entries = await api.tree();
    treeEntries.set(entries);
  });

  let grouped = $derived(
    entries.reduce<Record<string, TreeEntry[]>>((acc, e) => {
      (acc[e.dir] ??= []).push(e);
      return acc;
    }, {}),
  );
</script>

<QueryViews />

<nav class="sidebar">
  {#each Object.keys(grouped).sort() as dir}
    <div class="dir">{dir || "/"}</div>
    {#each grouped[dir] as e}
      <button
        class="entry"
        class:index={e.is_index}
        onclick={() => currentNote.set(e.path)}
      >
        {e.name}
      </button>
    {/each}
  {/each}
</nav>

<style>
  .sidebar {
    overflow-y: auto;
    padding: 8px;
    font-size: 13px;
  }
  .dir {
    font-weight: 600;
    opacity: 0.55;
    margin: 10px 0 2px;
    font-family: monospace;
  }
  .entry {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: 0;
    padding: 2px 6px;
    cursor: pointer;
    border-radius: 4px;
    color: inherit;
    font: inherit;
  }
  .entry:hover {
    background: rgba(127, 127, 127, 0.15);
  }
  .entry.index {
    font-style: italic;
  }
</style>
