import type { Settings } from "./api";
import { api, ApiError, type AnswerView, type ArchiveEntry, type ModuleView, type Outcome, type PairingPreview, type RefreshReport, type RequestView } from "./api";
import { biometricAuth, inTauri, trace, requestPushPermission, startPush, type PushMessage, type PushState } from "./native";
import * as mock from "./mock";

export type Screen =
  | { name: "welcome" }
  | { name: "lock" }
  | { name: "home" }
  | { name: "modules" }
  | { name: "module"; pid: string }
  | { name: "pair" }
  | { name: "pairConfirm"; preview: PairingPreview; raw: string }
  | { name: "request"; id: string }
  | { name: "result"; outcome: Outcome; title: string; detail?: string }
  | { name: "archive"; pid?: string }
  | { name: "settings" }
  | { name: "about" }
  | { name: "problem"; message: string; code?: string };

export type Tab = "home" | "modules" | "archive" | "settings";

const SETTINGS_KEY = "encedo.settings.v2";

function loadLocalSettings(): Settings {
  const fallback: Settings = { biometric_lock: true, lock_on_background: true, theme: "system", onboarded: false };
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    return raw ? { ...fallback, ...JSON.parse(raw) } : fallback;
  } catch {
    return fallback;
  }
}

class AppState {
  settings = $state<Settings>(loadLocalSettings());
  screen = $state<Screen>({ name: "welcome" });
  modules = $state<ModuleView[]>(inTauri ? [] : mock.modules);
  pending = $state<RequestView[]>([]);
  archive = $state<ArchiveEntry[]>(inTauri ? [] : mock.archive);
  online = $state(true);
  busy = $state(false);
  lastError = $state<string | null>(null);
  lastRefresh = $state<RefreshReport | null>(null);
  push = $state<PushState>({ status: inTauri ? "pending" : "unavailable" });
  pushLog = $state<PushMessage[]>([]);
  ready = $state(!inTauri);
  /** Lock screen state. */
  unlocking = $state(false);
  lockError = $state<{ code?: string; message: string } | null>(null);
  private returnTo: Screen | null = null;
  private static readonly LOCK_AFTER_MS = 5000;

  constructor() {
    $effect.root(() => {
      $effect(() => {
        const t = this.settings.theme;
        if (t === "system") document.documentElement.removeAttribute("data-theme");
        else document.documentElement.setAttribute("data-theme", t);
        if (!inTauri) {
          try { localStorage.setItem(SETTINGS_KEY, JSON.stringify(this.settings)); } catch {}
        }
      });
    });
    if (!inTauri && this.settings.onboarded) this.screen = this.settings.biometric_lock ? { name: "lock" } : { name: "home" };
  }

  /** Load everything from the Rust core, then start push. Called once at launch. */
  async boot() {
    if (!inTauri) return;
    trace("boot");
    // Second source of the same signal, independent of the plugin channel:
    // the WebView reports page visibility when the app leaves the screen.
    let hiddenAt = 0;
    document.addEventListener("visibilitychange", () => {
      if (document.visibilityState === "hidden") { hiddenAt = Date.now(); trace("visibility hidden"); }
      else { const away = hiddenAt ? Date.now() - hiddenAt : 0; hiddenAt = 0; trace(`visibility visible away=${away}`); this.onLifecycle("resumed", away); }
    });
    try {
      const [settings, modules, archive] = await Promise.all([api.settingsGet(), api.modules(), api.archive()]);
      this.settings = settings;
      this.modules = modules;
      this.archive = archive;
      this.screen = !settings.onboarded ? { name: "welcome" } : settings.biometric_lock ? { name: "lock" } : { name: "home" };
    } catch (e) {
      this.screen = { name: "problem", message: `Storage could not be opened: ${String((e as Error).message ?? e)}`, code: "storage" };
    } finally {
      this.ready = true;
    }
    this.push = await startPush({
      onToken: (token) => { this.push = { ...this.push, status: "registered", token }; void api.pushTokenChanged(token).catch(() => {}); },
      onMessage: (m) => { this.pushLog = [m, ...this.pushLog].slice(0, 20); void this.onPush(m); },
      onLifecycle: (state, awayMs) => this.onLifecycle(state, awayMs),
    });
    if (this.push.token) void api.pushTokenChanged(this.push.token).catch(() => {});
    if (this.settings.onboarded && !this.settings.biometric_lock) void this.refresh();
  }

  async onPush(m: PushMessage) {
    const encedo = m.data?.encedo;
    let refresh = true;
    if (typeof encedo === "string") {
      try { refresh = await api.pushPayload(encedo); } catch { refresh = true; }
      if (!refresh) await this.reloadModules();
    }
    if (refresh && this.screen.name !== "lock" && this.screen.name !== "welcome") await this.refresh();
  }

