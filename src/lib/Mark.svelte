<script lang="ts">
  let { size = 26 }: { size?: number } = $props();
</script>

<!-- The family mark: the module outline with what stays inside it. -->
<svg class="mark" width={size} height={size} viewBox="0 0 64 64" role="img" aria-label="Encedo">
  <rect width="64" height="64" rx="13" fill="#0F5F4B" />
  <rect x="16" y="16" width="32" height="32" rx="7" fill="none" stroke="#AFD2C5" stroke-width="4" />
  <circle cx="32" cy="32" r="6" fill="#F2F8F5" />
</svg>
