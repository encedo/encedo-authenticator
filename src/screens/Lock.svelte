<script lang="ts">
  import { app, count } from "../lib/state.svelte";
  import Mark from "../lib/Mark.svelte";
</script>

<div class="screen">
  <div class="screen-body centered">
    <div class="stack">
      <Mark size={44} />
      <div class="page-head" style="padding-top:0">
        <p class="eyebrow muted">Locked</p>
        <h1>{app.pending.length ? `${count(app.pending.length, "request")} waiting for you.` : "Nothing is waiting for you."}</h1>
        <p>Unlock to see who is asking and for what.</p>
      </div>
    </div>
  </div>
  <div class="screen-actions">
    <button class="button" onclick={() => app.unlock()}>Unlock with biometrics</button>
  </div>
</div>
