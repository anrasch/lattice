<script lang="ts">
  import { openTabs, currentNote, closeTab } from "$lib/stores";

  function label(path: string) {
    return path.split("/").pop()?.replace(/\.md$/, "") ?? path;
  }
</script>

<div class="tabs">
  {#each $openTabs as path (path)}
    <div class="tab" class:active={path === $currentNote}>
      <button class="lbl" onclick={() => currentNote.set(path)} title={path}>
        {label(path)}
      </button>
      <button class="x" aria-label="Close" onclick={() => closeTab(path)}>
        <svg viewBox="0 0 12 12"
          ><path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" /></svg
        >
      </button>
    </div>
  {/each}
</div>

<style>
  .tabs {
    display: flex;
    align-items: stretch;
    height: 38px;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    overflow-x: auto;
    scrollbar-width: none;
  }
  .tabs::-webkit-scrollbar {
    display: none;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 0 6px 0 12px;
    max-width: 200px;
    border-right: 1px solid var(--border);
    color: var(--text-faint);
    position: relative;
  }
  .tab.active {
    background: var(--surface);
    color: var(--text);
  }
  .tab.active::after {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    height: 2px;
    background: var(--accent);
  }
  .lbl {
    background: none;
    border: 0;
    color: inherit;
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
    padding: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .x {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border: 0;
    background: none;
    border-radius: 4px;
    color: var(--text-faint);
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.12s ease;
  }
  .tab:hover .x,
  .tab.active .x {
    opacity: 1;
  }
  .x:hover {
    background: var(--surface-3);
    color: var(--text);
  }
  .x svg {
    width: 11px;
    height: 11px;
  }
</style>
