<script lang="ts">
  import { api } from "$lib/api";

  let { note }: { note: string } = $props();
  let html = $state("");

  $effect(() => {
    const n = note;
    api.render(n).then((h) => (html = h));
  });
</script>

<!-- html is sanitized server-side by ammonia -->
<article class="note">{@html html}</article>

<style>
  .note {
    max-width: 760px;
    line-height: 1.6;
  }
  .note :global(pre) {
    background: rgba(127, 127, 127, 0.12);
    padding: 12px;
    border-radius: 6px;
    overflow-x: auto;
  }
  .note :global(code) {
    font-family: ui-monospace, monospace;
  }
</style>
