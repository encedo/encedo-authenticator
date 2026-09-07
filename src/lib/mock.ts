import type { AccessRequest, ArchiveEntry, Module, ScopeKind } from "./types";

// Demo data for the Phase 1 mockup. Replaced by Tauri commands in Phases 2 and 3.

export const modules: Module[] = [
  { pid: "3f9a7d1c-4f2b-4e8a-9c61-0b5f2e7ac21e", label: "Office PPA", host: "my.ence.do", email: "chris@encedo.com", pairedAt: "2026-06-12", lastUsed: "2026-09-07T08:58:00" },
  { pid: "7b02f6c4-9a7e-4d3c-8e55-3f9d1a6c9d4f", label: "Lab bench EPA", host: "192.168.7.1", email: "chris@encedo.com", pairedAt: "2026-09-01", lastUsed: "2026-09-06T18:40:00" },
];

export const archive: ArchiveEntry[] = [
  { id: "a1", pid: modules[0].pid, title: "Use the key", detail: "PGP main · 2ba3…c91e · 1 h", outcome: "granted", at: "2026-09-07T08:58:00" },
  { id: "a2", pid: modules[1].pid, title: "Unlock a drive", detail: "disk0 · read-write · 1 h", outcome: "granted", at: "2026-09-06T18:40:00" },
  { id: "a3", pid: modules[0].pid, title: "Delete the log", detail: "", outcome: "denied", at: "2026-09-05T11:03:00" },
  { id: "a4", pid: modules[0].pid, title: "Use the key", detail: "SSH deploy · 8f71…0a2d", outcome: "expired", at: "2026-09-04T22:15:00" },
  { id: "a5", pid: modules[1].pid, title: "Update the software", detail: "", outcome: "cancelled", at: "2026-09-02T08:30:00" },
  { id: "a6", pid: modules[0].pid, title: "Pair a phone", detail: "Lab bench EPA", outcome: "granted", at: "2026-09-01T14:22:00" },
];

const catalogue: Record<ScopeKind, { title: string; phrase: string }> = {
  "system:config": { title: "Change the configuration", phrase: "change its configuration" },
  "system:upgrade": { title: "Update the software", phrase: "install a software update" },
  "system:shutdown": { title: "Shut down", phrase: "shut down" },
  "storage:disk": { title: "Unlock a drive", phrase: "unlock disk0" },
  "logger:get": { title: "Read the log", phrase: "hand over the log" },
  "logger:del": { title: "Delete the log", phrase: "delete the log" },
  "keymgmt:use": { title: "Use the key", phrase: "use key PGP main" },
  "keymgmt:del": { title: "Delete a key", phrase: "delete a key" },
  "keymgmt:list": { title: "List the keys", phrase: "list the keys" },
  "keymgmt:get": { title: "Read a public key", phrase: "hand over a public key" },
  "keymgmt:gen": { title: "Generate a key", phrase: "generate a new key" },
  "keymgmt:upd": { title: "Update a key", phrase: "update a key" },
  "keymgmt:imp": { title: "Import a public key", phrase: "import a public key" },
  "keymgmt:derive": { title: "Derive a key", phrase: "derive a new key" },
  "auth:ext:pair": { title: "Pair a phone", phrase: "pair another phone" },
};

export const demoScopes: ScopeKind[] = ["keymgmt:use", "storage:disk", "system:upgrade", "logger:del", "auth:ext:pair"];

let seq = 0;

export function makeRequest(kind: ScopeKind, mod: Module): AccessRequest {
  seq += 1;
  const c = catalogue[kind];
  const base: AccessRequest = {
    id: `req-${seq}`,
    pid: mod.pid,
    kind,
    scope: kind,
    phrase: c.phrase,
    title: c.title,
    details: [],
    host: mod.host,
    expiresAt: Date.now() + 90_000,
    askPeriod: false,
    askWritable: false,
    writableDefault: false,
  };
  switch (kind) {
    case "keymgmt:use":
      return {
        ...base,
        scope: "keymgmt:use:2ba3f1e0c9a4b7d8e6f5a3c2b1d0e9f84c91e#eyJ0IjoiMDEiLCJsIjoiUEdQIG1haW4ifQ",
        askPeriod: true,
        details: [
          { label: "Key id", value: "2ba3f1e0c9a4b7d8e6f5a3c2b1d0e9f84c91e", mono: true },
          { label: "Type", value: "Ed25519" },
          { label: "Label", value: "PGP main" },
        ],
      };
    case "storage:disk":
      return {
        ...base,
        scope: "storage:disk:disk0:encrypted",
        askPeriod: true,
        askWritable: true,
        writableDefault: true,
        details: [
          { label: "Drive", value: "disk0", mono: true },
          { label: "Kind", value: "Encrypted" },
        ],
      };
    case "auth:ext:pair":
      return { ...base, details: [{ label: "New phone", value: "Pixel 8" }] };
    default:
      return base;
  }
}

export function scopeTitle(kind: ScopeKind): string {
  return catalogue[kind].title;
}
