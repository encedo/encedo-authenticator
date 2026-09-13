<script lang="ts">
  import { onMount } from "svelte";
  import { app } from "../lib/state.svelte";
  import Masthead from "../lib/Masthead.svelte";
  import Help from "../lib/Help.svelte";
  import { copyText } from "../lib/native";
  import { product } from "../lib/publisher";

  interface Item { n: string; v: string; l: string; c?: string; u?: string }
  interface Group { id: string; short: string; name: string; items: Item[] }
  interface Data { generated: string; groups: Group[]; texts: Record<string, string> }

  // Fetched when the screen opens, not bundled into the first paint: the list is
  // seventy kilobytes and nobody reads it on the way to answering a request.
  let data = $state<Data | null>(null);
  let group = $state("all");
  let q = $state("");
  let open = $state<Record<string, boolean>>({});
  let copied = $state(false);

  onMount(async () => {
    data = (await import("../lib/licences.json")).default as Data;
  });

  const groups = $derived(data?.groups ?? []);
  const rows = $derived.by(() => {
    const needle = q.trim().toLowerCase();
    const chosen = groups.filter((g) => group === "all" || g.id === group);
    return chosen.flatMap((g) =>
      g.items
        .filter((i) => !needle || (i.n + " " + i.l + " " + (i.c ?? "")).toLowerCase().includes(needle))
        .map((i) => ({ ...i, group: g.name })),
    );
  });

  /** The text of the licence a library is under; a choice shows the first we carry. */
  function textOf(expression: string): { id: string; body: string } | null {
    for (const part of expression.split(/\s+(?:OR|AND)\s+|\//)) {
      const id = part.trim();
      const body = data?.texts[id];
      if (body) return { id, body };
    }
    return null;
  }

  async function copyAll() {
    if (!data) return;
    const lines = [`${product} — open source libraries (${data.generated})`, ""];
    for (const g of data.groups) {
      lines.push(`## ${g.name}`);
      for (const i of g.items) lines.push(`${i.n} ${i.v} — ${i.l}${i.c ? ` — ${i.c}` : ""}`);
      lines.push("");
    }
    lines.push("## Licence texts", "");
    for (const [id, body] of Object.entries(data.texts)) lines.push(`### ${id}`, body, "");
    copied = await copyText(lines.join("\n"));
    setTimeout(() => (copied = false), 2000);
  }

  function isFirstOfGroup(i: number): boolean {
    return i === 0 || rows[i - 1].group !== rows[i].group;
  }
</script>

<div class="screen">
  <Masthead back={() => app.go({ name: "about" })} backLabel="About" />
  <div class="screen-body">
    <div class="page-head">
      <p class="eyebrow">Open source</p>
      <h1>
        What this app is built on.
        <Help
          label="About this list"
          parts={[
            { term: "What is in it", text: "Every library that ships inside the app: the Rust in its core, the JavaScript in its screens, and the Android and iOS libraries underneath. Not the tools used to build it." },
            { term: "Why it is here", text: "Most of these licences ask for their notice to travel with the software. Tap a library for its copyright line and the text of its licence." },
            { term: "Keeping it true", text: "Generated from the build itself by scripts/licences.py, not written by hand, so it cannot drift from what ships." },
          ]}
        />
      </h1>
    </div>

    {#if data}
      <div class="field">
        <label for="q">Search</label>
        <input id="q" type="search" bind:value={q} placeholder="Library, licence, holder" />
      </div>

      <div class="chips" role="group" aria-label="Where it is used">
        <button aria-pressed={group === "all"} onclick={() => (group = "all")}>All</button>
        {#each groups as g}
          <button aria-pressed={group === g.id} onclick={() => (group = g.id)}>{g.short} {g.items.length}</button>
        {/each}
      </div>

      <div class="card">
        <div class="card-head"><span>Libraries</span><span class="v">{rows.length}</span></div>
        {#if rows.length}
          <ul class="records">
            {#each rows as item, i (item.group + item.n + item.v)}
              {#if isFirstOfGroup(i)}
                <li class="group"><div class="daymark"><span>{item.group}</span><span></span></div></li>
              {/if}
              <li>
                <button class="rowbtn" aria-expanded={!!open[item.n + item.v]} onclick={() => (open = { ...open, [item.n + item.v]: !open[item.n + item.v] })}>
                  <span class="main">
                    <span class="name">{item.n}</span>
                    <span class="sub">{item.v} · {item.l}</span>
                  </span>
                  <svg class="chev" class:open={!!open[item.n + item.v]} viewBox="0 0 16 16"><path d="m6 3 5 5-5 5" /></svg>
                </button>
                {#if open[item.n + item.v]}
                  {@const text = textOf(item.l)}
                  <div class="detail">
                    <dl class="status-grid">
                      <div><dt>Version</dt><dd class="mono">{item.v}</dd></div>
                      <div><dt>Licence</dt><dd class="mono">{item.l}</dd></div>
                      <div class="wide"><dt>Copyright</dt><dd>{item.c ?? "not stated in the package"}</dd></div>
                      {#if item.u}<div class="wide"><dt>Source</dt><dd class="mono">{item.u}</dd></div>{/if}
                    </dl>
                    {#if text}
                      <div class="raw">
                        <div class="blob" style="max-height:40vh;overflow:auto;white-space:pre-wrap;word-break:normal">{text.body}</div>
                      </div>
                      <div class="acts"><span class="kindtag">{text.id}{item.l.includes(" OR ") ? " · one of the choices offered" : ""}</span></div>
                    {:else}
                      <div class="acts"><span class="kindtag">the text of {item.l} is not carried here; it is published with the project</span></div>
                    {/if}
                  </div>
                {/if}
              </li>
            {/each}
          </ul>
        {:else}
          <div class="empty">Nothing matches.</div>
        {/if}
        <div class="card-foot">
          <span>collected {data.generated}</span>
          <button class="button quiet small" onclick={copyAll}>{copied ? "Copied" : "Copy all"}</button>
        </div>
      </div>
    {:else}
      <div class="card"><div class="empty">Reading the list…</div></div>
    {/if}
  </div>
</div>
