<script lang="ts">
  import { api, type Node, type Edge } from "$lib/api";
  import { currentNote } from "$lib/stores";

  let { count = 0 }: { count?: number } = $props();

  let view = $state<"none" | "orphans" | "broken" | "search">("none");
  let nodes = $state<Node[]>([]);
  let edges = $state<Edge[]>([]);
  let term = $state("");

  async function showOrphans() {
    view = view === "orphans" ? "none" : "orphans";
    if (view === "orphans") nodes = await api.orphans();
  }
  async function showBroken() {
    view = view === "broken" ? "none" : "broken";
    if (view === "broken") edges = await api.brokenLinks();
  }
  async function runSearch() {
    if (!term.trim()) {
      view = "none";
      return;
    }
    view = "search";
    nodes = await api.search(term);
  }
</script>

<div class="queries">
  <div class="search">
    <svg viewBox="0 0 16 16" class="icon" aria-hidden="true">
      <circle cx="7" cy="7" r="4.5" fill="none" stroke="currentColor" stroke-width="1.5" />
      <line x1="10.5" y1="10.5" x2="14" y2="14" stroke="currentColor" stroke-width="1.5" />
    </svg>
    <input
      placeholder="Search notes"
      bind:value={term}
      onkeydown={(e) => e.key === "Enter" && runSearch()}
    />
  </div>

  <div class="pills">
    <button class="pill" class:on={view === "orphans"} onclick={showOrphans}>
      Orphans
    </button>
    <button class="pill" class:on={view === "broken"} onclick={showBroken}>
      Broken
    </button>
    <span class="total">{count}</span>
  </div>

  {#if view !== "none"}
    <div class="results">
      <div class="count">
        {view === "broken" ? edges.length : nodes.length}
        {view}
      </div>
      {#if view === "broken"}
        {#each edges as e}
          <button class="hit" onclick={() => currentNote.set(e.src)}>
            <span class="src">{e.src.split("/").pop()}</span>
            <span class="arrow">→</span>
            <span class="tgt">{e.raw_target}</span>
          </button>
        {/each}
      {:else}
        {#each nodes as n}
          <button class="hit" onclick={() => currentNote.set(n.path)}>
            {n.path}
          </button>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .queries {
    padding: 8px 12px 12px;
    border-bottom: 1px solid var(--border);
  }
  .search {
    display: flex;
    align-items: center;
    gap: 7px;
    background: var(--bg);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    padding: 0 9px;
    transition: border-color 0.15s ease;
  }
  .search:focus-within {
    border-color: var(--accent-line);
  }
  .icon {
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
    font-size: 12.5px;
    padding: 7px 0;
  }
  input::placeholder {
    color: var(--text-faint);
  }
  .pills {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
  }
  .pill {
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text-dim);
    font-size: 11.5px;
    font-weight: 550;
    padding: 4px 10px;
    border-radius: 20px;
    cursor: pointer;
    transition: all 0.13s ease;
  }
  .pill:hover {
    color: var(--text);
    border-color: var(--border-strong);
  }
  .pill.on {
    background: var(--accent-dim);
    border-color: var(--accent-line);
    color: var(--accent-bright);
  }
  .total {
    margin-left: auto;
    font-family: var(--font-mono);
    font-size: 10.5px;
    color: var(--text-faint);
  }
  .results {
    margin-top: 10px;
    max-height: 240px;
    overflow-y: auto;
  }
  .count {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-faint);
    margin-bottom: 4px;
  }
  .hit {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: 0;
    padding: 3px 6px;
    border-radius: 5px;
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
    margin: 0 4px;
  }
  .tgt {
    color: var(--danger);
  }
</style>
