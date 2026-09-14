<script lang="ts">
  import { app, fmtDateTime } from "../lib/state.svelte";
  import { demoScopes, scopeTitle } from "../lib/mock";
  import Masthead from "../lib/Masthead.svelte";
  import Switch from "../lib/Switch.svelte";
  import Help from "../lib/Help.svelte";
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { copyText, inTauri } from "../lib/native";

  let copied = $state(false);
  let lockNote = $state<string | null>(null);
  // The update screens cannot be reached without publishing a release, so a
  // development build can pretend Play said something.
  let isDev = $state(false);
  onMount(async () => {
    if (inTauri) isDev = (await api.appInfo().catch(() => null))?.version.includes("-dev.") ?? false;
  });

  async function copyToken() {
    if (!app.push.token) return;
    copied = await copyText(app.push.token);
    setTimeout(() => (copied = false), 2000);
  }

  const pushWord = $derived(
    app.push.status === "registered" ? "registered" : app.push.status === "pending" ? "registering" : app.push.status === "unavailable" ? "not on this platform" : app.push.status,
  );
  // Android opens a window after one confirmation; iOS asks on every use.
  const window = $derived(
    app.storeStatus?.window_seconds ? `for ${app.storeStatus.window_seconds} s after your confirmation` : "only when you confirm, every time",
  );
  const themes: { id: "light" | "dark" | "system"; label: string }[] = [
    { id: "light", label: "Light" },
    { id: "dark", label: "Dark" },
    { id: "system", label: "System" },
  ];
  const heading = $derived(
    app.settings.biometric_lock ? "This phone asks who you are before it answers." : "This phone answers without asking who you are.",
  );
</script>

