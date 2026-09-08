//! From a scope string to what the screen shows. Mirrors `_scopes` in v1
//! `scopes.js` (exact match first, then the `keymgmt:use` patterns) and
//! `keytype2string` in `base.js`. The scope itself is always shown verbatim.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Detail {
    pub label: String,
    pub value: String,
    pub mono: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ScopeView {
    /// Family for the UI: "system", "storage", "logger", "keymgmt", "auth", "unknown".
    pub kind: String,
    /// Short name for the archive: "Use the key".
    pub title: String,
    /// Completes "<host> wants to …".
    pub phrase: String,
    pub details: Vec<Detail>,
    pub ask_period: bool,
    pub ask_writable: bool,
    pub writable_default: bool,
    /// False for scopes v1 did not know: archived, never shown.
    pub known: bool,
}

fn view(kind: &str, title: &str, phrase: &str) -> ScopeView {
    ScopeView { kind: kind.into(), title: title.into(), phrase: phrase.into(), details: Vec::new(), ask_period: true, ask_writable: false, writable_default: false, known: true }
}

pub fn describe(scope: &str) -> ScopeView {
    match scope {
        "system:config" => view("system", "Change the configuration", "change its configuration"),
        "system:upgrade" => view("system", "Update the software", "install a software update"),
        "system:shutdown" => view("system", "Shut down", "shut down"),
        "storage:disk" => view("storage", "Lock the drives", "lock its drives"),
        "logger:get" => view("logger", "Read the log", "hand over the log"),
        "logger:del" => view("logger", "Delete the log", "delete the log"),
        "keymgmt:del" => view("keymgmt", "Delete a key", "delete a key"),
        "keymgmt:list" => view("keymgmt", "List the keys", "list the keys"),
        "keymgmt:get" => view("keymgmt", "Read a public key", "hand over a public key"),
        "keymgmt:gen" => view("keymgmt", "Generate a key", "generate a new key"),
        "keymgmt:upd" => view("keymgmt", "Update a key", "update a key"),
        "keymgmt:imp" => view("keymgmt", "Import a public key", "import a public key"),
        "keymgmt:ecdh" => view("keymgmt", "Derive a key", "derive a shared key"),
        "keymgmt:derive" => view("keymgmt", "Derive a key", "derive a new key"),
        "auth:ext:pair" => view("auth", "Pair a phone", "pair another phone"),
        s if s.starts_with("storage:disk") => storage(s),
        s if s.starts_with("keymgmt:use:") => key_use(s),
        _ => ScopeView { known: false, ask_period: false, ..view("unknown", "Unknown request", "do something this app does not know") },
    }
}

fn storage(scope: &str) -> ScopeView {
    let rw = scope.ends_with(":rw");
    let disk = scope.trim_end_matches(":rw").trim_start_matches("storage:");
    let mut v = view("storage", "Unlock a drive", &format!("unlock {disk}"));
    v.ask_writable = true;
    v.writable_default = rw;
    v.details.push(Detail { label: "Drive".into(), value: disk.into(), mono: true });
    v
}

/// `keymgmt:use:<kid>#<base64 json {t: "<hex type>", l: "<label>"}>`, optionally
/// `-<suffix>` after the json (v1 had a third pattern for it).
fn key_use(scope: &str) -> ScopeView {
    let rest = &scope["keymgmt:use:".len()..];
    let (kid, meta) = rest.split_once('#').unwrap_or((rest, ""));
    let mut v = view("keymgmt", "Use the key", "use a key");
    v.details.push(Detail { label: "Key id".into(), value: kid.into(), mono: true });
    if !meta.is_empty() {
        let (json_b64, _suffix) = meta.split_once('-').unwrap_or((meta, ""));
        if let Some(m) = decode_meta(json_b64) {
            if let Some(t) = m.get("t").and_then(|t| t.as_str()) {
                let code = u32::from_str_radix(t, 16).unwrap_or(0);
                v.details.push(Detail { label: "Type".into(), value: keytype_to_string(code), mono: false });
            }
            if let Some(l) = m.get("l").and_then(|l| l.as_str()) {
                v.phrase = format!("use key {l}");
                v.details.push(Detail { label: "Label".into(), value: l.into(), mono: false });
            }
        }
    }
    v
}

fn decode_meta(b64: &str) -> Option<serde_json::Value> {
    let bytes = encedo_protocol::codec::b64url_decode(b64)?;
    serde_json::from_slice(&bytes).ok()
}

const TYPE_ASYMMETRIC: u32 = 1 << 15;
const TYPE_ATTESTED: u32 = 1 << 14;
const TYPE_PRIVATEKEY: u32 = 1 << 13;
const TYPE_ECDH: u32 = 1 << 11;
const TYPE_ECDSA: u32 = 1 << 10;
const TYPE_CERT: u32 = 1 << 9;
const KEYS_MASK: u32 = 0x00ff;

/// v1 `keytype2string`: flags then the curve or algorithm, comma separated.
pub fn keytype_to_string(t: u32) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if t & TYPE_ATTESTED != 0 {
        parts.push("ATT");
    }
    let alg = if t & TYPE_ASYMMETRIC != 0 {
        if t & TYPE_PRIVATEKEY != 0 { parts.push("PKEY"); }
        if t & TYPE_ECDH != 0 { parts.push("ECDH"); }
        if t & TYPE_ECDSA != 0 { parts.push("ExDSA"); }
        if t & TYPE_CERT != 0 { parts.push("CERT"); }
        match t & KEYS_MASK {
            1 => "GENERIC_DER", 10 => "SECP256R1", 11 => "SECP384R1", 12 => "SECP521R1", 13 => "SECP256K1",
            20 => "CURVE25519", 21 => "CURVE448", 22 => "ED25519", 23 => "ED448", _ => "",
        }
    } else {
        match t & KEYS_MASK {
            10 => "SHA2-256", 11 => "SHA2-384", 12 => "SHA2-512", 13 => "SHA3-256", 14 => "SHA3-384", 15 => "SHA3-512",
            30 => "AES-128", 31 => "AES-192", 32 => "AES-256", _ => "",
        }
    };
    if !alg.is_empty() {
        parts.push(alg);
    }
    parts.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_rw_and_plain() {
        let v = describe("storage:disk0:rw");
        assert!(v.ask_writable && v.writable_default);
        assert_eq!(v.phrase, "unlock disk0");
        assert!(!describe("storage:disk1").writable_default);
        assert_eq!(describe("storage:disk").title, "Lock the drives");
    }

    #[test]
    fn key_use_decodes_label_and_type() {
        let meta = encedo_protocol::codec::b64(br#"{"t":"AC16","l":"PGP main"}"#);
        let v = describe(&format!("keymgmt:use:2ba3#{meta}"));
        assert_eq!(v.phrase, "use key PGP main");
        assert_eq!(v.details[1].value, "PKEY,ECDH,ExDSA,ED25519");
        assert!(v.known);
        assert!(!describe("weird:thing").known);
    }
}