  get tab(): Tab | null {
    switch (this.screen.name) {
      case "home": return "home";
      case "modules": case "module": case "pairConfirm": return "modules";
      case "pair": return null;
      case "archive": return "archive";
      case "settings": case "about": return "settings";
      default: return null;
    }
  }

  go(screen: Screen) { this.screen = screen; }
  goTab(tab: Tab) { this.screen = { name: tab }; }

  fail(e: unknown, fallback = "Something went wrong.") {
    const err = e instanceof ApiError ? e : null;
    this.lastError = err?.message ?? String((e as Error)?.message ?? fallback);
    this.screen = { name: "problem", message: this.lastError, code: err?.code };
  }

  async saveSettings() {
    if (!inTauri) return;
    try { await api.settingsSet($state.snapshot(this.settings)); } catch (e) { this.fail(e); }
  }

  async finishOnboarding() {
    this.settings.onboarded = true;
    if (this.settings.biometric_lock) await this.setBiometricLock(true);
    await this.saveSettings();
    this.screen = { name: "home" };
    void this.askPushPermission();
    void this.refresh();
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

  /** Activity pause/resume from the native side; `awayMs` is measured there,
   *  because the webview only wakes up to read these on resume. */
  onLifecycle(state: "paused" | "resumed", awayMs: number) {
    trace(`lifecycle ${state} away=${awayMs} screen=${this.screen.name} unlocking=${this.unlocking} lock=${this.settings.biometric_lock}/${this.settings.lock_on_background} onboarded=${this.settings.onboarded}`);
    if (state === "paused") return;
    if (this.unlocking) return; // the biometric prompt itself pauses the activity
    const away = awayMs;
    const mustLock = this.settings.onboarded && this.settings.biometric_lock && this.settings.lock_on_background && away >= AppState.LOCK_AFTER_MS;
    if (mustLock && this.screen.name !== "lock" && this.screen.name !== "welcome") this.lockNow();
    else if (this.settings.onboarded && this.screen.name !== "lock" && this.screen.name !== "welcome") void this.refresh();
  }

  lockNow() {
    const s = this.screen;
    if (s.name === "lock") return;
    this.returnTo = s.name === "welcome" || s.name === "problem" || s.name === "pair" ? null : s;
    this.lockError = null;
    this.screen = { name: "lock" };
  }

  /** Confirm the person with biometrics or the device credential, then carry on. */
  async unlock() {
    if (this.unlocking) return;
    if (!inTauri || !this.settings.biometric_lock) { this.leaveLock(); return; }
    this.unlocking = true;
    this.lockError = null;
    try {
      const r = await biometricAuth("Confirm it is you");
      if (r.ok) { this.leaveLock(); return; }
      if (r.code === "biometryNotEnrolled" || r.code === "noDeviceCredential" || r.code === "passcodeNotSet") {
        // Nothing on this phone can confirm the person: the lock cannot hold.
        this.settings.biometric_lock = false;
        await this.saveSettings();
        this.lockError = { code: r.code, message: "No screen lock is set on this phone, so the app lock was turned off." };
        return;
      }
      this.lockError = { code: r.code, message: r.message ?? "Not confirmed." };
    } finally {
      this.unlocking = false;
    }
  }

  private leaveLock() {
    this.screen = this.returnTo ?? { name: "home" };
    this.returnTo = null;
    void this.refresh(this.screen.name === "home");
  }

  /** Turning the lock on means confirming once, with biometrics or the screen
   *  lock; a phone with neither cannot hold a lock. Returns a message when it cannot. */
  async setBiometricLock(on: boolean): Promise<string | null> {
    if (on && inTauri) {
      this.unlocking = true; // the prompt pauses the activity; do not treat the return as "came back"
      try {
        const r = await biometricAuth("Confirm to turn the lock on");
        if (!r.ok) {
          this.settings.biometric_lock = false;
          await this.saveSettings();
          if (r.code === "noDeviceCredential" || r.code === "passcodeNotSet" || r.code === "biometryNotEnrolled") return "This phone has no screen lock, so the app cannot lock itself. Set one in the system settings first.";
          return r.message ?? "Not confirmed; the lock stays off.";
        }
      } finally {
        this.unlocking = false;
      }
    }
    this.settings.biometric_lock = on;
    await this.saveSettings();
    return null;
  }

  module(pid: string): ModuleView | undefined { return this.modules.find((m) => m.pid === pid); }
  request(id: string): RequestView | undefined { return this.pending.find((r) => r.id === id); }

  async reloadModules() {
    if (!inTauri) return;
    try { [this.modules, this.archive] = await Promise.all([api.modules(), api.archive()]); } catch (e) { this.fail(e); }
  }

  /** Ask the broker what is waiting. Shows the first new request straight away. */
  async refresh(show = true) {
    if (!inTauri || this.busy) return;
    this.busy = true;
    try {
      const list = await api.refresh();
      this.online = true;
      this.pending = list;
      this.archive = await api.archive();
      this.lastRefresh = await api.lastRefresh().catch(() => null);
      if (show && list.length && this.screen.name !== "request") this.screen = { name: "request", id: list[0].id };
    } catch (e) {
      const err = e as ApiError;
      this.lastRefresh = await api.lastRefresh().catch(() => null);
      if (err.code === "network" || err.code === "timeout" || err.code === "unavailable") this.online = false;
      else this.fail(e);
    } finally {
      this.busy = false;
    }
  }

  /** Demo only (browser): pretend a push arrived for the first module. */
  simulateRequest(kind: Parameters<typeof mock.makeRequest>[0]) {
    const mod = this.modules[0] ?? mock.modules[0];
    const req = mock.makeRequest(kind, mod);
    this.pending = [...this.pending, req];
    this.screen = { name: "request", id: req.id };
  }

  async resolve(id: string, allow: boolean, periodSecs: number, writable: boolean) {
    const req = this.request(id);
    if (!req) return;
    if (!inTauri) {
      const detail = mock.summary(req, periodSecs, writable);
      this.pending = this.pending.filter((r) => r.id !== id);
      this.archive = [{ id: `ar-${id}`, pid: req.pid, title: req.title, detail, outcome: allow ? "granted" : "denied", at: Math.floor(Date.now() / 1000) }, ...this.archive];
      this.screen = { name: "result", outcome: allow ? "granted" : "denied", title: req.title, detail };
      return;
    }
    this.busy = true;
    try {
      const a: AnswerView = allow ? await api.allow(id, periodSecs, writable) : await api.deny(id);
      this.pending = this.pending.filter((r) => r.id !== id);
      this.archive = await api.archive();
      this.modules = await api.modules();
      this.screen = { name: "result", outcome: a.outcome, title: a.title, detail: a.detail };
    } catch (e) {
      this.pending = this.pending.filter((r) => r.id !== id);
      this.fail(e);
    } finally {
      this.busy = false;
    }
  }

  expire(id: string) {
    const req = this.request(id);
    if (!req) return;
    this.pending = this.pending.filter((r) => r.id !== id);
    this.screen = { name: "result", outcome: "expired", title: req.title };
    void this.refresh(false);
  }

  async scanned(raw: string) {
    if (!inTauri) {
      this.screen = { name: "pairConfirm", preview: { link: raw, user: "chris", hostname: "hem-wh.encedo.local", email: "chris@encedo.com", issuer: { city: "Warsaw", country: "PL", ip: "203.0.113.7" }, already_paired: false }, raw };
      return;
    }
    this.busy = true;
    try {
      const preview = await api.pairScan(raw);
      this.screen = { name: "pairConfirm", preview, raw };
    } catch (e) {
      this.fail(e, "The code could not be read.");
    } finally {
      this.busy = false;
    }
  }

  async completePairing(label: string, preview: PairingPreview) {
    if (!inTauri) {
      const pid = crypto.randomUUID();
      this.modules = [...this.modules, { pid, label, host: preview.hostname, user: preview.user, email: preview.email, paired_at: Math.floor(Date.now() / 1000), last_used: null }];
      this.screen = { name: "result", outcome: "paired", title: "Pair this phone", detail: `${label} · ${preview.hostname}` };
      return;
    }
    this.busy = true;
    try {
      const m = await api.pairConfirm(label, this.push.token);
      await this.reloadModules();
      this.screen = { name: "result", outcome: "paired", title: "Pair this phone", detail: `${m.label} · ${m.host}` };
    } catch (e) {
      this.fail(e, "Pairing failed.");
    } finally {
      this.busy = false;
    }
  }

  async refusePairing(preview: PairingPreview) {
    if (inTauri) { try { await api.pairRefuse(this.push.token); } catch { /* best effort */ } await this.reloadModules(); }
    this.screen = { name: "result", outcome: "denied", title: "Pair this phone", detail: preview.hostname };
  }

  async unpair(pid: string) {
    if (!inTauri) { this.modules = this.modules.filter((m) => m.pid !== pid); this.screen = { name: "modules" }; return; }
    this.busy = true;
    try {
      await api.unpair(pid);
      await this.reloadModules();
      this.screen = { name: "modules" };
    } catch (e) {
      this.fail(e, "Unpairing failed.");
    } finally {
      this.busy = false;
    }
  }
}

export const app = new AppState();

export function fmtDate(secs: number): string {
  return new Date(secs * 1000).toLocaleDateString("en-GB", { year: "numeric", month: "short", day: "numeric" });
}
export function fmtDateTime(secs: number): string {
  const d = new Date(secs * 1000);
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
export const PERIODS: { secs: number; label: string }[] = [
  { secs: 900, label: "15 min" }, { secs: 3600, label: "1 h" }, { secs: 28800, label: "8 h" }, { secs: 86400, label: "24 h" },
];
