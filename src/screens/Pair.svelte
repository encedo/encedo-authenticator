<script lang="ts">
  import { app } from "../lib/state.svelte";
  import Masthead from "../lib/Masthead.svelte";
</script>

<div class="screen">
  <Masthead back={() => app.go({ name: "modules" })} backLabel="Modules" />
  <div class="screen-body">
    <div class="page-head">
      <p class="eyebrow">Pair a module</p>
      <h1>Scan the code the Manager shows.</h1>
      <p>It carries the pairing link and a hash of the module's challenge. The phone checks that hash against the request it fetches next.</p>
    </div>
    <div class="viewfinder">
      <div class="frame"><i></i><i></i><i></i><i></i></div>
      <div class="hint">camera</div>
    </div>
  </div>
  <div class="screen-actions row">
    <button class="button plain" onclick={() => app.go({ name: "modules" })}>Cancel</button>
    <button class="button" onclick={() => app.go({ name: "pairConfirm", label: "Warehouse PPA", host: "hem-wh.encedo.local" })}>Simulate a scan</button>
  </div>
</div>
