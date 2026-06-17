<script lang="ts">
  import Ribbon from "$lib/components/Ribbon.svelte";
  import FileTree from "$lib/components/FileTree.svelte";
  import SearchPanel from "$lib/components/SearchPanel.svelte";
  import ListPanel from "$lib/components/ListPanel.svelte";
  import Tabs from "$lib/components/Tabs.svelte";
  import NoteView from "$lib/components/NoteView.svelte";
  import LinksPanel from "$lib/components/LinksPanel.svelte";
  import Editor from "$lib/components/Editor.svelte";
  import Welcome from "$lib/components/Welcome.svelte";
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { api, type ChangedEntry } from "$lib/api";
  import {
    vault,
    currentNote,
    openTabs,
    mode,
    leftView,
    leftOpen,
    rightOpen,
    treeEntries,
    linksRevision,
    noteRevision,
    externalUpdate,
    applyChanges,
    pinNote,
    previewTab,
    previewNote,
  } from "$lib/stores";

  let editor = $state<{ save: () => void } | undefined>();
  let recentList = $state<string[]>([]);

  onMount(() => {
    let unChanged: (() => void) | undefined;
    let unOpen: (() => void) | undefined;
    (async () => {
      vault.set(await api.currentVault());
      recentList = await api.recents();
      unChanged = await listen<ChangedEntry[]>("vault://changed", (e) => applyChanges(e.payload));
      unOpen = await listen<string>("vault://open", (e) => previewNote(e.payload));
    })();
    return () => {
      unChanged?.();
      unOpen?.();
    };
  });

  function resetWorkspace() {
    currentNote.set(null);
    openTabs.set([]);
    previewTab.set(null);
    leftView.set("files");
  }

  /** Toggle read/edit; entering edit pins the preview tab so it isn't lost. */
  function toggleEdit() {
    if (!$currentNote) return;
    if ($mode === "read") pinNote($currentNote);
    mode.update((m) => (m === "read" ? "edit" : "read"));
  }

  async function setVault(path: string) {
    const info = await api.openVault(path);
    resetWorkspace();
    vault.set(info);
    recentList = await api.recents();
  }

  async function pickVault() {
    const path = await api.pickVault();
    if (path) await setVault(path);
  }

  async function reloadVault() {
    await api.resync();
    treeEntries.set(await api.tree());
    linksRevision.update((n) => n + 1);
    noteRevision.update((n) => n + 1);
  }

  let menuOpen = $state(false);

  let showUpdated = $state(false);
  $effect(() => {
    if ($externalUpdate === 0) return;
    showUpdated = true;
    const t = setTimeout(() => (showUpdated = false), 2000);
    return () => clearTimeout(t);
  });

  let otherRecents = $derived(recentList.filter((p) => p !== $vault?.path));

  async function switchTo(path: string) {
    menuOpen = false;
    if (path !== $vault?.path) await setVault(path);
  }
  async function openOther() {
    menuOpen = false;
    await pickVault();
  }
  function leaf(p: string) {
    return p.split("/").filter(Boolean).pop() ?? p;
  }

  function onKey(e: KeyboardEvent) {
    if (!(e.metaKey || e.ctrlKey)) return;
    if (e.key === "e") {
      e.preventDefault();
      toggleEdit();
    } else if (e.key === "s") {
      e.preventDefault();
      editor?.save();
    } else if (e.key === "\\") {
      e.preventDefault();
      leftOpen.update((v) => !v);
    } else if (e.key === "r") {
      e.preventDefault();
      reloadVault();
    }
  }

  const labels: Record<string, string> = {
    files: "Files",
    search: "Search",
    orphans: "Orphans",
    broken: "Broken links",
  };

  let cols = $derived(
    `var(--ribbon) ${$leftOpen ? "248px" : "0px"} minmax(0, 1fr) ${$rightOpen ? "280px" : "28px"}`,
  );
  let fileName = $derived($currentNote ? $currentNote.split("/").pop() : "");
  let fileDir = $derived(
    $currentNote && $currentNote.includes("/")
      ? $currentNote.slice(0, $currentNote.lastIndexOf("/") + 1)
      : "",
  );
</script>

<svelte:window onkeydown={onKey} />

