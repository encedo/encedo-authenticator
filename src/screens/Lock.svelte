<script lang="ts">
  import { onMount } from "svelte";
  import { app, count } from "../lib/state.svelte";
  import Mark from "../lib/Mark.svelte";
  onMount(() => {
    // Give the activity a moment to be in front before the system prompt.
    const t = setTimeout(() => void app.unlock(), 250);
    return () => clearTimeout(t);
  });
</script>

<div class="screen">
  <div class="screen-body centered">
    <div class="stack">
      <Mark size={44} />
      <div class="page-head" style="padding-top:0">
        <p class="eyebrow" class:exposed={!!app.lockError} class:muted={!app.lockError}>{app.lockError ? "Not confirmed" : "Locked"}</p>
        <h1>{app.pending.length ? `${count(app.pending.length, "request")} waiting for you.` : "Nothing is waiting for you."}</h1>
        {#if app.lockError}
          <p>{app.lockError.message}</p>
        {:else if app.unlocking}
          <p>Confirm it is you with your fingerprint, face or screen lock.</p>
        {:else}
          <p>Unlock to see who is asking and for what.</p>
        {/if}
      </div>
    </div>
  </div>
  <div class="screen-actions">
    <button class="button" disabled={app.unlocking} onclick={() => app.unlock()}>{app.unlocking ? "Waiting for the system…" : app.lockError ? "Try again" : "Unlock"}</button>
  </div>
</div>
