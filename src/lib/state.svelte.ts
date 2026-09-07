import type { AccessRequest, ArchiveEntry, Module, Outcome, Settings } from "./types";
import * as mock from "./mock";
import { inTauri, requestPushPermission, startPush, type PushMessage, type PushState } from "./native";

export type Screen =
  | { name: "welcome" }
  | { name: "lock" }
  | { name: "home" }
  | { name: "modules" }
  | { name: "module"; pid: string }
  | { name: "pair" }
  | { name: "pairConfirm"; label: string; host: string; raw?: string }
  | { name: "request"; id: string }
  | { name: "result"; outcome: Outcome; title: string; detail?: string }
  | { name: "archive"; pid?: string }
  | { name: "settings" }
  | { name: "about" }
  | { name: "problem"; message: string };

export type Tab = "home" | "modules" | "archive" | "settings";

const SETTINGS_KEY = "encedo.settings.v2";

function loadSettings(): Settings {
  const fallback: Settings = { biometricLock: true, lockOnBackground: true, theme: "system", onboarded: false };
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    return raw ? { ...fallback, ...JSON.parse(raw) } : fallback;
  } catch {
    return fallback;
  }
}

class AppState {
  settings = $state<Settings>(loadSettings());
  screen = $state<Screen>({ name: "welcome" });
  modules = $state<Module[]>([...mock.modules]);
  pending = $state<AccessRequest[]>([]);
  archive = $state<ArchiveEntry[]>([...mock.archive]);
  online = $state(true);
  push = $state<PushState>({ status: inTauri ? "pending" : "unavailable" });
  pushLog = $state<PushMessage[]>([]);

  constructor() {
    if (this.settings.onboarded) {
      this.screen = this.settings.biometricLock ? { name: "lock" } : { name: "home" };
    }
    $effect.root(() => {
      $effect(() => {
        try {
          localStorage.setItem(SETTINGS_KEY, JSON.stringify(this.settings));
        } catch {}
        const t = this.settings.theme;
        if (t === "system") document.documentElement.removeAttribute("data-theme");
        else document.documentElement.setAttribute("data-theme", t);
      });
    });
  }

  get tab(): Tab | null {
    switch (this.screen.name) {
      case "home": return "home";
      case "modules": case "module": case "pairConfirm": return "modules";
      case "pair": return null; // camera takes the screen
      case "archive": return "archive";
      case "settings": case "about": return "settings";
      default: return null;
    }
  }

  go(screen: Screen) { this.screen = screen; }
  goTab(tab: Tab) { this.screen = { name: tab }; }

  finishOnboarding() {
    this.settings.onboarded = true;
    this.screen = { name: "home" };
    void this.askPushPermission();
  }

  /** Start listening for push tokens and messages. Called once at launch. */
  async startNative() {
    if (!inTauri) return;
    this.push = await startPush({
      onToken: (token) => { this.push = { ...this.push, status: "registered", token }; },
      onMessage: (m) => { this.pushLog = [m, ...this.pushLog].slice(0, 20); },
    });
  }

  async askPushPermission() {
    if (!inTauri) return;
    try {
      const granted = await requestPushPermission();
      this.push = { ...this.push, permission: granted ? "granted" : "denied" };
    } catch (e) {
      this.push = { ...this.push, error: String(e) };
    }
  }

  unlock() {
    this.screen = this.pending.length ? { name: "request", id: this.pending[0].id } : { name: "home" };
  }

  module(pid: string): Module | undefined { return this.modules.find((m) => m.pid === pid); }
  request(id: string): AccessRequest | undefined { return this.pending.find((r) => r.id === id); }

  /** Demo only: pretend a push arrived and allbypid returned one request. */
  simulateRequest(kind: Parameters<typeof mock.makeRequest>[0]) {
    const mod = this.modules[0] ?? mock.modules[0];
    const req = mock.makeRequest(kind, mod);
    this.pending = [...this.pending, req];
    this.screen = { name: "request", id: req.id };
  }

  resolve(id: string, outcome: Outcome, detail = "") {
    const req = this.request(id);
    if (!req) return;
    this.pending = this.pending.filter((r) => r.id !== id);
    this.archive = [{ id: `ar-${id}`, pid: req.pid, title: req.title, detail, outcome, at: new Date().toISOString() }, ...this.archive];
    if (outcome === "granted" || outcome === "denied") {
      const m = this.module(req.pid);
      if (m) m.lastUsed = new Date().toISOString();
    }
    this.screen = { name: "result", outcome, title: req.title, detail };
  }

  completePairing(label: string, host: string) {
    const pid = crypto.randomUUID();
    const today = new Date().toISOString();
    this.modules = [...this.modules, { pid, label, host, email: "chris@encedo.com", pairedAt: today.slice(0, 10) }];
    this.archive = [{ id: `ar-pair-${pid}`, pid, title: "Pair this phone", detail: label, outcome: "granted", at: today }, ...this.archive];
    this.screen = { name: "result", outcome: "granted", title: "Pair this phone", detail: `${label} · ${host}` };
  }

  unpair(pid: string) {
    this.modules = this.modules.filter((m) => m.pid !== pid);
    this.screen = { name: "modules" };
  }
}

export const app = new AppState();

export function fmtDate(iso: string): string {
  return new Date(iso).toLocaleDateString("en-GB", { year: "numeric", month: "short", day: "numeric" });
}
export function fmtDateTime(iso: string): string {
  const d = new Date(iso);
  const today = new Date().toDateString() === d.toDateString();
  const time = d.toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" });
  return today ? `today, ${time}` : d.toLocaleDateString("en-GB", { day: "numeric", month: "short" }) + ", " + time;
}
export function shortPid(pid: string): string {
  return pid.slice(0, 4) + "…" + pid.slice(-4);
}
export function count(n: number, one: string, many = one + "s"): string {
  const words = ["No", "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine"];
  return `${n < 10 ? words[n] : n} ${n === 1 ? one : many}`;
}
