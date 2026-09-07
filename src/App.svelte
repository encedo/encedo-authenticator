<script lang="ts">
  import { app } from "./lib/state.svelte";
  import Nav from "./lib/Nav.svelte";
  import Welcome from "./screens/Welcome.svelte";
  import Lock from "./screens/Lock.svelte";
  import Home from "./screens/Home.svelte";
  import Modules from "./screens/Modules.svelte";
  import ModuleDetails from "./screens/ModuleDetails.svelte";
  import Pair from "./screens/Pair.svelte";
  import PairConfirm from "./screens/PairConfirm.svelte";
  import Request from "./screens/Request.svelte";
  import Result from "./screens/Result.svelte";
  import Archive from "./screens/Archive.svelte";
  import Settings from "./screens/Settings.svelte";
  import About from "./screens/About.svelte";
  import Problem from "./screens/Problem.svelte";

  const s = $derived(app.screen);
</script>

{#key s}
  {#if s.name === "welcome"}<Welcome />
  {:else if s.name === "lock"}<Lock />
  {:else if s.name === "home"}<Home />
  {:else if s.name === "modules"}<Modules />
  {:else if s.name === "module"}<ModuleDetails pid={s.pid} />
  {:else if s.name === "pair"}<Pair />
  {:else if s.name === "pairConfirm"}<PairConfirm label={s.label} host={s.host} />
  {:else if s.name === "request"}<Request id={s.id} />
  {:else if s.name === "result"}<Result outcome={s.outcome} title={s.title} detail={s.detail} />
  {:else if s.name === "archive"}<Archive pid={s.pid} />
  {:else if s.name === "settings"}<Settings />
  {:else if s.name === "about"}<About />
  {:else if s.name === "problem"}<Problem message={s.message} />
  {/if}
{/key}

{#if app.tab}
  <Nav />
{/if}