<div class="screen">
  <Masthead />
  <div class="screen-body">
    <div class="page-head">
      <p class="eyebrow">Settings</p>
      <h1>
        {heading}
        <Help
          label="About the lock"
          parts={[
            { term: "What the lock decides", text: "Keys stay in the secure element either way. The lock decides who may press Allow." },
            { term: "No screen lock on the phone", text: "Then the app cannot lock itself. Set one in the system settings first." },
            ...(lockNote ? [{ term: "Just now", text: lockNote }] : []),
          ]}
        />
      </h1>
    </div>
    <div>
      <Switch label="Lock with biometrics" hint="On launch, and it binds the storage key to you" bind:checked={app.settings.biometric_lock} onchange={async () => { lockNote = await app.setBiometricLock(app.settings.biometric_lock); }} />
      <Switch label="Lock when in background" hint="Ask again after switching apps" bind:checked={app.settings.lock_on_background} onchange={() => app.saveSettings()} />
    </div>
    <div class="field">
      <span class="label">Appearance</span>
      <div class="segmented" role="group" aria-label="Appearance">
        {#each themes as t}
          <button aria-pressed={app.settings.theme === t.id} onclick={() => { app.settings.theme = t.id; void app.saveSettings(); }}>{t.label}</button>
        {/each}
      </div>
      <span class="hint">System follows the phone, and is what this app starts with.</span>
    </div>

    {#if inTauri}
      <div class="card" class:exposed={app.storeStatus ? !app.storeStatus.bound_to_user : false}>
        <div class="card-head">
          <span>Storage key
            <Help
              label="About the storage key"
              parts={[
                { term: "What it protects", text: "One key encrypts everything this phone keeps: the private key of every paired module, the history, the push token." },
                { term: "Bound to you", text: `The secure hardware releases that key ${window}. Someone in full control of this phone can ask for it as this app, but not without you.` },
                { term: "Not bound", text: "The key is released to the app whenever it runs, so anything running with the app's rights can read the storage." },
                { term: "What it cannot do", text: "While the app is open and unlocked, the key is in its memory. A phone taken over at that moment is a phone taken over." },
                { term: "Where it lives", text: app.storeStatus?.strong_box ? "In a separate secure element (StrongBox), not only in the processor's secure world." : "In the processor's secure world; this phone has no separate secure element." },
              ]}
            />
          </span>
          <span class="v" class:safe={!!app.storeStatus?.bound_to_user}>{app.storeStatus?.bound_to_user ? "bound to you" : "not bound"}</span>
        </div>
        <dl class="status-grid">
          <div><dt>Released after</dt><dd>{app.storeStatus?.bound_to_user ? window : "nothing; whenever the app runs"}</dd></div>
          <div><dt>Secure element</dt><dd>{app.storeStatus?.strong_box ? "StrongBox" : "TEE"}</dd></div>
          <div><dt>Screen lock</dt><dd class:risk={app.storeStatus ? !app.storeStatus.credential : false}>{app.storeStatus?.credential ? "set on this phone" : "none, so the key cannot be bound"}</dd></div>
          <div><dt>While locked</dt><dd>nothing is decrypted</dd></div>
        </dl>
        {#if app.storeStatus && !app.storeStatus.bound_to_user && app.storeStatus.credential}
          <div class="status-note">Turn the lock on above to bind the key to you.</div>
        {/if}
      </div>
    {/if}
    <div class="card" class:exposed={app.push.status === "error" || app.push.permission === "denied"}>
      <div class="card-head">
        <span>Push
          <Help
            label="About push"
            parts={[
              { term: "What it is for", text: "The broker wakes this app when a module asks for something. Without it, requests are only seen while the app is open." },
              { term: "The token", text: "The name the notification service knows this phone by. It goes to the broker when you pair, and again whenever it changes." },
              { term: "Testing it", text: "Paste the token into Firebase console, Messaging, Send test message. Everything that arrives lands in the history." },
            ]}
          />
        </span>
        <span class="v" class:safe={app.push.status === "registered"}>{pushWord}</span>
      </div>
      <dl class="status-grid">
        <div><dt>Provider</dt><dd class="mono">fcm</dd></div>
        <div><dt>Permission</dt><dd class:risk={app.push.permission === "denied"}>{app.push.permission ?? (inTauri ? "not asked" : "n/a")}</dd></div>
        <div class="wide">
          <dt class="between"><span>Token</span>{#if app.push.token}<button class="copy" class:done={copied} onclick={copyToken}>{copied ? "Copied" : "Copy"}</button>{/if}</dt>
          <dd class:mono={!!app.push.token}>{app.push.token ?? (app.push.error ?? "—")}</dd>
        </div>
      </dl>
      {#if inTauri && app.push.permission !== "granted"}
        <div class="card-foot"><span>Android 13+ asks once</span><button class="button quiet small" onclick={() => app.askPushPermission()}>Request permission</button></div>
      {:else}
        <div class="card-foot"><span>arrivals and token changes</span><button class="button quiet small" onclick={() => app.go({ name: "history" })}>In history</button></div>
      {/if}
    </div>

    {#if inTauri}
      <div class="card" class:exposed={!!app.lastRefresh?.error}>
        <div class="card-head">
          <span>Broker
            <Help
              label="About the broker"
              parts={[
                { term: "What it does", text: "api.encedo.com carries requests from a module to this phone and your answer back. It never sees a key or a decrypted scope." },
                { term: "What is checked", text: "Every check asks for the modules paired here. Every one of them, and what came back, is in the history under Broker." },
              ]}
            />
          </span>
          <span class="v">{app.lastRefresh ? fmtDateTime(app.lastRefresh.at) : "not yet"}</span>
        </div>
        {#if app.lastRefresh}
          <dl class="status-grid">
            <div><dt>Waiting / shown</dt><dd>{app.lastRefresh.pending} / {app.lastRefresh.shown}</dd></div>
            <div><dt>Modules asked for</dt><dd>{app.lastRefresh.pids.length || "none"}</dd></div>
            {#if app.lastRefresh.error}<div class="wide"><dt>Error</dt><dd class="risk">{app.lastRefresh.error}</dd></div>{/if}
          </dl>
        {/if}
        <div class="card-foot"><span>allbypid</span><button class="button quiet small" disabled={app.busy} onclick={() => app.refresh(false)}>Check now</button></div>
      </div>

      <div class="card" class:exposed={!!app.health?.broken_at}>
        <div class="card-head">
          <span>History
            <Help
              label="About the history"
              parts={[
                { term: "What is kept", text: "Ninety days of everything this phone did: answers, pairings, pushes, broker checks, launches and locks." },
                { term: "Sealed", text: "Each answer, pairing and push carries a seal over the one before it. An edited or missing entry breaks the chain, and this card says so." },
                { term: "Where it lives", text: "In the same encrypted file as your keys, on this phone only." },
              ]}
            />
          </span>
          <span class="v" class:safe={!!app.health && !app.health.broken_at}>{app.health ? (app.health.broken_at ? "chain broken" : "chain holds") : "—"}</span>
        </div>
        <dl class="status-grid">
          <div><dt>Sealed entries</dt><dd>{app.health?.entries ?? 0}</dd></div>
          <div><dt>Past 90 days</dt><dd>{app.health?.pruned ?? 0} dropped</dd></div>
        </dl>
        <div class="card-foot"><span>answers, pushes, the trail</span><button class="button quiet small" onclick={() => app.go({ name: "history" })}>Open history</button></div>
      </div>
    {/if}

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

    {#if isDev}
      <div class="card">
        <div class="card-head"><span>Update screens</span><span class="v">development build</span></div>
        <div class="status-note">Pretend Play answered, to see what a person would be shown. Blocking takes over every screen; recommended is a line on Now, so this jumps there. Nothing is downloaded and nothing leaves this phone.</div>
        <div class="segmented" role="group" aria-label="Pretend an update">
          <button onclick={() => app.simulateUpdate("none")}>None</button>
          <button onclick={() => app.simulateUpdate("recommended")}>Recommended</button>
          <button onclick={() => app.simulateUpdate("critical")}>Blocking</button>
        </div>
      </div>
    {/if}

    <button class="button plain" onclick={() => app.go({ name: "about" })}>About this app</button>
  </div>
</div>
