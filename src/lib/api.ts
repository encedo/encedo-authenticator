// Typed calls into the Rust core. Shapes mirror src-tauri/src/core.rs views.
import { inTauri } from "./native";

export interface ErrorView { code: string; message: string }
export interface ModuleView { pid: string; label: string; host: string; user: string; email: string; paired_at: number; last_used: number | null }
export interface IpInfo { city: string; country: string; ip: string }
export interface PairingPreview { link: string; user: string; hostname: string; email: string; issuer: IpInfo | null; already_paired: boolean }
export interface Detail { label: string; value: string; mono: boolean }
export interface RequestView {
  id: string; pid: string; module_label: string; host: string; scope: string; exp: number; issuer: IpInfo | null;
  kind: string; title: string; phrase: string; details: Detail[]; ask_period: boolean; ask_writable: boolean; writable_default: boolean; known: boolean;
}
export type Outcome = "granted" | "denied" | "expired" | "cancelled" | "rejected" | "error" | "paired" | "unpaired";
export interface ArchiveEntry { id: string; pid: string; title: string; detail: string; outcome: Outcome; at: number }
export interface AnswerView { outcome: Outcome; title: string; detail: string }
export interface Settings { biometric_lock: boolean; lock_on_background: boolean; theme: "system" | "light" | "dark"; onboarded: boolean }
export interface AppInfo { version: string; platform: string; broker: string }
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
  settingsGet: () => call<Settings>("settings_get"),
  settingsSet: (settings: Settings) => call<void>("settings_set", { settings }),
  modules: () => call<ModuleView[]>("modules_list"),
  archive: () => call<ArchiveEntry[]>("archive_list"),
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
