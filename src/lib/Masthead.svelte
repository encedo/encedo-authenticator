<script lang="ts">
  import Mark from "./Mark.svelte";
  let { back, backLabel = "Back" }: { back?: () => void; backLabel?: string } = $props();
</script>

<div class="masthead">
  <Mark />
  <span><b>Encedo</b> Authenticator</span>
  {#if back}
    <button class="button plain small back" onclick={back}>
      <svg viewBox="0 0 20 20"><path d="M12 4 6 10l6 6" /></svg>
      {backLabel}
    </button>
  {/if}
</div>
