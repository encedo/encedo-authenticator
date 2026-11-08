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
    /// base64(SHA-256(request)): the Manager's promise of what the link serves.
    #[serde(default)]
    pub hash: String,
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
    ///
    /// The Manager on the same phone hands the same JSON over as
    /// `encedo-auth://pair?qr=<base64url>` (the "Open in Authenticator" button,
    /// or pasted). That form is unwrapped here, so a scan, a paste and an opened
    /// link all go through one parser; a code that came by link must carry both
    /// `link` and `hash`.
    pub fn parse(raw: &str) -> Result<Self, NotifyError> {
        let raw = raw.trim();
        if let Some(text) = pair_link_text(raw)? {
            let code = Self::parse_text(&text)?;
            if code.hash.trim().is_empty() {
                return Err(NotifyError::BadResponse("the pairing link carries no hash; show a fresh code in the Manager".into()));
            }
            return Ok(code);
        }
        Self::parse_text(raw)
    }

    fn parse_text(raw: &str) -> Result<Self, NotifyError> {
        let raw = raw.trim();
        let code: PairingCode = if raw.starts_with("https://") {
            PairingCode { link: raw.to_string(), hash: String::new(), user: String::new(), hostname: String::new(), email: String::new() }
        } else {
            serde_json::from_str(raw).map_err(|_| NotifyError::BadResponse("this code is not an Encedo pairing code".into()))?
        };
        if !link_allowed(&code.link) {
            return Err(NotifyError::BadResponse("the pairing link is not https".into()));
        }
        Ok(code)
    }
}

/// The scheme the Manager's "Open in Authenticator" button opens.
pub const PAIR_SCHEME: &str = "encedo-auth";

/// `encedo-auth://pair?qr=<base64url of the QR JSON>` → the QR JSON. `None`
/// when `raw` is not in the app's scheme at all (a scanned or pasted JSON); an
/// error when it is, but is not a pairing link this app can read. Only the
/// transport is unwrapped: what the JSON says is checked by the caller.
pub fn pair_link_text(raw: &str) -> Result<Option<String>, NotifyError> {
    let bad = |why: &str| NotifyError::BadResponse(format!("this is not a pairing link: {why}"));
    let Some((scheme, rest)) = raw.split_once(':') else { return Ok(None) };
    if !scheme.eq_ignore_ascii_case(PAIR_SCHEME) {
        return Ok(None);
    }
    let rest = rest.strip_prefix("//").ok_or_else(|| bad("no host"))?;
    let rest = rest.split('#').next().unwrap_or_default();
    let (authority, query) = rest.split_once('?').unwrap_or((rest, ""));
    let host = authority.strip_suffix('/').unwrap_or(authority);
    if !host.eq_ignore_ascii_case("pair") {
        return Err(bad("the link is not for pairing"));
    }
    let qr = query
        .split('&')
        .find_map(|p| p.strip_prefix("qr="))
        .map(percent_decode)
        .filter(|q| !q.is_empty())
        .ok_or_else(|| bad("the link carries no code"))?;
    // base64url without padding; `b64url_decode` also takes padding and the
    // standard alphabet, and refuses anything else.
    let bytes = encedo_protocol::codec::b64url_decode(&qr).ok_or_else(|| bad("the code in the link is damaged"))?;
    let text = String::from_utf8(bytes).map_err(|_| bad("the code in the link is not text"))?;
    Ok(Some(text))
}

