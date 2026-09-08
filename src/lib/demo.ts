import { app } from "./state.svelte";
import type { DemoKind } from "./mock";

/** Mockup phase only: `?demo=<screen>&theme=dark` opens a screen directly, for
 *  screenshots and browser previews. Goes away with the mock data. */
export function openDemo(d: string | null, theme?: string | null) {
  if (theme === "dark" || theme === "light" || theme === "system") app.settings.theme = theme;
  if (!d) return;
  app.pending = [];
  app.online = true;
  app.settings.onboarded = true;
  const first = app.modules[0];
  switch (d) {
    case "welcome": app.settings.onboarded = false; app.go({ name: "welcome" }); break;
    case "lock": app.go({ name: "lock" }); break;
    case "home": app.go({ name: "home" }); break;
    case "home-pending": app.simulateRequest("keymgmt:use"); app.simulateRequest("storage:disk"); app.go({ name: "home" }); break;
    case "offline": app.online = false; app.go({ name: "home" }); break;
    case "modules": app.go({ name: "modules" }); break;
    case "module": app.go({ name: "module", pid: first.pid }); break;
    case "pair": app.go({ name: "pair" }); break;
    case "pair-confirm": void app.scanned(JSON.stringify({ link: "https://api.encedo.com/notify/pairing/3f9a…", user: "chris", hostname: "hem-wh.encedo.local", email: "chris@encedo.com" })); break;
    case "archive": app.go({ name: "archive" }); break;
    case "settings": app.go({ name: "settings" }); break;
    case "about": app.go({ name: "about" }); break;
    case "granted": app.go({ name: "result", outcome: "granted", title: "Unlock a drive", detail: "disk0 · read-write · 1 h" }); break;
    case "denied": app.go({ name: "result", outcome: "denied", title: "Delete the log" }); break;
    case "expired": app.go({ name: "result", outcome: "expired", title: "Use the key" }); break;
    case "problem": app.go({ name: "problem", message: "api.encedo.com did not answer within 10 seconds. The request, if any, is still open on the module." }); break;
    default:
      if (d.startsWith("request-")) app.simulateRequest(d.slice(8) as DemoKind);
  }
}

export function applyDemoParams() {
  const p = new URLSearchParams(location.search);
  openDemo(p.get("demo"), p.get("theme"));
  (window as unknown as { __encedoDemo: typeof openDemo }).__encedoDemo = openDemo;
}