{#if $vault}
  {#key $vault.path}
    <main class="shell" style="grid-template-columns: {cols}">
      <Ribbon />

      <aside class="left">
        <div class="vault-bar">
          <button
            class="vault-switch"
            class:active={menuOpen}
            onclick={() => (menuOpen = !menuOpen)}
            title={$vault.path}
          >
            <span class="vname">{$vault.name}</span>
            <svg class="chev" class:open={menuOpen} viewBox="0 0 16 16" aria-hidden="true"
              ><path d="M4 6.5 8 10l4-3.5" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" /></svg
            >
          </button>
          {#if showUpdated}
            <span class="updated-pill">updated</span>
          {/if}
          <button class="reload-btn" title="Reload vault (⌘R)" aria-label="Reload vault" onclick={reloadVault}>
            <svg viewBox="0 0 16 16" aria-hidden="true"
              ><path d="M13 8a5 5 0 1 1-1.46-3.54M13 3v2.5h-2.5" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" /></svg
            >
          </button>
          {#if menuOpen}
            <button class="menu-backdrop" aria-label="Close menu" onclick={() => (menuOpen = false)}
            ></button>
            <div class="vault-menu">
              {#if otherRecents.length}
                <div class="mh">Switch to</div>
                {#each otherRecents as p}
                  <button class="mi" onclick={() => switchTo(p)} title={p}>
                    <span class="mname">{leaf(p)}</span>
                    <span class="mpath">{p}</span>
                  </button>
                {/each}
                <div class="mdiv"></div>
              {/if}
              <button class="mi open" onclick={openOther}>Open a folder…</button>
            </div>
          {/if}
        </div>
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
            onclick={toggleEdit}
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

  <aside class="right" class:collapsed={!$rightOpen}>
    {#if $rightOpen}
      <div class="panel-head conn-head">
        <span>Connections</span>
        <button
          class="collapse-btn"
          title="Collapse connections"
          aria-label="Collapse connections"
          onclick={() => rightOpen.set(false)}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true"
            ><path d="M6 4l4 4-4 4" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" /></svg
          >
        </button>
      </div>
      <div class="panel-body">
        {#if $currentNote}<LinksPanel note={$currentNote} />{/if}
      </div>
    {:else}
      <button
        class="conn-rail"
        title="Show connections"
        aria-label="Show connections"
        onclick={() => rightOpen.set(true)}
      >
        <svg viewBox="0 0 16 16" aria-hidden="true"
          ><path d="M10 4L6 8l4 4" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" /></svg
        >
        <span class="rail-label">Connections</span>
      </button>
    {/if}
  </aside>
</main>
  {/key}
{:else}
  <Welcome onpick={pickVault} onopen={setVault} recents={recentList} />
{/if}

<style>
  .vault-bar {
    position: relative;
    display: flex;
    align-items: stretch;
    border-bottom: 1px solid var(--border);
  }
  .vault-switch {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    flex: 1;
    min-width: 0;
    background: none;
    border: 0;
    color: var(--text);
    cursor: pointer;
    padding: 10px 14px;
    font: inherit;
    font-size: 12.5px;
    font-weight: 550;
  }
  .reload-btn {
    display: grid;
    place-items: center;
    width: 34px;
    flex-shrink: 0;
    background: none;
    border: 0;
    color: var(--text-faint);
    cursor: pointer;
  }
  .reload-btn svg {
    width: 13px;
    height: 13px;
  }
  .reload-btn:hover {
    color: var(--text-dim);
    background: var(--surface-2);
  }
  .updated-pill {
    position: absolute;
    right: 40px;
    top: 50%;
    transform: translateY(-50%);
    font-family: var(--font-mono);
    font-size: 9.5px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--accent);
    background: var(--accent-dim);
    border-radius: 10px;
    padding: 1px 7px;
    pointer-events: none;
    animation: pill-in 0.18s ease;
  }
  @keyframes pill-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
  .vault-switch:hover,
  .vault-switch.active {
    background: var(--surface-2);
  }
  .vname {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .chev {
    width: 14px;
    height: 14px;
    color: var(--text-faint);
    flex-shrink: 0;
    transition: transform 0.14s ease;
  }
  .chev.open {
    transform: rotate(180deg);
  }
  .menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
    background: none;
    border: 0;
    cursor: default;
  }
  .vault-menu {
    position: absolute;
    z-index: 41;
    top: calc(100% - 1px);
    left: 8px;
    right: 8px;
    background: var(--surface-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.35);
    padding: 5px;
    max-height: 60vh;
    overflow-y: auto;
  }
  .mh {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--text-faint);
    padding: 5px 8px 3px;
  }
  .mi {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: 0;
    border-radius: var(--radius-sm);
    padding: 6px 8px;
    cursor: pointer;
    color: var(--text);
    font: inherit;
  }
  .mi:hover {
    background: var(--surface-3);
  }
  .mi.open {
    font-size: 12.5px;
    color: var(--accent);
  }
  .mname {
    display: block;
    font-size: 12.5px;
  }
  .mpath {
    display: block;
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .mdiv {
    height: 1px;
    background: var(--border);
    margin: 5px 4px;
  }
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
  .conn-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .collapse-btn {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    margin: -2px -4px -2px 0;
    background: none;
    border: 0;
    border-radius: var(--radius-sm);
    color: var(--text-faint);
    cursor: pointer;
  }
  .collapse-btn svg {
    width: 13px;
    height: 13px;
  }
  .collapse-btn:hover {
    color: var(--text-dim);
    background: var(--surface-2);
  }
  .conn-rail {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 100%;
    padding: 12px 0;
    background: none;
    border: 0;
    color: var(--text-faint);
    cursor: pointer;
    transition: color 0.12s ease, background 0.12s ease;
  }
  .conn-rail:hover {
    color: var(--text-dim);
    background: var(--surface-2);
  }
  .conn-rail svg {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
  }
  .rail-label {
    writing-mode: vertical-rl;
    transform: rotate(180deg);
    font-family: var(--font-mono);
    font-size: 10.5px;
    letter-spacing: 0.15em;
    text-transform: uppercase;
    white-space: nowrap;
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
