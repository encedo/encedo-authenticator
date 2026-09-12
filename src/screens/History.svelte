<script lang="ts">
  import { app, dayLabel, fmtDateTime, fmtTime, shortPid } from "../lib/state.svelte";
  import type { Family, LogEntry } from "../lib/api";
  import Masthead from "../lib/Masthead.svelte";
  import Help from "../lib/Help.svelte";
  import Switch from "../lib/Switch.svelte";
  import { copyText } from "../lib/native";

  let { pid }: { pid?: string } = $props();

  const families: { id: Family | "all"; label: string }[] = [
    { id: "all", label: "All" },
    { id: "answers", label: "Answers" },
    { id: "modules", label: "Modules" },
    { id: "push", label: "Push" },
    { id: "broker", label: "Broker" },
    { id: "app", label: "App" },
    { id: "trace", label: "Trace" },
  ];
  const orders = [
    { id: "new", label: "Newest" },
    { id: "old", label: "Oldest" },
    { id: "module", label: "By module" },
  ] as const;

  let family = $state<Family | "all">("all");
  let order = $state<(typeof orders)[number]["id"]>("new");
  // svelte-ignore state_referenced_locally
  let module = $state<string>(pid ?? "");
  let onlyBad = $state(false);
  let q = $state("");
  let sheet = $state(false);
  let open = $state<Record<string, boolean>>({});
  let copied = $state<string | null>(null);

  // The commentary is fetched only when asked for: it is long and it is not
  // part of the record.
  $effect(() => {
    if (family === "trace") void app.loadTrace();
  });

  const source = $derived(family === "trace" ? app.traceLog : app.log);
  const rows = $derived.by(() => {
    const needle = q.trim().toLowerCase();
    let out = source.filter((e) => {
      if (family !== "all" && family !== "trace" && e.family !== family) return false;
      if (module && e.pid !== module) return false;
      if (onlyBad && e.level !== "bad") return false;
      if (!needle) return true;
      const hay = [e.title, e.summary, e.kind, app.module(e.pid)?.label ?? "", e.raw ?? "", ...e.fields.map((f) => f.label + " " + f.value)].join(" ").toLowerCase();
      return hay.includes(needle);
    });
    if (order === "old") out = [...out].reverse();
    if (order === "module") out = [...out].sort((a, b) => (a.pid || "￿").localeCompare(b.pid || "￿") || b.at - a.at);
    return out;
  });

  type Cell = { label: string; value: string; mono: boolean; wide: boolean };

  /** What the expanded entry shows, as cells of the two-column grid. A literal
   *  takes the whole row; so does a last cell that would otherwise sit alone. */
  function cells(e: LogEntry): Cell[] {
    const second: Cell =
      e.repeat > 1 && e.first_at
        ? { label: `First of ${e.repeat}`, value: fmtDateTime(e.first_at), mono: false, wide: false }
        : e.pid
          ? { label: "Module", value: app.module(e.pid)?.label ?? shortPid(e.pid), mono: false, wide: false }
          : { label: "Event", value: e.kind, mono: true, wide: false };
    const out: Cell[] = [
      { label: "When", value: fmtDateTime(e.at), mono: false, wide: false },
      second,
      ...e.fields.map((f) => ({ label: f.label, value: f.value, mono: f.mono, wide: f.mono })),
    ];
    let column = 0;
    for (const c of out) {
      column = c.wide ? 0 : column === 0 ? 1 : 0;
    }
    if (column === 1) out[out.length - 1].wide = true;
    return out;
  }

  /** The heading above a run of rows: the day, or the module when sorted by it. */
  function groupOf(e: LogEntry): string {
    if (order !== "module") return dayLabel(e.at);
    return app.module(e.pid)?.label ?? (e.pid ? shortPid(e.pid) : "This app");
  }
  function groupNote(e: LogEntry): string {
    if (order !== "module") return "";
    return app.module(e.pid)?.host ?? (e.pid ? "no longer paired" : "");
  }
  function isFirstOfGroup(i: number): boolean {
    return i === 0 || groupOf(rows[i - 1]) !== groupOf(rows[i]);
  }

  async function copyEntry(e: LogEntry) {
    const text = [
      `${fmtDateTime(e.at)} · ${e.kind}`,
      e.title + (e.summary ? ` — ${e.summary}` : ""),
      ...(e.pid ? [`Module: ${app.module(e.pid)?.label ?? e.pid}`] : []),
      ...e.fields.map((f) => `${f.label}: ${f.value}`),
      ...(e.raw ? [e.raw] : []),
      ...(e.seal ? [`Seal: ${e.seal}`] : []),
    ].join("\n");
    if (await copyText(text)) {
      copied = e.id;
      setTimeout(() => (copied = null), 1600);
    }
  }

  async function copyAll() {
    const text = rows.map((e) => `${fmtDateTime(e.at)} ${e.kind} ${e.title}${e.summary ? " — " + e.summary : ""}${e.repeat > 1 ? ` (×${e.repeat})` : ""}`).join("\n");
    if (await copyText(text)) {
      copied = "all";
      setTimeout(() => (copied = null), 1600);
    }
  }

  const shown = $derived(rows.length);
  const broken = $derived(app.health?.broken_at ?? null);
</script>

