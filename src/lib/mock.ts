import type { ArchiveEntry, ModuleView, RequestView } from "./api";

// Browser-only demo data, shaped exactly like the Rust views, so the mockup
// keeps working in a tab (and in the preview artifact) without a core.

const now = Math.floor(Date.now() / 1000);
const day = 86400;

export const modules: ModuleView[] = [
  { pid: "3f9a7d1c4f2b4e8a9c610b5f2e7ac21e", label: "Office PPA", host: "my.ence.do", user: "chris", email: "chris@encedo.com", paired_at: now - 87 * day, last_used: now - 3 * 3600 },
  { pid: "7b02f6c49a7e4d3c8e553f9d1a6c9d4f", label: "Lab bench EPA", host: "192.168.7.1", user: "chris", email: "chris@encedo.com", paired_at: now - 6 * day, last_used: now - day },
];

export const archive: ArchiveEntry[] = [
  { id: "a1", pid: modules[0].pid, title: "Use the key", detail: "keymgmt:use:2ba3…c91e · 1 h", outcome: "granted", at: now - 3 * 3600 },
  { id: "a2", pid: modules[1].pid, title: "Unlock a drive", detail: "storage:disk0:rw · 1 h", outcome: "granted", at: now - day },
  { id: "a3", pid: modules[0].pid, title: "Delete the log", detail: "logger:del", outcome: "denied", at: now - 2 * day },
  { id: "a4", pid: modules[0].pid, title: "Use the key", detail: "SSH deploy", outcome: "expired", at: now - 3 * day },
  { id: "a5", pid: modules[1].pid, title: "Update the software", detail: "", outcome: "cancelled", at: now - 5 * day },
  { id: "a6", pid: modules[0].pid, title: "Pair this phone", detail: "Lab bench EPA · 192.168.7.1", outcome: "paired", at: now - 6 * day },
];

export type DemoKind = "keymgmt:use" | "storage:disk" | "system:upgrade" | "logger:del" | "auth:ext:pair";
export const demoScopes: DemoKind[] = ["keymgmt:use", "storage:disk", "system:upgrade", "logger:del", "auth:ext:pair"];

const catalogue: Record<DemoKind, { title: string; phrase: string; scope: string; details: RequestView["details"]; ask_writable?: boolean; writable_default?: boolean }> = {
  "keymgmt:use": { title: "Use the key", phrase: "use key PGP main", scope: "keymgmt:use:2ba3f1e0c9a4b7d8e6f5a3c2b1d0e9f84c91e#eyJ0IjoiQUMxNiIsImwiOiJQR1AgbWFpbiJ9", details: [{ label: "Key id", value: "2ba3f1e0c9a4b7d8e6f5a3c2b1d0e9f84c91e", mono: true }, { label: "Type", value: "PKEY,ECDH,ExDSA,ED25519", mono: false }, { label: "Label", value: "PGP main", mono: false }] },
  "storage:disk": { title: "Unlock a drive", phrase: "unlock disk0", scope: "storage:disk0:rw", details: [{ label: "Drive", value: "disk0", mono: true }], ask_writable: true, writable_default: true },
  "system:upgrade": { title: "Update the software", phrase: "install a software update", scope: "system:upgrade", details: [] },
  "logger:del": { title: "Delete the log", phrase: "delete the log", scope: "logger:del", details: [] },
  "auth:ext:pair": { title: "Pair a phone", phrase: "pair another phone", scope: "auth:ext:pair", details: [] },
};

let seq = 0;
export function makeRequest(kind: DemoKind, mod: ModuleView): RequestView {
  seq += 1;
  const c = catalogue[kind];
  return {
    id: `req-${seq}`, pid: mod.pid, module_label: mod.label, host: mod.host, scope: c.scope, exp: Math.floor(Date.now() / 1000) + 90,
    issuer: { city: "Warsaw", country: "PL", ip: "203.0.113.7" },
    kind: kind.split(":")[0], title: c.title, phrase: c.phrase, details: c.details, ask_period: true, ask_writable: !!c.ask_writable, writable_default: !!c.writable_default, known: true,
  };
}

export function scopeTitle(kind: DemoKind): string { return catalogue[kind].title; }

export function summary(req: RequestView, periodSecs: number, writable: boolean): string {
  const base = req.scope.replace(":rw", "") + (req.ask_writable && writable ? ":rw" : "");
  const label = { 900: "15 min", 3600: "1 h", 28800: "8 h", 86400: "24 h" }[periodSecs] ?? `${periodSecs / 60} min`;
  return `${base} · ${label}`;
}
