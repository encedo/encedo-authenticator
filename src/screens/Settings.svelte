<script lang="ts">
  import { app } from "../lib/state.svelte";
  import { demoScopes, scopeTitle } from "../lib/mock";
  import Masthead from "../lib/Masthead.svelte";
  import Switch from "../lib/Switch.svelte";
  import { copyText, inTauri } from "../lib/native";
  import { fmtDateTime } from "../lib/state.svelte";
  let copied = $state(false);
  let lockNote = $state<string | null>(null);
  async function copyToken() {
    if (!app.push.token) return;
    copied = await copyText(app.push.token);
    setTimeout(() => (copied = false), 2000);
  }
  const pushWord = $derived(
    app.push.status === "registered" ? "registered" : app.push.status === "pending" ? "registering" : app.push.status === "unavailable" ? "not on this platform" : app.push.status,
  );
  const themes = ["system", "light", "dark"] as const;
  const heading = $derived(
    app.settings.biometric_lock ? "This phone asks who you are before it answers." : "This phone answers without asking who you are.",
  );
</script>

<div class="screen">
  <Masthead />
  <div class="screen-body">
    <div class="page-head">
      <p class="eyebrow">Settings</p>
      <h1>{heading}</h1>
      <p>Keys stay in the secure element either way. The lock decides who may press Allow.</p>
    </div>
    <div>
      <Switch label="Lock with biometrics" hint={lockNote ?? "Face, fingerprint or screen lock on launch"} bind:checked={app.settings.biometric_lock} onchange={async () => { lockNote = await app.setBiometricLock(app.settings.biometric_lock); }} />
      <Switch label="Lock when in background" hint="Ask again after switching apps" bind:checked={app.settings.lock_on_background} onchange={() => app.saveSettings()} />
    </div>
    <div class="field">
      <span class="label">Appearance</span>
      <div class="segmented" role="group" aria-label="Theme">
        {#each themes as t}
          <button aria-pressed={app.settings.theme === t} onclick={() => { app.settings.theme = t; void app.saveSettings(); }}>{t}</button>
        {/each}
      </div>
    </div>
    <div class="card" class:exposed={app.push.status === "error" || app.push.permission === "denied"}>
      <div class="card-head"><span>Push</span><span class="v" class:safe={app.push.status === "registered"}>{pushWord}</span></div>
      <dl class="status-grid">
        <div><dt>Provider</dt><dd class="mono">fcm</dd></div>
        <div><dt>Permission</dt><dd class:risk={app.push.permission === "denied"}>{app.push.permission ?? (inTauri ? "not asked" : "n/a")}</dd></div>
        <div class="wide">
          <dt class="between"><span>Token</span>{#if app.push.token}<button class="copy" class:done={copied} onclick={copyToken}>{copied ? "Copied" : "Copy"}</button>{/if}</dt>
          <dd class="mono">{app.push.token ?? (app.push.error ?? "—")}</dd>
        </div>
      </dl>
      {#if inTauri && app.push.permission !== "granted"}
        <div class="card-foot"><span>Android 13+ asks once</span><button class="button quiet small" onclick={() => app.askPushPermission()}>Request permission</button></div>
      {:else}
        <div class="status-note">Paste the token into Firebase console, Messaging, Send test message. A message with a title shows as a system notification; the app lists what it received below.</div>
      {/if}
    </div>
    {#if inTauri}
    <div class="card" class:exposed={!!app.lastRefresh?.error}>
      <div class="card-head"><span>Broker check</span><span class="v">{app.lastRefresh ? fmtDateTime(app.lastRefresh.at) : "not yet"}</span></div>
      {#if app.lastRefresh}
        <dl class="status-grid">
          <div><dt>Asked for</dt><dd class="mono">{app.lastRefresh.pids.length ? app.lastRefresh.pids.join(" ") : "no modules"}</dd></div>
          <div><dt>Pending / shown</dt><dd>{app.lastRefresh.pending} / {app.lastRefresh.shown}</dd></div>
          <div class="wide"><dt>Broker said</dt><dd class="mono" style="font-size:11.5px">{app.lastRefresh.broker_said || "—"}</dd></div>
          {#if app.lastRefresh.discarded.length}<div class="wide"><dt>Discarded</dt><dd class="mono" style="font-size:11.5px">{app.lastRefresh.discarded.join(" · ")}</dd></div>{/if}
          {#if app.lastRefresh.error}<div class="wide"><dt>Error</dt><dd class="risk">{app.lastRefresh.error}</dd></div>{/if}
        </dl>
      {/if}
      <div class="card-foot"><span>allbypid</span><button class="button quiet small" disabled={app.busy} onclick={() => app.refresh(false)}>Check now</button></div>
    </div>
    {/if}
    <div class="card">
      <div class="card-head"><span>Push received</span><span class="v">{app.pushLog.length}</span></div>
      {#if app.pushLog.length}
        <ul class="records">
          {#each app.pushLog as m, i (i)}
            <li><div class="row">
              <span class="main"><span class="name">{m.title ?? "(data only)"}</span><span class="sub">{m.body ?? ""}{Object.keys(m.data).length ? " · " + JSON.stringify(m.data) : ""}</span></span>
              <span class="when">{m.tapped ? "tapped · " : ""}{fmtDateTime(m.at)}</span>
            </div></li>
          {/each}
        </ul>
      {:else}
        <div class="empty">Nothing has arrived while the app was open.</div>
      {/if}
    </div>
    {#if !inTauri}
    <div class="card">
      <div class="card-head"><span>Mockup</span><span class="v">pretend a push arrived</span></div>
      <ul class="records">
        {#each demoScopes as s}
          <li><button class="rowbtn" onclick={() => app.simulateRequest(s)}>
            <span class="main"><span class="name">{scopeTitle(s)}</span><span class="sub">{s}</span></span>
            <svg class="chev" viewBox="0 0 16 16"><path d="m6 3 5 5-5 5" /></svg>
          </button></li>
        {/each}
        <li><button class="rowbtn" onclick={() => (app.online = !app.online)}><span class="main"><span class="name">Toggle network</span><span class="sub">{app.online ? "online" : "offline"}</span></span></button></li>
        <li><button class="rowbtn" onclick={() => app.go({ name: "problem", message: "api.encedo.com did not answer within 10 seconds. The request, if any, is still open on the module." })}><span class="main"><span class="name">Show a failure</span><span class="sub">broker unreachable</span></span></button></li>
        <li><button class="rowbtn" onclick={() => app.go({ name: "lock" })}><span class="main"><span class="name">Lock now</span></span></button></li>
      </ul>
    </div>
    {/if}
    <button class="button plain" onclick={() => app.go({ name: "about" })}>About this app</button>
  </div>
</div>
