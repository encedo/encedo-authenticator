<script lang="ts">
  import { app } from "../lib/state.svelte";
  let { message, code }: { message: string; code?: string } = $props();
  const head = $derived(
    code === "network" || code === "timeout" || code === "unavailable" ? "The broker could not be reached." : code === "key_lost" ? "This phone cannot read its own storage." : code === "storage" ? "The phone's storage could not be opened." : code === "unauthorized" ? "The broker refused this pairing." : code === "rejected" ? "The broker refused this." : code === "bad_response" ? "That code could not be used." : "The operation stopped.",
  );
  const lost = $derived(code === "key_lost");
  let confirm = $state(false);
</script>

<div class="screen">
  <div class="screen-body centered">
    <div class="stack">
      <div class="mark-big exposed"><svg viewBox="0 0 20 20"><path d="M10 4v7M10 15v1" /></svg></div>
      <div class="page-head" style="padding-top:0">
        <p class="eyebrow exposed">Stopped</p>
        <h1>{head}</h1>
        <p>{message}</p>
      </div>
    </div>
  </div>
  {#if lost}
    <div class="screen-body" style="flex:0 0 auto">
      <dl class="status-grid">
        <div class="wide"><dt>Why</dt><dd>The key that protected this storage is gone from this phone: it was restored from a backup of another phone, or the screen lock was removed. The contents cannot be decrypted by anything, here or anywhere else.</dd></div>
        <div class="wide"><dt>What starting over costs</dt><dd>The paired modules and the history kept until now. Your modules are not harmed; each one has to be paired again, and the old phone key can be removed in the Manager.</dd></div>
      </dl>
    </div>
    <div class="screen-actions" class:row={confirm}>
      {#if confirm}
        <button class="button plain" onclick={() => (confirm = false)}>Keep waiting</button>
        <button class="button exposed" disabled={app.busy} onclick={() => app.resetStorage()}>{app.busy ? "Starting over…" : "Start over"}</button>
      {:else}
        <button class="button exposed" onclick={() => (confirm = true)}>Start over with empty storage</button>
      {/if}
    </div>
  {:else}
    <div class="screen-actions row">
      <button class="button plain" onclick={() => app.go({ name: "home" })}>Back</button>
      <button class="button" onclick={() => { app.go({ name: "home" }); void app.refresh(); }}>Try again</button>
    </div>
  {/if}
</div>
