<script lang="ts">
  import { app, count, fmtDate, shortPid } from "../lib/state.svelte";
  import Masthead from "../lib/Masthead.svelte";
</script>

<div class="screen">
  <Masthead />
  <div class="screen-body">
    <div class="page-head">
      <p class="eyebrow">Paired modules</p>
      <h1>{app.modules.length ? `${count(app.modules.length, "module")} can ask you.` : "No module can ask you yet."}</h1>
      <p>A paired module sends its requests to this phone. Unpairing removes the phone from the module's keychain as well.</p>
    </div>
    <div class="card">
      <div class="card-head"><span>Paired</span><span class="v">{count(app.modules.length, "module").toLowerCase()}</span></div>
      {#if app.modules.length}
        <ul class="records">
          {#each app.modules as m (m.pid)}
            <li><button class="rowbtn" onclick={() => app.go({ name: "module", pid: m.pid })}>
              <span class="main"><span class="name">{m.label}</span><span class="sub">{m.host} · pid {shortPid(m.pid)}</span></span>
              <span class="when">paired {fmtDate(m.pairedAt)}</span>
              <svg class="chev" viewBox="0 0 16 16"><path d="m6 3 5 5-5 5" /></svg>
            </button></li>
          {/each}
        </ul>
      {:else}
        <div class="empty">Open the Manager, choose Paired phones, and scan the code it shows.</div>
      {/if}
    </div>
  </div>
  <div class="screen-actions">
    <button class="button" onclick={() => app.go({ name: "pair" })}>Pair a module</button>
  </div>
</div>
