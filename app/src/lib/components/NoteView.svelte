<script lang="ts">
  import { api } from "$lib/api";
  import { noteRevision } from "$lib/stores";

  let { note }: { note: string } = $props();
  let html = $state("");
  let gone = $state(false);

  // Re-renders on note switch and when the open note changed on disk
  // (noteRevision bump). Unrelated external changes do not retrigger this.
  $effect(() => {
    const n = note;
    $noteRevision;
    api
      .render(n)
      .then((h) => {
        html = h;
        gone = false;
      })
      .catch(() => {
        html = "";
        gone = true;
      });
  });
</script>

{#if gone}
  <div class="gone">This note is no longer on disk.</div>
{:else}
  <!-- html is sanitized server-side by ammonia -->
  <article class="prose">{@html html}</article>
{/if}

<style>
  .gone {
    color: var(--text-faint);
    font-size: 13px;
    padding: 8px 2px;
  }
</style>
