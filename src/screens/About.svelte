<script lang="ts">
  import { onMount } from "svelte";
  import { app } from "../lib/state.svelte";
  import { api } from "../lib/api";
  import { missingForStore, publisher } from "../lib/publisher";
  import Masthead from "../lib/Masthead.svelte";
  import Help from "../lib/Help.svelte";
  import { inTauri } from "../lib/native";

  let info = $state<{ version: string; platform: string; broker: string }>({ version: "2.0.0", platform: "browser", broker: "api.encedo.com" });
  let libraries = $state<number | null>(null);

  onMount(async () => {
    if (inTauri) info = await api.appInfo().catch(() => info);
    const data = await import("../lib/licences.json");
    libraries = data.default.groups.reduce((n, g) => n + g.items.length, 0);
  });

  const missing = $derived(missingForStore());
</script>

<div class="screen">
  <Masthead back={() => app.go({ name: "settings" })} backLabel="Settings" />
  <div class="screen-body">
    <div class="page-head">
      <p class="eyebrow">About</p>
      <h1>
        Keys stay inside; this phone answers for you.
        <Help
          label="What this app is"
          parts={[
            { term: "Its half of the job", text: "The Authenticator is the other half of every sensitive operation on an Encedo HEM: the module asks, this phone signs the answer." },
            { term: "What leaves the phone", text: "A signed yes or no, and a push token so the broker can wake the app. Nothing else." },
            { term: "What stays", text: "One private key per module, in the secure element, and ninety days of history, encrypted on this phone." },
          ]}
        />
      </h1>
    </div>

    <div class="card">
      <div class="card-head"><span>This app</span><span class="v mono">{info.version}</span></div>
      <dl class="status-grid">
        <div><dt>Platform</dt><dd class="mono">{info.platform}</dd></div>
        <div><dt>Made for</dt><dd>Encedo HEM</dd></div>
        <div class="wide"><dt>Broker</dt><dd class="mono">{info.broker}</dd></div>
        <div class="wide"><dt>Protocol</dt><dd class="mono">X25519 · HMAC-SHA256 · AES-128-CBC · JWT HS256</dd></div>
        <div class="wide"><dt>Storage</dt><dd class="mono">AES-256-GCM, key in the secure element</dd></div>
      </dl>
    </div>

    <div class="card" class:exposed={missing.length > 0}>
      <div class="card-head">
        <span>Publisher
          <Help
            label="Why this is here"
            parts={[
              { term: "What the stores ask for", text: "Google Play and the App Store both want a legal name, a contact address and a public privacy policy before they take an app." },
              { term: "Where it comes from", text: "One file in the source, src/lib/publisher.ts, so the app and both listings say the same thing." },
            ]}
          />
        </span>
        <span class="v">{publisher.name}</span>
      </div>
      <dl class="status-grid">
        <div><dt>Legal name</dt><dd class:pending={!publisher.legal}>{publisher.legal || "not settled"}</dd></div>
        <div><dt>Contact</dt><dd class:pending={!publisher.contact}>{publisher.contact || "not settled"}</dd></div>
        <div class="wide"><dt>Privacy policy</dt><dd class:pending={!publisher.privacy}>{publisher.privacy || "not settled"}</dd></div>
        {#if publisher.terms}<div class="wide"><dt>Terms</dt><dd>{publisher.terms}</dd></div>{/if}
      </dl>
      {#if missing.length}
        <div class="status-note">Before a store will take this app: {missing.join(", ")}. The company move decides them (see the release plan).</div>
      {/if}
    </div>

    <div class="card">
      <div class="card-head"><span>Open source</span><span class="v">{libraries ?? "…"} libraries</span></div>
      <div class="status-note">This app is built on work other people gave away. Every library it ships, its version, its licence and who holds the copyright.</div>
      <div class="card-foot">
        <span>attribution</span>
        <button class="button quiet small" onclick={() => app.go({ name: "licences" })}>Show the list</button>
      </div>
    </div>

    <p class="note">© {new Date().getFullYear()} {publisher.name}</p>
  </div>
</div>
