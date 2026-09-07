<script lang="ts">
  import { app } from "../lib/state.svelte";
  let { message }: { message: string } = $props();
</script>

<div class="screen">
  <div class="screen-body centered">
    <div class="stack">
      <div class="mark-big exposed"><svg viewBox="0 0 20 20"><path d="M10 4v7M10 15v1" /></svg></div>
      <div class="page-head" style="padding-top:0">
        <p class="eyebrow exposed">Stopped</p>
        <h1>The broker could not be reached.</h1>
        <p>{message}</p>
      </div>
    </div>
  </div>
  <div class="screen-actions row">
    <button class="button plain" onclick={() => app.go({ name: "home" })}>Back</button>
    <button class="button" onclick={() => app.go({ name: "home" })}>Try again</button>
  </div>
</div>
