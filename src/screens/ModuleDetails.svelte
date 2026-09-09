<script lang="ts">
  import { app, fmtDate, fmtDateTime } from "../lib/state.svelte";
  import Masthead from "../lib/Masthead.svelte";
  let { pid }: { pid: string } = $props();
  const m = $derived(app.module(pid));
  const history = $derived(app.archive.filter((e) => e.pid === pid));
  let confirm = $state(false);
  let objection = $state<string | null>(null);
  async function unpair() { objection = await app.unpair(pid); }
</script>

<div class="screen">
  <Masthead back={() => app.go({ name: "modules" })} backLabel="Modules" />
  {#if m}
    <div class="screen-body">
      <div class="page-head">
        <p class="eyebrow">Module</p>
        <h1>{m.label} can ask you.</h1>
        <p>{m.last_used ? `Last answered ${fmtDateTime(m.last_used)}.` : "This phone has not answered for it yet."}</p>
      </div>
      <div class="card">
        <dl class="status-grid">
          <div><dt>Host</dt><dd class="mono">{m.host}</dd></div>
          <div><dt>Account</dt><dd>{m.email || m.user || "—"}</dd></div>
          <div><dt>Paired</dt><dd>{fmtDate(m.paired_at)}</dd></div>
          <div><dt>Answers</dt><dd>{history.length}</dd></div>
          <div class="wide"><dt>pid</dt><dd class="mono">{m.pid}</dd></div>
        </dl>
        <div class="card-foot"><span>archive</span><button class="button quiet small" onclick={() => app.go({ name: "archive", pid })}>Show {history.length} answers</button></div>
      </div>
      <dl>
        <div class="item"><dt>What unpairing costs</dt><dd>The module forgets this phone and removes it from its keychain. Requests stop at once. Pairing again means a new QR code from the Manager.</dd></div>
      </dl>
    </div>
    <div class="screen-actions" class:row={confirm}>
      {#if objection}
        <button class="button plain" onclick={() => { objection = null; confirm = false; }}>Keep</button>
        <button class="button exposed" disabled={app.busy} onclick={() => app.forget(pid)}>Remove from this phone anyway</button>
      {:else if confirm}
        <button class="button plain" onclick={() => (confirm = false)}>Keep</button>
        <button class="button exposed" disabled={app.busy} onclick={unpair}>{app.busy ? "Unpairing…" : "Unpair now"}</button>
      {:else}
        <button class="button exposed" onclick={() => (confirm = true)}>Unpair this module</button>
      {/if}
    </div>
  {:else}
    <div class="screen-body"><div class="page-head"><p class="eyebrow exposed">Module</p><h1>This module is no longer paired.</h1></div></div>
  {/if}
</div>
