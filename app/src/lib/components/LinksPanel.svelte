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
  let outBroken = $derived(outbound.filter((e) => !e.dst));
</script>

<div class="links">
  <section>
    <div class="label">
      Backlinks <span class="n">{backlinks.length}</span>
    </div>
    {#each backlinks as e}
      <button class="ref" onclick={() => currentNote.set(e.src)}>
        <span class="dot in"></span>{e.src.split("/").pop()}
      </button>
    {/each}
    {#if backlinks.length === 0}<p class="empty">No notes link here</p>{/if}
  </section>

  <section>
    <div class="label">
      Links out <span class="n">{outResolved.length}</span>
    </div>
    {#each outResolved as e}
      <button class="ref" onclick={() => currentNote.set(e.dst!)}>
        <span class="dot out"></span>{e.dst!.split("/").pop()}
      </button>
    {/each}
    {#if outResolved.length === 0}<p class="empty">No outgoing links</p>{/if}
  </section>

  {#if outBroken.length > 0}
    <section>
      <div class="label">
        Unresolved <span class="n">{outBroken.length}</span>
      </div>
      {#each outBroken as e}
        <div class="ref broken"><span class="dot bad"></span>{e.raw_target}</div>
      {/each}
    </section>
  {/if}
</div>

<style>
  .links {
    padding: 0 12px 16px;
  }
  section {
    margin-bottom: 18px;
  }
  .label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--text-dim);
    margin: 0 4px 6px;
  }
  .n {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-faint);
    background: var(--surface-2);
    border-radius: 10px;
    padding: 0 6px;
    line-height: 16px;
  }
  .ref {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    text-align: left;
    background: none;
    border: 0;
    padding: 4px 6px;
    border-radius: 5px;
    cursor: pointer;
    color: var(--text-dim);
    font-family: var(--font-mono);
    font-size: 11.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  button.ref:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .ref.broken {
    color: var(--text-faint);
    cursor: default;
  }
  .dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .dot.in {
    background: var(--accent);
  }
  .dot.out {
    background: var(--text-dim);
  }
  .dot.bad {
    background: var(--danger);
  }
  .empty {
    color: var(--text-faint);
    font-size: 11.5px;
    margin: 2px 4px;
  }
</style>
