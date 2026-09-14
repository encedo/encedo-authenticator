<script lang="ts">
  import { app } from "../lib/state.svelte";
  import Mark from "../lib/Mark.svelte";
  import Help from "../lib/Help.svelte";

  const u = $derived(app.update);
  const inPlace = $derived(u?.can_update_in_app ?? false);
</script>

<div class="screen">
  <div class="screen-body centered">
    <div class="stack">
      <div class="mark-big exposed"><svg viewBox="0 0 20 20"><path d="M10 4v7M10 15v1" /></svg></div>
      <div class="page-head" style="padding-top:0">
        <p class="eyebrow exposed">Must be updated</p>
        <h1>
          This version cannot be used.
          <Help
            label="Why the app stopped"
            parts={[
              { term: "What happened", text: "The publisher marked a newer release as one nobody may skip. That is only done for a fault that puts your modules at risk, so this build answers nothing until it is replaced." },
              { term: "What was wrong", text: "The release notes say it: What's new, on the app's page in Play." },
              { term: "Your modules", text: "They are untouched. Nothing was unpaired, and the keys on this phone are where they were." },
              { term: "If Play cannot do it here", text: "This build did not come from Play, so the button opens the store page instead and the new version is installed from there." },
            ]}
          />
        </h1>
        <p>A release that fixes a security fault is waiting. Until it is installed, this app will not answer a module.</p>
      </div>

      <div class="card exposed">
        <div class="card-head"><span>This build</span><span class="v mono">{u?.current_version ?? "—"}</span></div>
        <dl class="status-grid">
          <div><dt>Needed</dt><dd class="mono">{u?.required_version ?? "—"}</dd></div>
          <div><dt>Urgency</dt><dd>{u ? `${u.priority} of 5` : "—"}</dd></div>
          {#if (u?.stale_days ?? -1) >= 0}
            <div class="wide"><dt>Behind by</dt><dd>{u?.stale_days} day(s)</dd></div>
          {/if}
          <div class="wide"><dt>What was wrong</dt><dd>Written in the release notes; the button below opens them with the update.</dd></div>
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
    <button class="button plain" onclick={() => app.checkUpdate()}>Check again</button>
  </div>
</div>
