<script lang="ts">
  import { app } from "../lib/state.svelte";
  import Masthead from "../lib/Masthead.svelte";
  let { label, host, raw }: { label: string; host: string; raw?: string } = $props();
  // svelte-ignore state_referenced_locally
  let name = $state(label);
</script>

<div class="screen">
  <Masthead back={() => app.go({ name: "pair" })} backLabel="Scan" />
  <div class="screen-body">
    <div class="page-head">
      <p class="eyebrow">Pairing · hash verified</p>
      <h1>{host} wants this phone to answer for it.</h1>
      <p>Pairing puts a new key in the module's keychain and subscribes this phone at the broker. The Manager waits for your reply.</p>
    </div>
    <div class="card asking">
      <div class="card-head"><span>Request</span><span class="v">auth:ext:pair</span></div>
      <dl class="status-grid">
        <div><dt>Host</dt><dd class="mono">{host}</dd></div>
        <div><dt>Account</dt><dd>chris@encedo.com</dd></div>
        <div><dt>Asked from</dt><dd>Manager · 192.168.7.20</dd></div>
        <div><dt>This phone</dt><dd>Pixel 8</dd></div>
      </dl>
    </div>
    {#if raw}
      <div class="field">
        <span class="label">What the code says</span>
        <div class="blob">{raw}</div>
        <span class="hint">Phase 2 fetches this link, checks the hash, and only then asks you.</span>
      </div>
    {/if}
    <div class="field">
      <label for="label">Name on this phone</label>
      <input id="label" bind:value={name} placeholder="Module label" />
      <span class="hint">Only this phone sees the name. The module knows the phone by its key.</span>
    </div>
  </div>
  <div class="screen-actions row">
    <button class="button exposed" onclick={() => app.go({ name: "result", outcome: "denied", title: "Pair this phone", detail: host })}>Refuse</button>
    <button class="button" onclick={() => app.completePairing(name.trim() || label, host)}>Pair</button>
  </div>
</div>
