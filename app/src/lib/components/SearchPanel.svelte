<script lang="ts">
  import { api, type Node } from "$lib/api";
  import { previewNote, pinNote } from "$lib/stores";

  let term = $state("");
  let results = $state<Node[]>([]);
  let searched = $state(false);

  async function run() {
    if (!term.trim()) {
      results = [];
      searched = false;
      return;
    }
    results = await api.search(term);
    searched = true;
  }
</script>

<div class="search-panel">
  <div class="field">
    <svg viewBox="0 0 16 16" class="ico" aria-hidden="true">
      <circle cx="7" cy="7" r="4.5" fill="none" stroke="currentColor" stroke-width="1.4" />
      <line x1="10.5" y1="10.5" x2="14" y2="14" stroke="currentColor" stroke-width="1.4" />
    </svg>
    <!-- svelte-ignore a11y_autofocus -->
    <input
      autofocus
      placeholder="Search notes…"
      bind:value={term}
      oninput={run}
      onkeydown={(e) => e.key === "Enter" && run()}
    />
  </div>

  {#if searched}
    <div class="meta">{results.length} results</div>
    {#each results as n}
      <button class="hit" onclick={() => previewNote(n.path)} ondblclick={() => pinNote(n.path)}>
        <span class="t">{n.title}</span>
        <span class="p">{n.path}</span>
      </button>
    {/each}
  {/if}
</div>

<style>
  .search-panel {
    padding: 10px;
  }
  .field {
    display: flex;
    align-items: center;
    gap: 7px;
    background: var(--bg);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    padding: 0 9px;
  }
  .field:focus-within {
    border-color: var(--accent-line);
  }
  .ico {
    width: 13px;
    height: 13px;
    color: var(--text-faint);
    flex-shrink: 0;
  }
  input {
    flex: 1;
    background: none;
    border: 0;
    outline: none;
    color: var(--text);
    font: inherit;
    font-size: 13px;
    padding: 7px 0;
  }
  input::placeholder {
    color: var(--text-faint);
  }
  .meta {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-faint);
    margin: 12px 4px 6px;
  }
  .hit {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: 0;
    padding: 5px 8px;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .hit:hover {
    background: var(--surface-2);
  }
  .t {
    display: block;
    color: var(--text);
    font-size: 13px;
  }
  .p {
    display: block;
    color: var(--text-faint);
    font-family: var(--font-mono);
    font-size: 10.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
