//! HTTP client for `api.encedo.com/notify`, the broker between a HEM and its
//! paired phones. Endpoints and status codes are the ones v1 used
//! (`docs/PROTOCOL.md`); every error is typed so the UI can say what happened.

use std::collections::HashMap;
use std::time::Duration;

use encedo_protocol::codec::pid_to_path;
use encedo_protocol::event::{AllowReply, Event, IpInfo};
use encedo_protocol::pairing::PairingReply;
use encedo_protocol::unpair::{DeleteBody, TokenBody};
use reqwest::{Method, Response, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub const DEFAULT_BASE: &str = "https://api.encedo.com";
const TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum NotifyError {
    #[error("no network: {0}")]
    Network(String),
    #[error("the broker did not answer in time")]
    Timeout,
    /// 401: v1 treats it as fatal for the pairing.
    #[error("the broker rejected this pairing (401)")]
    Unauthorized,
    /// 404 on an event: it expired.
    #[error("the request expired (404)")]
    Expired,
    /// 410 with `{reason: "cancelled"}`: withdrawn on the HEM side.
    #[error("the request was cancelled on the module (410)")]
    Cancelled,
    /// 410 otherwise: another authenticator answered first.
    #[error("another phone already answered (410)")]
    HandledElsewhere,
    /// 5xx.
    #[error("the broker is unavailable ({0})")]
    Unavailable(u16),
    /// Anything else non-2xx.
    #[error("the broker rejected the request ({0}): {1}")]
    Rejected(u16, String),
    #[error("unexpected answer from the broker: {0}")]
    BadResponse(String),
}

impl NotifyError {
    /// Stable identifier for the webview.
    pub fn code(&self) -> &'static str {
        match self {
            NotifyError::Network(_) => "network",
            NotifyError::Timeout => "timeout",
            NotifyError::Unauthorized => "unauthorized",
            NotifyError::Expired => "expired",
            NotifyError::Cancelled => "cancelled",
            NotifyError::HandledElsewhere => "handled_elsewhere",
            NotifyError::Unavailable(_) => "unavailable",
            NotifyError::Rejected(..) => "rejected",
            NotifyError::BadResponse(_) => "bad_response",
        }
    }
}

/// What the Manager's QR code carries.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PairingCode {
    pub link: String,
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub hostname: String,
    #[serde(default)]
    pub email: String,
}

impl PairingCode {
    /// v1 Manager: JSON `{link, user, hostname, email}`. A bare https link is
    /// accepted too, so a future Manager that encodes only the link still pairs.
    pub fn parse(raw: &str) -> Result<Self, NotifyError> {
        let raw = raw.trim();
        let code: PairingCode = if raw.starts_with("https://") {
            PairingCode { link: raw.to_string(), user: String::new(), hostname: String::new(), email: String::new() }
        } else {
            serde_json::from_str(raw).map_err(|_| NotifyError::BadResponse("this code is not an Encedo pairing code".into()))?
        };
        if !code.link.starts_with("https://") {
            return Err(NotifyError::BadResponse("the pairing link is not https".into()));
        }
        Ok(code)
    }
}

/// `GET link`.
#[derive(Debug, Clone, Deserialize)]
pub struct PairingOffer {
    pub request: String,
    #[serde(default)]
    pub ipinfo_eid: Option<IpInfo>,
}

/// `POST link`.
#[derive(Debug, Clone, Deserialize)]
pub struct Paired {
    pub pid: String,
}

/// `POST /notify/event/data/allbypid`. An empty list comes back as an array,
/// a non-empty one as an object `{eventId: pid}`.
#[derive(Debug, Clone, Deserialize)]
struct AllByPidRaw {
    #[serde(default)]
    eventid: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingEvent {
    pub event_id: String,
    pub pid: String,
}

/// `POST /notify/session`.
#[derive(Debug, Clone, Deserialize)]
pub struct Session {
    pub epk: String,
}

#[derive(Serialize)]
struct AidBody<'a> {
    aid: &'a str,
}

#[derive(Serialize)]
struct PidsBody<'a> {
    pid: &'a [String],
}

#[derive(Clone)]
pub struct NotifyClient {
    http: reqwest::Client,
    base: String,
}

