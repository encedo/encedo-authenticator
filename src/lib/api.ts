// Typed calls into the Rust core. Shapes mirror src-tauri/src/core.rs views.
import { inTauri } from "./native";

export interface ErrorView { code: string; message: string }
/** `aid` is the phone's public key for this module; v1 showed it as the KID. */
export interface ModuleView { pid: string; aid: string; label: string; host: string; user: string; email: string; paired_at: number; last_used: number | null }
export interface IpInfo { city: string; country: string; ip: string }
export interface PairingPreview { link: string; user: string; hostname: string; email: string; issuer: IpInfo | null; already_paired: boolean }
export interface Detail { label: string; value: string; mono: boolean }
export interface RequestView {
  id: string; pid: string; module_label: string; host: string; scope: string; exp: number; issuer: IpInfo | null;
  kind: string; title: string; phrase: string; details: Detail[]; ask_period: boolean; ask_writable: boolean; writable_default: boolean; known: boolean;
}
export type Outcome = "granted" | "denied" | "expired" | "cancelled" | "rejected" | "error" | "paired" | "unpaired";

/** Which part of the app an entry came from. `trace` is the running commentary
 *  and is only ever fetched by name. */
export type Family = "answers" | "modules" | "push" | "broker" | "app" | "trace";
export type Level = "good" | "plain" | "bad";
export interface LogField { label: string; value: string; mono: boolean }
export interface LogEntry {
  id: string; at: number; ms: number; kind: string; family: Family; level: Level;
  pid: string; title: string; summary: string; fields: LogField[]; raw: string | null;
  outcome: Outcome | null;
  /** Folded repeats: how many, and when the first of them was (`at` is the last). */
  repeat: number; first_at: number | null;
  /** Set on answers, pairings and pushes: the seal over the entry before this one. */
  seal: string;
}
/** Whether the sealed chain still holds, and where it starts. */
export interface AuditHealth { entries: number; pruned: number; broken_at: string | null }
export interface AnswerView { outcome: Outcome; title: string; detail: string }
export interface Settings { biometric_lock: boolean; lock_on_background: boolean; theme: "system" | "light" | "dark"; onboarded: boolean }
export interface AppInfo { version: string; platform: string; broker: string }
/** Whether this build may still be used, as Play sees it. */
export type UpdateLevel = "none" | "recommended" | "critical";
export interface UpdateStatus {
  level: UpdateLevel;
  /** The build Play offers, or the one this phone was told to reach. */
  required_version: number;
  current_version: number;
  /** What the publisher set on the release in the Play Console, 0-5. */
  priority: number;
  stale_days: number;
  /** Play can replace this build in place; false for a sideloaded APK. */
  can_update_in_app: boolean;
  checked_at: number;
  note: string | null;
  /** Pretended in a development build; the blocking screen then offers a way out. */
  pretended: boolean;
}

/** Whether the storage is open, and what protects its key on this phone. */
export interface StoreStatus {
  open: boolean;
  /** The key only works within `window_seconds` of a confirmation. */
  bound_to_user: boolean;
  /** This phone has a screen lock, so binding is possible at all. */
  credential: boolean;
  /** The key lives in a separate secure element, not only the TEE. */
  strong_box: boolean;
  window_seconds: number;
}
export interface RefreshReport { at: number; pids: string[]; broker_said: string; pending: number; shown: number; discarded: string[]; error: string | null }

export class ApiError extends Error {
  code: string;
  constructor(e: ErrorView) { super(e.message); this.code = e.code; }
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    const v = e as ErrorView;
    throw v && typeof v === "object" && "code" in v ? new ApiError(v) : new ApiError({ code: "unknown", message: String(e) });
  }
}

export const api = {
  available: inTauri,
  appInfo: () => call<AppInfo>("app_info"),
  updateStatus: () => call<UpdateStatus>("update_status"),
  updateStart: () => call<void>("update_start"),
  /** Development builds only: pretend Play said something. */
  updateSimulate: (level: UpdateLevel) => call<UpdateStatus>("update_simulate", { level }),
  storeStatus: () => call<StoreStatus>("store_status"),
  storeOpen: () => call<StoreStatus>("store_open"),
  storeReset: () => call<StoreStatus>("store_reset"),
  settingsGet: () => call<Settings>("settings_get"),
  settingsSet: (settings: Settings) => call<void>("settings_set", { settings }),
  modules: () => call<ModuleView[]>("modules_list"),
  log: (family?: Family) => call<LogEntry[]>("log_list", { family }),
  logVerify: () => call<AuditHealth>("log_verify"),
  logClearTrace: () => call<void>("log_clear_trace"),
  /** A push arrived or was tapped; only the webview sees the notification text. */
  logPush: (title: string | undefined, body: string | undefined, data: string | undefined, tapped: boolean) => call<void>("log_push", { title, body, data, tapped }),
  /** Lifecycle and lock events. The core refuses anything not on its list. */
  logApp: (kind: string, title: string, summary: string) => call<void>("log_app", { kind, title, summary }),
  /** Going to the background: write out what the commentary has collected. */
  logFlush: () => call<void>("log_flush"),
  pairScan: (raw: string) => call<PairingPreview>("pair_scan", { raw }),
  pairConfirm: (label: string, fid?: string) => call<ModuleView>("pair_confirm", { label, fid }),
  pairRefuse: (fid?: string) => call<void>("pair_refuse", { fid }),
  refresh: () => call<RequestView[]>("requests_refresh"),
  lastRefresh: () => call<RefreshReport>("last_refresh"),
  allow: (id: string, periodSecs: number, writable: boolean) => call<AnswerView>("request_allow", { id, periodSecs, writable }),
  deny: (id: string) => call<AnswerView>("request_deny", { id }),
  unpair: (pid: string) => call<void>("module_unpair", { pid }),
  forget: (pid: string) => call<void>("module_forget", { pid }),
  diagLog: () => call<string[]>("diag_log"),
  diagClear: () => call<void>("diag_clear"),
  pushPayload: (encedo: string) => call<boolean>("push_payload", { encedo }),
  pushTokenChanged: (token: string) => call<void>("push_token_changed", { token }),
};
