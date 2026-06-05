<script lang="ts">
  import Sidebar from "$lib/components/Sidebar.svelte";
  import NoteView from "$lib/components/NoteView.svelte";
  import LinksPanel from "$lib/components/LinksPanel.svelte";
  import Editor from "$lib/components/Editor.svelte";
  import { currentNote, mode } from "$lib/stores";

  let editor = $state<{ save: () => void } | undefined>();

  // Switching notes always returns to read mode (never land in a stale editor).
  let lastNote: string | null = null;
  $effect(() => {
    if ($currentNote !== lastNote) {
      lastNote = $currentNote;
      mode.set("read");
    }
  });

  function onKey(e: KeyboardEvent) {
    if (!(e.metaKey || e.ctrlKey)) return;
    if (e.key === "e") {
      e.preventDefault();
      if ($currentNote) mode.update((m) => (m === "read" ? "edit" : "read"));
    } else if (e.key === "s") {
      e.preventDefault();
      editor?.save();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<main class="shell">
  <aside class="left"><Sidebar /></aside>
  <section class="center">
    {#if $currentNote}
      <div class="topbar">
        <span class="path">{$currentNote}</span>
        <button onclick={() => mode.update((m) => (m === "read" ? "edit" : "read"))}>
          {$mode === "read" ? "Edit (⌘E)" : "Reading"}
        </button>
      </div>
      {#if $mode === "edit"}
        <Editor bind:this={editor} note={$currentNote} />
      {:else}
        <NoteView note={$currentNote} />
      {/if}
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
    display: flex;
    flex-direction: column;
  }
  .topbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
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
