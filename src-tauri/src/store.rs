//! Local storage: one JSON document (settings, paired modules, the journal)
//! encrypted with AES-256-GCM into a single file, written atomically. The
//! 32-byte data key comes from a [`SecretStore`]: the Android Keystore on the
//! phone, a plain file in the app data dir on desktop (development only).
//!
//! The journal is two rings. **Audit** holds what someone might later dispute —
//! answers, pairings, the pushes that asked for them — and every entry carries a
//! seal over the one before it, so a removed or edited entry breaks the chain
//! and [`Data::verify_audit`] says where. **Trace** holds the trail behind those
//! events and folds repeats. Both keep 90 days.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use encedo_protocol::codec::b64;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop};

const MAGIC: &[u8; 4] = b"ENA2";
const FORMAT_VERSION: u8 = 1;
/// Age is the rule for both rings (the owner's call: 90 days, memory be damned).
const RETAIN_SECS: u64 = 90 * 24 * 3600;
/// Ceilings are only a guard, so a runaway cannot grow the file without bound.
const AUDIT_MAX: usize = 20_000;
const TRACE_MAX: usize = 5_000;
/// How far back a repeat may fold: far enough to catch a check that repeats
/// with a line of commentary between each, short enough that an old row does
/// not keep collecting for ever.
const FOLD_WINDOW: usize = 12;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("storage io: {0}")]
    Io(#[from] std::io::Error),
    #[error("storage file is not ours or is damaged")]
    Corrupt,
    #[error("storage key unavailable: {0}")]
    Key(String),
    #[error("storage json: {0}")]
    Json(#[from] serde_json::Error),
    /// The key is bound to the person and nobody confirmed lately. Ask, then retry.
    #[error("confirm who you are to open this phone's storage")]
    AuthRequired,
    /// The key is gone: a backup restored onto another phone, or the screen lock
    /// was removed. Nothing can read this store again.
    #[error("the key that protects this phone's storage is gone; its contents cannot be read again")]
    KeyLost,
    /// No screen lock, so no key can be bound to the person.
    #[error("this phone has no screen lock, so the storage key cannot be bound to you")]
    NoCredential,
}

/// What protects the storage key on this phone. Shown in Settings, because it is
/// the difference between "a thief needs you" and "a thief needs the phone".
#[derive(Debug, Clone, Copy, Serialize, Default, PartialEq, Eq)]
pub struct Protection {
    /// The key only works within a short window of a confirmation.
    pub bound_to_user: bool,
    /// This phone has a screen lock, so binding is possible at all.
    pub credential: bool,
    /// The key lives in a separate secure element, not only the TEE.
    pub strong_box: bool,
    /// How long one confirmation authorises the key for.
    pub window_seconds: u32,
}

/// Where the data key lives. Implementations must return the same 32 bytes
/// on every call for the lifetime of the installation.
pub trait SecretStore: Send + Sync {
    fn data_key(&self) -> Result<DataKey, StoreError>;

    /// What protects it here.
    fn protection(&self) -> Protection {
        Protection::default()
    }

    /// Re-wrap the data key with or without the binding to the person. Reads the
    /// key first, so it can ask for a confirmation.
    fn bind_to_user(&self, _bound: bool) -> Result<(), StoreError> {
        Ok(())
    }

    /// Throw the wrapped key away: the next open starts from nothing.
    fn forget(&self) -> Result<(), StoreError>;
}

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct DataKey(pub [u8; 32]);

/// Development only: the key sits next to the data, readable by the user.
/// Fine for `cargo tauri dev` on a laptop, never for a phone build.
pub struct DevFileSecret {
    pub path: PathBuf,
}

impl SecretStore for DevFileSecret {
    fn forget(&self) -> Result<(), StoreError> {
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    fn data_key(&self) -> Result<DataKey, StoreError> {
        if let Ok(bytes) = fs::read(&self.path) {
            let k: [u8; 32] = bytes.try_into().map_err(|_| StoreError::Corrupt)?;
            return Ok(DataKey(k));
        }
        let mut k = [0u8; 32];
        OsRng.fill_bytes(&mut k);
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        write_private(&self.path, &k)?;
        Ok(DataKey(k))
    }
}

// ---- data model ------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleRecord {
    pub pid: String,
    pub eid: String,
    pub aid: String,
    pub aid_prv: String,
    pub label: String,
    pub host: String,
    pub user: String,
    pub email: String,
    /// Paired at, unix seconds.
    pub iat: u64,
    #[serde(default)]
    pub last_used: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Granted,
    Denied,
    Expired,
    Cancelled,
    Rejected,
    Error,
    Paired,
    Unpaired,
}

/// What the store held before the journal: one row per answer. Read once on
/// upgrade, turned into audit entries, then gone.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArchiveEntry {
    pub id: String,
    pub pid: String,
    pub title: String,
    pub detail: String,
    pub outcome: Outcome,
    /// Unix seconds.
    pub at: u64,
}