<div class="screen">
  <Masthead />
  <div class="screen-body">
    <div class="page-head">
      <p class="eyebrow" class:exposed={!!broken}>History</p>
      <h1>
        Everything this phone did.
        <Help
          label="What the history keeps"
          parts={[
            { term: "Where it lives", text: "On this phone, in the same encrypted file as your keys. Nothing is sent anywhere." },
            { term: "How long", text: "Ninety days. Answers, pairings and pushes are sealed: each one covers the one before it, so an edited or missing entry is noticed." },
            { term: "Repeats", text: "The same event over and over is one row with a count, from the first time to the last." },
            { term: "Trace", text: "The running commentary of what the app did, kept apart because it is not part of the record." },
          ]}
        />
      </h1>
    </div>

    <div class="logbar">
      <div class="field">
        <label for="q">Search</label>
        <input id="q" type="search" bind:value={q} placeholder="Scope, module, text" />
      </div>
      <button class="sortbtn" aria-haspopup="dialog" onclick={() => (sheet = true)}>
        <svg viewBox="0 0 16 16"><path d="M3 4h10M5 8h6M7 12h2" /></svg>
        {orders.find((o) => o.id === order)?.label}
      </button>
    </div>

    <div class="chips" role="group" aria-label="What happened">
      {#each families as f}
        <button aria-pressed={family === f.id} onclick={() => (family = f.id)}>{f.label}</button>
      {/each}
    </div>

    <div class="card" class:exposed={!!broken}>
      <div class="card-head">
        <span>{family === "all" ? "Timeline" : families.find((f) => f.id === family)?.label}</span>
        <span class="v">{shown}</span>
      </div>
      {#if broken}
        <div class="status-note">The sealed chain does not follow from one entry to the next. Something changed this file outside the app; treat the entries around this point with suspicion.</div>
      {/if}
      {#if shown}
        <ul class="records">
          {#each rows as e, i (e.id)}
            {#if isFirstOfGroup(i)}
              <li class="group"><div class="daymark"><span>{groupOf(e)}</span><span>{groupNote(e)}</span></div></li>
            {/if}
            <li>
              <button class="rowbtn log" aria-expanded={!!open[e.id]} onclick={() => (open = { ...open, [e.id]: !open[e.id] })}>
                <span class="at">{fmtTime(e.at)}</span>
                <span class="dot" class:good={e.level === "good"} class:bad={e.level === "bad"}></span>
                <span class="main">
                  <span class="name">{e.title}</span>
                  <span class="sub">{e.summary || e.kind}</span>
                </span>
                {#if e.repeat > 1}<span class="rep">×{e.repeat}</span>{/if}
                {#if e.outcome}
                  <span class="pill" class:sealed={e.outcome === "granted" || e.outcome === "paired"} class:exposed={e.level === "bad"}><i></i>{e.outcome}</span>
                {/if}
                <svg class="chev" class:open={!!open[e.id]} viewBox="0 0 16 16"><path d="m6 3 5 5-5 5" /></svg>
              </button>
              {#if open[e.id]}
                <div class="detail">
                  <dl class="status-grid">
                    {#each cells(e) as c}
                      <div class:wide={c.wide}><dt>{c.label}</dt><dd class:mono={c.mono}>{c.value}</dd></div>
                    {/each}
                  </dl>
                  {#if e.raw}
                    <div class="raw"><div class="blob">{e.raw}</div></div>
                  {/if}
                  <div class="acts">
                    <span class="kindtag">{e.kind}{e.seal ? " · sealed" : ""}</span>
                    <span style="display:flex;gap:8px">
                      {#if e.pid && app.module(e.pid)}
                        <button class="button quiet small" onclick={() => app.go({ name: "module", pid: e.pid })}>Module</button>
                      {/if}
                      <button class="button quiet small" onclick={() => copyEntry(e)}>{copied === e.id ? "Copied" : "Copy"}</button>
                    </span>
                  </div>
                </div>
              {/if}
            </li>
          {/each}
        </ul>
      {:else}
        <div class="empty">{q || module || onlyBad || family !== "all" ? "Nothing matches." : "This phone has not done anything yet."}</div>
      {/if}
      <div class="card-foot">
        <span>{app.health ? `${app.health.entries} sealed${app.health.pruned ? ` · ${app.health.pruned} past 90 days` : ""}` : `${shown} entries`}</span>
        <span style="display:flex;gap:8px">
          <button class="button quiet small" disabled={!shown} onclick={copyAll}>{copied === "all" ? "Copied" : "Copy all"}</button>
          {#if family === "trace"}
            <button class="button quiet small" onclick={() => app.clearTrace()}>Clear</button>
          {/if}
        </span>
      </div>
    </div>
  </div>
</div>

{#if sheet}
  <div class="sheet" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) sheet = false; }}>
    <div class="panel" role="dialog" aria-label="Sort and filter">
      <h2>Sort and filter</h2>
      <div class="field">
        <span class="label">Order</span>
        <div class="segmented" role="group" aria-label="Order">
          {#each orders as o}
            <button aria-pressed={order === o.id} onclick={() => (order = o.id)}>{o.label}</button>
          {/each}
        </div>
      </div>
      <div class="field">
        <span class="label">Module</span>
        <div class="chips" role="group" aria-label="Module">
          <button aria-pressed={module === ""} onclick={() => (module = "")}>All modules</button>
          {#each app.modules as m (m.pid)}
            <button aria-pressed={module === m.pid} onclick={() => (module = m.pid)}>{m.label}</button>
          {/each}
        </div>
      </div>
      <Switch label="Only what went wrong" hint="Denied, refused, expired, errors" bind:checked={onlyBad} />
      <button class="button" onclick={() => (sheet = false)}>Show {shown} {shown === 1 ? "entry" : "entries"}</button>
    </div>
  </div>
{/if}
