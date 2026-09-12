import type { LogEntry, Level, ModuleView, Outcome, RequestView } from "./api";

// Browser-only demo data, shaped exactly like the Rust views, so the mockup
// keeps working in a tab (and in the preview artifact) without a core.

const now = Math.floor(Date.now() / 1000);
const day = 86400;

export const modules: ModuleView[] = [
  { pid: "3f9a7d1c4f2b4e8a9c610b5f2e7ac21e", aid: "W8iz6Wcb05LNq7PXDiAw7M1sgcaVpFppNFDbbJDD8SQ=", label: "Office PPA", host: "my.ence.do", user: "chris", email: "chris@encedo.com", paired_at: now - 87 * day, last_used: now - 3 * 3600 },
  { pid: "7b02f6c49a7e4d3c8e553f9d1a6c9d4f", aid: "wya5aqIoQ+TVSBL1UytcqGio9BTxUUMH6wk8lsUep0A=", label: "Lab bench EPA", host: "192.168.7.1", user: "chris", email: "chris@encedo.com", paired_at: now - 6 * day, last_used: now - day },
];

let mockId = 0;

/** One journal entry, shaped like the Rust one. */
export function entry(kind: string, title: string, summary: string, pid = "", outcome?: Outcome, at = Math.floor(Date.now() / 1000), extras: Partial<LogEntry> = {}): LogEntry {
  mockId += 1;
  const family = kind.startsWith("request.") ? "answers" : kind.startsWith("pair") || kind.startsWith("unpair") ? "modules" : kind.startsWith("push.") ? "push" : kind.startsWith("broker.") ? "broker" : kind === "app.trace" ? "trace" : "app";
  const level: Level = outcome === "granted" || outcome === "paired" || kind === "app.unlocked" ? "good" : outcome === "denied" || outcome === "rejected" || outcome === "error" || kind.endsWith(".error") || kind.endsWith(".failed") ? "bad" : "plain";
  return {
    id: `mock-${mockId}`, at, ms: 0, kind, family, level, pid, title, summary,
    fields: [], raw: null, outcome: outcome ?? null, repeat: 1, first_at: null,
    seal: family === "answers" || family === "modules" || family === "push" ? "mock-seal" : "",
    ...extras,
  };
}

// Browser demo: a journal with every family in it, so the History screen has
// something to sort and expand without a core behind it.
export const journal: LogEntry[] = [
  entry("request.granted", "Use the key", "keymgmt:use:2ba3…c91e · 1 h", modules[0].pid, "granted", now - 3 * 3600, {
    fields: [
      { label: "Granted scope", value: "keymgmt:use:2ba3f1e0c9a4b7d8e6f5a3c2b1d0e9f84c91e", mono: true },
      { label: "For", value: "1 h", mono: false },
      { label: "Writing", value: "not allowed", mono: false },
      { label: "Asked from", value: "Warsaw, PL · 203.0.113.7", mono: false },
      { label: "Broker said", value: "accepted", mono: false },
    ],
  }),
  entry("request.shown", "Request shown", "keymgmt:use:2ba3…c91e · expires 18:22:04", modules[0].pid, undefined, now - 3 * 3600 - 9),
  entry("push.received", "Push received", "woke the app", modules[0].pid, undefined, now - 3 * 3600 - 11, {
    fields: [{ label: "Kind", value: "data only, no notification text", mono: false }],
    raw: '{"encedo":{"event":{"id":"g0O7U","pid":["3f9a…c21e"]}}}',
  }),
  entry("broker.checked", "Broker checked", "nothing waiting", "", undefined, now - 4 * 3600, {
    repeat: 12, first_at: now - 5 * 3600,
    fields: [{ label: "Waiting / shown", value: "0 / 0", mono: false }],
    raw: '{"eventid":[]}',
  }),
  entry("request.granted", "Unlock a drive", "storage:disk0:rw · 1 h", modules[1].pid, "granted", now - day),
  entry("app.unlocked", "Unlocked", "confirmed on this phone", "", undefined, now - day - 60),
  entry("request.denied", "Delete the log", "logger:del", modules[0].pid, "denied", now - 2 * day),
  entry("request.rejected", "Request refused by this phone", "the scope did not match its MAC", modules[0].pid, "rejected", now - 2 * day - 1800, {
    fields: [{ label: "Why", value: "The scope and its MAC disagree, so the phone never showed the request and never answered it.", mono: false }],
  }),
  entry("request.expired", "Use the key", "SSH deploy · expired", modules[0].pid, "expired", now - 3 * day),
  entry("push.token_changed", "Push token changed", "Office PPA: accepted", "", undefined, now - 4 * day, {
    fields: [{ label: "New token", value: "fMEp9c…1bQ8", mono: true }],
  }),
  entry("broker.error", "Broker could not be asked", "the broker did not answer in time", "", undefined, now - 5 * day, {
    fields: [{ label: "Endpoint", value: "POST /notify/event/data/allbypid", mono: true }, { label: "Code", value: "timeout", mono: true }],
  }),
  entry("request.cancelled", "Update the software", "withdrawn on the module", modules[1].pid, "cancelled", now - 5 * day),
  entry("pair.paired", "Paired with a module", "Lab bench EPA · 192.168.7.1", modules[1].pid, "paired", now - 6 * day, {
    fields: [
      { label: "Host", value: "192.168.7.1", mono: false },
      { label: "Name the module sees", value: "Galaxy S24 (Android)", mono: false },
      { label: "pid", value: modules[1].pid, mono: true },
    ],
  }),
  entry("app.launched", "App launched", "2.0.0-dev.16 · 2 module(s) paired", "", undefined, now - 6 * day - 120),
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