// ---- the journal -----------------------------------------------------------

/// Which part of the app an entry came from; the History screen filters on it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    /// Requests: shown, granted, denied, expired, refused by this phone.
    Answers,
    /// Pairing and unpairing, from either side.
    Modules,
    /// Registration, token changes, every message that arrived.
    Push,
    /// What the broker was asked and what it said.
    Broker,
    /// Launch, background, lock, settings, storage.
    App,
    /// The running commentary that used to be the Diagnostics card.
    Trace,
}

impl Family {
    /// Answers, pairings and the pushes that asked for them are the evidence:
    /// sealed, never folded, never dropped to make room for noise.
    pub fn is_audit(self) -> bool {
        matches!(self, Family::Answers | Family::Modules | Family::Push)
    }
}

/// How the row reads: sealed green, plain grey, exposed rust.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Good,
    Plain,
    Bad,
}

/// One label and value in the expanded view of an entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LogField {
    pub label: String,
    pub value: String,
    /// Literal (an id, a scope, a token): shown in mono, across both columns.
    #[serde(default)]
    pub mono: bool,
}

fn one() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LogEntry {
    pub id: String,
    /// Unix seconds, and the millisecond inside them so two entries in the same
    /// second still have an order.
    pub at: u64,
    #[serde(default)]
    pub ms: u32,
    /// Machine-readable name: `request.granted`, `push.received`, `app.locked`.
    pub kind: String,
    pub family: Family,
    pub level: Level,
    /// The module this is about; empty for the app's own events.
    #[serde(default)]
    pub pid: String,
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub fields: Vec<LogField>,
    /// A blob worth keeping verbatim: the broker's answer, a push payload.
    #[serde(default)]
    pub raw: Option<String>,
    /// Set when the entry is an answer, so the row can carry its pill.
    #[serde(default)]
    pub outcome: Option<Outcome>,
    /// Identical trace entries fold into one row: how many times it happened,
    /// and when the first of them was — `at` is always the latest, so the row
    /// sits where it last happened. Audit entries never fold.
    #[serde(default = "one")]
    pub repeat: u32,
    #[serde(default)]
    pub first_at: Option<u64>,
    /// `base64(SHA-256(seal of the entry before ‖ this entry))`, audit only.
    #[serde(default)]
    pub seal: String,
}

impl LogEntry {
    /// Everything the seal covers, in a fixed order, with separators that
    /// cannot appear in the fields themselves.
    fn digest(&self, previous: &str) -> String {
        let mut h = Sha256::new();
        h.update(previous.as_bytes());
        for part in [self.id.as_str(), &self.at.to_string(), &self.ms.to_string(), &self.kind, &self.pid, &self.title, &self.summary] {
            h.update(b"\x1f");
            h.update(part.as_bytes());
        }
        for f in &self.fields {
            h.update(b"\x1e");
            h.update(f.label.as_bytes());
            h.update(b"\x1d");
            h.update(f.value.as_bytes());
        }
        if let Some(raw) = &self.raw {
            h.update(b"\x1c");
            h.update(raw.as_bytes());
        }
        b64(&h.finalize())
    }

    /// Same event happening again: kind, module and text identical.
    fn same_as(&self, other: &LogEntry) -> bool {
        self.kind == other.kind && self.pid == other.pid && self.title == other.title && self.summary == other.summary && self.fields == other.fields && self.raw == other.raw
    }
}

