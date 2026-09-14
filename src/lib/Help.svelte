<script lang="ts">
  // The explanation a screen used to print in full: one question mark, one
  // bubble. Sections have their own, so the help sits next to what it explains.
  let { label = "What this means", parts }: { label?: string; parts: { term?: string; text: string }[] } = $props();
  let open = $state(false);
  let wrap: HTMLElement | undefined;

  /** Screen padding, matching --pad; the bubble never comes closer than this. */
  const EDGE = 20;
  const WANTED = 290;

  // The bubble belongs to the screen, not to the question mark: anchoring it to
  // the button is what sent it off the left edge when the button sat on the
  // right. So it is placed against the screen and the arrow moves instead.
  let width = $state(WANTED);
  let offset = $state(0);
  let arrow = $state(8);

  $effect(() => {
    if (!open || !wrap) return;
    const r = wrap.getBoundingClientRect();
    const screen = window.innerWidth;
    width = Math.min(WANTED, screen - 2 * EDGE);
    const centre = r.left + r.width / 2;
    const left = Math.max(EDGE, Math.min(centre - width / 2, screen - EDGE - width));
    offset = left - r.left;
    // Half the arrow is 5px; keep it inside the bubble's rounded corners.
    arrow = Math.max(10, Math.min(centre - left - 5, width - 20));
  });

  function away(e: MouseEvent) {
    if (open && wrap && !wrap.contains(e.target as Node)) open = false;
  }
</script>

<svelte:window onclick={away} onkeydown={(e) => { if (e.key === "Escape") open = false; }} />

<span class="helpwrap" bind:this={wrap}>
  <button class="help" aria-expanded={open} aria-label={label} onclick={() => (open = !open)}>?</button>
  {#if open}
    <span class="bubble" role="note" style="left:{offset}px; width:{width}px; --arrow:{arrow}px">
      {#each parts as p}
        <span class="part">
          {#if p.term}<b>{p.term}</b>{/if}
          {p.text}
        </span>
      {/each}
    </span>
  {/if}
</span>
