<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { EditorView, keymap } from "@codemirror/view";
  import { EditorState } from "@codemirror/state";
  import { defaultKeymap } from "@codemirror/commands";
  import { markdown } from "@codemirror/lang-markdown";
  import { api, type WriteOutcome } from "$lib/api";
  import { mode } from "$lib/stores";

  let { note }: { note: string } = $props();
  let el: HTMLDivElement;
  let view: EditorView | undefined;
  let loadedHash = $state("");
  let conflict = $state<string | null>(null);

  onMount(async () => {
    const raw = await api.readRaw(note);
    loadedHash = raw.hash;
    view = new EditorView({
      parent: el,
      state: EditorState.create({
        doc: raw.content,
        extensions: [keymap.of(defaultKeymap), markdown()],
      }),
    });
  });
  onDestroy(() => view?.destroy());

  // Exposed so the shell can trigger save via Cmd-S.
  export async function save() {
    if (!view) return;
    const content = view.state.doc.toString();
    const out: WriteOutcome = await api.save(note, content, loadedHash);
    if (out.outcome === "written") {
      loadedHash = out.hash;
      mode.set("read");
    } else {
      conflict = out.on_disk;
    }
  }

  async function reload() {
    const raw = await api.readRaw(note);
    loadedHash = raw.hash;
    conflict = null;
  }
</script>

<div class="editor-wrap">
  <div class="toolbar">
    <button onclick={save}>Save (⌘S)</button>
    <button onclick={() => mode.set("read")}>Cancel</button>
  </div>
  <div class="cm" bind:this={el}></div>
  {#if conflict !== null}
    <div class="conflict">
      <strong>File changed on disk since you opened it.</strong>
      <p>Save was blocked to avoid overwriting. On-disk version:</p>
      <pre>{conflict}</pre>
      <button onclick={reload}>Reload on-disk version</button>
    </div>
  {/if}
</div>

<style>
  .editor-wrap {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .toolbar {
    display: flex;
    gap: 8px;
    margin-bottom: 8px;
  }
  .cm {
    flex: 1;
    border: 1px solid rgba(127, 127, 127, 0.3);
    border-radius: 6px;
    overflow: auto;
  }
  .cm :global(.cm-editor) {
    height: 100%;
  }
  .conflict {
    margin-top: 8px;
    padding: 8px 12px;
    background: rgba(252, 129, 129, 0.12);
    border: 1px solid #fc8181;
    border-radius: 6px;
    font-size: 12px;
  }
  pre {
    white-space: pre-wrap;
    max-height: 160px;
    overflow: auto;
  }
</style>
