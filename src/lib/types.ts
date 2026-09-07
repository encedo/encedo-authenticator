// Display-only data shapes: what Rust hands to the webview. No private keys,
// no protocol bytes.

export interface Module {
  pid: string;
  label: string;
  host: string;
  email: string;
  pairedAt: string; // ISO date
  lastUsed?: string; // ISO datetime
}

export type ScopeKind =
  | "system:config" | "system:upgrade" | "system:shutdown"
  | "storage:disk"
  | "logger:get" | "logger:del"
  | "keymgmt:use" | "keymgmt:del" | "keymgmt:list" | "keymgmt:get"
  | "keymgmt:gen" | "keymgmt:upd" | "keymgmt:imp" | "keymgmt:derive"
  | "auth:ext:pair";

export interface AccessRequest {
  id: string;
  pid: string;
  kind: ScopeKind;
  scope: string; // the literal scope string, shown as is
  phrase: string; // "use key PGP main" — completes "<host> wants to …"
  title: string; // short name for the archive: "Use the key"
  details: { label: string; value: string; mono?: boolean }[];
  host: string;
  expiresAt: number; // epoch ms
  askPeriod: boolean;
  askWritable: boolean;
  writableDefault: boolean;
}

export type Outcome = "granted" | "denied" | "expired" | "cancelled" | "error";

export interface ArchiveEntry {
  id: string;
  pid: string;
  title: string;
  detail: string;
  outcome: Outcome;
  at: string; // ISO datetime
}

export type Period = 15 | 60 | 480 | 1440;

export interface Settings {
  biometricLock: boolean;
  lockOnBackground: boolean;
  theme: "system" | "light" | "dark";
  onboarded: boolean;
}
