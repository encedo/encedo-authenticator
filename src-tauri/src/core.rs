//! The application core: pairing, requests, unpair, push handling. Holds the
//! store and the broker client; hands the webview display data only.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use encedo_protocol::codec::{b64, b64_decode_32};
use encedo_protocol::event::{self, Event, IpInfo};
use encedo_protocol::keys::KeyPair;
use encedo_protocol::pairing::{self, PairingRequest};
use encedo_protocol::unpair;
use rand_core::{OsRng, RngCore};
use serde::Serialize;

use crate::diag::brief;
use crate::notify::{NotifyClient, NotifyError, PairingCode};
use crate::scope::{describe, ScopeView};
use crate::store::{AuditHealth, Family, Level, LogEntry, LogField, ModuleRecord, Outcome, Protection, SecretStore, Settings, Store, StoreError};

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// How many trace entries may wait for a write before one is forced.
const PENDING_MAX: usize = 200;

/// Unix seconds and the millisecond inside them, so two entries in the same
/// second keep their order in the journal.
fn now_ms() -> (u64, u32) {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| (d.as_secs(), d.subsec_millis())).unwrap_or((0, 0))
}

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error(transparent)]
    Notify(#[from] NotifyError),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("protocol: {0}")]
    Protocol(#[from] encedo_protocol::Error),
    #[error("{0}")]
    State(String),
    /// The storage has not been opened yet: the key is bound to the person and
    /// nobody has confirmed. Every read and write says this until it is open.
    #[error("this phone's storage is not open yet")]
    Locked,
}

/// What the webview gets on error: a stable code and a sentence.
#[derive(Debug, Serialize, Clone)]
pub struct ErrorView {
    pub code: String,
    pub message: String,
}

impl ErrorView {
    /// The same view without consuming the error, so a failure can be both
    /// written to the journal and handed to the webview.
    pub fn of(e: &CoreError) -> Self {
        let code = match e {
            CoreError::Notify(n) => n.code().to_string(),
            CoreError::Locked => "locked".into(),
            CoreError::Store(StoreError::AuthRequired) => "auth_required".into(),
            CoreError::Store(StoreError::KeyLost) => "key_lost".into(),
            CoreError::Store(StoreError::NoCredential) => "no_credential".into(),
            CoreError::Store(_) => "storage".into(),
            CoreError::Protocol(encedo_protocol::Error::BadMac) => "bad_mac".into(),
            CoreError::Protocol(encedo_protocol::Error::Expired) => "expired".into(),
            CoreError::Protocol(_) => "protocol".into(),
            CoreError::State(_) => "state".into(),
        };
        ErrorView { code, message: e.to_string() }
    }
}

impl From<CoreError> for ErrorView {
    fn from(e: CoreError) -> Self {
        ErrorView::of(&e)
    }
}

impl serde::Serialize for CoreError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        ErrorView::from(CoreError::State(self.to_string())).serialize(s)
    }
}

// ---- views -----------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct ModuleView {
    pub pid: String,
    /// The phone's public key for this module: v1 called it the KID
    /// (`index.js`: `element.kid = element.aid`). The private half never leaves
    /// the store.
    pub aid: String,
    pub label: String,
    pub host: String,
    pub user: String,
    pub email: String,
    pub paired_at: u64,
    pub last_used: Option<u64>,
}

