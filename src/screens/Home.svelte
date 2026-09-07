<script lang="ts">
  import { app, count, fmtDateTime, shortPid } from "../lib/state.svelte";
  import Masthead from "../lib/Masthead.svelte";
  const recent = $derived(app.archive.slice(0, 4));
  const n = $derived(app.pending.length);
  const heading = $derived(
    !app.online ? "This phone is off the network." : n ? `${count(n, "request")} ${n === 1 ? "is" : "are"} waiting for you.` : "Nothing is waiting for you.",
  );
</script>

<div class="screen">
  <Masthead />
  <div class="screen-body">
    <div class="page-head">
      <p class="eyebrow" class:exposed={!app.online}>Now</p>
      <h1>{heading}</h1>
      {#if app.online}
        <p>Push is registered. {count(app.modules.length, "module")} can ask you.</p>
      {:else}
        <p>A request cannot be answered until the network is back. The module keeps waiting, then gives up on its own.</p>
      {/if}
    </div>

    {#if n}
      <div class="card asking lifted">
        <div class="card-head"><span>Needs you</span><span class="v">{n}</span></div>
        <ul class="records">
          {#each app.pending as r (r.id)}
            <li><button class="rowbtn" onclick={() => app.go({ name: "request", id: r.id })}>
              <span class="main"><span class="name">{r.host} wants to {r.phrase}</span><span class="sub">{app.module(r.pid)?.label ?? shortPid(r.pid)} · {r.scope.split("#")[0].slice(0, 28)}</span></span>
              <svg class="chev" viewBox="0 0 16 16"><path d="m6 3 5 5-5 5" /></svg>
            </button></li>
          {/each}
        </ul>
      </div>
    {/if}

    <div class="card">
      <div class="card-head"><span>Recent answers</span><span class="v">{app.archive.length}</span></div>
      {#if recent.length}
        <ul class="records">
          {#each recent as e (e.id)}
            <li><div class="row">
              <span class="main"><span class="name">{e.title}</span><span class="sub">{app.module(e.pid)?.label ?? shortPid(e.pid)}{e.detail ? " · " + e.detail : ""}</span></span>
              <span class="pill" class:sealed={e.outcome === "granted"} class:exposed={e.outcome === "denied" || e.outcome === "error"}><i></i>{e.outcome}</span>
            </div></li>
          {/each}
        </ul>
      {:else}
        <div class="empty">This phone has not answered anything yet.</div>
      {/if}
      <div class="card-foot"><span>{recent.length} of {app.archive.length}</span><button class="button quiet small" onclick={() => app.go({ name: "archive" })}>Full archive</button></div>
    </div>

    <div class="card">
      <div class="card-head"><span>Modules</span><span class="v">{app.modules.length}</span></div>
      <ul class="records">
        {#each app.modules as m (m.pid)}
          <li><button class="rowbtn" onclick={() => app.go({ name: "module", pid: m.pid })}>
            <span class="main"><span class="name">{m.label}</span><span class="sub">{m.host} · pid {shortPid(m.pid)}</span></span>
            <span class="when">{m.lastUsed ? fmtDateTime(m.lastUsed) : "never used"}</span>
            <svg class="chev" viewBox="0 0 16 16"><path d="m6 3 5 5-5 5" /></svg>
          </button></li>
        {/each}
      </ul>
    </div>
  </div>
</div>
