<script lang="ts">
  import Sidebar from "$lib/components/Sidebar.svelte";
  import NoteView from "$lib/components/NoteView.svelte";
  import LinksPanel from "$lib/components/LinksPanel.svelte";
  import { currentNote } from "$lib/stores";
</script>

<main class="shell">
  <aside class="left"><Sidebar /></aside>
  <section class="center">
    {#if $currentNote}
      <div class="topbar"><span class="path">{$currentNote}</span></div>
      <NoteView note={$currentNote} />
    {:else}
      <p class="empty">Select a note from the sidebar.</p>
    {/if}
  </section>
  <aside class="right">
    {#if $currentNote}<LinksPanel note={$currentNote} />{/if}
  </aside>
</main>

<style>
  :global(body) {
    margin: 0;
  }
  .shell {
    display: grid;
    grid-template-columns: 260px 1fr 280px;
    height: 100vh;
    font-family: Inter, system-ui, sans-serif;
  }
  .left {
    border-right: 1px solid rgba(127, 127, 127, 0.25);
    overflow-y: auto;
  }
  .right {
    border-left: 1px solid rgba(127, 127, 127, 0.25);
    padding: 8px;
    overflow-y: auto;
  }
  .center {
    padding: 16px 24px;
    overflow-y: auto;
  }
  .topbar {
    margin-bottom: 12px;
  }
  .path {
    font-family: monospace;
    opacity: 0.5;
    font-size: 12px;
  }
  .empty {
    opacity: 0.5;
  }
</style>
