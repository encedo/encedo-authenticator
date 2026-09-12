<script lang="ts">
  import { app } from "../lib/state.svelte";
  import Masthead from "../lib/Masthead.svelte";
  import Switch from "../lib/Switch.svelte";
  import Help from "../lib/Help.svelte";
</script>

<div class="screen">
  <Masthead />
  <div class="screen-body">
    <div class="page-head">
      <p class="eyebrow">Encedo Authenticator</p>
      <h1>This phone will answer for your module.
        <Help
          label="How this works"
          parts={[
            { term: "What happens here", text: "Every request to use a key, unlock a drive or change the module comes here first. Nothing happens until you allow it." },
            { term: "What this phone holds", text: "One private key per paired module, kept in the secure element. It signs your answers and never leaves the phone." },
            { term: "What it needs", text: "Notifications, to hear a request. The camera, to pair. Both are asked for when first used." },
          ]}
        />
      </h1>
    </div>
    <div>
      <Switch label="Lock with biometrics" hint="Face, fingerprint or screen lock on launch and after time away" bind:checked={app.settings.biometric_lock} />
      <Switch label="Lock when in background" hint="Ask again after switching apps" bind:checked={app.settings.lock_on_background} />
    </div>
  </div>
  <div class="screen-actions">
    <button class="button" onclick={() => app.finishOnboarding()}>Continue</button>
  </div>
</div>
