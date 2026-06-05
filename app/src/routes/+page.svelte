<script lang="ts">
  import Ribbon from "$lib/components/Ribbon.svelte";
  import FileTree from "$lib/components/FileTree.svelte";
  import SearchPanel from "$lib/components/SearchPanel.svelte";
  import ListPanel from "$lib/components/ListPanel.svelte";
  import Tabs from "$lib/components/Tabs.svelte";
  import NoteView from "$lib/components/NoteView.svelte";
  import LinksPanel from "$lib/components/LinksPanel.svelte";
  import Editor from "$lib/components/Editor.svelte";
  import { currentNote, mode, leftView, leftOpen, rightOpen } from "$lib/stores";

  let editor = $state<{ save: () => void } | undefined>();

  function onKey(e: KeyboardEvent) {
    if (!(e.metaKey || e.ctrlKey)) return;
    if (e.key === "e") {
      e.preventDefault();
      if ($currentNote) mode.update((m) => (m === "read" ? "edit" : "read"));
    } else if (e.key === "s") {
      e.preventDefault();
      editor?.save();
    } else if (e.key === "\\") {
      e.preventDefault();
      leftOpen.update((v) => !v);
    }
  }

  const labels: Record<string, string> = {
    files: "Files",
    search: "Search",
    orphans: "Orphans",
    broken: "Broken links",
  };

  let cols = $derived(
    `var(--ribbon) ${$leftOpen ? "248px" : "0px"} minmax(0, 1fr) ${$rightOpen ? "280px" : "0px"}`,
  );
  let fileName = $derived($currentNote ? $currentNote.split("/").pop() : "");
  let fileDir = $derived(
    $currentNote && $currentNote.includes("/")
      ? $currentNote.slice(0, $currentNote.lastIndexOf("/") + 1)
      : "",
  );
</script>

<svelte:window onkeydown={onKey} />

<main class="shell" style="grid-template-columns: {cols}">
  <Ribbon />

  <aside class="left">
    <div class="panel-head">{labels[$leftView]}</div>
    <div class="panel-body">
      {#if $leftView === "files"}
        <FileTree />
      {:else if $leftView === "search"}
        <SearchPanel />
      {:else if $leftView === "orphans"}
        {#key $leftView}<ListPanel kind="orphans" />{/key}
      {:else}
        {#key $leftView}<ListPanel kind="broken" />{/key}
      {/if}
    </div>
  </aside>

  <section class="center">
    <Tabs />
    {#if $currentNote}
      <div class="note-head">
        <div class="crumb">
          <span class="dir">{fileDir}</span><span class="file">{fileName}</span>
        </div>
        <div class="actions">
          <button
            class="icon-btn"
            class:on={$mode === "edit"}
            title="Edit (⌘E)"
            aria-label="Edit"
            onclick={() => mode.update((m) => (m === "read" ? "edit" : "read"))}
          >
            <svg viewBox="0 0 16 16"
              ><path
                d="M10.5 2.5l3 3L6 13l-3.5.5L3 10l7.5-7.5Z"
                fill="none"
                stroke="currentColor"
                stroke-width="1.3"
                stroke-linejoin="round"
              /></svg
            >
          </button>
          <button
            class="icon-btn"
            class:on={$rightOpen}
            title="Toggle connections"
            aria-label="Toggle connections"
            onclick={() => rightOpen.update((v) => !v)}
          >
            <svg viewBox="0 0 16 16"
              ><rect x="1.5" y="2.5" width="13" height="11" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.3" /><line
                x1="10"
                y1="2.5"
                x2="10"
                y2="13.5"
                stroke="currentColor"
                stroke-width="1.3"
              /></svg
            >
          </button>
        </div>
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
        <div class="motif" aria-hidden="true"></div>
        <p>No note open</p>
        <span>Pick a file, or search the vault.</span>
      </div>
    {/if}
  </section>

  <aside class="right">
    <div class="panel-head">Connections</div>
    <div class="panel-body">
      {#if $currentNote}<LinksPanel note={$currentNote} />{/if}
    </div>
  </aside>
</main>

<style>
  .shell {
    display: grid;
    height: 100vh;
    overflow: hidden;
    transition: grid-template-columns 0.16s ease;
  }
  .left,
  .right {
    background: var(--surface);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
  }
  .left {
    border-right: 1px solid var(--border);
  }
  .right {
    border-left: 1px solid var(--border);
  }
  .panel-head {
    font-family: var(--font-mono);
    font-size: 10.5px;
    letter-spacing: 0.15em;
    text-transform: uppercase;
    color: var(--text-faint);
    padding: 12px 14px 8px;
    flex-shrink: 0;
    white-space: nowrap;
  }
  .panel-body {
    flex: 1;
    overflow-y: auto;
  }

  .center {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--surface);
    min-width: 0;
  }
  .note-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 40px;
    padding: 0 16px 0 22px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .crumb {
    font-family: var(--font-mono);
    font-size: 11.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .crumb .dir {
    color: var(--text-faint);
  }
  .crumb .file {
    color: var(--text-dim);
  }
  .actions {
    display: flex;
    gap: 2px;
  }
  .icon-btn {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    background: none;
    border: 0;
    border-radius: var(--radius-sm);
    color: var(--text-faint);
    cursor: pointer;
    transition: all 0.12s ease;
  }
  .icon-btn svg {
    width: 15px;
    height: 15px;
  }
  .icon-btn:hover {
    color: var(--text-dim);
    background: var(--surface-2);
  }
  .icon-btn.on {
    color: var(--accent);
    background: var(--accent-dim);
  }
  .content {
    flex: 1;
    overflow-y: auto;
    padding: 36px 40px 80px;
    display: flex;
    flex-direction: column;
  }

  .empty {
    margin: auto;
    text-align: center;
    color: var(--text-dim);
  }
  .empty p {
    margin: 0 0 4px;
    color: var(--text);
    font-size: 14px;
  }
  .empty span {
    font-size: 12.5px;
    color: var(--text-faint);
  }
  .motif {
    width: 92px;
    height: 92px;
    margin: 0 auto 22px;
    background-image:
      linear-gradient(var(--border-strong) 1px, transparent 1px),
      linear-gradient(90deg, var(--border-strong) 1px, transparent 1px);
    background-size: 18px 18px;
    -webkit-mask-image: radial-gradient(circle, #000 30%, transparent 70%);
    mask-image: radial-gradient(circle, #000 30%, transparent 70%);
  }
</style>