/// What [`Data::verify_audit`] found.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AuditHealth {
    pub entries: usize,
    /// How many fell off the front by age, so the chain's start is explained.
    pub pruned: u64,
    /// The first entry whose seal does not follow from the one before it.
    pub broken_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settings {
    pub biometric_lock: bool,
    pub lock_on_background: bool,
    pub theme: String,
    pub onboarded: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { biometric_lock: true, lock_on_background: true, theme: "system".into(), onboarded: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Data {
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub modules: Vec<ModuleRecord>,
    /// Pre-journal answers, kept only until [`Data::migrate`] moves them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub archive: Vec<ArchiveEntry>,
    /// Newest last, so a new entry seals over the previous one.
    #[serde(default)]
    pub audit: Vec<LogEntry>,
    #[serde(default)]
    pub trace: Vec<LogEntry>,
    /// Seal of the newest audit entry, and of the last one pruned by age, so the
    /// chain stays verifiable from where it now starts.
    #[serde(default)]
    pub audit_seal: String,
    #[serde(default)]
    pub pruned_seal: String,
    #[serde(default)]
    pub pruned: u64,
    /// Push token last sent to the broker, to notice rotations.
    #[serde(default)]
    pub push_token: Option<String>,
    /// Event ids already answered or discarded, so a stale `allbypid` does not
    /// show them again. Pruned to the last 200.
    #[serde(default)]
    pub handled_events: Vec<String>,
}

impl Data {
    pub fn module(&self, pid: &str) -> Option<&ModuleRecord> {
        self.modules.iter().find(|m| m.pid == pid)
    }

    /// Put an entry in its ring: audit entries are sealed over the previous one;
    /// a trace entry folds into a recent identical one instead of adding a row.
    /// Prunes by age relative to this entry's clock.
    pub fn record(&mut self, mut entry: LogEntry) {
        if entry.family.is_audit() {
            entry.seal = entry.digest(&self.audit_seal);
            self.audit_seal = entry.seal.clone();
            self.audit.push(entry);
            self.prune();
            return;
        }
        let from = self.trace.len().saturating_sub(FOLD_WINDOW);
        match self.trace[from..].iter().rposition(|l| l.same_as(&entry)).map(|p| from + p) {
            Some(i) => {
                // The row moves to where it last happened, so the timeline stays in order.
                let mut folded = self.trace.remove(i);
                folded.repeat += 1;
                folded.first_at = folded.first_at.or(Some(folded.at));
                folded.at = entry.at;
                folded.ms = entry.ms;
                self.trace.push(folded);
            }
            None => self.trace.push(entry),
        }
        self.prune();
    }

    /// Age decides; the ceilings only stop the file from growing without bound.
    fn prune(&mut self) {
        let newest = self.audit.last().map(|e| e.at).unwrap_or(0).max(self.trace.last().map(|e| e.at).unwrap_or(0));
        let cutoff = newest.saturating_sub(RETAIN_SECS);
        let mut drop_audit = self.audit.iter().take_while(|e| e.at < cutoff).count();
        drop_audit += self.audit.len().saturating_sub(drop_audit).saturating_sub(AUDIT_MAX);
        if drop_audit > 0 {
            if let Some(last) = self.audit.get(drop_audit - 1) {
                self.pruned_seal = last.seal.clone();
            }
            self.pruned += drop_audit as u64;
            self.audit.drain(..drop_audit);
        }
        let mut drop_trace = self.trace.iter().take_while(|e| e.at < cutoff).count();
        drop_trace += self.trace.len().saturating_sub(drop_trace).saturating_sub(TRACE_MAX);
        self.trace.drain(..drop_trace);
    }

    /// Walk the audit chain from where it starts. `broken_at` names the first
    /// entry that does not follow, which is what an edited file looks like.
    pub fn verify_audit(&self) -> AuditHealth {
        let mut previous = self.pruned_seal.clone();
        let mut broken_at = None;
        for e in &self.audit {
            if e.digest(&previous) != e.seal {
                broken_at = Some(e.id.clone());
                break;
            }
            previous = e.seal.clone();
        }
        AuditHealth { entries: self.audit.len(), pruned: self.pruned, broken_at }
    }

    /// The journal, newest first, for the screen. `family` narrows it; trace is
    /// only ever returned when asked for by name.
    pub fn journal(&self, family: Option<Family>) -> Vec<LogEntry> {
        let mut out: Vec<LogEntry> = match family {
            Some(Family::Trace) => self.trace.iter().filter(|e| e.family == Family::Trace).cloned().collect(),
            Some(f) if f.is_audit() => self.audit.iter().filter(|e| e.family == f).cloned().collect(),
            Some(f) => self.trace.iter().filter(|e| e.family == f).cloned().collect(),
            None => self.audit.iter().chain(self.trace.iter().filter(|e| e.family != Family::Trace)).cloned().collect(),
        };
        out.sort_by(|a, b| (b.at, b.ms).cmp(&(a.at, a.ms)));
        out
    }

    /// Turn the old archive into audit entries, once, on first open after the
    /// upgrade. Nobody loses the history of the dev builds.
    pub fn migrate(&mut self) -> bool {
        if self.archive.is_empty() {
            return false;
        }
        let mut old = std::mem::take(&mut self.archive);
        old.sort_by_key(|e| e.at);
        for a in old {
            let (kind, family, level) = match a.outcome {
                Outcome::Granted => ("request.granted", Family::Answers, Level::Good),
                Outcome::Denied => ("request.denied", Family::Answers, Level::Bad),
                Outcome::Expired => ("request.expired", Family::Answers, Level::Plain),
                Outcome::Cancelled => ("request.cancelled", Family::Answers, Level::Plain),
                Outcome::Rejected => ("request.rejected", Family::Answers, Level::Bad),
                Outcome::Error => ("request.error", Family::Answers, Level::Bad),
                Outcome::Paired => ("pair.paired", Family::Modules, Level::Good),
                Outcome::Unpaired => ("unpair.done", Family::Modules, Level::Plain),
            };
            self.record(LogEntry {
                id: a.id,
                at: a.at,
                ms: 0,
                kind: kind.into(),
                family,
                level,
                pid: a.pid,
                title: a.title,
                summary: a.detail,
                fields: vec![LogField { label: "Kept from".into(), value: "the archive this app had before the journal".into(), mono: false }],
                raw: None,
                outcome: Some(a.outcome),
                repeat: 1,
                first_at: None,
                seal: String::new(),
            });
        }
        true
    }

    pub fn mark_handled(&mut self, event_id: &str) {
        if !self.handled_events.iter().any(|e| e == event_id) {
            self.handled_events.push(event_id.to_string());
            if self.handled_events.len() > 200 {
                let drop = self.handled_events.len() - 200;
                self.handled_events.drain(..drop);
            }
        }
    }
}

// ---- encrypted file --------------------------------------------------------

pub struct Store {
    path: PathBuf,
    key: DataKey,
    pub data: Data,
}

impl Store {
    /// Open (or create) the store at `path` with the key from `secrets`.
    pub fn open(path: PathBuf, secrets: &dyn SecretStore) -> Result<Self, StoreError> {
        let key = secrets.data_key()?;
        let data = match fs::read(&path) {
            Ok(bytes) => decrypt(&key.0, &bytes)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Data::default(),
            Err(e) => return Err(e.into()),
        };
        let mut store = Self { path, key, data };
        if store.data.migrate() {
            store.save()?;
        }
        Ok(store)
    }

    /// Encrypt and write atomically: temp file in the same directory, then rename.
    pub fn save(&self) -> Result<(), StoreError> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let bytes = encrypt(&self.key.0, &self.data)?;
        let tmp = self.path.with_extension("tmp");
        write_private(&tmp, &bytes)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    /// Mutate and persist in one step.
    pub fn update<T>(&mut self, f: impl FnOnce(&mut Data) -> T) -> Result<T, StoreError> {
        let out = f(&mut self.data);
        self.save()?;
        Ok(out)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn encrypt(key: &[u8; 32], data: &Data) -> Result<Vec<u8>, StoreError> {
    let plain = serde_json::to_vec(data)?;
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let cipher = Aes256Gcm::new(key.into());
    let aad = [FORMAT_VERSION];
    let ct = cipher
        .encrypt(Nonce::from_slice(&nonce), Payload { msg: &plain, aad: &aad })
        .map_err(|_| StoreError::Corrupt)?;
    let mut out = Vec::with_capacity(4 + 1 + 12 + ct.len());
    out.extend_from_slice(MAGIC);
    out.push(FORMAT_VERSION);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

fn decrypt(key: &[u8; 32], bytes: &[u8]) -> Result<Data, StoreError> {
    if bytes.len() < 4 + 1 + 12 + 16 || &bytes[..4] != MAGIC {
        return Err(StoreError::Corrupt);
    }
    let version = bytes[4];
    if version != FORMAT_VERSION {
        return Err(StoreError::Corrupt);
    }
    let nonce = &bytes[5..17];
    let ct = &bytes[17..];
    let cipher = Aes256Gcm::new(key.into());
    let plain = cipher
        .decrypt(Nonce::from_slice(nonce), Payload { msg: ct, aad: &[version] })
        .map_err(|_| StoreError::Corrupt)?;
    Ok(serde_json::from_slice(&plain)?)
}

fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path)?;
    f.write_all(bytes)?;
    f.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(at: u64, kind: &str, family: Family, title: &str) -> LogEntry {
        LogEntry {
            id: format!("{at}-{kind}"),
            at,
            ms: 0,
            kind: kind.into(),
            family,
            level: Level::Plain,
            pid: String::new(),
            title: title.into(),
            summary: String::new(),
            fields: Vec::new(),
            raw: None,
            outcome: None,
            repeat: 1,
            first_at: None,
            seal: String::new(),
        }
    }

    #[test]
    fn round_trip_and_atomic_file() {
        let dir = std::env::temp_dir().join(format!("encedo-store-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let secrets = DevFileSecret { path: dir.join("dev.key") };
        let mut s = Store::open(dir.join("store.bin"), &secrets).unwrap();
        assert_eq!(s.data, Data::default());
        s.update(|d| {
            d.settings.onboarded = true;
            d.modules.push(ModuleRecord { pid: "p".into(), eid: "e".into(), aid: "a".into(), aid_prv: "s".into(), label: "L".into(), host: "h".into(), user: "u".into(), email: "".into(), iat: 1, last_used: None });
            d.record(entry(2, "request.granted", Family::Answers, "Unlock a drive"));
        })
        .unwrap();
        let again = Store::open(dir.join("store.bin"), &secrets).unwrap();
        assert_eq!(again.data, s.data);
        assert!(!dir.join("store.bin.tmp").exists());
        // Wrong key: refused, not garbage.
        let other = DevFileSecret { path: dir.join("other.key") };
        assert!(matches!(Store::open(dir.join("store.bin"), &other), Err(StoreError::Corrupt)));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_audit_chain_notices_an_edited_or_missing_entry() {
        let mut d = Data::default();
        for i in 1..=4 {
            d.record(entry(1_700_000_000 + i, "request.granted", Family::Answers, "Unlock a drive"));
        }
        let health = d.verify_audit();
        assert_eq!((health.entries, health.pruned, health.broken_at), (4, 0, None));

        // Someone rewrites the summary of the second answer in the file.
        let mut edited = d.clone();
        edited.audit[1].summary = "storage:disk:home:rw · 24 h".into();
        assert_eq!(edited.verify_audit().broken_at.as_deref(), Some(edited.audit[1].id.as_str()));

        // Someone cuts an answer out: the entry after it no longer follows.
        let mut cut = d.clone();
        let orphan = cut.audit[2].id.clone();
        cut.audit.remove(1);
        assert_eq!(cut.verify_audit().broken_at, Some(orphan));

        // A fresh entry seals over the newest one, so the chain keeps growing.
        d.record(entry(1_700_000_005, "request.denied", Family::Answers, "Use the key"));
        assert_eq!(d.verify_audit().broken_at, None);
        assert_eq!(d.audit_seal, d.audit.last().unwrap().seal);
    }

    #[test]
    fn the_trail_folds_repeats_but_answers_never_do() {
        let mut d = Data::default();
        for i in 0..12 {
            d.record(entry(1_700_000_000 + i, "broker.checked", Family::Broker, "Broker checked"));
        }
        assert_eq!(d.trace.len(), 1, "twelve identical checks are one row");
        assert_eq!(d.trace[0].repeat, 12);
        assert_eq!(d.trace[0].at, 1_700_000_011, "the row sits where it last happened");
        assert_eq!(d.trace[0].first_at, Some(1_700_000_000), "and remembers when it started");

        // Something else in between does not stop the next one from folding: a
        // check every 15 s has a line of commentary between each.
        d.record(entry(1_700_000_012, "broker.error", Family::Broker, "Broker could not be asked"));
        d.record(entry(1_700_000_013, "broker.checked", Family::Broker, "Broker checked"));
        assert_eq!(d.trace.len(), 2);
        assert_eq!(d.trace.last().unwrap().repeat, 13);

        // Past the window it starts a new row rather than reviving an old one.
        for i in 0..FOLD_WINDOW {
            d.record(entry(1_700_000_020 + i as u64, "app.trace", Family::Trace, &format!("line {i}")));
        }
        d.record(entry(1_700_000_100, "broker.checked", Family::Broker, "Broker checked"));
        assert_eq!(d.trace.last().unwrap().repeat, 1);

        // Two identical grants are two grants, whatever they look like.
        d.record(entry(1_700_000_014, "request.granted", Family::Answers, "Unlock a drive"));
        d.record(entry(1_700_000_014, "request.granted", Family::Answers, "Unlock a drive"));
        assert_eq!(d.audit.len(), 2);
        assert_eq!(d.verify_audit().broken_at, None);
    }

    #[test]
    fn ninety_days_is_the_rule_and_the_chain_says_where_it_starts() {
        let mut d = Data::default();
        let day = 86_400;
        let base = 1_700_000_000;
        for i in 0..5 {
            d.record(entry(base + i * 30 * day, "request.granted", Family::Answers, "Unlock a drive"));
        }
        // The newest is 120 days after the oldest, so only the oldest is past the
        // 90 days; the one exactly on the line stays.
        assert_eq!(d.audit.len(), 4);
        assert_eq!(d.pruned, 1);
        assert_eq!(d.audit[0].at, base + 30 * day);
        assert!(!d.pruned_seal.is_empty(), "the chain is anchored where it now starts");
        assert_eq!(d.verify_audit().broken_at, None);

        let mut trail = Data::default();
        trail.record(entry(base, "app.trace", Family::Trace, "old line"));
        trail.record(entry(base + 100 * day, "app.trace", Family::Trace, "new line"));
        assert_eq!(trail.trace.len(), 1);
        assert_eq!(trail.trace[0].title, "new line");
    }

    #[test]
    fn the_old_archive_becomes_sealed_audit_entries() {
        let mut d = Data::default();
        d.archive = vec![
            ArchiveEntry { id: "2".into(), pid: "p".into(), title: "Use the key".into(), detail: "keymgmt:use:9f2a".into(), outcome: Outcome::Denied, at: 200 },
            ArchiveEntry { id: "1".into(), pid: "p".into(), title: "Pair this phone".into(), detail: "hem · host".into(), outcome: Outcome::Paired, at: 100 },
        ];
        assert!(d.migrate());
        assert!(d.archive.is_empty(), "and it is not written again");
        assert_eq!(d.audit.len(), 2);
        // Oldest first, so the chain follows the order things happened in.
        assert_eq!(d.audit[0].kind, "pair.paired");
        assert_eq!(d.audit[0].family, Family::Modules);
        assert_eq!(d.audit[1].kind, "request.denied");
        assert_eq!(d.audit[1].summary, "keymgmt:use:9f2a");
        assert_eq!(d.audit[1].outcome, Some(Outcome::Denied));
        assert_eq!(d.verify_audit().broken_at, None);
        assert!(!d.migrate(), "only once");
    }

    #[test]
    fn the_journal_is_newest_first_and_trace_only_when_asked_for() {
        let mut d = Data::default();
        d.record(entry(10, "request.granted", Family::Answers, "Unlock a drive"));
        d.record(entry(20, "broker.checked", Family::Broker, "Broker checked"));
        d.record(entry(30, "app.trace", Family::Trace, "allbypid for 2 pid(s)"));
        let all = d.journal(None);
        assert_eq!(all.iter().map(|e| e.kind.as_str()).collect::<Vec<_>>(), ["broker.checked", "request.granted"]);
        assert_eq!(d.journal(Some(Family::Trace)).len(), 1);
        assert_eq!(d.journal(Some(Family::Answers)).len(), 1);
        assert!(d.journal(Some(Family::Modules)).is_empty());
    }
}