impl NotifyClient {
    pub fn new(base: &str) -> Self {
        let http = reqwest::Client::builder()
            .timeout(TIMEOUT)
            .user_agent(concat!("EncedoAuthenticator/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("reqwest client");
        Self { http, base: base.trim_end_matches('/').to_string() }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    // ---- pairing ---------------------------------------------------------

    pub async fn pairing_offer(&self, link: &str) -> Result<PairingOffer, NotifyError> {
        self.json(Method::GET, link, None::<&()>).await
    }

    pub async fn pairing_accept(&self, link: &str, reply: &PairingReply) -> Result<Paired, NotifyError> {
        self.json(Method::POST, link, Some(reply)).await
    }

    pub async fn pairing_refuse(&self, link: &str, reply: &PairingReply) -> Result<(), NotifyError> {
        self.send(Method::DELETE, link, Some(reply)).await.map(|_| ())
    }

    // ---- events ----------------------------------------------------------

    pub async fn pending(&self, pids: &[String]) -> Result<Vec<PendingEvent>, NotifyError> {
        if pids.is_empty() {
            return Ok(Vec::new());
        }
        let raw: AllByPidRaw = self.json(Method::POST, &self.url("/notify/event/data/allbypid"), Some(&PidsBody { pid: pids })).await?;
        let map: HashMap<String, String> = match raw.eventid {
            serde_json::Value::Object(m) => m.into_iter().filter_map(|(k, v)| v.as_str().map(|p| (k, p.to_string()))).collect(),
            _ => HashMap::new(),
        };
        let mut out: Vec<PendingEvent> = map.into_iter().map(|(event_id, pid)| PendingEvent { event_id, pid }).collect();
        out.sort_by(|a, b| a.event_id.cmp(&b.event_id));
        Ok(out)
    }

    fn event_url(&self, event_id: &str, pid: &str) -> String {
        self.url(&format!("/notify/event/data/{event_id}/{}", pid_to_path(pid)))
    }

    pub async fn event(&self, event_id: &str, pid: &str) -> Result<Event, NotifyError> {
        self.json(Method::GET, &self.event_url(event_id, pid), None::<&()>).await
    }

    pub async fn allow(&self, event_id: &str, pid: &str, reply: &AllowReply) -> Result<(), NotifyError> {
        self.send(Method::POST, &self.event_url(event_id, pid), Some(reply)).await.map(|_| ())
    }

    pub async fn deny(&self, event_id: &str, pid: &str) -> Result<(), NotifyError> {
        self.send(Method::DELETE, &self.event_url(event_id, pid), Some(&serde_json::json!({}))).await.map(|_| ())
    }

    // ---- session, unpair, token -------------------------------------------

    pub async fn session(&self, aid: &str) -> Result<Session, NotifyError> {
        self.json(Method::POST, &self.url("/notify/session"), Some(&AidBody { aid })).await
    }

    pub async fn unsubscribe(&self, body: &DeleteBody) -> Result<(), NotifyError> {
        self.send(Method::POST, &self.url("/notify/subscribers/delete"), Some(body)).await.map(|_| ())
    }

    /// [v2] endpoint; 404 from the current backend means "not deployed yet".
    pub async fn push_token(&self, body: &TokenBody) -> Result<(), NotifyError> {
        self.send(Method::POST, &self.url("/notify/subscribers/token"), Some(body)).await.map(|_| ())
    }

    // ---- plumbing --------------------------------------------------------

    async fn send<B: Serialize>(&self, method: Method, url: &str, body: Option<&B>) -> Result<Response, NotifyError> {
        let mut req = self.http.request(method, url);
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req.send().await.map_err(|e| {
            if e.is_timeout() { NotifyError::Timeout } else { NotifyError::Network(e.without_url().to_string()) }
        })?;
        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }
        let text = resp.text().await.unwrap_or_default();
        Err(match status {
            StatusCode::UNAUTHORIZED => NotifyError::Unauthorized,
            StatusCode::NOT_FOUND => NotifyError::Expired,
            StatusCode::GONE => {
                let cancelled = serde_json::from_str::<serde_json::Value>(&text)
                    .ok()
                    .and_then(|v| v.get("reason").and_then(|r| r.as_str()).map(|r| r == "cancelled"))
                    .unwrap_or(false);
                if cancelled { NotifyError::Cancelled } else { NotifyError::HandledElsewhere }
            }
            s if s.is_server_error() => NotifyError::Unavailable(s.as_u16()),
            s => NotifyError::Rejected(s.as_u16(), text.chars().take(200).collect()),
        })
    }

    async fn json<B: Serialize, T: DeserializeOwned>(&self, method: Method, url: &str, body: Option<&B>) -> Result<T, NotifyError> {
        let resp = self.send(method, url, body).await?;
        let text = resp.text().await.map_err(|e| NotifyError::Network(e.to_string()))?;
        serde_json::from_str(&text).map_err(|e| NotifyError::BadResponse(format!("{e}: {}", text.chars().take(120).collect::<String>())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qr_code_needs_https_link() {
        assert!(PairingCode::parse(r#"{"link":"http://x/y","user":"u"}"#).is_err());
        let c = PairingCode::parse(r#"{"link":"https://api.encedo.com/notify/pairing/abc","user":"chris","hostname":"my.ence.do"}"#).unwrap();
        assert_eq!(c.hostname, "my.ence.do");
        assert_eq!(c.email, "");
        let bare = PairingCode::parse(" https://api.encedo.com/notify/pairing/abc \n").unwrap();
        assert_eq!(bare.link, "https://api.encedo.com/notify/pairing/abc");
        assert!(PairingCode::parse("hello").is_err());
    }

    #[tokio::test]
    async fn pending_handles_empty_array_and_object() {
        // Shape only; no network. Exercised through `AllByPidRaw` parsing.
        let arr: AllByPidRaw = serde_json::from_str(r#"{"eventid":[]}"#).unwrap();
        assert!(matches!(arr.eventid, serde_json::Value::Array(_)));
        let obj: AllByPidRaw = serde_json::from_str(r#"{"eventid":{"ev1":"pid1"}}"#).unwrap();
        assert_eq!(obj.eventid["ev1"], "pid1");
    }
}

#[cfg(test)]
mod live {
    //! Talks to the real broker; run with `cargo test --lib live -- --ignored`.
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn broker_answers_with_typed_errors() {
        let c = NotifyClient::new(DEFAULT_BASE);
        // A pairing link that cannot exist: whatever the code, it must be a typed HTTP answer, not a TLS or DNS failure.
        let r = c.pairing_offer(&format!("{DEFAULT_BASE}/notify/pairing/does-not-exist")).await;
        eprintln!("pairing_offer: {r:?}");
        assert!(!matches!(r, Err(NotifyError::Network(_)) | Err(NotifyError::Timeout)), "network layer failed: {r:?}");
        let r = c.pending(&["AAAA".to_string()]).await;
        eprintln!("pending: {r:?}");
        assert!(!matches!(r, Err(NotifyError::Network(_)) | Err(NotifyError::Timeout)), "network layer failed: {r:?}");
        let r = c.session("AAAA").await;
        eprintln!("session: {r:?}");
    }
}
