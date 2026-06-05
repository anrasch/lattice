<script lang="ts">
  import Sidebar from "$lib/components/Sidebar.svelte";
  import NoteView from "$lib/components/NoteView.svelte";
  import LinksPanel from "$lib/components/LinksPanel.svelte";
  import Editor from "$lib/components/Editor.svelte";
  import { currentNote, mode } from "$lib/stores";

  let editor = $state<{ save: () => void } | undefined>();

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

  let fileName = $derived($currentNote ? $currentNote.split("/").pop() : "");
  let fileDir = $derived(
    $currentNote && $currentNote.includes("/")
      ? $currentNote.slice(0, $currentNote.lastIndexOf("/") + 1)
      : "",
  );
</script>

<svelte:window onkeydown={onKey} />

<main class="shell">
  <aside class="left">
    <header class="brand">
      <span class="mark" aria-hidden="true"></span>
      <span class="wordmark">lattice</span>
    </header>
    <Sidebar />
  </aside>

  <section class="center">
    {#if $currentNote}
      <div class="topbar">
        <div class="crumb">
          <span class="dir">{fileDir}</span><span class="file">{fileName}</span>
        </div>
        <button
          class="edit-toggle"
          class:active={$mode === "edit"}
          onclick={() => mode.update((m) => (m === "read" ? "edit" : "read"))}
        >
          {$mode === "read" ? "Edit" : "Editing"}
          <kbd>⌘E</kbd>
        </button>
      </div>
      <div class="content">
        {#if $mode === "edit"}
          <Editor bind:this={editor} note={$currentNote} />
        {:else}
          <NoteView note={$currentNote} />
        {/if}
      </div>
    {:else}
      <div class="empty">
        <div class="lattice-motif" aria-hidden="true"></div>
        <p class="empty-title">A graph of your notes</p>
        <p class="empty-hint">
          Pick a note from the left, or run <em>Orphans</em>, <em>Broken</em>, or
          <em>Search</em> to explore the vault.
        </p>
      </div>
    {/if}
  </section>

  <aside class="right">
    {#if $currentNote}
      <div class="pane-head">Connections</div>
      <LinksPanel note={$currentNote} />
    {/if}
  </aside>
</main>

<style>
  .shell {
    display: grid;
    grid-template-columns: 272px 1fr 300px;
    height: 100vh;
    overflow: hidden;
  }

  .left {
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border-right: 1px solid var(--border);
    overflow: hidden;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 16px 16px 12px;
  }
  .mark {
    width: 16px;
    height: 16px;
    border-radius: 4px;
    background:
      linear-gradient(var(--accent), var(--accent)) 0 0 / 100% 1px no-repeat,
      linear-gradient(var(--accent), var(--accent)) 0 100% / 100% 1px no-repeat,
      linear-gradient(var(--accent), var(--accent)) 0 0 / 1px 100% no-repeat,
      linear-gradient(var(--accent), var(--accent)) 100% 0 / 1px 100% no-repeat,
      linear-gradient(var(--accent-line), var(--accent-line)) 50% 0 / 1px 100%
        no-repeat,
      linear-gradient(var(--accent-line), var(--accent-line)) 0 50% / 100% 1px
        no-repeat;
    opacity: 0.9;
  }
  .wordmark {
    font-family: var(--font-mono);
    font-weight: 600;
    font-size: 13px;
    letter-spacing: 0.14em;
    color: var(--text);
  }

  .center {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .topbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 28px;
    height: 52px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .crumb {
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .crumb .dir {
    color: var(--text-faint);
  }
  .crumb .file {
    color: var(--text-dim);
  }
  .edit-toggle {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    background: var(--surface-2);
    border: 1px solid var(--border-strong);
    color: var(--text-dim);
    padding: 5px 10px 5px 12px;
    border-radius: var(--radius-sm);
    font-size: 12.5px;
    font-weight: 550;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .edit-toggle:hover {
    color: var(--text);
    border-color: var(--accent-line);
  }
  .edit-toggle.active {
    background: var(--accent-dim);
    border-color: var(--accent-line);
    color: var(--accent-bright);
  }
  kbd {
    font-family: var(--font-mono);
    font-size: 10px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 1px 4px;
    color: var(--text-faint);
  }
  .content {
    flex: 1;
    overflow-y: auto;
    padding: 32px 28px 64px;
    display: flex;
    flex-direction: column;
  }

  .right {
    background: var(--surface);
    border-left: 1px solid var(--border);
    overflow-y: auto;
  }
  .pane-head,
  :global(.pane-head) {
    font-family: var(--font-mono);
    font-size: 10.5px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--text-faint);
    padding: 16px 16px 8px;
  }

  .empty {
    margin: auto;
    text-align: center;
    max-width: 340px;
    padding: 24px;
  }
  .lattice-motif {
    width: 132px;
    height: 132px;
    margin: 0 auto 28px;
    background-image:
      linear-gradient(var(--border-strong) 1px, transparent 1px),
      linear-gradient(90deg, var(--border-strong) 1px, transparent 1px);
    background-size: 22px 22px;
    -webkit-mask-image: radial-gradient(circle, #000 35%, transparent 72%);
    mask-image: radial-gradient(circle, #000 35%, transparent 72%);
    position: relative;
  }
  .lattice-motif::after {
    content: "";
    position: absolute;
    inset: 0;
    background:
      radial-gradient(circle at 50% 50%, var(--accent) 3px, transparent 4px),
      radial-gradient(circle at 27% 27%, var(--accent-line) 2.5px, transparent 3px),
      radial-gradient(circle at 73% 73%, var(--accent-line) 2.5px, transparent 3px);
  }
  .empty-title {
    font-size: 16px;
    font-weight: 600;
    color: var(--text);
    margin: 0 0 8px;
  }
  .empty-hint {
    color: var(--text-dim);
    font-size: 13px;
    line-height: 1.6;
    margin: 0;
  }
  .empty-hint em {
    font-style: normal;
    color: var(--accent);
    font-weight: 550;
  }
</style>
