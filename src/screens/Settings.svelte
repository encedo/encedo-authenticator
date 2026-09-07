<script lang="ts">
  import { app } from "../lib/state.svelte";
  import { demoScopes, scopeTitle } from "../lib/mock";
  import Masthead from "../lib/Masthead.svelte";
  import Switch from "../lib/Switch.svelte";
  const themes = ["system", "light", "dark"] as const;
  const heading = $derived(
    app.settings.biometricLock ? "This phone asks who you are before it answers." : "This phone answers without asking who you are.",
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
      <Switch label="Lock with biometrics" hint="Face or fingerprint on launch" bind:checked={app.settings.biometricLock} />
      <Switch label="Lock when in background" hint="Ask again after switching apps" bind:checked={app.settings.lockOnBackground} />
    </div>
    <div class="field">
      <span class="label">Appearance</span>
      <div class="segmented" role="group" aria-label="Theme">
        {#each themes as t}
          <button aria-pressed={app.settings.theme === t} onclick={() => (app.settings.theme = t)}>{t}</button>
        {/each}
      </div>
    </div>
    <div class="card">
      <div class="card-head"><span>Push</span><span class="v safe">registered</span></div>
      <dl class="status-grid">
        <div><dt>Provider</dt><dd class="mono">fcm</dd></div>
        <div><dt>Token</dt><dd class="mono">fXk9…a2Qe</dd></div>
        <div><dt>Renewed</dt><dd>today, 08:12</dd></div>
        <div><dt>Broker</dt><dd class="mono">api.encedo.com</dd></div>
      </dl>
    </div>
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
    <button class="button plain" onclick={() => app.go({ name: "about" })}>About this app</button>
  </div>
</div>
