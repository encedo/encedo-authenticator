<script lang="ts">
  import { app } from "../lib/state.svelte";
  import type { Outcome } from "../lib/api";
  let { outcome, title, detail }: { outcome: Outcome; title: string; detail?: string } = $props();

  const copy: Record<Outcome, { eyebrow: string; head: string; body: string; tone: "sealed" | "exposed" | ""; icon: string }> = {
    granted: { eyebrow: "Answered", head: "Access granted.", body: "The module has your signed answer and is carrying on.", tone: "sealed", icon: "M5 10.5 8.5 14 15 6" },
    denied: { eyebrow: "Answered", head: "Access denied.", body: "The module has your refusal. Nothing on it changed.", tone: "exposed", icon: "M6 6l8 8M14 6l-8 8" },
    expired: { eyebrow: "Too late", head: "The request expired.", body: "The module stopped waiting before you answered. Ask for the operation again.", tone: "", icon: "M10 5v5l3 2M10 17a7 7 0 1 0 0-14 7 7 0 0 0 0 14z" },
    cancelled: { eyebrow: "Withdrawn", head: "The module withdrew the request.", body: "The operation was cancelled on the module's side. There is nothing to answer.", tone: "", icon: "M6 6l8 8M14 6l-8 8" },
    error: { eyebrow: "Not delivered", head: "The answer did not reach the module.", body: "The broker could not be reached. The request is still open until it expires; try again when the network is back.", tone: "exposed", icon: "M10 4v7M10 15v1" },
    rejected: { eyebrow: "Refused", head: "The broker refused the answer.", body: "The request was rejected or its scope could not be verified. Nothing on the module changed.", tone: "exposed", icon: "M6 6l8 8M14 6l-8 8" },
    paired: { eyebrow: "Paired", head: "This phone now answers for the module.", body: "The module has this phone in its keychain and the broker will wake it for every request.", tone: "sealed", icon: "M5 10.5 8.5 14 15 6" },
    unpaired: { eyebrow: "Unpaired", head: "The module forgot this phone.", body: "Requests stop at once. Pairing again means a new QR code from the Manager.", tone: "", icon: "M6 6l8 8M14 6l-8 8" },
  };
  const c = $derived(copy[outcome]);
  const next = $derived(app.pending[0]);
</script>

<div class="screen">
  <div class="screen-body centered">
    <div class="stack">
      <div class="mark-big" class:sealed={c.tone === "sealed"} class:exposed={c.tone === "exposed"}>
        <svg viewBox="0 0 20 20"><path d={c.icon} /></svg>
      </div>
      <div class="page-head" style="padding-top:0">
        <p class="eyebrow" class:exposed={c.tone === "exposed"} class:muted={c.tone === ""}>{c.eyebrow}</p>
        <h1>{c.head}</h1>
        <p>{c.body}</p>
      </div>
      <div class="card">
        <dl class="status-grid">
          <div class="wide"><dt>Request</dt><dd>{title}</dd></div>
          {#if detail}<div class="wide"><dt>Answer</dt><dd class="mono">{detail}</dd></div>{/if}
        </dl>
      </div>
    </div>
  </div>
  <div class="screen-actions">
    {#if next}
      <button class="button" onclick={() => app.go({ name: "request", id: next.id })}>Next: {next.host} wants to {next.phrase}</button>
    {/if}
    <button class="button plain" onclick={() => app.go({ name: "home" })}>Done</button>
  </div>
</div>
