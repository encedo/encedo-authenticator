<script lang="ts">
  import { app, fmtDateTime, shortPid } from "../lib/state.svelte";
  import Masthead from "../lib/Masthead.svelte";
  let { pid }: { pid?: string } = $props();
  // svelte-ignore state_referenced_locally
  let filter = $state<string | undefined>(pid);
  let q = $state("");
  const rows = $derived(
    app.archive.filter((e) => (!filter || e.pid === filter) && (!q || (e.title + " " + e.detail).toLowerCase().includes(q.toLowerCase()))),
  );
  const granted = $derived(app.archive.filter((e) => e.outcome === "granted").length);
</script>

<div class="screen">
  <Masthead />
  <div class="screen-body">
    <div class="page-head">
      <p class="eyebrow">Archive</p>
      <h1>Every answer this phone gave.</h1>
      <p>{granted} granted, {app.archive.length - granted} otherwise. Kept on the phone only, encrypted, never sent anywhere.</p>
    </div>
    <div class="field">
      <label for="q">Search</label>
      <input id="q" type="search" bind:value={q} placeholder="Key label, drive, scope" />
    </div>
    <div class="chips">
      <button aria-pressed={!filter} onclick={() => (filter = undefined)}>All modules</button>
      {#each app.modules as m (m.pid)}
        <button aria-pressed={filter === m.pid} onclick={() => (filter = m.pid)}>{m.label}</button>
      {/each}
    </div>
    <div class="card">
      <div class="card-head"><span>Answers</span><span class="v">{rows.length}</span></div>
      {#if rows.length}
        <ul class="records">
          {#each rows as e (e.id)}
            <li><div class="row">
              <span class="main"><span class="name">{e.title}</span><span class="sub">{app.module(e.pid)?.label ?? shortPid(e.pid)}{e.detail ? " · " + e.detail : ""}</span></span>
              <span class="main" style="flex:0 0 auto;align-items:flex-end">
                <span class="pill" class:sealed={e.outcome === "granted" || e.outcome === "paired"} class:exposed={e.outcome === "denied" || e.outcome === "error" || e.outcome === "rejected"}><i></i>{e.outcome}</span>
                <span class="when">{fmtDateTime(e.at)}</span>
              </span>
            </div></li>
          {/each}
        </ul>
      {:else}
        <div class="empty">Nothing matches.</div>
      {/if}
    </div>
  </div>
</div>