impl From<&ModuleRecord> for ModuleView {
    fn from(m: &ModuleRecord) -> Self {
        Self { pid: m.pid.clone(), aid: m.aid.clone(), label: m.label.clone(), host: m.host.clone(), user: m.user.clone(), email: m.email.clone(), paired_at: m.iat, last_used: m.last_used }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PairingPreview {
    pub link: String,
    pub user: String,
    pub hostname: String,
    pub email: String,
    pub issuer: Option<IpInfo>,
    pub already_paired: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestView {
    pub id: String,
    pub pid: String,
    pub module_label: String,
    pub host: String,
    pub scope: String,
    pub exp: u64,
    pub issuer: Option<IpInfo>,
    #[serde(flatten)]
    pub view: ScopeView,
}

/// What the last broker check did; shown on the Settings screen so a field
/// test can be read without adb.
#[derive(Debug, Clone, Serialize, Default)]
pub struct RefreshReport {
    pub at: u64,
    pub pids: Vec<String>,
    pub broker_said: String,
    pub pending: usize,
    pub shown: usize,
    pub discarded: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnswerView {
    pub outcome: Outcome,
    pub title: String,
    pub detail: String,
}

// ---- journal -----------------------------------------------------------------

/// One line on its way to the journal. [`Core::record`] stamps the time, the id
/// and (for audit families) the seal.
pub struct Note {
    kind: &'static str,
    family: Family,
    level: Level,
    pid: String,
    title: String,
    summary: String,
    fields: Vec<LogField>,
    raw: Option<String>,
    outcome: Option<Outcome>,
}

impl Note {
    pub fn new(kind: &'static str, family: Family, level: Level, title: impl Into<String>) -> Self {
        Self { kind, family, level, pid: String::new(), title: title.into(), summary: String::new(), fields: Vec::new(), raw: None, outcome: None }
    }

    /// The module this is about.
    pub fn pid(mut self, pid: &str) -> Self {
        self.pid = pid.to_string();
        self
    }

    /// The line under the title: what happened, in one breath.
    pub fn about(mut self, summary: impl Into<String>) -> Self {
        self.summary = summary.into();
        self
    }

    pub fn field(mut self, label: &str, value: impl Into<String>) -> Self {
        self.fields.push(LogField { label: label.into(), value: value.into(), mono: false });
        self
    }

    /// A literal: an id, a scope, a token. Shown in mono across both columns.
    pub fn mono(mut self, label: &str, value: impl Into<String>) -> Self {
        self.fields.push(LogField { label: label.into(), value: value.into(), mono: true });
        self
    }

    pub fn raw(mut self, raw: impl Into<String>) -> Self {
        self.raw = Some(raw.into());
        self
    }

    pub fn outcome(mut self, outcome: Outcome) -> Self {
        self.outcome = Some(outcome);
        self
    }

    /// How the row reads, when only the answer decides it.
    pub fn level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    /// What happened, when that is only known once the broker has spoken.
    pub fn kind(mut self, kind: &'static str) -> Self {
        self.kind = kind;
        self
    }

    /// The error as the webview would see it, as fields.
    pub fn because(self, e: &CoreError) -> Self {
        let v = ErrorView::of(e);
        self.field("Error", v.message).mono("Code", v.code)
    }

    fn entry(self) -> LogEntry {
        let (at, ms) = now_ms();
        LogEntry {
            id: format!("{at}{ms:03}-{:08x}", OsRng.next_u32()),
            at,
            ms,
            kind: self.kind.into(),
            family: self.family,
            level: self.level,
            pid: self.pid,
            title: self.title,
            summary: self.summary,
            fields: self.fields,
            raw: self.raw,
            outcome: self.outcome,
            repeat: 1,
            first_at: None,
            seal: String::new(),
        }
    }
}

// ---- state -------------------------------------------------------------------

struct PendingPairing {
    code: PairingCode,
    request: PairingRequest,
}

struct LiveRequest {
    pid: String,
    event: Event,
    scope: String,
    view: ScopeView,
}

/// What the webview needs to know before it can show anything: is the storage
/// open, and what protects its key.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct StoreStatus {
    pub open: bool,
    #[serde(flatten)]
    pub protection: Protection,
}

pub struct Core {
    /// Empty until the key can be had: a key bound to the person needs a
    /// confirmation first, and that happens on the Locked screen.
    store: Mutex<Option<Store>>,
    secrets: Box<dyn SecretStore>,
    store_path: PathBuf,
    client: NotifyClient,
    pairing: Mutex<Option<PendingPairing>>,
    live: Mutex<HashMap<String, LiveRequest>>,
    /// Device name for the pairing label, from the OS.
    device_name: String,
    last_refresh: Mutex<RefreshReport>,
    /// Trace lines waiting for the next write, so a running commentary does not
    /// mean a file write per line. Drained by [`Core::with_store`].
    pending_trace: Mutex<Vec<LogEntry>>,
}

impl Core {
    pub fn new(secrets: Box<dyn SecretStore>, store_path: PathBuf, client: NotifyClient, device_name: String) -> Self {
        Self {
            store: Mutex::new(None),
            secrets,
            store_path,
            client,
            pairing: Mutex::new(None),
            live: Mutex::new(HashMap::new()),
            device_name,
            last_refresh: Mutex::new(RefreshReport::default()),
            pending_trace: Mutex::new(Vec::new()),
        }
    }

    /// Open the storage. Fails with [`StoreError::AuthRequired`] when the key is
    /// bound to the person and the confirmation is stale; the app then asks and
    /// calls this again.
    pub fn open_storage(&self) -> Result<(), CoreError> {
        let mut slot = self.store.lock().map_err(|_| CoreError::State("store poisoned".into()))?;
        if slot.is_some() {
            return Ok(());
        }
        *slot = Some(Store::open(self.store_path.clone(), self.secrets.as_ref())?);
        Ok(())
    }

    pub fn is_open(&self) -> bool {
        self.store.lock().map(|s| s.is_some()).unwrap_or(false)
    }

    pub fn status(&self) -> StoreStatus {
        StoreStatus { open: self.is_open(), protection: self.secrets.protection() }
    }

    /// Bind the storage key to the person, or stop binding it. The key has to be
    /// read to be re-wrapped, so a stale confirmation says so.
    pub fn bind_to_user(&self, bound: bool) -> Result<(), CoreError> {
        self.secrets.bind_to_user(bound)?;
        Ok(())
    }

    /// The key is gone (a backup restored onto another phone, the screen lock
    /// removed): throw the unreadable file away and start clean, with a line in
    /// the journal saying why there is nothing left.
    pub fn reset(&self) -> Result<(), CoreError> {
        {
            let mut slot = self.store.lock().map_err(|_| CoreError::State("store poisoned".into()))?;
            *slot = None;
        }
        for path in [self.store_path.clone(), self.store_path.with_extension("tmp")] {
            match std::fs::remove_file(&path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(CoreError::Store(StoreError::Io(e))),
            }
        }
        self.secrets.forget()?;
        self.open_storage()?;
        self.record(
            Note::new("app.storage_reset", Family::App, Level::Bad, "Storage started from nothing")
                .about("the key that protected the old contents was gone")
                .field("What was lost", "paired modules and the journal that came before this line")
                .field("What to do", "pair your modules again"),
        )
    }

    fn with_store<T>(&self, f: impl FnOnce(&mut Store) -> Result<T, StoreError>) -> Result<T, CoreError> {
        let mut slot = self.store.lock().map_err(|_| CoreError::State("store poisoned".into()))?;
        let s = slot.as_mut().ok_or(CoreError::Locked)?;
        if let Ok(mut pending) = self.pending_trace.lock() {
            for e in pending.drain(..) {
                s.data.record(e);
            }
        }
        Ok(f(s)?)
    }

    /// Write one entry to the journal. The record (answers, pairings, pushes) is
    /// written at once; the trail waits for the next write, so a broker check
    /// every 15 s does not mean a file write every 15 s.
    fn record(&self, note: Note) -> Result<(), CoreError> {
        let entry = note.entry();
        if entry.family.is_audit() {
            return self.with_store(|s| s.update(|d| d.record(entry)));
        }
        self.buffer(entry)
    }

    /// The running commentary: logcat as before, plus a trace entry.
    /// Never key material.
    fn note(&self, line: impl AsRef<str>) {
        let line = line.as_ref();
        crate::diag::log(line);
        let _ = self.buffer(Note::new("app.trace", Family::Trace, Level::Plain, line).entry());
    }

    /// Park an entry until the next write; write out when too many have piled up.
    fn buffer(&self, entry: LogEntry) -> Result<(), CoreError> {
        let full = {
            let Ok(mut pending) = self.pending_trace.lock() else { return Ok(()) };
            pending.push(entry);
            pending.len() >= PENDING_MAX
        };
        if full {
            self.flush()?;
        }
        Ok(())
    }

    /// Write out whatever the commentary has collected. Called when the app
    /// goes to the background, where it might not come back.
    pub fn flush(&self) -> Result<(), CoreError> {
        self.with_store(|s| s.save())
    }

    // ---- updates -----------------------------------------------------------

    /// What Play said, folded into what this phone already knew. Writes a line
    /// in the journal when the verdict changes, and nothing when it does not.
    pub fn update_seen(&self, play: &crate::update::PlayAnswer, current: i64, note: Option<String>) -> crate::update::UpdateStatus {
        let path = self.update_path();
        let (mut status, changed) = crate::update::settle(&path, play, current, now());
        status.note = note;
        if changed {
            let level = match status.level {
                crate::update::Level::Critical => Level::Bad,
                _ => Level::Plain,
            };
            let title = match status.level {
                crate::update::Level::Critical => "This version must not be used",
                crate::update::Level::Recommended => "A newer version is waiting",
                crate::update::Level::None => "Up to date again",
            };
            let _ = self.record(
                Note::new("app.update", Family::App, level, title)
                    .about(match status.level {
                        crate::update::Level::None => "this phone runs a version nobody has objected to".to_string(),
                        _ => format!("Play offers build {}, this phone runs {}", status.required_version, status.current_version),
                    })
                    .field("Priority", status.priority.to_string())
                    .field("Behind by", if status.stale_days >= 0 { format!("{} day(s)", status.stale_days) } else { "not stated".into() })
                    .field("Play can update it here", if status.can_update_in_app { "yes" } else { "no, this build did not come from Play" }),
            );
        }
        status
    }

    /// The remembered verdict, for a launch that cannot reach Play at all.
    pub fn update_known(&self, current: i64) -> crate::update::UpdateStatus {
        let (mut status, _) = crate::update::settle(&self.update_path(), &crate::update::PlayAnswer::default(), current, now());
        status.note = Some("Play could not be asked".into());
        status
    }

    /// Drop the remembered verdict. Only the development simulation uses this:
    /// in the field a verdict is lifted by installing the version it names.
    pub fn update_forget(&self) {
        crate::update::forget(&self.update_path());
    }

    fn update_path(&self) -> PathBuf {
        self.store_path.with_file_name("update.json")
    }

    /// The person is on their way to Play; say so in the journal.
    pub fn update_started(&self, how: &str) -> Result<(), CoreError> {
        self.record(Note::new("app.update_started", Family::App, Level::Plain, "Sent to Play to update").about(how.to_string()))
    }

    /// The first things done with a newly opened store: sweep what the previous
    /// version left behind, and write the line that opens the session.
    pub fn opened(&self, version: &str) -> Result<(), CoreError> {
        if let Some(dir) = self.store_path.parent() {
            let leftovers = crate::legacy::sweep_v1(dir);
            if !leftovers.is_empty() {
                let _ = self.note_v1_removed(&leftovers);
            }
        }
        self.note_launched(version)?;
        self.flush()
    }

    /// The app is up and the store opened: the first line of the session.
    pub fn note_launched(&self, version: &str) -> Result<(), CoreError> {
        let (modules, audit, trace) = self.with_store(|s| Ok((s.data.modules.len(), s.data.audit.len(), s.data.trace.len())))?;
        self.record(
            Note::new("app.launched", Family::App, Level::Plain, "App launched")
                .about(format!("{version} · {modules} module(s) paired"))
                .mono("Version", version.to_string())
                .field("Platform", std::env::consts::OS)
                .field("Storage", "opened")
                .field("Journal", format!("{audit} sealed, {trace} traced")),
        )
    }

    /// The previous app's data was found and removed: say so in the journal,
    /// where it stays until the person pairs again.
    pub fn note_v1_removed(&self, files: &[String]) -> Result<(), CoreError> {
        self.record(
            Note::new("app.legacy_removed", Family::App, Level::Bad, "Data from the previous version removed")
                .about("pair your modules again")
                .mono("Files", files.join(", ")),
        )
    }

    // ---- read side ---------------------------------------------------------

    pub fn settings(&self) -> Result<Settings, CoreError> {
        self.with_store(|s| Ok(s.data.settings.clone()))
    }

    pub fn set_settings(&self, settings: Settings) -> Result<(), CoreError> {
        let before = self.with_store(|s| Ok(s.data.settings.clone()))?;
        let mut changed: Vec<String> = Vec::new();
        if before.biometric_lock != settings.biometric_lock {
            changed.push(format!("lock {}", if settings.biometric_lock { "on" } else { "off" }));
        }
        if before.lock_on_background != settings.lock_on_background {
            changed.push(format!("lock in background {}", if settings.lock_on_background { "on" } else { "off" }));
        }
        if before.theme != settings.theme {
            changed.push(format!("theme {}", settings.theme));
        }
        if before.onboarded != settings.onboarded && settings.onboarded {
            changed.push("first run finished".into());
        }
        // The lock is no longer only about the screen: it decides whether the
        // storage key is bound to the person. Compare with what actually protects
        // the key, not with the old setting — a fresh store starts unbound while
        // the setting already says the lock is on, and the first save is what
        // binds it. A refusal is only passed on when this call was about the lock;
        // changing the theme must not fail because the phone cannot bind.
        let want = settings.biometric_lock;
        if self.secrets.protection().bound_to_user != want {
            match self.bind_to_user(want) {
                Ok(()) => changed.push(format!("storage key {}", if want { "bound to you" } else { "not bound" })),
                Err(e) if before.biometric_lock != want => return Err(e),
                Err(e) => self.note(format!("storage key left as it is: {e}")),
            }
        }
        if !changed.is_empty() {
            self.record(Note::new("settings.changed", Family::App, Level::Plain, "Settings changed").about(changed.join(" · ")))?;
        }
        self.with_store(|s| s.update(|d| d.settings = settings))
    }

    pub fn modules(&self) -> Result<Vec<ModuleView>, CoreError> {
        self.with_store(|s| Ok(s.data.modules.iter().map(ModuleView::from).collect()))
    }

    /// The journal, newest first. `family` narrows it to one kind of event;
    /// without it, everything but the running commentary.
    pub fn journal(&self, family: Option<Family>) -> Result<Vec<LogEntry>, CoreError> {
        self.with_store(|s| Ok(s.data.journal(family)))
    }

    /// Whether the sealed chain of answers and pairings still holds.
    pub fn audit_health(&self) -> Result<AuditHealth, CoreError> {
        self.with_store(|s| Ok(s.data.verify_audit()))
    }

    /// Throw away the commentary. The sealed audit entries stay: they are the
    /// evidence, and dropping them would break the chain.
    pub fn clear_trace(&self) -> Result<(), CoreError> {
        if let Ok(mut pending) = self.pending_trace.lock() {
            pending.clear();
        }
        crate::diag::clear();
        self.with_store(|s| s.update(|d| d.trace.clear()))
    }

    /// A push arrived (or was tapped): the webview knows the title, body and
    /// payload, so it hands them here for the journal.
    pub fn log_push(&self, title: Option<String>, body: Option<String>, data: Option<String>, tapped: bool) -> Result<(), CoreError> {
        let mut note = Note::new(if tapped { "push.tapped" } else { "push.received" }, Family::Push, Level::Plain, title.clone().unwrap_or_else(|| "Push received".into()))
            .about(if tapped { "tapped the notification" } else { "woke the app" });
        if let Some(body) = body.filter(|b| !b.is_empty()) {
            note = note.field("Body", body);
        }
        if title.is_none() {
            note = note.field("Kind", "data only, no notification text");
        }
        if let Some(data) = data.filter(|d| !d.is_empty()) {
            note = note.raw(data);
        }
        self.record(note)
    }

    /// Lifecycle and lock events, which only the webview can see. The kind is
    /// checked against a list so the journal cannot be filled with anything.
    pub fn log_app(&self, kind: &str, title: &str, summary: &str) -> Result<(), CoreError> {
        let (kind, level) = match kind {
            "app.foreground" => ("app.foreground", Level::Plain),
            "app.background" => ("app.background", Level::Plain),
            "app.locked" => ("app.locked", Level::Plain),
            "app.unlocked" => ("app.unlocked", Level::Good),
            "app.lock_failed" => ("app.lock_failed", Level::Bad),
            "app.failed" => ("app.failed", Level::Bad),
            other => return Err(CoreError::State(format!("not a journal event: {other}"))),
        };
        self.record(Note::new(kind, Family::App, level, title).about(summary))
    }

    pub fn last_refresh(&self) -> RefreshReport {
        self.last_refresh.lock().map(|r| r.clone()).unwrap_or_default()
    }

    pub fn push_token(&self) -> Result<Option<String>, CoreError> {
        self.with_store(|s| Ok(s.data.push_token.clone()))
    }

    // ---- pairing -------------------------------------------------------------

    pub async fn pair_scan(&self, raw: &str) -> Result<PairingPreview, CoreError> {
        // A code that is not a pairing code at all is nobody's business; from here
        // on, a refused pairing is worth keeping.
        let code = PairingCode::parse(raw)?;
        let out = self.read_pairing_code(code).await;
        if let Err(e) = &out {
            let _ = self.record(Note::new("pair.failed", Family::Modules, Level::Bad, "Pairing code refused").about(ErrorView::of(e).message).because(e));
        }
        out
    }

    async fn read_pairing_code(&self, code: PairingCode) -> Result<PairingPreview, CoreError> {
        self.note(format!("pair: code link={} host={} user={} hash={}", brief(&code.link, 24), code.hostname, code.user, brief(&code.hash, 6)));
        let offer = match self.client.pairing_offer(&code.link).await {
            Ok(o) => o,
            // The Manager shows the code for a short time; past that the broker no longer knows the link.
            Err(NotifyError::Expired) => return Err(CoreError::Notify(NotifyError::Rejected(404, "the pairing code has expired; show a fresh one in the Manager".into()))),
            Err(e) => return Err(e.into()),
        };
        // [v2] The QR carries a hash of the request the link must serve.
        if !code.hash.is_empty() {
            use sha2::{Digest, Sha256};
            let digest = b64(&Sha256::digest(offer.request.as_bytes()));
            if digest != code.hash.trim() {
                return Err(CoreError::State("the pairing link served a different request than the code promised".into()));
            }
        }
        let request = PairingRequest::from_jwt(&offer.request)?;
        self.note(format!("pair: offer jti={} exp={:?} iss={} aud={} ipinfo={:?}", brief(&request.jti, 6), request.exp, brief(&request.iss, 6), brief(&request.aud, 6), offer.ipinfo_eid));
        if request.expired(now()) {
            return Err(CoreError::Notify(NotifyError::Rejected(410, "the pairing request has expired; show a fresh code in the Manager".into())));
        }
        let already = self.with_store(|s| Ok(s.data.modules.iter().any(|m| m.eid == request.iss)))?;
        let preview = PairingPreview { link: code.link.clone(), user: code.user.clone(), hostname: code.hostname.clone(), email: code.email.clone(), issuer: offer.ipinfo_eid.clone(), already_paired: already };
        self.record(
            Note::new("pair.scanned", Family::Modules, Level::Plain, "Pairing code read")
                .about(if code.hash.is_empty() { "the code carried no hash to check".into() } else { format!("hash matched the link · {}", code.hostname) })
                .mono("Link", code.link.clone())
                .field("Host", code.hostname.clone())
                .field("Account", if code.email.is_empty() { code.user.clone() } else { code.email.clone() })
                .field("Already paired", if already { "yes, this module is in the list" } else { "no" })
                .field("Asked from", place(&offer.ipinfo_eid))
                .mono("Request expires", request.exp.map(fmt_at).unwrap_or_else(|| "not stated".into())),
        )?;
        *self.pairing.lock().map_err(|_| CoreError::State("pairing poisoned".into()))? = Some(PendingPairing { code, request });
        Ok(preview)
    }

    fn take_pairing(&self) -> Result<PendingPairing, CoreError> {
        self.pairing.lock().map_err(|_| CoreError::State("pairing poisoned".into()))?.take().ok_or_else(|| CoreError::State("no pairing in progress".into()))
    }

    pub async fn pair_confirm(&self, label: &str, fid: &str) -> Result<ModuleView, CoreError> {
        let p = self.take_pairing()?;
        let app = KeyPair::generate();
        let reply = pairing::reply(&p.request, &app, &self.device_name, fid)?;
        self.note(format!("pair: POST link label={:?} fid={} aid={} mac={} reply={}", self.device_name, brief(fid, 8), brief(&reply.aid, 6), brief(&reply.mac, 6), brief(&reply.reply, 12)));
        let paired = match self.client.pairing_accept(&p.code.link, &reply).await {
            Ok(x) => x,
            Err(e) => {
                let e = CoreError::from(e);
                self.note(format!("pair: POST link failed: {e}"));
                self.record(
                    Note::new("pair.failed", Family::Modules, Level::Bad, "Pairing was not completed")
                        .about(format!("{} refused the reply", p.code.hostname))
                        .field("Host", p.code.hostname.clone())
                        .because(&e),
                )?;
                return Err(e);
            }
        };
        self.note(format!("pair: broker answered pid={}", brief(&paired.pid, 8)));
        let label = if label.trim().is_empty() { p.code.hostname.clone() } else { label.trim().to_string() };
        let record = ModuleRecord {
            pid: paired.pid.clone(),
            eid: p.request.iss.clone(),
            aid: b64(&app.public_bytes()),
            aid_prv: b64(&app.secret_bytes()),
            label,
            host: p.code.hostname.clone(),
            user: p.code.user.clone(),
            email: p.code.email.clone(),
            iat: now(),
            last_used: None,
        };
        let view = ModuleView::from(&record);
        let aid = record.aid.clone();
        self.with_store(|s| s.update(|d| { d.modules.retain(|m| m.pid != record.pid); d.modules.push(record); }))?;
        self.record(
            Note::new("pair.paired", Family::Modules, Level::Good, "Paired with a module")
                .pid(&paired.pid)
                .about(format!("{} · {}", view.label, view.host))
                .outcome(Outcome::Paired)
                .field("Host", view.host.clone())
                .field("Account", if view.email.is_empty() { view.user.clone() } else { view.email.clone() })
                .field("Label on this phone", view.label.clone())
                .field("Name the module sees", self.device_name.clone())
                .mono("pid", paired.pid.clone())
                .mono("This phone's key", aid),
        )?;
        let _ = self.with_store(|s| s.update(|d| d.push_token = Some(fid.to_string())));
        Ok(view)
    }

    pub async fn pair_refuse(&self, fid: &str) -> Result<(), CoreError> {
        let p = self.take_pairing()?;
        let app = KeyPair::generate();
        let reply = pairing::reply(&p.request, &app, &self.device_name, fid)?;
        let refused = self.client.pairing_refuse(&p.code.link, &reply).await;
        self.record(
            Note::new("pair.refused", Family::Modules, Level::Plain, "Pairing refused on this phone")
                .about(format!("{} · refused", p.code.hostname))
                .outcome(Outcome::Denied)
                .field("Host", p.code.hostname.clone())
                .field("Broker told", match &refused { Ok(()) => "yes".to_string(), Err(e) => format!("no: {e}") }),
        )?;
        Ok(())
    }

    // ---- requests ------------------------------------------------------------

    /// Ask the broker what is waiting for every paired module, open each event,
    /// discard what expired or fails its MAC, and return what needs an answer.
    pub async fn refresh(&self) -> Result<Vec<RequestView>, CoreError> {
        let (pids, handled) = self.with_store(|s| Ok((s.data.modules.iter().map(|m| m.pid.clone()).collect::<Vec<_>>(), s.data.handled_events.clone())))?;
        let mut report = RefreshReport { at: now(), pids: pids.iter().map(|p| crate::core::short(p)).collect(), ..Default::default() };
        let (pending, raw) = match self.client.pending(&pids).await {
            Ok(x) => x,
            Err(e) => {
                report.error = Some(e.to_string());
                if let Ok(mut r) = self.last_refresh.lock() { *r = report; }
                let e = CoreError::from(e);
                let _ = self.record(
                    Note::new("broker.error", Family::Broker, Level::Bad, "Broker could not be asked")
                        .about(ErrorView::of(&e).message)
                        .mono("Endpoint", "POST /notify/event/data/allbypid")
                        .field("Modules asked for", pids.len().to_string())
                        .because(&e),
                );
                return Err(e);
            }
        };
        report.broker_said = raw.chars().take(400).collect();
        report.pending = pending.len();
        self.note(format!("allbypid for {} pid(s) [{}]: {}", pids.len(), report.pids.join(" "), report.broker_said));
        let mut out = Vec::new();
        for p in pending {
            if handled.contains(&p.event_id) {
                report.discarded.push(format!("{}: already handled", short(&p.event_id)));
                continue;
            }
            if let Some(v) = self.live.lock().ok().and_then(|l| l.get(&p.event_id).map(|r| self.view_of(&p.event_id, r))) {
                out.push(v);
                continue;
            }
            match self.open(&p.event_id, &p.pid).await {
                Ok(v) => out.push(v),
                Err(CoreError::Notify(NotifyError::Expired)) | Err(CoreError::Protocol(encedo_protocol::Error::Expired)) => {
                    let told = self.client.deny(&p.event_id, &p.pid).await;
                    self.discard(
                        &p.event_id,
                        Note::new("request.expired", Family::Answers, Level::Plain, "Request expired")
                            .pid(&p.pid)
                            .about("expired before it was shown")
                            .outcome(Outcome::Expired)
                            .mono("Event", p.event_id.clone())
                            .field("Broker told", match &told { Ok(()) => "yes, refused on your behalf".into(), Err(e) => format!("no: {e}") }),
                    )?;
                    report.discarded.push(format!("{}: expired", short(&p.event_id)));
                }
                Err(CoreError::Protocol(encedo_protocol::Error::BadMac)) => {
                    self.discard(
                        &p.event_id,
                        Note::new("request.rejected", Family::Answers, Level::Bad, "Request refused by this phone")
                            .pid(&p.pid)
                            .about("the scope did not match its MAC")
                            .outcome(Outcome::Rejected)
                            .mono("Event", p.event_id.clone())
                            .field("Why", "The scope and its MAC disagree, so the phone never showed the request and never answered it.")
                            .field("Broker told", "nothing; the module will time out"),
                    )?;
                    report.discarded.push(format!("{}: bad scope MAC", short(&p.event_id)));
                }
                Err(CoreError::Notify(NotifyError::HandledElsewhere)) | Err(CoreError::Notify(NotifyError::Cancelled)) => {
                    self.discard(
                        &p.event_id,
                        Note::new("request.cancelled", Family::Answers, Level::Plain, "Request gone")
                            .pid(&p.pid)
                            .about("answered elsewhere or withdrawn")
                            .outcome(Outcome::Cancelled)
                            .mono("Event", p.event_id.clone())
                            .field("Broker said", "410, so another phone answered or the module withdrew it"),
                    )?;
                    report.discarded.push(format!("{}: gone (410)", short(&p.event_id)));
                }
                Err(e) => {
                    report.error = Some(format!("{}: {e}", short(&p.event_id)));
                    if let Ok(mut r) = self.last_refresh.lock() { *r = report; }
                    return Err(e);
                }
            }
        }
        report.shown = out.len();
        // Nothing is written when there was nobody to ask about: the journal says
        // what happened, and a check that did not happen is not an event.
        // Folded at the write, not only on the screen: a check every 15 s must not
        // eat ninety days of history.
        if !pids.is_empty() {
            self.record(
                Note::new("broker.checked", Family::Broker, Level::Plain, "Broker checked")
                    .about(if report.pending == 0 { "nothing waiting".to_string() } else { format!("{} waiting, {} shown", report.pending, report.shown) })
                    .mono("Asked for", report.pids.join(" "))
                    .field("Waiting / shown", format!("{} / {}", report.pending, report.shown))
                    .field("Discarded", if report.discarded.is_empty() { "none".into() } else { report.discarded.join(" · ") })
                    .raw(report.broker_said.clone()),
            )?;
        }
        if let Ok(mut r) = self.last_refresh.lock() { *r = report; }
        Ok(out)
    }

    async fn open(&self, event_id: &str, pid: &str) -> Result<RequestView, CoreError> {
        let module = self.with_store(|s| Ok(s.data.module(pid).cloned()))?.ok_or_else(|| CoreError::State(format!("unknown pid {pid}")))?;
        let event = self.client.event(event_id, pid).await?;
        self.note(format!("event {}: jti={} exp={} epk={} scope={}", brief(event_id, 6), brief(&event.jti, 6), event.exp, brief(&event.epk, 6), brief(&event.scope, 12)));
        let app = KeyPair::from_secret(b64_decode_32(&module.aid_prv).ok_or(CoreError::State("bad aid_prv".into()))?);
        let eid = b64_decode_32(&module.eid).ok_or(CoreError::State("bad eid".into()))?;
        let opened = event::open_scope(&event, &app, &eid, now())?;
        let view = describe(&opened.scope);
        if !view.known {
            self.discard(
                event_id,
                Note::new("request.rejected", Family::Answers, Level::Bad, "Unknown request")
                    .pid(pid)
                    .about(opened.scope.clone())
                    .outcome(Outcome::Rejected)
                    .mono("Scope", opened.scope.clone())
                    .mono("Event", event_id.to_string())
                    .field("Why", "This app does not know the scope, so it cannot say what would be allowed."),
            )?;
            return Err(CoreError::State(format!("unknown scope {}", opened.scope)));
        }
        self.record(
            Note::new("request.shown", Family::Answers, Level::Plain, "Request shown")
                .pid(pid)
                .about(format!("{} · expires {}", opened.scope, fmt_at(event.exp)))
                .mono("Scope", opened.scope.clone())
                .mono("Event", event_id.to_string())
                .mono("jti", event.jti.clone())
                .field("Expires", fmt_at(event.exp))
                .field("Module", module.label.clone())
                .field("Asked from", place(&event.ipinfo_eid)),
        )?;
        let live = LiveRequest { pid: pid.into(), event, scope: opened.scope, view };
        let v = self.view_of(event_id, &live);
        self.live.lock().map_err(|_| CoreError::State("live poisoned".into()))?.insert(event_id.into(), live);
        Ok(v)
    }

    fn view_of(&self, id: &str, r: &LiveRequest) -> RequestView {
        let (label, host) = self.with_store(|s| Ok(s.data.module(&r.pid).map(|m| (m.label.clone(), m.host.clone())))).ok().flatten().unwrap_or_default();
        RequestView { id: id.into(), pid: r.pid.clone(), module_label: label, host, scope: r.scope.clone(), exp: r.event.exp, issuer: r.event.ipinfo_eid.clone(), view: r.view.clone() }
    }

    /// This request will never be shown: remember it as handled and say why.
    fn discard(&self, event_id: &str, note: Note) -> Result<(), CoreError> {
        self.with_store(|s| s.update(|d| d.mark_handled(event_id)))?;
        self.record(note)
    }

    fn take_live(&self, id: &str) -> Result<LiveRequest, CoreError> {
        self.live.lock().map_err(|_| CoreError::State("live poisoned".into()))?.remove(id).ok_or_else(|| CoreError::State("request no longer open".into()))
    }

    pub async fn allow(&self, id: &str, period_secs: u64, writable: bool) -> Result<AnswerView, CoreError> {
        let r = self.take_live(id)?;
        let module = self.with_store(|s| Ok(s.data.module(&r.pid).cloned()))?.ok_or_else(|| CoreError::State("module gone".into()))?;
        let app = KeyPair::from_secret(b64_decode_32(&module.aid_prv).ok_or(CoreError::State("bad aid_prv".into()))?);
        let period = if event::PERIODS.contains(&period_secs) { period_secs } else { event::PERIODS[0] };
        let reply = event::allow(&r.event, &app, &module.eid, &module.pid, &r.scope, writable, now(), period)?;
        let detail = format!("{} · {}", reply.scope, period_label(period));
        let result = self.client.allow(id, &r.pid, &reply).await;
        self.note(format!("allow {} scope={} period={}: {:?}", brief(id, 6), reply.scope, period, result));
        self.with_store(|s| s.update(|d| { d.mark_handled(id); if let Some(m) = d.modules.iter_mut().find(|m| m.pid == r.pid) { m.last_used = Some(now()); } }))?;
        let note = Note::new("request.granted", Family::Answers, Level::Good, r.view.title.clone())
            .pid(&r.pid)
            .mono("Granted scope", reply.scope.clone())
            .mono("Asked scope", r.scope.clone())
            .field("For", period_label(period))
            .field("Until", fmt_at(reply.exp))
            .field("Writing", if writable { "allowed" } else { "not allowed" })
            .mono("Event", id.to_string())
            .field("Asked from", place(&r.event.ipinfo_eid));
        self.finish(&r, result, &detail, Outcome::Granted, note)
    }

    pub async fn deny(&self, id: &str) -> Result<AnswerView, CoreError> {
        let r = self.take_live(id)?;
        let result = self.client.deny(id, &r.pid).await;
        self.note(format!("deny {}: {:?}", brief(id, 6), result));
        self.with_store(|s| s.update(|d| d.mark_handled(id)))?;
        let note = Note::new("request.denied", Family::Answers, Level::Bad, r.view.title.clone())
            .pid(&r.pid)
            .mono("Refused scope", r.scope.clone())
            .mono("Event", id.to_string())
            .field("Asked from", place(&r.event.ipinfo_eid));
        self.finish(&r, result, &r.scope.clone(), Outcome::Denied, note)
    }

    /// The answer is out; what the broker made of it decides the outcome, and
    /// the journal keeps both the answer and the broker's word on it.
    fn finish(&self, r: &LiveRequest, result: Result<(), NotifyError>, detail: &str, ok: Outcome, note: Note) -> Result<AnswerView, CoreError> {
        let (outcome, detail, said) = match result {
            Ok(()) => (ok, detail.to_string(), "accepted".to_string()),
            Err(NotifyError::Expired) => (Outcome::Expired, format!("{detail} · expired"), "404, the request had expired".into()),
            Err(NotifyError::Cancelled) => (Outcome::Cancelled, format!("{detail} · withdrawn"), "410, the module withdrew it".into()),
            Err(NotifyError::HandledElsewhere) => (Outcome::Cancelled, format!("{detail} · another phone answered"), "410, another phone answered first".into()),
            Err(e) => (Outcome::Error, format!("{detail} · {e}"), format!("not delivered: {e}")),
        };
        let level = match outcome {
            Outcome::Granted | Outcome::Paired => Level::Good,
            Outcome::Denied | Outcome::Error | Outcome::Rejected => Level::Bad,
            _ => Level::Plain,
        };
        // What the entry is called follows what happened, not what was attempted.
        let kind = match outcome {
            Outcome::Granted => "request.granted",
            Outcome::Denied => "request.denied",
            Outcome::Expired => "request.expired",
            Outcome::Cancelled => "request.cancelled",
            Outcome::Rejected => "request.rejected",
            _ => "request.error",
        };
        self.record(note.kind(kind).level(level).about(detail.clone()).outcome(outcome).field("Broker said", said))?;
        Ok(AnswerView { outcome, title: r.view.title.clone(), detail })
    }

    // ---- unpair, push ------------------------------------------------------------

    pub async fn unpair(&self, pid: &str) -> Result<(), CoreError> {
        let module = self.with_store(|s| Ok(s.data.module(pid).cloned()))?.ok_or_else(|| CoreError::State("module gone".into()))?;
        let app = KeyPair::from_secret(b64_decode_32(&module.aid_prv).ok_or(CoreError::State("bad aid_prv".into()))?);
        let session = match self.client.session(&module.aid).await {
            Ok(s) => s,
            Err(e) => {
                let e = CoreError::from(e);
                self.note(format!("unpair {}: session failed: {e}", brief(pid, 8)));
                self.record(
                    Note::new("unpair.failed", Family::Modules, Level::Bad, "Unpairing did not start")
                        .pid(pid)
                        .about(format!("{} · the broker would not open a session", module.label))
                        .because(&e),
                )?;
                return Err(e);
            }
        };
        let mut nonce = [0u8; 32];
        OsRng.fill_bytes(&mut nonce);
        let body = unpair::delete_body(pid, &session.epk, &app, &nonce)?;
        if let Err(e) = self.client.unsubscribe(&body).await {
            let e = CoreError::from(e);
            self.note(format!("unpair {}: subscribers/delete failed: {e}", brief(pid, 8)));
            self.record(
                Note::new("unpair.failed", Family::Modules, Level::Bad, "Broker would not unsubscribe")
                    .pid(pid)
                    .about(format!("{} · still paired", module.label))
                    .mono("Endpoint", "POST /notify/subscribers/delete")
                    .field("What it means", "The module can still think this phone answers for it. Remove the phone in the Manager, or remove the module from this phone only.")
                    .because(&e),
            )?;
            return Err(e);
        }
        self.note(format!("unpair {}: broker ok", brief(pid, 8)));
        self.remove_module(pid, "unpair.done", Level::Plain, "unpaired from this phone, and the broker agreed")
    }

    /// Drop the module locally without the broker's agreement (it said 404, or is unreachable).
    pub fn forget(&self, pid: &str) -> Result<(), CoreError> {
        self.note(format!("forget {}: removed locally only", brief(pid, 8)));
        self.remove_module(pid, "unpair.local", Level::Bad, "removed from this phone only; the broker did not confirm")
    }

    fn remove_module(&self, pid: &str, kind: &'static str, level: Level, why: &str) -> Result<(), CoreError> {
        let label = self.with_store(|s| s.update(|d| { let l = d.module(pid).map(|m| m.label.clone()); d.modules.retain(|m| m.pid != pid); l }))?;
        let label = label.unwrap_or_default();
        self.record(
            Note::new(kind, Family::Modules, level, "Module unpaired")
                .pid(pid)
                .about(format!("{label} · {why}"))
                .outcome(Outcome::Unpaired)
                .field("Module", label)
                .mono("pid", pid.to_string())
                .field("What happened", why.to_string()),
        )
    }

    /// FCM `data.encedo` payload. Returns true when the UI should refresh.
    pub fn push_payload(&self, encedo: &str) -> Result<bool, CoreError> {
        let v: serde_json::Value = serde_json::from_str(encedo).map_err(|e| CoreError::State(format!("push json: {e}")))?;
        if let Some(p) = v.get("pairing") {
            if p.get("status").and_then(|s| s.as_str()) == Some("DELETED") {
                if let Some(pid) = p.get("pid").and_then(|s| s.as_str()) {
                    self.remove_module(pid, "unpair.remote", Level::Bad, "the module unpaired this phone")?;
                }
            }
            return Ok(false);
        }
        Ok(v.get("event").is_some())
    }

    /// Remember the push token; the [v2] broker endpoint is called when it
    /// exists (404 from today's backend is not an error).
    pub async fn push_token_changed(&self, fid: &str) -> Result<(), CoreError> {
        let same = self.with_store(|s| Ok(s.data.push_token.as_deref() == Some(fid)))?;
        if same {
            return Ok(());
        }
        let modules = self.with_store(|s| Ok(s.data.modules.clone()))?;
        let mut told: Vec<String> = Vec::new();
        for m in modules {
            let Some(secret) = b64_decode_32(&m.aid_prv) else { continue };
            let app = KeyPair::from_secret(secret);
            let Ok(session) = self.client.session(&m.aid).await else { continue };
            let mut nonce = [0u8; 32];
            OsRng.fill_bytes(&mut nonce);
            if let Ok(body) = unpair::token_body(&m.pid, &session.epk, &app, &nonce, fid) {
                let said = match self.client.push_token(&body).await {
                    Ok(()) => "accepted".to_string(),
                    Err(NotifyError::Expired) => "404, the endpoint is not deployed yet".into(),
                    Err(e) => format!("refused: {e}"),
                };
                told.push(format!("{}: {said}", m.label));
            }
        }
        self.record(
            Note::new("push.token_changed", Family::Push, Level::Plain, "Push token changed")
                .about(if told.is_empty() { "no module to tell".to_string() } else { told.join(" · ") })
                .mono("New token", fid.to_string())
                .field("Modules told", told.len().to_string()),
        )?;
        self.with_store(|s| s.update(|d| d.push_token = Some(fid.to_string())))
    }
}

/// Where the issuer looked to be, as the broker reported it.
fn place(info: &Option<IpInfo>) -> String {
    match info {
        Some(i) if !i.city.is_empty() || !i.country.is_empty() => format!("{}, {} · {}", i.city, i.country, i.ip),
        Some(i) => i.ip.clone(),
        None => "unknown place".into(),
    }
}

/// A unix time as the journal shows it: the day and the second, in UTC, so an
/// entry copied out of the app still says when it happened.
pub fn fmt_at(secs: u64) -> String {
    // Civil date from a unix timestamp (Howard Hinnant's algorithm), no chrono.
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = era * 400 + yoe + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02} UTC", rem / 3600, (rem % 3600) / 60, rem % 60)
}

/// First and last four characters, enough to tell ids apart in a report.
pub fn short(s: &str) -> String {
    if s.len() <= 12 { s.to_string() } else { format!("{}…{}", &s[..4], &s[s.len() - 4..]) }
}

pub fn period_label(secs: u64) -> String {
    match secs {
        900 => "15 min".into(),
        3600 => "1 h".into(),
        28800 => "8 h".into(),
        86400 => "24 h".into(),
        s => format!("{} min", s / 60),
    }
}
