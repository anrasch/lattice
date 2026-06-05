<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { buildTree, folderPaths, type TreeItem } from "$lib/tree";
  import { currentNote, treeEntries, collapsed, openNote } from "$lib/stores";

  let tree = $state<TreeItem[]>([]);

  onMount(async () => {
    const entries = await api.tree();
    treeEntries.set(entries);
    tree = buildTree(entries);
    // Start fully collapsed; the vault is large.
    collapsed.set(new Set(folderPaths(entries)));
  });

  function toggle(path: string) {
    collapsed.update((s) => {
      const n = new Set(s);
      if (n.has(path)) n.delete(path);
      else n.add(path);
      return n;
    });
  }
</script>

{#snippet item(node: TreeItem, depth: number)}
  {#if node.kind === "folder"}
    <button
      class="row folder"
      style="padding-left: {depth * 13 + 8}px"
      onclick={() => toggle(node.path)}
    >
      <svg class="chev" class:open={!$collapsed.has(node.path)} viewBox="0 0 12 12">
        <path d="M4.5 3 L8 6 L4.5 9" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
      <span class="fname">{node.name}</span>
    </button>
    {#if !$collapsed.has(node.path)}
      {#each node.children as child}
        {@render item(child, depth + 1)}
      {/each}
    {/if}
  {:else}
    <button
      class="row file"
      class:active={node.path === $currentNote}
      class:index={node.isIndex}
      style="padding-left: {depth * 13 + 24}px"
      title={node.path}
      onclick={() => openNote(node.path)}
    >
      {node.name.replace(/\.md$/, "")}
    </button>
  {/if}
{/snippet}

<div class="filetree">
  {#each tree as node}
    {@render item(node, 0)}
  {/each}
</div>

<style>
  .filetree {
    padding: 4px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    text-align: left;
    background: none;
    border: 0;
    padding: 3px 8px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    color: var(--text-dim);
    font: inherit;
    font-size: 13px;
    line-height: 1.35;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .row:hover {
    background: var(--surface-2);
  }
  .folder {
    color: var(--text);
    font-weight: 500;
  }
  .chev {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
    color: var(--text-faint);
    transition: transform 0.12s ease;
  }
  .chev.open {
    transform: rotate(90deg);
  }
  .fname {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .file:hover {
    color: var(--text);
  }
  .file.index {
    color: var(--text);
  }
  .file.active {
    background: var(--accent-dim);
    color: var(--accent-bright);
  }
</style>
