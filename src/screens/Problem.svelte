<script lang="ts">
  import { app } from "../lib/state.svelte";
  let { message, code }: { message: string; code?: string } = $props();
  const head = $derived(
    code === "network" || code === "timeout" || code === "unavailable" ? "The broker could not be reached." : code === "storage" ? "The phone's storage could not be opened." : code === "unauthorized" ? "The broker refused this pairing." : code === "rejected" ? "The broker refused this." : code === "bad_response" ? "That code could not be used." : "The operation stopped.",
  );
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
  <div class="screen-actions row">
    <button class="button plain" onclick={() => app.go({ name: "home" })}>Back</button>
    <button class="button" onclick={() => { app.go({ name: "home" }); void app.refresh(); }}>Try again</button>
  </div>
</div>
