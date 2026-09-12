<script lang="ts">
  import { app, fmtDate, fmtDateTime } from "../lib/state.svelte";
  import Masthead from "../lib/Masthead.svelte";
  import Help from "../lib/Help.svelte";
  import { copyText } from "../lib/native";
  let { pid }: { pid: string } = $props();
  let copied = $state<"kid" | "pid" | null>(null);
  async function copy(what: "kid" | "pid", value: string) {
    if (await copyText(value)) {
      copied = what;
      setTimeout(() => (copied = null), 2000);
    }
  }
  const m = $derived(app.module(pid));
  const history = $derived(app.log.filter((e) => e.pid === pid));
  const answers = $derived(history.filter((e) => e.outcome !== null));
  let confirm = $state(false);
  let objection = $state<string | null>(null);
  async function unpair() { objection = await app.unpair(pid); }
</script>

<div class="screen">
  <Masthead back={() => app.go({ name: "modules" })} backLabel="Modules" />
  {#if m}
    <div class="screen-body">
      <div class="page-head">
        <p class="eyebrow">Module</p>
        <h1>{m.label} can ask you.
          <Help
            label="About this module"
            parts={[
              { term: "Last answer", text: m.last_used ? `This phone last answered for it on ${fmtDateTime(m.last_used)}.` : "This phone has not answered for it yet." },
              { term: "KID", text: "The public half of the key this phone made for this module. The module knows this phone by it, and it is the name to look for in the Manager's list of paired phones. The private half never leaves the secure element." },
              { term: "What unpairing costs", text: "The module forgets this phone and removes it from its keychain. Requests stop at once. Pairing again means a new QR code from the Manager." },
            ]}
          />
        </h1>
      </div>
      <div class="card">
        <dl class="status-grid">
          <div><dt>Host</dt><dd class="mono">{m.host}</dd></div>
          <div><dt>Account</dt><dd>{m.email || m.user || "—"}</dd></div>
          <div><dt>Paired</dt><dd>{fmtDate(m.paired_at)}</dd></div>
          <div><dt>Answers</dt><dd>{answers.length}</dd></div>
          <div class="wide">
            <dt class="between"><span>KID</span><button class="copy" class:done={copied === "kid"} onclick={() => copy("kid", m.aid)}>{copied === "kid" ? "Copied" : "Copy"}</button></dt>
            <dd class="mono">{m.aid}</dd>
          </div>
          <div class="wide">
            <dt class="between"><span>pid</span><button class="copy" class:done={copied === "pid"} onclick={() => copy("pid", m.pid)}>{copied === "pid" ? "Copied" : "Copy"}</button></dt>
            <dd class="mono">{m.pid}</dd>
          </div>
        </dl>
        <div class="card-foot"><span>history</span><button class="button quiet small" onclick={() => app.go({ name: "history", pid })}>Show {history.length} entries</button></div>
      </div>
    </div>
    <div class="screen-actions" class:row={confirm}>
      {#if objection}
        <button class="button plain" onclick={() => { objection = null; confirm = false; }}>Keep</button>
        <button class="button exposed" disabled={app.busy} onclick={() => app.forget(pid)}>Remove from this phone anyway</button>
      {:else if confirm}
        <button class="button plain" onclick={() => (confirm = false)}>Keep</button>
        <button class="button exposed" disabled={app.busy} onclick={unpair}>{app.busy ? "Unpairing…" : "Unpair now"}</button>
      {:else}
        <button class="button exposed" onclick={() => (confirm = true)}>Unpair this module</button>
      {/if}
    </div>
  {:else}
    <div class="screen-body"><div class="page-head"><p class="eyebrow exposed">Module</p><h1>This module is no longer paired.</h1></div></div>
  {/if}
</div>
