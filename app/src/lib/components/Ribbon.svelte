<script lang="ts">
  import { leftView, leftOpen, type LeftView } from "$lib/stores";

  const items: { id: LeftView; label: string }[] = [
    { id: "files", label: "Files" },
    { id: "search", label: "Search" },
    { id: "orphans", label: "Orphans" },
    { id: "broken", label: "Broken links" },
  ];

  function pick(id: LeftView) {
    if ($leftView === id && $leftOpen) {
      leftOpen.set(false);
    } else {
      leftView.set(id);
      leftOpen.set(true);
    }
  }
</script>

<nav class="ribbon">
  {#each items as it}
    <button
      class="rb"
      class:active={$leftView === it.id && $leftOpen}
      title={it.label}
      aria-label={it.label}
      onclick={() => pick(it.id)}
    >
      {#if it.id === "files"}
        <svg viewBox="0 0 18 18"
          ><path
            d="M3 4.5h5l1.5 1.5H15v7.5a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V4.5Z"
            fill="none"
            stroke="currentColor"
            stroke-width="1.4"
          /></svg
        >
      {:else if it.id === "search"}
        <svg viewBox="0 0 18 18"
          ><circle cx="8" cy="8" r="5" fill="none" stroke="currentColor" stroke-width="1.4" /><line
            x1="11.8"
            y1="11.8"
            x2="15.5"
            y2="15.5"
            stroke="currentColor"
            stroke-width="1.4"
          /></svg
        >
      {:else if it.id === "orphans"}
        <svg viewBox="0 0 18 18"
          ><circle cx="9" cy="9" r="3.2" fill="none" stroke="currentColor" stroke-width="1.4" /><circle
            cx="9"
            cy="9"
            r="6.5"
            fill="none"
            stroke="currentColor"
            stroke-width="1.1"
            stroke-dasharray="2 2.4"
            opacity="0.6"
          /></svg
        >
      {:else}
        <svg viewBox="0 0 18 18"
          ><path
            d="M7.3 10.7 5.6 12.4a2.4 2.4 0 0 1-3.4-3.4l1.7-1.7M10.7 7.3l1.7-1.7a2.4 2.4 0 0 1 3.4 3.4l-1.7 1.7"
            fill="none"
            stroke="currentColor"
            stroke-width="1.4"
            stroke-linecap="round"
          /><line
            x1="11.5"
            y1="6.5"
            x2="6.5"
            y2="11.5"
            stroke="currentColor"
            stroke-width="1.4"
            stroke-linecap="round"
            opacity="0.45"
          /></svg
        >
      {/if}
    </button>
  {/each}
</nav>

<style>
  .ribbon {
    grid-column: 1;
    background: var(--bg);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    align-items: center;
    padding-top: 8px;
    gap: 2px;
  }
  .rb {
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    background: none;
    border: 0;
    border-radius: var(--radius-sm);
    color: var(--text-faint);
    cursor: pointer;
    transition:
      color 0.12s ease,
      background 0.12s ease;
  }
  .rb svg {
    width: 17px;
    height: 17px;
  }
  .rb:hover {
    color: var(--text-dim);
    background: var(--surface-2);
  }
  .rb.active {
    color: var(--accent);
    background: var(--accent-dim);
  }
</style>
