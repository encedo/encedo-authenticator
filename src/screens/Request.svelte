<script lang="ts">
  import { onMount } from "svelte";
  import { app, shortPid } from "../lib/state.svelte";
  import type { Period } from "../lib/types";
  import Masthead from "../lib/Masthead.svelte";
  import Switch from "../lib/Switch.svelte";

  let { id }: { id: string } = $props();
  const req = $derived(app.request(id));
  const mod = $derived(req ? app.module(req.pid) : undefined);

  const periods: { v: Period; label: string }[] = [
    { v: 15, label: "15 min" }, { v: 60, label: "1 h" }, { v: 480, label: "8 h" }, { v: 1440, label: "24 h" },
  ];
  let period = $state<Period>(60);
  let writable = $state(false);
  let now = $state(Date.now());
  let total = 90_000;

  onMount(() => {
    if (req) { writable = req.writableDefault; total = Math.max(1, req.expiresAt - Date.now()); }
    const t = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(t);
  });

  const left = $derived(req ? Math.max(0, req.expiresAt - now) : 0);
  const pct = $derived(Math.round((left / total) * 100));
  const secs = $derived(Math.ceil(left / 1000));
  $effect(() => { if (req && left === 0) app.resolve(id, "expired"); });

  const periodLabel = $derived(periods.find((p) => p.v === period)!.label);
  const allowLabel = $derived(
    !req ? "" : req.askWritable ? `Allow ${writable ? "read-write" : "read-only"} for ${periodLabel}` : req.askPeriod ? `Allow for ${periodLabel}` : "Allow",
  );
  function summary(): string {
    if (!req) return "";
    const parts = req.details.filter((d) => !d.mono).map((d) => d.value);
    if (req.details[0]?.mono) parts.unshift(shortPid(req.details[0].value));
    if (req.askWritable) parts.push(writable ? "read-write" : "read-only");
    if (req.askPeriod) parts.push(periodLabel);
    return parts.join(" · ");
  }
</script>

<div class="screen">
  <Masthead />
  {#if req}
    <div class="screen-body">
      <div class="page-head">
        <p class="eyebrow">Access request</p>
        <h1>{req.host} wants to {req.phrase}.</h1>
        <div class="countdown" class:low={secs <= 20}>
          <span class="bar"><i style="width:{pct}%"></i></span>
          <span>{secs}s left</span>
        </div>
      </div>

      <div class="card asking">
        <div class="card-head"><span>scope</span><span class="v truncate">{req.scope}</span></div>
        <dl class="status-grid">
          <div><dt>Module</dt><dd>{mod?.label ?? shortPid(req.pid)}</dd></div>
          <div><dt>Host</dt><dd class="mono">{req.host}</dd></div>
          {#each req.details as d}
            <div class:wide={d.mono}><dt>{d.label}</dt><dd class:mono={d.mono}>{d.value}</dd></div>
          {/each}
        </dl>
        {#if req.askWritable}
          <div class="status-note">{writable ? "While the drive is unlocked read-write, its contents are as safe as the host." : "Read-only: the host can read the drive but not change it."}</div>
        {/if}
      </div>

      {#if req.askPeriod}
        <div class="field">
          <span class="label">For how long</span>
          <div class="segmented" role="group" aria-label="Access period">
            {#each periods as p}
              <button aria-pressed={period === p.v} onclick={() => (period = p.v)}>{p.label}</button>
            {/each}
          </div>
        </div>
      {/if}
      {#if req.askWritable}
        <div>
          <Switch label="Allow writing" hint="The host may change the drive, not only read it" bind:checked={writable} />
        </div>
      {/if}
    </div>
    <div class="screen-actions row">
      <button class="button exposed" onclick={() => app.resolve(id, "denied", summary())}>Deny</button>
      <button class="button" onclick={() => app.resolve(id, "granted", summary())}>{allowLabel}</button>
    </div>
  {:else}
    <div class="screen-body">
      <div class="page-head">
        <p class="eyebrow muted">Access request</p>
        <h1>This request was already answered.</h1>
        <p>Another phone replied first, or the module stopped waiting.</p>
      </div>
    </div>
    <div class="screen-actions"><button class="button plain" onclick={() => app.go({ name: "home" })}>Back</button></div>
  {/if}
</div>
