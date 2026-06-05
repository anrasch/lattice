<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { theme } from "$lib/stores";
  let { children } = $props();

  onMount(() => {
    const saved = localStorage.getItem("lattice-theme");
    if (saved === "light" || saved === "dark") theme.set(saved);
  });

  $effect(() => {
    document.documentElement.dataset.theme = $theme;
    try {
      localStorage.setItem("lattice-theme", $theme);
    } catch (e) {
      // ignore (private mode, etc.)
    }
  });
</script>

{@render children()}
