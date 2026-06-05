<script lang="ts">
  import { api, type Edge } from "$lib/api";
  import { currentNote } from "$lib/stores";

  let { note }: { note: string } = $props();
  let backlinks = $state<Edge[]>([]);
  let outbound = $state<Edge[]>([]);

  $effect(() => {
    const n = note;
    api.backlinks(n).then((b) => (backlinks = b));
    api.links(n).then((l) => (outbound = l));
  });

  let outResolved = $derived(outbound.filter((e) => e.dst));
</script>

<div class="panel">
  <h4>Backlinks ({backlinks.length})</h4>
  {#each backlinks as e}
    <button onclick={() => currentNote.set(e.src)}>{e.src}</button>
  {/each}
  {#if backlinks.length === 0}<p class="empty">none</p>{/if}

  <h4>Links out ({outResolved.length})</h4>
  {#each outResolved as e}
    <button onclick={() => currentNote.set(e.dst!)}>{e.dst}</button>
  {/each}
  {#if outResolved.length === 0}<p class="empty">none</p>{/if}
</div>

<style>
  .panel {
    font-size: 12px;
  }
  h4 {
    margin: 14px 0 4px;
  }
  .empty {
    opacity: 0.4;
    margin: 0;
  }
  button {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: 0;
    padding: 2px 4px;
    cursor: pointer;
    color: #4a8fe0;
    font: inherit;
  }
  button:hover {
    text-decoration: underline;
  }
</style>
