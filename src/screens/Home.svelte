<script lang="ts">
  import { app, count, fmtDateTime, fmtTime, shortPid } from "../lib/state.svelte";
  import Masthead from "../lib/Masthead.svelte";
  import Help from "../lib/Help.svelte";
  import { onMount } from "svelte";
  import { inTauri } from "../lib/native";

  // Until push is certain, the phone asks the broker itself while this screen
  // is open. Cheap: allbypid with a handful of pids.
  onMount(() => {
    if (!inTauri) return;
    const t = setInterval(() => { if (app.modules.length && !app.busy) void app.refresh(); }, 15_000);
    return () => clearInterval(t);
  });

  const DAY = 86_400;
  const answers = $derived(app.log.filter((e) => e.outcome !== null));
  const recent = $derived(answers.filter((e) => e.at > Math.floor(Date.now() / 1000) - DAY).slice(0, 4));
  const n = $derived(app.pending.length);
  const heading = $derived(
    !app.online ? "This phone is off the network." : n ? `${count(n, "request")} ${n === 1 ? "is" : "are"} waiting for you.` : "Nothing is waiting for you.",
  );
  const pushLine = $derived(
    app.push.status === "registered"
      ? "Registered on this phone, so a request wakes the app even when it is closed."
      : app.push.status === "pending"
        ? "Registering with the notification service."
        : "Off in this build, so this screen asks the broker every 15 seconds instead.",
  );
</script>

<div class="screen">
  <Masthead />
  <div class="screen-body">
    <div class="page-head">
      <p class="eyebrow" class:exposed={!app.online}>Now</p>
      <h1>
        {heading}
        <Help
          label="What this screen is doing"
          parts={[
            { term: "Push", text: pushLine },
            { term: "Modules", text: `${count(app.modules.length, "module")} can ask you. A request from anything else is refused.` },
            ...(app.online ? [] : [{ term: "Off the network", text: "An answer cannot be sent. The module keeps waiting, then gives up on its own." }]),
            ...(app.busy ? [{ term: "Right now", text: "Asking the broker what is waiting." }] : []),
          ]}
        />
      </h1>
    </div>

    {#if app.modules.length && !n}
      <button class="button plain" disabled={app.busy} onclick={() => app.refresh()}>{app.busy ? "Asking the broker…" : "Check for requests"}</button>
    {/if}

    {#if n}
      <div class="card asking lifted">
        <div class="card-head"><span>Needs you</span><span class="v">{n}</span></div>
        <ul class="records">
          {#each app.pending as r (r.id)}
            <li><button class="rowbtn" onclick={() => app.go({ name: "request", id: r.id })}>
              <span class="main"><span class="name">{r.host} wants to {r.phrase}</span><span class="sub">{r.module_label || shortPid(r.pid)} · {r.scope.split("#")[0].slice(0, 28)}</span></span>
              <svg class="chev" viewBox="0 0 16 16"><path d="m6 3 5 5-5 5" /></svg>
            </button></li>
          {/each}
        </ul>
      </div>
    {/if}

    <div class="card">
      <div class="card-head"><span>Last 24 hours</span><span class="v">{count(recent.length, "answer")}</span></div>
      {#if recent.length}
        <ul class="records">
          {#each recent as e (e.id)}
            <li><div class="row">
              <span class="at">{fmtTime(e.at)}</span>
              <span class="dot" class:good={e.level === "good"} class:bad={e.level === "bad"}></span>
              <span class="main"><span class="name">{e.title}</span><span class="sub">{app.module(e.pid)?.label ?? shortPid(e.pid)}{e.summary ? " · " + e.summary : ""}</span></span>
              <span class="pill" class:sealed={e.outcome === "granted" || e.outcome === "paired"} class:exposed={e.level === "bad"}><i></i>{e.outcome}</span>
            </div></li>
          {/each}
        </ul>
      {:else}
        <div class="empty">{answers.length ? `Nothing today. ${count(answers.length, "answer")} before that.` : "This phone has not answered anything yet."}</div>
      {/if}
      <div class="card-foot"><span>{app.health ? `${app.health.entries} sealed entries` : "history"}</span><button class="button quiet small" onclick={() => app.go({ name: "history" })}>Open history</button></div>
    </div>

    <div class="card">
      <div class="card-head"><span>Modules</span><span class="v">{app.modules.length}</span></div>
      <ul class="records">
        {#each app.modules as m (m.pid)}
          <li><button class="rowbtn" onclick={() => app.go({ name: "module", pid: m.pid })}>
            <span class="main"><span class="name">{m.label}</span><span class="sub">{m.host} · pid {shortPid(m.pid)}</span></span>
            <span class="when">{m.last_used ? fmtDateTime(m.last_used) : "never used"}</span>
            <svg class="chev" viewBox="0 0 16 16"><path d="m6 3 5 5-5 5" /></svg>
          </button></li>
        {/each}
      </ul>
    </div>
  </div>
</div>
