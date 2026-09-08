<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { app } from "../lib/state.svelte";
  import { cancelScan, inTauri, openAppSettings, scanQr, scanZoomRange, setScanZoom } from "../lib/native";
  import { PREFERRED_START, clampToStep, zoomPlan, type ZoomPlan } from "../lib/qrzoom";
  import Masthead from "../lib/Masthead.svelte";

  let status = $state<"idle" | "scanning" | "denied" | "error">("idle");
  let error = $state("");
  let pasted = $state("");
  let pasting = $state(!inTauri);
  let alive = true;

  // The camera's own zoom, where it has one: same plan as encedo-chat
  // (lib/qrzoom.ts), opening at arm's length rather than 1x. The range is only
  // known once CameraX has bound the lens, so it is polled for a few seconds.
  let zoom = $state<ZoomPlan | null>(null);
  let ratio = $state(1);

  async function watchZoom() {
    zoom = null;
    for (let i = 0; i < 12 && alive && status === "scanning"; i++) {
      const r = await scanZoomRange();
      if (r) {
        zoom = zoomPlan({ zoom: { min: r.min, max: r.max, step: 0.1 } });
        ratio = zoom ? clampToStep(r.current, zoom.min, zoom.max, zoom.step) : 1;
        return;
      }
      await new Promise((f) => setTimeout(f, 300));
    }
  }

  async function onZoom(e: Event) {
    if (!zoom) return;
    const v = clampToStep(Number((e.target as HTMLInputElement).value), zoom.min, zoom.max, zoom.step);
    ratio = v;
    await setScanZoom(v);
  }

  async function start() {
    status = "scanning";
    void watchZoom();
    try {
      const raw = await scanQr(PREFERRED_START);
      if (!alive) return;
      if (raw === null) { status = "idle"; return; }
      await app.scanned(raw);
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
    {#if pasting || status === "error" || status === "denied"}
      <div class="field">
        <label for="paste">Paste what the code says</label>
        <input id="paste" class="mono" bind:value={pasted} placeholder={'{"link":"https://api.encedo.com/…","user":…}'} />
        <button class="button small" disabled={!pasted.trim() || app.busy} onclick={() => app.scanned(pasted.trim())}>Use this code</button>
      </div>
    {/if}
    <div class="viewfinder">
      <div class="frame"><i></i><i></i><i></i><i></i></div>
      {#if zoom && status === "scanning"}
        <div class="zoom-row">
          <span class="zoom-lab">{ratio.toFixed(1)}x</span>
          <input type="range" min={zoom.min} max={zoom.max} step={zoom.step} value={ratio} oninput={onZoom} aria-label="Camera zoom" />
        </div>
      {:else}
        <div class="hint">{status === "scanning" ? "Manager · Paired phones · Pair a phone" : inTauri ? "camera off" : "no camera in a browser"}</div>
      {/if}
    </div>
  </div>
  <div class="screen-actions row">
    <button class="button plain" onclick={leave}>Cancel</button>
    {#if status === "denied"}
      <button class="button" onclick={() => openAppSettings()}>Open settings</button>
    {:else if inTauri && status !== "scanning"}
      <button class="button" onclick={start}>Scan again</button>
    {:else if inTauri && !pasting}
      <button class="button plain" onclick={async () => { pasting = true; await cancelScan(); status = "idle"; }}>Paste a code</button>
    {:else if !inTauri}
      <button class="button" onclick={() => app.scanned(JSON.stringify({ link: "https://api.encedo.com/notify/pairing/3f9a…", user: "chris", hostname: "hem-wh.encedo.local", email: "chris@encedo.com" }))}>Simulate a scan</button>
    {/if}
  </div>
</div>
