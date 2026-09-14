import type { Settings } from "./api";
import { api, ApiError, type AnswerView, type AuditHealth, type Family, type LogEntry, type ModuleView, type Outcome, type PairingPreview, type RefreshReport, type RequestView, type StoreStatus, type UpdateStatus } from "./api";
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
  | { name: "history"; pid?: string }
  | { name: "settings" }
  | { name: "about" }
  | { name: "licences" }
  | { name: "problem"; message: string; code?: string };

export type Tab = "home" | "modules" | "history" | "settings";

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
  /** The journal, newest first: everything but the running commentary. */
  log = $state<LogEntry[]>(inTauri ? [] : mock.journal);
  /** The commentary, fetched only when the History screen asks for it. */
  traceLog = $state<LogEntry[]>([]);
  health = $state<AuditHealth | null>(null);
  /** Whether the storage is open, and what protects its key. */
  storeStatus = $state<StoreStatus | null>(null);
  /** Whether this build may still be used. */
  update = $state<UpdateStatus | null>(null);
  /** "Later" on the update screen: this launch carries on with this build. */
  updateDeferred = $state(false);
  online = $state(true);
  busy = $state(false);
  lastError = $state<string | null>(null);
  lastRefresh = $state<RefreshReport | null>(null);
  push = $state<PushState>({ status: inTauri ? "pending" : "unavailable" });
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
      this.storeStatus = await api.storeStatus();
      if (this.storeStatus.open) {
        await this.loadStore();
        this.screen = this.firstScreen();
      } else {
        // The storage key is bound to the person, so nothing can be read until
        // the Locked screen has a confirmation.
        this.screen = { name: "lock" };
      }
    } catch (e) {
      const err = e as ApiError;
      this.screen = { name: "problem", message: err.message ?? `Storage could not be opened: ${String(e)}`, code: err.code ?? "storage" };
    } finally {
      this.ready = true;
    }
    void this.checkUpdate();
    this.push = await startPush({
      onToken: (token) => { this.push = { ...this.push, status: "registered", token }; void api.pushTokenChanged(token).catch(() => {}); },
      onMessage: (m) => {
        // The payload is evidence of what the module asked for, so it goes into
        // the journal before anything is done about it; the History screen is
        // where it is read back.
        void api.logPush(m.title, m.body, Object.keys(m.data).length ? JSON.stringify(m.data) : undefined, !!m.tapped).catch(() => {});
        void this.onPush(m);
      },
      onLifecycle: (state, awayMs) => this.onLifecycle(state, awayMs),
    });
    if (this.push.token && this.storeStatus?.open) void api.pushTokenChanged(this.push.token).catch(() => {});
    if (this.storeStatus?.open && this.settings.onboarded && !this.settings.biometric_lock) void this.refresh();
  }

  /** Whether an update screen is in front of everything else. */
  get updateShowing(): boolean {
    if (this.update?.level === "critical") return true;
    return this.update?.level === "recommended" && !this.updateDeferred && this.screen.name !== "welcome";
  }

  /** Carry on with this build; the screen returns at the next launch. */
  deferUpdate() {
    this.updateDeferred = true;
  }

  /** What Play has to say about this build. Cheap, and the answer is folded
   *  into what the phone already knew, so cutting the network changes nothing. */
  async checkUpdate() {
    if (!inTauri) return;
    this.update = await api.updateStatus().catch(() => this.update);
  }

  /** Hand the person to Play. */
  async startUpdate() {
    if (!inTauri) return;
    this.busy = true;
    try {
      await api.updateStart();
    } catch (e) {
      this.lastError = (e as ApiError).message ?? "Play could not be opened.";
    } finally {
      this.busy = false;
    }
  }

  /** Development builds only: see the screens without publishing anything. */
  async simulateUpdate(level: "none" | "recommended" | "critical") {
    if (!inTauri) return;
    // Both verdicts take over the screen, so a new pretence starts undeferred.
    this.updateDeferred = false;
    this.update = await api.updateSimulate(level).catch(() => this.update);
  }

  /** Everything the screens read, once the storage is open. */
  private async loadStore() {
    const [settings, modules, log] = await Promise.all([api.settingsGet(), api.modules(), api.log()]);
    this.settings = settings;
    this.modules = modules;
    this.log = log;
    void this.loadHealth();
  }

  private firstScreen(): Screen {
    if (!this.settings.onboarded) return { name: "welcome" };
    return this.settings.biometric_lock ? { name: "lock" } : { name: "home" };
  }

  /** Open the storage with a confirmation that has just been given. */
  private async openStore(): Promise<boolean> {
    try {
      this.storeStatus = await api.storeOpen();
      await this.loadStore();
      if (this.push.token) void api.pushTokenChanged(this.push.token).catch(() => {});
      return true;
    } catch (e) {
      const err = e as ApiError;
      if (err.code === "key_lost") {
        this.screen = { name: "problem", message: err.message, code: "key_lost" };
      } else if (err.code === "auth_required") {
        this.lockError = { code: err.code, message: "That confirmation is too old now. Confirm again." };
      } else {
        this.fail(e);
      }
      return false;
    }
  }

  /** The key is gone: throw the unreadable store away and start over. */
  async resetStorage() {
    if (!inTauri) return;
    this.busy = true;
    try {
      this.storeStatus = await api.storeReset();
      await this.loadStore();
      this.screen = this.firstScreen();
    } catch (e) {
      this.fail(e, "The storage could not be started over.");
    } finally {
      this.busy = false;
    }
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
      case "history": return "history";
      case "settings": case "about": case "licences": return "settings";
      default: return null;
    }
  }

  go(screen: Screen) { this.screen = screen; }
  goTab(tab: Tab) { this.screen = { name: tab }; }

  fail(e: unknown, fallback = "Something went wrong.") {
    const err = e instanceof ApiError ? e : null;
    // The storage is shut, not broken: a push woke the app while the phone was
    // locked, or the confirmation timed out. Ask again instead of crying failure.
    if (err?.code === "locked") {
      void api.storeStatus().then((s) => (this.storeStatus = s)).catch(() => {});
      this.lockNow();
      return;
    }
    this.lastError = err?.message ?? String((e as Error)?.message ?? fallback);
    void this.logApp("app.failed", "Something went wrong", `${err?.code ?? "unknown"}: ${this.lastError}`);
    this.screen = { name: "problem", message: this.lastError, code: err?.code };
  }

  // ---- the journal -------------------------------------------------------

  async loadLog() {
    if (!inTauri) return;
    try { this.log = await api.log(); } catch (e) { this.fail(e); }
  }

  async loadTrace() {
    if (!inTauri) return;
    try { this.traceLog = await api.log("trace" as Family); } catch { this.traceLog = []; }
  }

  async loadHealth() {
    if (!inTauri) return;
    this.health = await api.logVerify().catch(() => null);
  }

  async clearTrace() {
    if (!inTauri) { this.traceLog = []; return; }
    try { await api.logClearTrace(); } catch { /* nothing to clear */ }
    await this.loadTrace();
  }

  /** Lifecycle, lock and failure events; the core refuses any other kind. */
  private async logApp(kind: string, title: string, summary: string) {
    if (!inTauri) return;
    await api.logApp(kind, title, summary).catch(() => {});
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
    if (state === "paused") {
      if (!this.unlocking) {
        void this.logApp("app.background", "App left the screen", "");
        void api.logFlush().catch(() => {});
      }
      return;
    }
    if (!this.unlocking) {
      void this.logApp("app.foreground", "App came back", awayMs ? `away ${Math.round(awayMs / 1000)} s` : "");
      void this.checkUpdate();
    }
    if (this.unlocking) return; // the biometric prompt itself pauses the activity
    const away = awayMs;
    const mustLock = this.settings.onboarded && this.settings.biometric_lock && this.settings.lock_on_background && away >= AppState.LOCK_AFTER_MS;
    if (mustLock && this.screen.name !== "lock" && this.screen.name !== "welcome") this.lockNow();
    else if (this.settings.onboarded && this.screen.name !== "lock" && this.screen.name !== "welcome") void this.refresh();
  }

  lockNow() {
    const s = this.screen;
    if (s.name === "lock") return;
    void this.logApp("app.locked", "Locked", "waiting for you to confirm");
    this.returnTo = s.name === "welcome" || s.name === "problem" || s.name === "pair" ? null : s;
    this.lockError = null;
    this.screen = { name: "lock" };
  }

  /** Confirm the person with biometrics or the device credential, then carry on. */
  async unlock() {
    if (this.unlocking) return;
    // A closed storage has to be opened whatever the lock setting says: its key
    // is bound to the person.
    const closed = inTauri && this.storeStatus?.open === false;
    if (!inTauri || (!this.settings.biometric_lock && !closed)) { this.leaveLock(); return; }
    this.unlocking = true;
    this.lockError = null;
    try {
      const r = await biometricAuth("Confirm it is you");
      if (r.ok) {
        if (closed && !(await this.openStore())) return;
        void this.logApp("app.unlocked", "Unlocked", closed ? "storage opened with your confirmation" : "confirmed on this phone");
        this.leaveLock();
        return;
      }
      void this.logApp("app.lock_failed", "Not unlocked", r.message ?? r.code ?? "not confirmed");
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

  /** The lock is both the screen lock and the protection of the storage key, so
   *  either direction needs one confirmation: turning it on binds the key to the
   *  person, turning it off unbinds it. Returns a message when it cannot. */
  async setBiometricLock(on: boolean): Promise<string | null> {
    const revert = () => { this.settings.biometric_lock = !on; };
    if (inTauri) {
      this.unlocking = true; // the prompt pauses the activity; do not treat the return as "came back"
      try {
        const r = await biometricAuth(on ? "Confirm to turn the lock on" : "Confirm to turn the lock off");
        if (!r.ok) {
          revert();
          if (r.code === "noDeviceCredential" || r.code === "passcodeNotSet" || r.code === "biometryNotEnrolled") {
            return "This phone has no screen lock, so the app cannot lock itself and cannot bind the storage key to you. Set one in the system settings first.";
          }
          return r.message ?? `Not confirmed; the lock stays ${on ? "off" : "on"}.`;
        }
      } finally {
        this.unlocking = false;
      }
    }
    this.settings.biometric_lock = on;
    if (!inTauri) return null;
    try {
      await api.settingsSet($state.snapshot(this.settings));
      this.storeStatus = await api.storeStatus();
      return null;
    } catch (e) {
      revert();
      const err = e as ApiError;
      if (err.code === "no_credential") return "This phone has no screen lock, so the storage key cannot be bound to you.";
      if (err.code === "auth_required") return "That confirmation is too old now. Try again.";
      return err.message ?? "The setting could not be saved.";
    }
  }

  module(pid: string): ModuleView | undefined { return this.modules.find((m) => m.pid === pid); }
  request(id: string): RequestView | undefined { return this.pending.find((r) => r.id === id); }

  async reloadModules() {
    if (!inTauri) return;
    try {
      [this.modules, this.log] = await Promise.all([api.modules(), api.log()]);
      void this.loadHealth();
    } catch (e) {
      this.fail(e);
    }
  }

  /** Ask the broker what is waiting. Shows the first new request straight away. */
  async refresh(show = true) {
    if (!inTauri || this.busy) return;
    this.busy = true;
    try {
      const list = await api.refresh();
      this.online = true;
      this.pending = list;
      this.log = await api.log();
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
      this.log = [mock.entry(allow ? "request.granted" : "request.denied", req.title, detail, req.pid, allow ? "granted" : "denied"), ...this.log];
      this.screen = { name: "result", outcome: allow ? "granted" : "denied", title: req.title, detail };
      return;
    }
    this.busy = true;
    try {
      const a: AnswerView = allow ? await api.allow(id, periodSecs, writable) : await api.deny(id);
      this.pending = this.pending.filter((r) => r.id !== id);
      this.log = await api.log();
      this.modules = await api.modules();
      void this.loadHealth();
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
      this.modules = [...this.modules, { pid, aid: "mock-key-not-a-real-aid", label, host: preview.hostname, user: preview.user, email: preview.email, paired_at: Math.floor(Date.now() / 1000), last_used: null }];
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

  /** Unpair at the broker and locally. Returns the broker's objection, if any, so the screen can offer a local-only removal. */
  async unpair(pid: string): Promise<string | null> {
    if (!inTauri) { this.modules = this.modules.filter((m) => m.pid !== pid); this.screen = { name: "modules" }; return null; }
    this.busy = true;
    try {
      await api.unpair(pid);
      await this.reloadModules();
      this.screen = { name: "modules" };
      return null;
    } catch (e) {
      return (e as Error).message ?? String(e);
    } finally {
      this.busy = false;
    }
  }

  /** Drop the module from this phone only. */
  async forget(pid: string) {
    if (!inTauri) { this.modules = this.modules.filter((m) => m.pid !== pid); this.screen = { name: "modules" }; return; }
    this.busy = true;
    try {
      await api.forget(pid);
      await this.reloadModules();
      this.screen = { name: "modules" };
    } catch (e) {
      this.fail(e, "Could not remove the module.");
    } finally {
      this.busy = false;
    }
  }
}

export const app = new AppState();

/** Just the clock, for a row in the journal. */
export function fmtTime(secs: number): string {
  return new Date(secs * 1000).toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" });
}

/** The heading above a run of entries: today, yesterday, or the date. */
export function dayLabel(secs: number): string {
  const d = new Date(secs * 1000);
  const midnight = new Date();
  midnight.setHours(0, 0, 0, 0);
  const days = Math.floor((midnight.getTime() - d.getTime()) / 86_400_000);
  if (days < 0) return "Today";
  if (days < 1) return "Yesterday";
  return d.toLocaleDateString("en-GB", { day: "numeric", month: "short", year: d.getFullYear() === new Date().getFullYear() ? undefined : "numeric" });
}

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
