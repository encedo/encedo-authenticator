//! The application core: pairing, requests, unpair, push handling. Holds the
//! store and the broker client; hands the webview display data only.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use encedo_protocol::codec::{b64, b64_decode_32};
use encedo_protocol::event::{self, Event, IpInfo};
use encedo_protocol::keys::KeyPair;
use encedo_protocol::pairing::{self, PairingRequest};
use encedo_protocol::unpair;
use rand_core::{OsRng, RngCore};
use serde::Serialize;

use crate::notify::{NotifyClient, NotifyError, PairingCode};
use crate::scope::{describe, ScopeView};
use crate::store::{ArchiveEntry, ModuleRecord, Outcome, Settings, Store, StoreError};

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
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
}

/// What the webview gets on error: a stable code and a sentence.
#[derive(Debug, Serialize, Clone)]
pub struct ErrorView {
    pub code: String,
    pub message: String,
}

impl From<CoreError> for ErrorView {
    fn from(e: CoreError) -> Self {
        let code = match &e {
            CoreError::Notify(n) => n.code().to_string(),
            CoreError::Store(_) => "storage".into(),
            CoreError::Protocol(encedo_protocol::Error::BadMac) => "bad_mac".into(),
            CoreError::Protocol(encedo_protocol::Error::Expired) => "expired".into(),
            CoreError::Protocol(_) => "protocol".into(),
            CoreError::State(_) => "state".into(),
        };
        ErrorView { code, message: e.to_string() }
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
    pub label: String,
    pub host: String,
    pub user: String,
    pub email: String,
    pub paired_at: u64,
    pub last_used: Option<u64>,
}

impl From<&ModuleRecord> for ModuleView {
    fn from(m: &ModuleRecord) -> Self {
        Self { pid: m.pid.clone(), label: m.label.clone(), host: m.host.clone(), user: m.user.clone(), email: m.email.clone(), paired_at: m.iat, last_used: m.last_used }
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

#[derive(Debug, Clone, Serialize)]
pub struct AnswerView {
    pub outcome: Outcome,
    pub title: String,
    pub detail: String,
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

pub struct Core {
    store: Mutex<Store>,
    client: NotifyClient,
    pairing: Mutex<Option<PendingPairing>>,
    live: Mutex<HashMap<String, LiveRequest>>,
    /// Device name for the pairing label, from the OS.
    device_name: String,
}

impl Core {
    pub fn new(store: Store, client: NotifyClient, device_name: String) -> Self {
        Self { store: Mutex::new(store), client, pairing: Mutex::new(None), live: Mutex::new(HashMap::new()), device_name }
    }

    fn with_store<T>(&self, f: impl FnOnce(&mut Store) -> Result<T, StoreError>) -> Result<T, CoreError> {
        let mut s = self.store.lock().map_err(|_| CoreError::State("store poisoned".into()))?;
        Ok(f(&mut s)?)
    }

    fn archive(&self, pid: &str, title: &str, detail: &str, outcome: Outcome) -> Result<(), CoreError> {
        let id = format!("{}-{}", now(), OsRng.next_u32());
        self.with_store(|s| s.update(|d| d.archive_push(ArchiveEntry { id, pid: pid.into(), title: title.into(), detail: detail.into(), outcome, at: now() })))
    }

    /// The previous app's data was found and removed: say so in the archive,
    /// where it stays until the person pairs again.
    pub fn note_v1_removed(&self, files: &[String]) -> Result<(), CoreError> {
        self.archive("", "Previous version", &format!("data from the old app removed ({}); pair your modules again", files.join(", ")), Outcome::Unpaired)
    }

    // ---- read side ---------------------------------------------------------

    pub fn settings(&self) -> Result<Settings, CoreError> {
        self.with_store(|s| Ok(s.data.settings.clone()))
    }

    pub fn set_settings(&self, settings: Settings) -> Result<(), CoreError> {
        self.with_store(|s| s.update(|d| d.settings = settings))
    }

    pub fn modules(&self) -> Result<Vec<ModuleView>, CoreError> {
        self.with_store(|s| Ok(s.data.modules.iter().map(ModuleView::from).collect()))
    }

    pub fn archive_list(&self) -> Result<Vec<ArchiveEntry>, CoreError> {
        self.with_store(|s| Ok(s.data.archive.clone()))
    }

    pub fn push_token(&self) -> Result<Option<String>, CoreError> {
        self.with_store(|s| Ok(s.data.push_token.clone()))
    }

    // ---- pairing -------------------------------------------------------------

    pub async fn pair_scan(&self, raw: &str) -> Result<PairingPreview, CoreError> {
        let code = PairingCode::parse(raw)?;
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
        if request.expired(now()) {
            return Err(CoreError::Notify(NotifyError::Rejected(410, "the pairing request has expired; show a fresh code in the Manager".into())));
        }
        let already = self.with_store(|s| Ok(s.data.modules.iter().any(|m| m.eid == request.iss)))?;
        let preview = PairingPreview { link: code.link.clone(), user: code.user.clone(), hostname: code.hostname.clone(), email: code.email.clone(), issuer: offer.ipinfo_eid.clone(), already_paired: already };
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
        let paired = match self.client.pairing_accept(&p.code.link, &reply).await {
            Ok(x) => x,
            Err(e) => {
                self.archive("", "Pair this phone", &format!("{}: {}", p.code.hostname, e), Outcome::Error)?;
                return Err(e.into());
            }
        };
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
        self.with_store(|s| s.update(|d| { d.modules.retain(|m| m.pid != record.pid); d.modules.push(record); }))?;
        self.archive(&paired.pid, "Pair this phone", &format!("{} · {}", view.label, view.host), Outcome::Paired)?;
        let _ = self.with_store(|s| s.update(|d| d.push_token = Some(fid.to_string())));
        Ok(view)
    }

    pub async fn pair_refuse(&self, fid: &str) -> Result<(), CoreError> {
        let p = self.take_pairing()?;
        let app = KeyPair::generate();
        let reply = pairing::reply(&p.request, &app, &self.device_name, fid)?;
        let _ = self.client.pairing_refuse(&p.code.link, &reply).await;
        self.archive("", "Pair this phone", &format!("{} · refused", p.code.hostname), Outcome::Denied)?;
        Ok(())
    }

    // ---- requests ------------------------------------------------------------

    /// Ask the broker what is waiting for every paired module, open each event,
    /// discard what expired or fails its MAC, and return what needs an answer.
    pub async fn refresh(&self) -> Result<Vec<RequestView>, CoreError> {
        let (pids, handled) = self.with_store(|s| Ok((s.data.modules.iter().map(|m| m.pid.clone()).collect::<Vec<_>>(), s.data.handled_events.clone())))?;
        let pending = self.client.pending(&pids).await?;
        let mut out = Vec::new();
        for p in pending {
            if handled.contains(&p.event_id) {
                continue;
            }
            if let Some(v) = self.live.lock().ok().and_then(|l| l.get(&p.event_id).map(|r| self.view_of(&p.event_id, r))) {
                out.push(v);
                continue;
            }
            match self.open(&p.event_id, &p.pid).await {
                Ok(v) => out.push(v),
                Err(CoreError::Notify(NotifyError::Expired)) | Err(CoreError::Protocol(encedo_protocol::Error::Expired)) => {
                    let _ = self.client.deny(&p.event_id, &p.pid).await;
                    self.discard(&p.event_id, &p.pid, "Request", "expired before it was shown", Outcome::Expired)?;
                }
                Err(CoreError::Protocol(encedo_protocol::Error::BadMac)) => {
                    self.discard(&p.event_id, &p.pid, "Request", "rejected: scope MAC did not verify", Outcome::Rejected)?;
                }
                Err(CoreError::Notify(NotifyError::HandledElsewhere)) | Err(CoreError::Notify(NotifyError::Cancelled)) => {
                    self.discard(&p.event_id, &p.pid, "Request", "answered elsewhere or withdrawn", Outcome::Cancelled)?;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(out)
    }

    async fn open(&self, event_id: &str, pid: &str) -> Result<RequestView, CoreError> {
        let module = self.with_store(|s| Ok(s.data.module(pid).cloned()))?.ok_or_else(|| CoreError::State(format!("unknown pid {pid}")))?;
        let event = self.client.event(event_id, pid).await?;
        let app = KeyPair::from_secret(b64_decode_32(&module.aid_prv).ok_or(CoreError::State("bad aid_prv".into()))?);
        let eid = b64_decode_32(&module.eid).ok_or(CoreError::State("bad eid".into()))?;
        let opened = event::open_scope(&event, &app, &eid, now())?;
        let view = describe(&opened.scope);
        if !view.known {
            self.discard(event_id, pid, "Unknown request", &opened.scope, Outcome::Rejected)?;
            return Err(CoreError::State(format!("unknown scope {}", opened.scope)));
        }
        let live = LiveRequest { pid: pid.into(), event, scope: opened.scope, view };
        let v = self.view_of(event_id, &live);
        self.live.lock().map_err(|_| CoreError::State("live poisoned".into()))?.insert(event_id.into(), live);
        Ok(v)
    }

    fn view_of(&self, id: &str, r: &LiveRequest) -> RequestView {
        let (label, host) = self.with_store(|s| Ok(s.data.module(&r.pid).map(|m| (m.label.clone(), m.host.clone())))).ok().flatten().unwrap_or_default();
        RequestView { id: id.into(), pid: r.pid.clone(), module_label: label, host, scope: r.scope.clone(), exp: r.event.exp, issuer: r.event.ipinfo_eid.clone(), view: r.view.clone() }
    }

    fn discard(&self, event_id: &str, pid: &str, title: &str, detail: &str, outcome: Outcome) -> Result<(), CoreError> {
        self.with_store(|s| s.update(|d| d.mark_handled(event_id)))?;
        self.archive(pid, title, detail, outcome)
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
        self.with_store(|s| s.update(|d| { d.mark_handled(id); if let Some(m) = d.modules.iter_mut().find(|m| m.pid == r.pid) { m.last_used = Some(now()); } }))?;
        self.finish(&r, result, &detail, Outcome::Granted)
    }

    pub async fn deny(&self, id: &str) -> Result<AnswerView, CoreError> {
        let r = self.take_live(id)?;
        let result = self.client.deny(id, &r.pid).await;
        self.with_store(|s| s.update(|d| d.mark_handled(id)))?;
        self.finish(&r, result, &r.scope.clone(), Outcome::Denied)
    }

    fn finish(&self, r: &LiveRequest, result: Result<(), NotifyError>, detail: &str, ok: Outcome) -> Result<AnswerView, CoreError> {
        let (outcome, detail) = match result {
            Ok(()) => (ok, detail.to_string()),
            Err(NotifyError::Expired) => (Outcome::Expired, format!("{detail} · expired")),
            Err(NotifyError::Cancelled) => (Outcome::Cancelled, format!("{detail} · withdrawn")),
            Err(NotifyError::HandledElsewhere) => (Outcome::Cancelled, format!("{detail} · another phone answered")),
            Err(e) => (Outcome::Error, format!("{detail} · {e}")),
        };
        self.archive(&r.pid, &r.view.title, &detail, outcome)?;
        Ok(AnswerView { outcome, title: r.view.title.clone(), detail })
    }

    // ---- unpair, push ------------------------------------------------------------

    pub async fn unpair(&self, pid: &str) -> Result<(), CoreError> {
        let module = self.with_store(|s| Ok(s.data.module(pid).cloned()))?.ok_or_else(|| CoreError::State("module gone".into()))?;
        let app = KeyPair::from_secret(b64_decode_32(&module.aid_prv).ok_or(CoreError::State("bad aid_prv".into()))?);
        let session = self.client.session(&module.aid).await?;
        let mut nonce = [0u8; 32];
        OsRng.fill_bytes(&mut nonce);
        let body = unpair::delete_body(pid, &session.epk, &app, &nonce)?;
        self.client.unsubscribe(&body).await?;
        self.remove_module(pid, "unpaired from this phone")
    }

    fn remove_module(&self, pid: &str, why: &str) -> Result<(), CoreError> {
        let label = self.with_store(|s| s.update(|d| { let l = d.module(pid).map(|m| m.label.clone()); d.modules.retain(|m| m.pid != pid); l }))?;
        self.archive(pid, "Unpair", &format!("{} · {why}", label.unwrap_or_default()), Outcome::Unpaired)
    }

    /// FCM `data.encedo` payload. Returns true when the UI should refresh.
    pub fn push_payload(&self, encedo: &str) -> Result<bool, CoreError> {
        let v: serde_json::Value = serde_json::from_str(encedo).map_err(|e| CoreError::State(format!("push json: {e}")))?;
        if let Some(p) = v.get("pairing") {
            if p.get("status").and_then(|s| s.as_str()) == Some("DELETED") {
                if let Some(pid) = p.get("pid").and_then(|s| s.as_str()) {
                    self.remove_module(pid, "unpaired from the module")?;
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
        for m in modules {
            let Some(secret) = b64_decode_32(&m.aid_prv) else { continue };
            let app = KeyPair::from_secret(secret);
            let Ok(session) = self.client.session(&m.aid).await else { continue };
            let mut nonce = [0u8; 32];
            OsRng.fill_bytes(&mut nonce);
            if let Ok(body) = unpair::token_body(&m.pid, &session.epk, &app, &nonce, fid) {
                let _ = self.client.push_token(&body).await;
            }
        }
        self.with_store(|s| s.update(|d| d.push_token = Some(fid.to_string())))
    }
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
