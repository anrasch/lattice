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
  let dirs = $derived(Object.keys(grouped).sort());
</script>

<QueryViews count={entries.length} />

<div class="files-head">Files</div>
<nav class="tree">
  {#each dirs as dir}
    {#if dir}<div class="dir">{dir}/</div>{/if}
    {#each grouped[dir] as e}
      <button
        class="entry"
        class:index={e.is_index}
        class:active={e.path === $currentNote}
        onclick={() => currentNote.set(e.path)}
        title={e.title}
      >
        {e.name}
      </button>
    {/each}
  {/each}
</nav>

<style>
  .files-head {
    font-family: var(--font-mono);
    font-size: 10.5px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--text-faint);
    padding: 14px 16px 6px;
  }
  .tree {
    flex: 1;
    overflow-y: auto;
    padding: 0 8px 16px;
  }
  .dir {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-faint);
    margin: 12px 0 3px;
    padding: 0 8px;
  }
  .entry {
    position: relative;
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: 0;
    padding: 4px 10px;
    margin: 1px 0;
    cursor: pointer;
    border-radius: var(--radius-sm);
    color: var(--text-dim);
    font: inherit;
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition:
      background 0.12s ease,
      color 0.12s ease;
  }
  .entry:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .entry.index {
    color: var(--text);
    font-weight: 550;
  }
  .entry.active {
    background: var(--accent-dim);
    color: var(--accent-bright);
  }
  .entry.active::before {
    content: "";
    position: absolute;
    left: 0;
    top: 6px;
    bottom: 6px;
    width: 2px;
    border-radius: 2px;
    background: var(--accent);
  }
</style>
