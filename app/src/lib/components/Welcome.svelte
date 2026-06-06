<script lang="ts">
  let {
    onpick,
    onopen,
    recents = [],
  }: { onpick: () => void; onopen: (p: string) => void; recents: string[] } = $props();

  function basename(p: string) {
    return p.split("/").filter(Boolean).pop() ?? p;
  }
</script>

<div class="welcome">
  <div class="card">
    <div class="mark" aria-hidden="true"></div>
    <h1>Lattice</h1>
    <p class="sub">Open a folder of Markdown to browse it as a graph.</p>
    <button class="primary" onclick={onpick}>Open a vault…</button>

    {#if recents.length}
      <div class="recents">
        <div class="rh">Recent</div>
        {#each recents as r}
          <button class="recent" onclick={() => onopen(r)} title={r}>
            <span class="rname">{basename(r)}</span>
            <span class="rpath">{r}</span>
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .welcome {
    height: 100vh;
    display: grid;
    place-items: center;
    background: var(--bg);
  }
  .card {
    width: 380px;
    max-width: 86vw;
    text-align: center;
  }
  .mark {
    width: 56px;
    height: 56px;
    margin: 0 auto 18px;
    border-radius: 14px;
    background-image:
      linear-gradient(var(--border-strong) 1px, transparent 1px),
      linear-gradient(90deg, var(--border-strong) 1px, transparent 1px);
    background-size: 14px 14px;
    -webkit-mask-image: radial-gradient(circle, #000 35%, transparent 72%);
    mask-image: radial-gradient(circle, #000 35%, transparent 72%);
    position: relative;
  }
  .mark::after {
    content: "";
    position: absolute;
    inset: 0;
    background: radial-gradient(circle at 72% 28%, var(--accent) 3px, transparent 4px);
  }
  h1 {
    margin: 0 0 6px;
    font-size: 22px;
    font-weight: 650;
    letter-spacing: -0.01em;
  }
  .sub {
    margin: 0 0 22px;
    color: var(--text-dim);
    font-size: 13.5px;
  }
  .primary {
    background: var(--accent);
    color: var(--on-accent);
    border: 0;
    border-radius: var(--radius);
    padding: 9px 18px;
    font-size: 13.5px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.14s ease;
  }
  .primary:hover {
    background: var(--accent-bright);
  }
  .recents {
    margin-top: 28px;
    text-align: left;
  }
  .rh {
    font-family: var(--font-mono);
    font-size: 10.5px;
    letter-spacing: 0.15em;
    text-transform: uppercase;
    color: var(--text-faint);
    margin: 0 4px 6px;
  }
  .recent {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    padding: 7px 10px;
    cursor: pointer;
    color: inherit;
  }
  .recent:hover {
    background: var(--surface-2);
    border-color: var(--border);
  }
  .rname {
    display: block;
    font-size: 13px;
    color: var(--text);
  }
  .rpath {
    display: block;
    font-family: var(--font-mono);
    font-size: 10.5px;
    color: var(--text-faint);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
