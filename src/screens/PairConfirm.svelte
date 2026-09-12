<script lang="ts">
  import { app } from "../lib/state.svelte";
  import type { PairingPreview } from "../lib/api";
  import Masthead from "../lib/Masthead.svelte";
  import Help from "../lib/Help.svelte";
  let { preview, raw }: { preview: PairingPreview; raw: string } = $props();
  // The whole host, not its first label: "my.ence.do" reads as a module,
  // "my" does not. The field is editable anyway.
  // svelte-ignore state_referenced_locally
  let name = $state(preview.hostname || "New module");
</script>

<div class="screen">
  <Masthead back={() => app.go({ name: "pair" })} backLabel="Scan" />
  <div class="screen-body">
    <div class="page-head">
      <p class="eyebrow" class:exposed={preview.already_paired}>{preview.already_paired ? "Pairing · already paired" : "Pairing · link verified"}</p>
      {#if preview.already_paired}
        <h1>This phone already answers for {preview.hostname}.
          <Help
            label="Why this is blocked"
            parts={[{ term: "Already paired", text: "The module is in the list. Pairing again would put a second key in its keychain; unpair first if that is what you want." }]}
          />
        </h1>
      {:else}
        <h1>{preview.hostname} wants this phone to answer for it.
          <Help
            label="What pairing does"
            parts={[
              { term: "On the module", text: "Pairing puts a new key in the module's keychain. The Manager waits for your reply." },
              { term: "On the broker", text: "This phone is subscribed, so the module can wake it for every request." },
              { term: "The link", text: "It was fetched and its request parsed; the reply is signed with a key made just now for this module." },
            ]}
          />
        </h1>
      {/if}
    </div>
    <div class="card asking">
      <div class="card-head"><span>Request</span><span class="v">auth:ext:pair</span></div>
      <dl class="status-grid">
        <div><dt>Host</dt><dd class="mono">{preview.hostname || "—"}</dd></div>
        <div><dt>Account</dt><dd>{preview.email || preview.user || "—"}</dd></div>
        <div><dt>Asked from</dt><dd>{preview.issuer ? `${preview.issuer.city}, ${preview.issuer.country}` : "unknown place"}</dd></div>
        <div><dt>Address</dt><dd class="mono">{preview.issuer?.ip ?? "—"}</dd></div>
      </dl>
    </div>
    <div class="field">
      <span class="label">Pairing link</span>
      <div class="blob">{preview.link}</div>
    </div>
    <div class="field">
      <label for="label">Name on this phone</label>
      <input id="label" bind:value={name} placeholder="Module label" />
      <span class="hint">Only this phone sees the name. The module knows the phone by its key.</span>
    </div>
  </div>
  <div class="screen-actions row">
    <button class="button exposed" disabled={app.busy} onclick={() => app.refusePairing(preview)}>Refuse</button>
    <button class="button" disabled={app.busy || preview.already_paired} onclick={() => app.completePairing(name.trim() || preview.hostname, preview)}>{app.busy ? "Pairing…" : "Pair"}</button>
  </div>
</div>
