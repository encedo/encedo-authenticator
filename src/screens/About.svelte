<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { app } from "../lib/state.svelte";
  import Masthead from "../lib/Masthead.svelte";
  let info = $state<{ version: string; platform: string }>({ version: "2.0.0", platform: "browser" });
  onMount(async () => {
    try { info = await invoke("app_info"); } catch { /* outside Tauri: keep the fallback */ }
  });
</script>

<div class="screen">
  <Masthead back={() => app.go({ name: "settings" })} backLabel="Settings" />
  <div class="screen-body">
    <div class="page-head">
      <p class="eyebrow">About</p>
      <h1>Keys stay inside; this phone answers for you.</h1>
      <p class="lede">The Authenticator is the other half of every sensitive operation on an Encedo HEM: the module asks, this phone signs the answer.</p>
    </div>
    <div class="card">
      <dl class="status-grid">
        <div><dt>Version</dt><dd class="mono">{info.version}</dd></div>
        <div><dt>Platform</dt><dd class="mono">{info.platform}</dd></div>
        <div class="wide"><dt>Protocol</dt><dd class="mono">X25519 · HMAC-SHA256 · AES-128-CBC · JWT HS256</dd></div>
      </dl>
    </div>
    <dl>
      <div class="item"><dt>What leaves the phone</dt><dd>A signed yes or no, and a push token so the broker can wake the app. Nothing else.</dd></div>
      <div class="item"><dt>What stays</dt><dd>One private key per module, in the secure element, and the archive of answers, encrypted on the phone.</dd></div>
    </dl>
    <p class="note">© Encedo · encedo.com/privacy</p>
  </div>
</div>
