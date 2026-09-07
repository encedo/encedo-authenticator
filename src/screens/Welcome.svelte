<script lang="ts">
  import { app } from "../lib/state.svelte";
  import Masthead from "../lib/Masthead.svelte";
  import Switch from "../lib/Switch.svelte";
</script>

<div class="screen">
  <Masthead />
  <div class="screen-body">
    <div class="page-head">
      <p class="eyebrow">Encedo Authenticator</p>
      <h1>This phone will answer for your module.</h1>
      <p class="lede">Every request to use a key, unlock a drive or change the module comes here first. Nothing happens until you allow it.</p>
    </div>
    <div>
      <Switch label="Lock with biometrics" hint="Face or fingerprint on launch" bind:checked={app.settings.biometricLock} />
      <Switch label="Lock when in background" hint="Ask again after switching apps" bind:checked={app.settings.lockOnBackground} />
    </div>
    <dl>
      <div class="item"><dt>What this phone holds</dt><dd>One private key per paired module, kept in the secure element. It signs your answers and never leaves the phone.</dd></div>
      <div class="item"><dt>What it needs</dt><dd>Notifications, to hear a request. The camera, to pair. Both are asked for when first used.</dd></div>
    </dl>
  </div>
  <div class="screen-actions">
    <button class="button" onclick={() => app.finishOnboarding()}>Continue</button>
  </div>
</div>
