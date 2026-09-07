<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { app } from "../lib/state.svelte";
  import { cancelScan, inTauri, openAppSettings, scanQr } from "../lib/native";
  import Masthead from "../lib/Masthead.svelte";

  let status = $state<"idle" | "scanning" | "denied" | "error">("idle");
  let error = $state("");
  let alive = true;

  function hostOf(raw: string): string {
    try { return new URL(raw).host; } catch { return raw.slice(0, 40); }
  }

  async function start() {
    status = "scanning";
    try {
      const raw = await scanQr();
      if (!alive) return;
      if (raw === null) { status = "idle"; return; }
      app.go({ name: "pairConfirm", label: hostOf(raw).split(".")[0] || "New module", host: hostOf(raw), raw });
    } catch (e) {
      const msg = String(e);
      status = msg.includes("permission") ? "denied" : "error";
      error = msg;
    }
  }

  onMount(() => { if (inTauri) void start(); });
  onDestroy(() => { alive = false; void cancelScan(); });

  function leave() { app.go({ name: "modules" }); }
</script>

<div class="screen">
  <Masthead back={leave} backLabel="Modules" />
  <div class="screen-body scan">
    <div class="page-head">
      <p class="eyebrow" class:exposed={status === "denied" || status === "error"}>Pair a module</p>
      {#if status === "denied"}
        <h1>The camera is off limits.</h1>
        <p>Allow the camera for Encedo Authenticator in the system settings, then come back.</p>
      {:else if status === "error"}
        <h1>The camera did not start.</h1>
        <p class="mono" style="font-size:12.5px">{error}</p>
      {:else}
        <h1>Scan the code the Manager shows.</h1>
      {/if}
    </div>
    <div class="viewfinder">
      <div class="frame"><i></i><i></i><i></i><i></i></div>
      <div class="hint">{status === "scanning" ? "Manager · Paired phones · Pair a phone" : inTauri ? "camera off" : "no camera in a browser"}</div>
    </div>
  </div>
  <div class="screen-actions row">
    <button class="button plain" onclick={leave}>Cancel</button>
    {#if status === "denied"}
      <button class="button" onclick={() => openAppSettings()}>Open settings</button>
    {:else if inTauri && status !== "scanning"}
      <button class="button" onclick={start}>Scan again</button>
    {:else if !inTauri}
      <button class="button" onclick={() => app.go({ name: "pairConfirm", label: "Warehouse PPA", host: "hem-wh.encedo.local", raw: "https://api.encedo.com/notify/pairing/3f9a…?h=…" })}>Simulate a scan</button>
    {/if}
  </div>
</div>
