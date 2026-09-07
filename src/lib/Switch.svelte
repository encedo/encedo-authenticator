<script lang="ts">
  let { label, hint, checked = $bindable(false) }: { label: string; hint?: string; checked?: boolean } = $props();
</script>

<label class="switch">
  <span class="text">
    <span>{label}</span>
    {#if hint}<span class="hint">{hint}</span>{/if}
  </span>
  <input type="checkbox" bind:checked />
  <span class="knob"></span>
</label>
