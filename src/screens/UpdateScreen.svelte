<script lang="ts">
  // One screen, two tempers. A release nobody may skip has no way past it; a
  // release that is merely newer says so once a launch and then steps aside.
  import { app } from "../lib/state.svelte";
  import Help from "../lib/Help.svelte";

  const u = $derived(app.update);
  const blocking = $derived(u?.level === "critical");
  const inPlace = $derived(u?.can_update_in_app ?? false);
  // The blocking screen covers Settings, including the button that started a
  // pretence, so a development build gets its way back here.
  const pretended = $derived(u?.pretended ?? false);
</script>

<div class="screen">
  <div class="screen-body centered">
    <div class="stack">
      <div class="mark-big" class:exposed={blocking}>
        {#if blocking}
          <svg viewBox="0 0 20 20"><path d="M10 4v7M10 15v1" /></svg>
        {:else}
          <svg viewBox="0 0 20 20"><path d="M10 15V5M6 9l4-4 4 4" /></svg>
        {/if}
      </div>
      <div class="page-head" style="padding-top:0">
        <p class="eyebrow" class:exposed={blocking} class:muted={!blocking}>{blocking ? "Must be updated" : "Update waiting"}</p>
        <h1>
          {blocking ? "This version cannot be used." : "A newer version is ready."}
          <Help
            label={blocking ? "Why the app stopped" : "What this is about"}
            parts={blocking
              ? [
                  { term: "What happened", text: "The publisher marked a newer release as one nobody may skip. That is only done for a fault that puts your modules at risk, so this build answers nothing until it is replaced." },
                  { term: "What was wrong", text: "The release notes say it: What's new, on the app's page in Play." },
                  { term: "Your modules", text: "They are untouched. Nothing was unpaired, and the keys on this phone are where they were." },
                  { term: "If Play cannot do it here", text: "This build did not come from Play, so the button opens the store page instead and the new version is installed from there." },
                ]
              : [
                  { term: "Nothing is wrong", text: "This build still answers your modules. The newer one is simply better, and the release notes say how." },
                  { term: "Later", text: "Carries on with this build. The screen comes back the next time the app starts, until the newer version is installed." },
                  { term: "Your modules", text: "An update changes nothing about them: the keys stay on this phone, in the secure element." },
                ]}
          />
        </h1>
        <p>
          {blocking
            ? "A release that fixes a security fault is waiting. Until it is installed, this app will not answer a module."
            : "A newer release is in Play. What changed is in its release notes."}
        </p>
      </div>

      <div class="card" class:exposed={blocking}>
        <div class="card-head"><span>This build</span><span class="v mono">{u?.current_version ?? "—"}</span></div>
        <dl class="status-grid">
          <div><dt>{blocking ? "Needed" : "Waiting"}</dt><dd class="mono">{u?.required_version ?? "—"}</dd></div>
          <div><dt>Urgency</dt><dd>{u ? `${u.priority} of 5` : "—"}</dd></div>
          {#if (u?.stale_days ?? -1) >= 0}
            <div class="wide"><dt>Behind by</dt><dd>{u?.stale_days} day(s)</dd></div>
          {/if}
          <div class="wide"><dt>What changed</dt><dd>Written in the release notes; the button below opens them with the update.</dd></div>
        </dl>
      </div>
    </div>
  </div>
  <div class="screen-actions">
    <button class="button" disabled={app.busy} onclick={() => app.startUpdate()}>
      {app.busy ? "Opening Play…" : inPlace ? "Update now" : "Open Play"}
    </button>
    {#if app.lastError}
      <p class="note risk">{app.lastError}</p>
    {/if}
    {#if blocking}
      <button class="button plain" onclick={() => app.checkUpdate()}>Check again</button>
      {#if pretended}
        <button class="button plain" onclick={() => app.simulateUpdate("none")}>Leave the pretence (development build)</button>
      {/if}
    {:else}
      <button class="button plain" onclick={() => app.deferUpdate()}>Later</button>
    {/if}
  </div>
</div>
