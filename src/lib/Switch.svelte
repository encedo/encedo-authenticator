<script lang="ts">
  let { label, hint, checked = $bindable(false), onchange }: { label: string; hint?: string; checked?: boolean; onchange?: () => void } = $props();
</script>

<label class="switch">
  <span class="text">
    <span>{label}</span>
    {#if hint}<span class="hint">{hint}</span>{/if}
  </span>
  <input type="checkbox" bind:checked onchange={() => onchange?.()} />
  <span class="knob"></span>
</label>
