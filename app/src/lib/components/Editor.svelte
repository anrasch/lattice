<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { EditorView, keymap } from "@codemirror/view";
  import { EditorState } from "@codemirror/state";
  import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
  import { markdown } from "@codemirror/lang-markdown";
  import { syntaxHighlighting, HighlightStyle } from "@codemirror/language";
  import { tags as t } from "@lezer/highlight";
  import { api, type WriteOutcome } from "$lib/api";
  import { mode } from "$lib/stores";

  let { note }: { note: string } = $props();
  let el: HTMLDivElement;
  let view: EditorView | undefined;
  let loadedHash = $state("");
  let conflict = $state<string | null>(null);

  const theme = EditorView.theme(
    {
      "&": { color: "var(--text)", backgroundColor: "transparent", height: "100%" },
      ".cm-content": {
        fontFamily: "var(--font-mono)",
        fontSize: "13.5px",
        lineHeight: "1.75",
        padding: "18px 20px",
        caretColor: "var(--accent)",
      },
      ".cm-cursor": { borderLeftColor: "var(--accent)", borderLeftWidth: "2px" },
      "&.cm-focused": { outline: "none" },
      ".cm-gutters": { display: "none" },
      ".cm-activeLine": { backgroundColor: "rgba(233,220,195,0.025)" },
      ".cm-selectionBackground, &.cm-focused .cm-selectionBackground, ::selection":
        { backgroundColor: "var(--accent-dim)" },
      ".cm-scroller": { overflow: "auto" },
    },
  );

  const highlight = syntaxHighlighting(
    HighlightStyle.define([
      { tag: t.heading, color: "var(--accent-bright)", fontWeight: "650" },
      { tag: t.strong, fontWeight: "700", color: "var(--text)" },
      { tag: t.emphasis, fontStyle: "italic" },
      { tag: [t.link, t.url], color: "var(--accent)" },
      { tag: t.monospace, color: "var(--text-dim)" },
      { tag: t.quote, color: "var(--text-dim)" },
      { tag: [t.list, t.processingInstruction], color: "var(--accent)" },
    ]),
  );

  onMount(async () => {
    const raw = await api.readRaw(note);
    loadedHash = raw.hash;
    view = new EditorView({
      parent: el,
      state: EditorState.create({
        doc: raw.content,
        extensions: [
          history(),
          keymap.of([...defaultKeymap, ...historyKeymap]),
          markdown(),
          highlight,
          theme,
          EditorView.lineWrapping,
        ],
      }),
    });
    view.focus();
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
  <div class="cm" bind:this={el}></div>

  <div class="bar">
    <span class="hint">Markdown · ⌘S to save</span>
    <div class="actions">
      <button class="ghost" onclick={() => mode.set("read")}>Cancel</button>
      <button class="primary" onclick={save}>Save</button>
    </div>
  </div>

  {#if conflict !== null}
    <div class="conflict">
      <div class="ctitle">Changed on disk since you opened it</div>
      <p>Save was blocked so your copy didn't overwrite it. On-disk version:</p>
      <pre>{conflict}</pre>
      <button class="ghost" onclick={reload}>Load on-disk version</button>
    </div>
  {/if}
</div>

<style>
  .editor-wrap {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .cm {
    flex: 1;
    min-height: 0;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    overflow: hidden;
    background: var(--surface);
  }
  .bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 10px;
  }
  .hint {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-faint);
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  button {
    font-size: 12.5px;
    font-weight: 550;
    padding: 6px 14px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: all 0.14s ease;
  }
  .ghost {
    background: none;
    border: 1px solid var(--border-strong);
    color: var(--text-dim);
  }
  .ghost:hover {
    color: var(--text);
    border-color: var(--text-faint);
  }
  .primary {
    background: var(--accent);
    border: 1px solid var(--accent);
    color: var(--on-accent);
  }
  .primary:hover {
    background: var(--accent-bright);
    border-color: var(--accent-bright);
  }
  .conflict {
    margin-top: 12px;
    padding: 12px 14px;
    background: var(--danger-dim);
    border: 1px solid var(--danger);
    border-radius: var(--radius);
    font-size: 12.5px;
  }
  .ctitle {
    font-weight: 600;
    color: var(--danger);
    margin-bottom: 4px;
  }
  .conflict p {
    margin: 0 0 8px;
    color: var(--text-dim);
  }
  pre {
    font-family: var(--font-mono);
    font-size: 11.5px;
    white-space: pre-wrap;
    max-height: 160px;
    overflow: auto;
    background: var(--bg);
    padding: 8px 10px;
    border-radius: 6px;
    margin: 0 0 10px;
  }
</style>
