<script lang="ts">
  import { api } from "$lib/api";

  let { note }: { note: string } = $props();
  let html = $state("");

  $effect(() => {
    const n = note;
    html = "";
    api.render(n).then((h) => (html = h));
  });
</script>

<!-- html is sanitized server-side by ammonia -->
<article class="prose">{@html html}</article>
