import type { Settings } from "./api";
import { api, ApiError, type AnswerView, type AuditHealth, type Family, type LogEntry, type ModuleView, type Outcome, type PairingPreview, type RefreshReport, type RequestView, type StoreStatus, type UpdateStatus } from "./api";
import { biometricAuth, biometricStatus, inTauri, trace, requestPushPermission, setPrivacy, startPush, uncover, type PushMessage, type PushState } from "./native";
import * as mock from "./mock";
import { listen } from "@tauri-apps/api/event";
import { tick } from "svelte";

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
  const fallback: Settings = { biometric_lock: true, lock_on_background: true, theme: "system", onboarded: false, confirm_each_allow: false };
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
  /** A pairing link that arrived while another pairing was on screen: the
   *  person decides whether it replaces that one. */
  incomingLink = $state<string | null>(null);
  /** DEV-512: why the last Allow was not sent ("confirm every Allow" refused). */
  allowNote = $state<string | null>(null);
  private returnTo: Screen | null = null;
  private static readonly LOCK_AFTER_MS = 5000;
  /** DEV-512: this long untouched in front locks the app like going away does. */
  private static readonly IDLE_LOCK_MS = 5 * 60 * 1000;
  private lastActivity = Date.now();

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
    // DEV-512: a touch or a key keeps the app open; five minutes without one locks it.
    const touched = () => { this.lastActivity = Date.now(); };
    for (const ev of ["pointerdown", "keydown", "touchstart"]) document.addEventListener(ev, touched, { passive: true });
    setInterval(() => this.lockIfIdle(), 15_000);
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
    // The Manager open on this phone hands its pairing code over by link.
    void listen("pair-link", () => void this.checkPairLink()).catch(() => {});
    void this.checkPairLink();
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
    this.syncPrivacy();
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

  /** The lock can be on while the key is not bound to the person: a store made
   *  before the lock was turned on, or one carried across an upgrade. Nothing
   *  fixes that by itself, and the confirmation that just happened is exactly
   *  what binding needs, so it is done here. */
  private async bindKeyIfNeeded() {
    if (!inTauri) return;
    const s = this.storeStatus;
    if (!s?.open || s.bound_to_user || !s.credential || !this.settings.biometric_lock) return;
    try {
      await api.settingsSet($state.snapshot(this.settings));
      this.storeStatus = await api.storeStatus();
    } catch {
      // The next confirmation can try again; nothing is lost by not shouting.
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
    this.syncPrivacy();
    try { await api.settingsSet($state.snapshot(this.settings)); } catch (e) { this.fail(e); }
  }

  /** Development builds only: lift FLAG_SECURE for test screenshots. Kept in
   *  this webview's storage; a release build ignores it natively as well. */
  allowCapture = $state(AppState.readAllowCapture());
  private static readonly CAPTURE_KEY = "encedo.dev.allowCapture";
  private static readAllowCapture(): boolean {
    try { return localStorage.getItem(AppState.CAPTURE_KEY) === "1"; } catch { return false; }
  }
  setAllowCapture(on: boolean) {
    this.allowCapture = on;
    try { localStorage.setItem(AppState.CAPTURE_KEY, on ? "1" : "0"); } catch {}
    this.syncPrivacy();
  }

  /** Tell the native side whether to cover the app while it is away: only when
   *  coming back after LOCK_AFTER_MS would lock it. */
  private syncPrivacy() {
    const cover = this.settings.onboarded && this.settings.biometric_lock && this.settings.lock_on_background;
    void setPrivacy(cover, AppState.LOCK_AFTER_MS, this.allowCapture);
  }

  async finishOnboarding() {
    this.settings.onboarded = true;
    if (this.settings.biometric_lock) await this.setBiometricLock(true);
    await this.saveSettings();
    this.screen = { name: "home" };
    void this.askPushPermission();
    void this.refresh();
    void this.checkPairLink();
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
    // The native cover stays up until the screen decided on here is drawn.
    void tick().then(() => requestAnimationFrame(() => requestAnimationFrame(() => void uncover())));
    if (this.unlocking) return; // the biometric prompt itself pauses the activity
    const away = awayMs;
    const mustLock = this.settings.onboarded && this.settings.biometric_lock && this.settings.lock_on_background && away >= AppState.LOCK_AFTER_MS;
    if (mustLock && this.screen.name !== "lock" && this.screen.name !== "welcome") this.lockNow();
    else if (this.settings.onboarded && this.screen.name !== "lock" && this.screen.name !== "welcome") void this.refresh();
  }

  /** DEV-512: left untouched in front for five minutes, the app locks as if the
   *  person had walked away. Same switches as locking in the background. */
  private lockIfIdle() {
    if (!this.settings.onboarded || !this.settings.biometric_lock || !this.settings.lock_on_background || this.unlocking) return;
    if (this.screen.name === "lock" || this.screen.name === "welcome") return;
    if (Date.now() - this.lastActivity < AppState.IDLE_LOCK_MS) return;
    trace("idle lock");
    this.lockNow();
  }

  /** Ask the system who is there. On success the core is told, and the login
   *  context of the next answer is dated from now (DEV-512). */
  private async confirmPerson(reason: string, biometricOnly = false) {
    const r = await biometricAuth(reason, biometricOnly);
    if (r.ok && inTauri) {
      const bio = await biometricStatus();
      await api.userVerified(bio.available).catch(() => {});
    }
    return r;
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
      const r = await this.confirmPerson("Confirm it is you");
      if (r.ok) {
        if (closed && !(await this.openStore())) return;
        void this.logApp("app.unlocked", "Unlocked", closed ? "storage opened with your confirmation" : "confirmed on this phone");
        void this.bindKeyIfNeeded();
        this.leaveLock();
        return;
      }
      void this.logApp("app.lock_failed", "Not unlocked", r.message ?? r.code ?? "not confirmed");
      if (r.code === "biometryNotEnrolled" || r.code === "noDeviceCredential" || r.code === "passcodeNotSet") {
        // Nothing on this phone can confirm the person: the lock cannot hold.
        // DEV-512: said plainly, and the person has to acknowledge it on the Lock screen.
        this.settings.biometric_lock = false;
        await this.saveSettings();
        this.lockError = {
          code: r.code,
          message: "This phone has no screen lock, so the app cannot ask who you are and its lock is off. Anyone holding the phone can approve requests, and every answer tells the module nobody was verified. Set a screen lock in the system settings, then turn the app lock back on.",
        };
        return;
      }
      this.lockError = { code: r.code, message: r.message ?? "Not confirmed." };
    } finally {
      this.unlocking = false;
      // leaveLock ran while `unlocking` was still set, so its own check of a
      // waiting pairing link was turned away; ask again now it is cleared.
      if (this.screen.name !== "lock") void this.checkPairLink();
    }
  }

  private leaveLock() {
    this.lastActivity = Date.now();
    this.screen = this.returnTo ?? { name: "home" };
    this.returnTo = null;
    void this.refresh(this.screen.name === "home");
    void this.checkPairLink();
  }

  /** The lock is both the screen lock and the protection of the storage key, so
   *  either direction needs one confirmation: turning it on binds the key to the
   *  person, turning it off unbinds it. Returns a message when it cannot. */
  async setBiometricLock(on: boolean): Promise<string | null> {
    const revert = () => { this.settings.biometric_lock = !on; };
    if (inTauri) {
      this.unlocking = true; // the prompt pauses the activity; do not treat the return as "came back"
      try {
        const r = await this.confirmPerson(on ? "Confirm to turn the lock on" : "Confirm to turn the lock off");
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
    this.allowNote = null;
    // DEV-512 / DEV-511: a confirmation for this Allow, asked before anything
    // is sent, when the requester demands one (`uv` in `ctx`) or "confirm every
    // Allow" is on. Biometrics only when it is demanded, or when the person
    // chose to confirm every Allow on a phone that has them. Refused, the
    // request stays on screen and nothing leaves the phone; the core checks the
    // demand again before it answers.
    if (allow) {
      const wanted = req.uv ?? null;
      const confirmEach = this.settings.biometric_lock && this.settings.confirm_each_allow;
      let method: "bio" | "pin" | null = null;
      if (wanted || confirmEach) {
        const bio = await biometricStatus();
        if (wanted === "bio" && !bio.available) {
          this.allowNote = "This request needs your fingerprint or face, and none is set up on this phone, so it cannot be allowed here. Deny it, or set up biometrics and ask again.";
          return;
        }
        method = wanted === "bio" || (confirmEach && bio.available) ? "bio" : "pin";
      }
      if (method) {
        this.unlocking = true; // the prompt pauses the activity; do not treat the return as "came back"
        try {
          const r = await this.confirmPerson(method === "bio" ? "Confirm to allow with your fingerprint or face" : "Confirm to allow", method === "bio");
          if (!r.ok) { this.allowNote = r.message ?? "Not confirmed, so nothing was sent."; return; }
          await api.approvalVerified(id, method);
        } finally {
          this.unlocking = false;
        }
      }
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
      // DEV-511: the demand of `ctx` was not met; nothing was sent, the request stays.
      if (e instanceof ApiError && e.code === "verification") { this.allowNote = e.message; return; }
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

  /** Take a waiting `encedo-auth://pair` link once a pairing can be shown: the
   *  storage is open and the person is past the Welcome and Lock screens. The
   *  link stays in the core until then. A pairing already on screen is not
   *  replaced without asking. */
  async checkPairLink() {
    if (!inTauri || !this.ready || !this.storeStatus?.open || !this.settings.onboarded) return;
    if (this.screen.name === "lock" || this.screen.name === "welcome" || this.unlocking) return;
    const link = await api.pairLinkTake().catch(() => null);
    if (!link) return;
    trace("pair link opened");
    if (this.screen.name === "pairConfirm") { this.incomingLink = link; return; }
    await this.scanned(link);
  }

  /** The person chose the link over the pairing on screen. */
  async acceptIncomingLink() {
    const link = this.incomingLink;
    this.incomingLink = null;
    if (link) await this.scanned(link);
  }

  dismissIncomingLink() { this.incomingLink = null; }

  /** "Paste code": the core reads the clipboard now, and only now. An empty
   *  clipboard is said on the Pair screen (the message is returned); a code
   *  that fails goes the way a scanned one does. */
  async pasteCode(): Promise<string | null> {
    if (!inTauri) {
      const text = (await navigator.clipboard?.readText().catch(() => "")) ?? "";
      if (!text.trim()) return "The clipboard is empty.";
      await this.scanned(text.trim());
      return null;
    }
    this.busy = true;
    try {
      const preview = await api.pairPaste();
      this.incomingLink = null;
      this.screen = { name: "pairConfirm", preview, raw: "" };
      return null;
    } catch (e) {
      if (e instanceof ApiError && e.code === "clipboard") return e.message;
      this.fail(e, "The code could not be read.");
      return null;
    } finally {
      this.busy = false;
    }
  }

  async scanned(raw: string) {
    if (!inTauri) {
      this.screen = { name: "pairConfirm", preview: { link: raw, user: "chris", hostname: "hem-wh.encedo.local", email: "chris@encedo.com", issuer: { city: "Warsaw", country: "PL", ip: "203.0.113.7" }, already_paired: false }, raw };
      return;
    }
    this.busy = true;
    try {
      const preview = await api.pairScan(raw);
      this.incomingLink = null;
      this.screen = { name: "pairConfirm", preview, raw };
    } catch (e) {
      this.fail(e, "The code could not be read.");
    } finally {
      this.busy = false;
    }
  }

  async completePairing(label: string, preview: PairingPreview) {
    this.incomingLink = null;
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
    this.incomingLink = null;
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