/// `%XX` escapes, which a browser may add around `=`. Anything malformed is
/// left as it is, so the base64 decoder refuses it.
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| (c as char).to_digit(16);
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The pairing link must be https. The one exception is a broker on this
/// device, which is how `ENCEDO_BROKER` points the app at a local one in
/// development and how the core tests run theirs; a code naming 127.0.0.1
/// reaches nothing that is not already inside the phone. Matched on the whole
/// host, never as a prefix, so `http://127.0.0.1.example.com/` stays refused.
fn link_allowed(link: &str) -> bool {
    if link.starts_with("https://") {
        return true;
    }
    let Some(rest) = link.strip_prefix("http://") else { return false };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    matches!(authority.split(':').next().unwrap_or_default(), "127.0.0.1" | "localhost")
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

fn pid_of(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(p) => Some(p.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Array(a) => a.first().and_then(pid_of),
        _ => None,
    }
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

    /// Returns the pending events and the broker's raw answer (for the diagnostics card).
    pub async fn pending(&self, pids: &[String]) -> Result<(Vec<PendingEvent>, String), NotifyError> {
        if pids.is_empty() {
            return Ok((Vec::new(), String::from("(no modules paired, broker not asked)")));
        }
        let text = self.text(Method::POST, &self.url("/notify/event/data/allbypid"), Some(&PidsBody { pid: pids })).await?;
        let raw: AllByPidRaw = serde_json::from_str(&text).map_err(|e| NotifyError::BadResponse(format!("{e}: {}", text.chars().take(120).collect::<String>())))?;
        // The broker answers `{eventid: {"<event>": ["<pid>"]}}`: the pid comes as a
        // one-element array. v1 did `String(value)`, which joins an array with
        // commas, so a single pid came out as itself. Accept a string, a number
        // or the first element of an array.
        let map: HashMap<String, String> = match raw.eventid {
            serde_json::Value::Object(m) => m.into_iter().filter_map(|(k, v)| pid_of(&v).map(|p| (k, p))).collect(),
            _ => HashMap::new(),
        };
        let mut out: Vec<PendingEvent> = map.into_iter().map(|(event_id, pid)| PendingEvent { event_id, pid }).collect();
        out.sort_by(|a, b| a.event_id.cmp(&b.event_id));
        Ok((out, text))
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

    async fn text<B: Serialize>(&self, method: Method, url: &str, body: Option<&B>) -> Result<String, NotifyError> {
        let resp = self.send(method, url, body).await?;
        resp.text().await.map_err(|e| NotifyError::Network(e.to_string()))
    }

    async fn json<B: Serialize, T: DeserializeOwned>(&self, method: Method, url: &str, body: Option<&B>) -> Result<T, NotifyError> {
        let text = self.text(method, url, body).await?;
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
        // A broker on this device is the one exception (`ENCEDO_BROKER`, tests).
        assert!(PairingCode::parse(r#"{"link":"http://127.0.0.1:1420/notify/pairing/abc"}"#).is_ok());
        assert!(PairingCode::parse(r#"{"link":"http://192.0.2.1/notify/pairing/abc"}"#).is_err());
        // A host that only begins with the loopback address is a different host.
        assert!(PairingCode::parse(r#"{"link":"http://127.0.0.1.example.com/notify/pairing/abc"}"#).is_err());
        let bare = PairingCode::parse(" https://api.encedo.com/notify/pairing/abc \n").unwrap();
        assert_eq!(bare.link, "https://api.encedo.com/notify/pairing/abc");
        assert!(PairingCode::parse("hello").is_err());
    }

    /// The example the Manager's task gives, link and JSON byte for byte.
    const LINK: &str = "encedo-auth://pair?qr=eyJsaW5rIjoiaHR0cHM6Ly9hcGkuZW5jZWRvLmNvbS9ub3RpZnkvcmVnaXN0ZXIvRVhBTVBMRSIsImhhc2giOiJuNGJRZ1loTWZXV2FMK3FneFZyUUZhTy9UeHNyQzRJczBWMXNGYkR2MnU4PSIsInVzZXIiOiJBbm4iLCJlbWFpbCI6ImFubkBleGFtcGxlLmNvbSIsImhvc3RuYW1lIjoidGVzdC5wbGF5Z3JvdW5kLmVuY2UuZG8ifQ";
    const JSON: &str = r#"{"link":"https://api.encedo.com/notify/register/EXAMPLE","hash":"n4bQgYhMfWWaL+qgxVrQFaO/TxsrC4Is0V1sFbDv2u8=","user":"Ann","email":"ann@example.com","hostname":"test.playground.ence.do"}"#;

    fn link_for(json: &str) -> String {
        format!("encedo-auth://pair?qr={}", encedo_protocol::codec::b64url(json.as_bytes()))
    }

    #[test]
    fn pair_link_carries_the_qr_json_exactly() {
        assert_eq!(pair_link_text(LINK).unwrap().as_deref(), Some(JSON));
        // Pasted with whitespace around it, or with the padding a decoder might want.
        assert_eq!(PairingCode::parse(&format!("  {LINK}\n")).unwrap(), PairingCode::parse(JSON).unwrap());
        let padded = format!("{LINK}{}", "=".repeat((4 - LINK.split("qr=").nth(1).unwrap().len() % 4) % 4));
        assert_eq!(pair_link_text(&padded).unwrap().as_deref(), Some(JSON));
        assert_eq!(pair_link_text(&padded.replace('=', "%3D").replacen("pair?qr%3D", "pair?qr=", 1)).unwrap().as_deref(), Some(JSON));
        let c = PairingCode::parse(LINK).unwrap();
        assert_eq!(c.link, "https://api.encedo.com/notify/register/EXAMPLE");
        assert_eq!(c.hash, "n4bQgYhMfWWaL+qgxVrQFaO/TxsrC4Is0V1sFbDv2u8=");
        assert_eq!((c.user.as_str(), c.email.as_str(), c.hostname.as_str()), ("Ann", "ann@example.com", "test.playground.ence.do"));
    }

    #[test]
    fn pair_link_keeps_utf8_and_both_alphabets() {
        // "Zoë" plus bytes that encode to `-` and `_` in base64url.
        let json = r#"{"link":"https://api.encedo.com/notify/register/X","hash":"h","user":"Zoë ~~~ ???","hostname":"h.ence.do"}"#;
        let link = link_for(json);
        assert!(link.contains('-') || link.contains('_'), "{link}");
        assert_eq!(pair_link_text(&link).unwrap().as_deref(), Some(json));
        assert_eq!(PairingCode::parse(&link).unwrap().user, "Zoë ~~~ ???");
    }

    #[test]
    fn pair_link_refuses_what_it_cannot_read() {
        let json = r#"{"link":"https://api.encedo.com/notify/register/X","hash":"h"}"#;
        let qr = encedo_protocol::codec::b64url(json.as_bytes());
        assert!(PairingCode::parse(&format!("encedo-auth://pair?qr={qr}")).is_ok());
        for bad in [
            "encedo-auth://pair".to_string(),
            "encedo-auth://pair?".to_string(),
            "encedo-auth://pair?qr=".to_string(),
            format!("encedo-auth://pair?code={qr}"),
            format!("encedo-auth://unpair?qr={qr}"),
            format!("encedo-auth://pairs?qr={qr}"),
            format!("encedo-auth:pair?qr={qr}"),
            "encedo-auth://pair?qr=@@not*base64@@".to_string(),
            "encedo-auth://pair?qr=abcde".to_string(),
            // Not UTF-8, not JSON, no link, no hash, an http link.
            format!("encedo-auth://pair?qr={}", encedo_protocol::codec::b64url(&[0xff, 0xfe, 0x00])),
            link_for("hello"),
            link_for(r#"{"hash":"h","user":"Ann"}"#),
            link_for(r#"{"link":"https://api.encedo.com/notify/register/X","user":"Ann"}"#),
            link_for(r#"{"link":"https://api.encedo.com/notify/register/X","hash":" "}"#),
            link_for("https://api.encedo.com/notify/register/X"),
            link_for(r#"{"link":"http://192.0.2.1/notify/register/X","hash":"h"}"#),
        ] {
            assert!(PairingCode::parse(&bad).is_err(), "accepted {bad}");
        }
        // Not this app's scheme: the parser treats it as a plain code, and refuses it as one.
        assert_eq!(pair_link_text("other://pair?qr=e30").unwrap(), None);
        assert!(PairingCode::parse("other://pair?qr=e30").is_err());
    }

    #[tokio::test]
    async fn pending_handles_empty_array_and_object() {
        // Shape only; no network. Exercised through `AllByPidRaw` parsing.
        let arr: AllByPidRaw = serde_json::from_str(r#"{"eventid":[]}"#).unwrap();
        assert!(matches!(arr.eventid, serde_json::Value::Array(_)));
        let obj: AllByPidRaw = serde_json::from_str(r#"{"eventid":{"ev1":"pid1"}}"#).unwrap();
        assert_eq!(obj.eventid["ev1"], "pid1");
        // Live broker shape (2026-09-09): the pid is a one-element array.
        let live: AllByPidRaw = serde_json::from_str(r#"{"eventid":{"5X8YX":["UyR+w6=="],"g0O7U":["UyR+w6=="]}}"#).unwrap();
        let pids: Vec<String> = live.eventid.as_object().unwrap().values().filter_map(pid_of).collect();
        assert_eq!(pids, vec!["UyR+w6==", "UyR+w6=="]);
        assert_eq!(pid_of(&serde_json::json!("x")), Some("x".into()));
        assert_eq!(pid_of(&serde_json::json!(null)), None);
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
