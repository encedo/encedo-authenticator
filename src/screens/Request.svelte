<script lang="ts">
  import { onMount } from "svelte";
  import { app, PERIODS, shortPid } from "../lib/state.svelte";
  import Masthead from "../lib/Masthead.svelte";
  import Switch from "../lib/Switch.svelte";

  let { id }: { id: string } = $props();
  const req = $derived(app.request(id));

  let period = $state(3600);
  let writable = $state(false);
  let now = $state(Math.floor(Date.now() / 1000));
  let total = 120;

  onMount(() => {
    if (req) { writable = req.writable_default; total = Math.max(1, req.exp - now); }
    const t = setInterval(() => (now = Math.floor(Date.now() / 1000)), 1000);
    return () => clearInterval(t);
  });

  const left = $derived(req ? Math.max(0, req.exp - now) : 0);
  const pct = $derived(Math.round((left / total) * 100));
  $effect(() => { if (req && left === 0) app.expire(id); });

  const periodLabel = $derived(PERIODS.find((p) => p.secs === period)?.label ?? "");
  const allowLabel = $derived(
    !req ? "" : req.ask_writable ? `Allow ${writable ? "read-write" : "read-only"} for ${periodLabel}` : req.ask_period ? `Allow for ${periodLabel}` : "Allow",
  );
</script>

<div class="screen">
  <Masthead />
  {#if req}
    <div class="screen-body">
      <div class="page-head">
        <p class="eyebrow">Access request</p>
        <h1>{req.host} wants to {req.phrase}.</h1>
        <div class="countdown" class:low={left <= 20}>
          <span class="bar"><i style="width:{pct}%"></i></span>
          <span>{left}s left</span>
        </div>
      </div>

      <div class="card asking">
        <div class="card-head"><span>scope</span><span class="v truncate">{req.scope}</span></div>
        <dl class="status-grid">
          <div><dt>Module</dt><dd>{req.module_label || shortPid(req.pid)}</dd></div>
          <div><dt>Host</dt><dd class="mono">{req.host}</dd></div>
          {#each req.details as d}
            <div class:wide={d.mono}><dt>{d.label}</dt><dd class:mono={d.mono}>{d.value}</dd></div>
          {/each}
          {#if req.issuer}
            <div><dt>Asked from</dt><dd>{req.issuer.city}, {req.issuer.country}</dd></div>
            <div><dt>Address</dt><dd class="mono">{req.issuer.ip}</dd></div>
          {/if}
        </dl>
        {#if req.ask_writable}
          <div class="status-note">{writable ? "While the drive is unlocked read-write, its contents are as safe as the host." : "Read-only: the host can read the drive but not change it."}</div>
        {/if}
      </div>

      {#if req.ask_period}
        <div class="field">
          <span class="label">For how long</span>
          <div class="segmented" role="group" aria-label="Access period">
            {#each PERIODS as p}
              <button aria-pressed={period === p.secs} onclick={() => (period = p.secs)}>{p.label}</button>
            {/each}
          </div>
        </div>
      {/if}
      {#if req.ask_writable}
        <div>
          <Switch label="Allow writing" hint="The host may change the drive, not only read it" bind:checked={writable} />
        </div>
      {/if}
    </div>
    <div class="screen-actions row">
      <button class="button exposed" disabled={app.busy} onclick={() => app.resolve(id, false, period, writable)}>Deny</button>
      <button class="button" disabled={app.busy} onclick={() => app.resolve(id, true, period, writable)}>{app.busy ? "Sending…" : allowLabel}</button>
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
