<script lang="ts">
  import { app, type Tab } from "./state.svelte";
  const tabs: { id: Tab; label: string; d: string }[] = [
    { id: "home", label: "Now", d: "M4 4h12v12H4z M10 8v4" },
    { id: "modules", label: "Modules", d: "M3 6h14v8H3z M6 14v2h8v-2" },
    { id: "history", label: "History", d: "M4 5h12 M4 10h12 M4 15h8" },
    { id: "settings", label: "Settings", d: "M4 6h12 M4 14h12 M8 4v4 M13 12v4" },
  ];
</script>

<nav class="nav" aria-label="Sections">
  {#each tabs as t}
    <button aria-current={app.tab === t.id ? "page" : undefined} onclick={() => app.goTab(t.id)}>
      <svg viewBox="0 0 20 20"><path d={t.d} /></svg>
      {t.label}
      {#if t.id === "home" && app.pending.length}
        <span class="count">{app.pending.length}</span>
      {/if}
    </button>
  {/each}
</nav>
