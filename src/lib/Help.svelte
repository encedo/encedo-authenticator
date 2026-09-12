<script lang="ts">
  // The explanation a screen used to print in full: one question mark, one
  // bubble. Sections have their own, so the help sits next to what it explains.
  let { label = "What this means", parts }: { label?: string; parts: { term?: string; text: string }[] } = $props();
  let open = $state(false);
  /** A question mark on the right half of the screen opens its bubble leftwards,
   *  so it never hangs off the edge. */
  let align = $state<"left" | "right">("left");
  let wrap: HTMLElement | undefined;

  // Whichever way it was opened, the side is decided before it is drawn: a
  // bubble hanging off the edge would also scroll the page sideways.
  $effect(() => {
    if (open && wrap) align = wrap.getBoundingClientRect().left > window.innerWidth / 2 ? "right" : "left";
  });

  function away(e: MouseEvent) {
    if (open && wrap && !wrap.contains(e.target as Node)) open = false;
  }
</script>

<svelte:window onclick={away} onkeydown={(e) => { if (e.key === "Escape") open = false; }} />

<span class="helpwrap" bind:this={wrap}>
  <button class="help" aria-expanded={open} aria-label={label} onclick={() => (open = !open)}>?</button>
  {#if open}
    <span class="bubble" class:right={align === "right"} role="note">
      {#each parts as p}
        <span class="part">
          {#if p.term}<b>{p.term}</b>{/if}
          {p.text}
        </span>
      {/each}
    </span>
  {/if}
</span>
