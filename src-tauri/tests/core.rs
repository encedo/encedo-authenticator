//! The core against a broker that answers like the real one and a module that
//! checks what the phone sends. Every flow the phone has (pair, refuse, refresh,
//! allow, deny, unpair, push) and every error the broker can return, through the
//! same public API the webview calls — including what each of them writes to the
//! journal, since that is the record the owner relies on afterwards.

mod fake_broker;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use encedo_authenticator_lib::core::{now, Core, CoreError, ErrorView};
use encedo_authenticator_lib::notify::NotifyClient;
use encedo_authenticator_lib::store::{DataKey, DevFileSecret, Family, Level, LogEntry, Outcome, Protection, SecretStore, StoreError};
use encedo_protocol::event::PERIODS;
use encedo_protocol::hem::Hem;
use encedo_protocol::pairing::{PairingReply, PairingRequest};
use fake_broker::FakeBroker;

/// A pid with both characters the URL form has to rewrite.
const PID: &str = "Ab+/cd==";
const PIDX: &str = "Ab-_cd";
/// The second module's pid, for the tests that pair twice.
const PID2: &str = "Zz9+ab==";
const DEVICE: &str = "Galaxy S24 (Android)";
const EVENT: &str = "/notify/event/data/ev1/";
/// `jti` is base64 on the wire: the session key is derived from its bytes.
const JTI: &str = "amNRPXl5";

struct Fixture {
    core: Core,
    hem: Hem,
    broker: FakeBroker,
    dir: PathBuf,
}

impl Fixture {
    async fn new() -> Self {
        static N: AtomicUsize = AtomicUsize::new(0);
        let broker = FakeBroker::start().await;
        let dir = std::env::temp_dir().join(format!("encedo-core-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        let _ = std::fs::remove_dir_all(&dir);
        let secrets = Box::new(DevFileSecret { path: dir.join("dev.key") });
        let core = Core::new(secrets, dir.join("store.bin"), NotifyClient::new(&broker.base), DEVICE.to_string());
        core.open_storage().expect("storage opens");
        Self { core, hem: Hem::with_seeds(7, 9), broker, dir }
    }

    fn link(&self) -> String {
        format!("{}/notify/pairing/abc", self.broker.base)
    }

    /// A pairing code for a request valid for five more minutes.
    fn offer(&self, jti: &str) -> (String, String) {
        let jwt = self.hem.pairing_request(jti, Some(now() + 300));
        let qr = self.hem.qr_code(&self.link(), &jwt);
        self.broker.only("GET", "/notify/pairing/", 200, &self.hem.pairing_offer(&jwt));
        (jwt, qr)
    }

    /// Pair one module the way the phone does it; returns the app key (`aid`)
    /// the module received, so it can address events at this phone.
    async fn pair(&self) -> String {
        let (jwt, qr) = self.offer("jti-pair");
        self.broker.on("POST", "/notify/pairing/", 200, &format!(r#"{{"pid":"{PID}"}}"#));
        self.core.pair_scan(&qr).await.expect("scan");
        self.core.pair_confirm("", "fid-abc").await.expect("confirm");
        let reply: PairingReply = serde_json::from_value(self.broker.last("POST", "/notify/pairing/").json()).expect("pairing reply");
        let request = PairingRequest::from_jwt(&jwt).expect("request");
        self.hem.check_pairing_reply(&request, &reply).expect("the module accepts the reply").aid
    }

    /// Pair, then hand the phone one pending event; returns `aid`.
    async fn pending_event(&self, scope: &str, exp: u64) -> String {
        let aid = self.pair().await;
        let event = self.hem.event(&aid, JTI, scope, exp).expect("event");
        self.broker.on("POST", "/allbypid", 200, &format!(r#"{{"eventid":{{"ev1":["{PID}"]}}}}"#));
        self.broker.on("GET", EVENT, 200, &serde_json::to_string(&event).expect("event json"));
        aid
    }

    /// The journal, newest first.
    fn journal(&self) -> Vec<LogEntry> {
        self.core.journal(None).expect("journal")
    }

    fn of_kind(&self, kind: &str) -> Vec<LogEntry> {
        self.journal().into_iter().filter(|e| e.kind == kind).collect()
    }

    /// The newest entry of this kind; panics when the flow wrote none.
    fn latest(&self, kind: &str) -> LogEntry {
        self.of_kind(kind).into_iter().next().unwrap_or_else(|| panic!("no {kind} in the journal: {:?}", self.journal().iter().map(|e| e.kind.clone()).collect::<Vec<_>>()))
    }

    fn field(&self, entry: &LogEntry, label: &str) -> String {
        entry.fields.iter().find(|f| f.label == label).map(|f| f.value.clone()).unwrap_or_else(|| panic!("no field {label} in {:?}", entry.fields))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn view(e: CoreError) -> ErrorView {
    ErrorView::from(e)
}

// ---- pairing ---------------------------------------------------------------

#[tokio::test]
async fn pairing_stores_the_module_and_the_module_accepts_the_reply() {
    let f = Fixture::new().await;
    let (jwt, qr) = f.offer("jti-pair");
    f.broker.on("POST", "/notify/pairing/", 200, &format!(r#"{{"pid":"{PID}"}}"#));

    let preview = f.core.pair_scan(&qr).await.expect("scan");
    assert_eq!(preview.user, "chris");
    assert_eq!(preview.hostname, "hem.example.org");
    assert_eq!(preview.email, "chris@example.org");
    assert_eq!(preview.issuer, Some(Hem::ipinfo()));
    assert!(!preview.already_paired);

    let module = f.core.pair_confirm("", "fid-abc").await.expect("confirm");
    assert_eq!(module.pid, PID);
    // No label typed: the module's hostname stands in.
    assert_eq!(module.label, "hem.example.org");
    assert_eq!(f.core.modules().expect("modules").len(), 1);
    assert_eq!(f.core.push_token().expect("token"), Some("fid-abc".into()));
    let read = f.latest("pair.scanned");
    assert!(read.summary.contains("hash matched the link"), "{read:?}");
    assert_eq!(f.field(&read, "Already paired"), "no");
    let entry = f.latest("pair.paired");
    assert_eq!((entry.outcome, entry.level, entry.family), (Some(Outcome::Paired), Level::Good, Family::Modules));
    assert_eq!(f.field(&entry, "Host"), "hem.example.org");
    assert_eq!(f.field(&entry, "Name the module sees"), DEVICE);
    assert_eq!(f.field(&entry, "pid"), PID);
    assert!(!entry.seal.is_empty(), "an answer is sealed");

    // The label the module sees is this phone's name, and the push token rides along.
    let reply: PairingReply = serde_json::from_value(f.broker.last("POST", "/notify/pairing/").json()).expect("reply");
    let request = PairingRequest::from_jwt(&jwt).expect("request");
    let seen = f.hem.check_pairing_reply(&request, &reply).expect("module accepts");
    assert_eq!(seen.label, DEVICE);
    assert_eq!(seen.fid, "fid-abc");
    // The Module screen shows this key as the KID, the way v1 did.
    assert_eq!(module.aid, seen.aid);

    // Same module again: the screen must say so instead of pairing twice.
    let (_, qr) = f.offer("jti-pair-2");
    assert!(f.core.pair_scan(&qr).await.expect("second scan").already_paired);
}

#[tokio::test]
async fn a_code_whose_hash_does_not_match_the_link_is_refused() {
    let f = Fixture::new().await;
    let (_, _) = f.offer("jti-pair");
    let qr = f.hem.qr_code(&f.link(), "a request the link does not serve");

    let e = view(f.core.pair_scan(&qr).await.expect_err("hash mismatch"));
    assert_eq!(e.code, "state");
    assert!(e.message.contains("different request"), "{}", e.message);
    assert_eq!(f.broker.count("POST", "/notify/pairing/"), 0, "nothing may be sent after a bad hash");
    let entry = f.latest("pair.failed");
    assert_eq!(entry.level, Level::Bad);
    assert!(f.field(&entry, "Error").contains("different request"), "{entry:?}");
}

#[tokio::test]
async fn an_expired_pairing_request_is_refused_before_it_is_shown() {
    let f = Fixture::new().await;
    let jwt = f.hem.pairing_request("jti-old", Some(now() - 10));
    f.broker.on("GET", "/notify/pairing/", 200, &f.hem.pairing_offer(&jwt));

    let e = view(f.core.pair_scan(&f.hem.qr_code(&f.link(), &jwt)).await.expect_err("expired"));
    assert_eq!(e.code, "rejected");
    assert!(e.message.contains("pairing request has expired"), "{}", e.message);
}

#[tokio::test]
async fn a_pairing_code_the_broker_forgot_says_it_is_stale() {
    let f = Fixture::new().await;
    let jwt = f.hem.pairing_request("jti-gone", Some(now() + 300));
    f.broker.on("GET", "/notify/pairing/", 404, "{}");

    let e = view(f.core.pair_scan(&f.hem.qr_code(&f.link(), &jwt)).await.expect_err("404"));
    assert_eq!(e.code, "rejected");
    assert!(e.message.contains("pairing code has expired"), "{}", e.message);
}

#[tokio::test]
async fn a_request_missing_a_claim_names_the_field() {
    let f = Fixture::new().await;
    let jwt = f.hem.pairing_request_without("jti-x", "iss");
    f.broker.on("GET", "/notify/pairing/", 200, &f.hem.pairing_offer(&jwt));

    let e = view(f.core.pair_scan(&f.hem.qr_code(&f.link(), &jwt)).await.expect_err("no iss"));
    assert_eq!(e.code, "protocol");
    assert!(e.message.contains("field iss"), "{}", e.message);
}

#[tokio::test]
async fn a_broker_that_refuses_the_pairing_leaves_a_trace_and_no_module() {
    let f = Fixture::new().await;
    let (_, qr) = f.offer("jti-pair");
    f.broker.on("POST", "/notify/pairing/", 401, "{}");

    f.core.pair_scan(&qr).await.expect("scan");
    let e = view(f.core.pair_confirm("", "fid-abc").await.expect_err("401"));
    assert_eq!(e.code, "unauthorized");
    assert!(f.core.modules().expect("modules").is_empty());
    let entry = f.latest("pair.failed");
    assert_eq!(entry.level, Level::Bad);
    assert_eq!(f.field(&entry, "Code"), "unauthorized");
    assert!(entry.summary.contains("hem.example.org"), "{entry:?}");
}

#[tokio::test]
async fn refusing_a_pairing_tells_the_module_and_is_archived() {
    let f = Fixture::new().await;
    let (_, qr) = f.offer("jti-pair");
    f.broker.on("DELETE", "/notify/pairing/", 200, "{}");

    f.core.pair_scan(&qr).await.expect("scan");
    f.core.pair_refuse("fid-abc").await.expect("refuse");
    assert_eq!(f.broker.count("DELETE", "/notify/pairing/"), 1);
    let entry = f.latest("pair.refused");
    assert_eq!(entry.outcome, Some(Outcome::Denied));
    assert_eq!(f.field(&entry, "Broker told"), "yes");
    assert!(f.core.modules().expect("modules").is_empty());
    // The pairing is spent: a second answer has nothing to answer.
    assert_eq!(view(f.core.pair_confirm("", "fid").await.expect_err("no pairing")).code, "state");
}

// ---- refresh ---------------------------------------------------------------

#[tokio::test]
async fn without_a_paired_module_the_broker_is_not_asked() {
    let f = Fixture::new().await;
    assert!(f.core.refresh().await.expect("refresh").is_empty());
    assert!(f.broker.seen().is_empty(), "{:?}", f.broker.seen());
    assert!(f.core.last_refresh().broker_said.contains("no modules paired"));
    // And the journal does not claim a check that never happened.
    assert!(f.of_kind("broker.checked").is_empty(), "{:?}", f.journal());
}

#[tokio::test]
async fn a_pending_event_is_opened_once_and_then_kept() {
    let f = Fixture::new().await;
    f.pending_event("storage:disk:home", now() + 300).await;

    let requests = f.core.refresh().await.expect("refresh");
    assert_eq!(requests.len(), 1);
    let r = &requests[0];
    assert_eq!(r.id, "ev1");
    assert_eq!(r.pid, PID);
    assert_eq!(r.scope, "storage:disk:home");
    assert_eq!(r.module_label, "hem.example.org");
    assert_eq!(r.issuer, Some(Hem::ipinfo()));
    assert_eq!(r.view.title, "Unlock a drive");
    assert!(r.view.ask_writable);
    // The pid travels in the path in its URL form.
    assert_eq!(f.broker.last("GET", EVENT).path, format!("/notify/event/data/ev1/{PIDX}"));

    let report = f.core.last_refresh();
    assert_eq!((report.pending, report.shown), (1, 1));
    assert!(report.discarded.is_empty());

    // A second look must not fetch the event again: it is already open.
    assert_eq!(f.core.refresh().await.expect("refresh").len(), 1);
    assert_eq!(f.broker.count("GET", EVENT), 1);

    let shown = f.latest("request.shown");
    assert_eq!(shown.pid, PID);
    assert_eq!(f.field(&shown, "Module"), "hem.example.org");
    assert_eq!(f.field(&shown, "Asked from"), "Warsaw, PL · 203.0.113.7");
    assert_eq!(f.field(&shown, "Scope"), "storage:disk:home");
    assert_eq!(f.of_kind("request.shown").len(), 1, "the second look does not show it again");
}

#[tokio::test]
async fn an_event_that_expired_is_denied_archived_and_not_shown_again() {
    let f = Fixture::new().await;
    f.pending_event("logger:get", now() - 1).await;
    f.broker.on("DELETE", EVENT, 200, "{}");

    assert!(f.core.refresh().await.expect("refresh").is_empty());
    assert_eq!(f.broker.count("DELETE", EVENT), 1, "an expired request is refused at the broker");
    let entry = f.latest("request.expired");
    assert_eq!(entry.outcome, Some(Outcome::Expired));
    assert!(entry.summary.contains("expired before it was shown"), "{entry:?}");
    assert_eq!(f.field(&entry, "Broker told"), "yes, refused on your behalf");

    // The broker still lists it; the phone remembers it is done.
    assert!(f.core.refresh().await.expect("refresh").is_empty());
    assert_eq!(f.broker.count("GET", EVENT), 1);
    assert!(f.core.last_refresh().discarded[0].contains("already handled"));
    assert_eq!(f.of_kind("request.expired").len(), 1, "and it is not written twice");
}

#[tokio::test]
async fn an_event_whose_scope_mac_fails_is_dropped_without_an_answer() {
    let f = Fixture::new().await;
    let aid = f.pair().await;
    let bad = f.hem.event_with_bad_mac(&aid, JTI, "storage:disk:home", now() + 300).expect("event");
    f.broker.on("POST", "/allbypid", 200, &format!(r#"{{"eventid":{{"ev1":["{PID}"]}}}}"#));
    f.broker.on("GET", EVENT, 200, &serde_json::to_string(&bad).expect("json"));

    assert!(f.core.refresh().await.expect("refresh").is_empty());
    let entry = f.latest("request.rejected");
    assert_eq!((entry.outcome, entry.level), (Some(Outcome::Rejected), Level::Bad));
    assert!(entry.summary.contains("did not match its MAC"), "{entry:?}");
    assert_eq!(f.field(&entry, "Broker told"), "nothing; the module will time out");
    assert_eq!(f.broker.count("DELETE", EVENT), 0, "a forged event is not answered at all");
    assert!(f.core.last_refresh().discarded[0].contains("bad scope MAC"));
}

#[tokio::test]
async fn an_event_the_module_withdrew_is_archived_as_cancelled() {
    let f = Fixture::new().await;
    f.pair().await;
    f.broker.on("POST", "/allbypid", 200, &format!(r#"{{"eventid":{{"ev1":["{PID}"],"ev2":["{PID}"]}}}}"#));
    // 410 with a reason: withdrawn on the module. 410 without: another phone was first.
    f.broker.on("GET", EVENT, 410, r#"{"reason":"cancelled"}"#);
    f.broker.on("GET", "/notify/event/data/ev2/", 410, "{}");

    assert!(f.core.refresh().await.expect("refresh").is_empty());
    let report = f.core.last_refresh();
    assert_eq!(report.discarded.len(), 2, "{report:?}");
    assert!(report.discarded.iter().all(|d| d.contains("gone (410)")), "{report:?}");
    assert_eq!(f.of_kind("request.cancelled").len(), 2);
}

#[tokio::test]
async fn an_event_that_vanished_from_the_broker_is_archived_as_expired() {
    let f = Fixture::new().await;
    f.pair().await;
    f.broker.on("POST", "/allbypid", 200, &format!(r#"{{"eventid":{{"ev1":["{PID}"]}}}}"#));
    f.broker.on("GET", EVENT, 404, "{}");
    f.broker.on("DELETE", EVENT, 404, "{}");

    assert!(f.core.refresh().await.expect("refresh").is_empty());
    assert_eq!(f.latest("request.expired").outcome, Some(Outcome::Expired));
}

#[tokio::test]
async fn a_scope_this_app_does_not_know_is_archived_and_never_shown() {
    let f = Fixture::new().await;
    f.pending_event("wat:ever", now() + 300).await;

    let e = view(f.core.refresh().await.expect_err("unknown scope"));
    assert_eq!(e.code, "state");
    assert!(e.message.contains("unknown scope wat:ever"), "{}", e.message);
    let entry = f.latest("request.rejected");
    assert_eq!(entry.outcome, Some(Outcome::Rejected));
    assert_eq!(entry.title, "Unknown request");
    assert_eq!(entry.summary, "wat:ever");
    assert_eq!(f.field(&entry, "Scope"), "wat:ever");
}

#[tokio::test]
async fn broker_failures_reach_the_screen_with_their_own_code() {
    let f = Fixture::new().await;
    f.pair().await;
    f.broker.on("POST", "/allbypid", 401, "{}");
    f.broker.on("POST", "/allbypid", 500, "boom");
    f.broker.on("POST", "/allbypid", 418, "nope");
    f.broker.on("POST", "/allbypid", 200, "not json at all");

    assert_eq!(view(f.core.refresh().await.expect_err("401")).code, "unauthorized");
    assert!(f.core.last_refresh().error.expect("error").contains("401"));
    let entry = f.latest("broker.error");
    assert_eq!(entry.level, Level::Bad);
    assert_eq!(f.field(&entry, "Endpoint"), "POST /notify/event/data/allbypid");
    assert_eq!(view(f.core.refresh().await.expect_err("500")).code, "unavailable");
    let e = view(f.core.refresh().await.expect_err("418"));
    assert_eq!(e.code, "rejected");
    assert!(e.message.contains("nope"), "{}", e.message);
    assert_eq!(view(f.core.refresh().await.expect_err("junk")).code, "bad_response");
}

// ---- allow and deny --------------------------------------------------------

#[tokio::test]
async fn allow_hands_the_module_a_grant_it_can_verify() {
    let f = Fixture::new().await;
    let aid = f.pending_event("storage:disk:home", now() + 300).await;
    f.broker.on("POST", EVENT, 200, "{}");
    let before = now();

    f.core.refresh().await.expect("refresh");
    let answer = f.core.allow("ev1", PERIODS[1], true).await.expect("allow");
    assert_eq!(answer.outcome, Outcome::Granted);
    assert_eq!(answer.title, "Unlock a drive");
    assert!(answer.detail.contains("storage:disk:home:rw") && answer.detail.contains("1 h"), "{answer:?}");

    let granted = f.hem.check_allow(&aid, JTI, &f.broker.last("POST", EVENT).json()).expect("the module accepts the grant");
    assert_eq!(granted.scope, "storage:disk:home:rw");
    assert_eq!(granted.pid, PID);
    assert_eq!(granted.exp - granted.iat, PERIODS[1]);
    assert!(granted.iat >= before);

    assert!(f.core.modules().expect("modules")[0].last_used.is_some());
    let entry = f.latest("request.granted");
    assert_eq!((entry.outcome, entry.level), (Some(Outcome::Granted), Level::Good));
    assert_eq!(f.field(&entry, "Granted scope"), "storage:disk:home:rw");
    assert_eq!(f.field(&entry, "For"), "1 h");
    assert_eq!(f.field(&entry, "Writing"), "allowed");
    assert_eq!(f.field(&entry, "Broker said"), "accepted");
    assert!(f.field(&entry, "Until").ends_with("UTC"), "{entry:?}");
    assert!(!entry.seal.is_empty());
    // Answered: the request is gone from this phone and from the next refresh.
    assert_eq!(view(f.core.allow("ev1", PERIODS[1], true).await.expect_err("spent")).code, "state");
    assert!(f.core.refresh().await.expect("refresh").is_empty());
}

#[tokio::test]
async fn a_period_the_app_does_not_offer_falls_back_to_fifteen_minutes() {
    let f = Fixture::new().await;
    // The event asks for write access; answering read-only must strip it.
    let aid = f.pending_event("storage:disk:home:rw", now() + 300).await;
    f.broker.on("POST", EVENT, 200, "{}");

    let request = &f.core.refresh().await.expect("refresh")[0];
    assert!(request.view.writable_default, "the event asked for write access");
    let answer = f.core.allow("ev1", 12345, false).await.expect("allow");
    assert!(answer.detail.contains("15 min"), "{answer:?}");

    let granted = f.hem.check_allow(&aid, JTI, &f.broker.last("POST", EVENT).json()).expect("grant");
    assert_eq!(granted.scope, "storage:disk:home");
    assert_eq!(granted.exp - granted.iat, PERIODS[0]);
}

#[tokio::test]
async fn a_grant_the_module_no_longer_wants_is_archived_as_withdrawn() {
    let f = Fixture::new().await;
    f.pending_event("logger:get", now() + 300).await;
    f.broker.on("POST", EVENT, 410, r#"{"reason":"cancelled"}"#);

    f.core.refresh().await.expect("refresh");
    let answer = f.core.allow("ev1", PERIODS[0], false).await.expect("allow answers even when the broker refuses");
    assert_eq!(answer.outcome, Outcome::Cancelled);
    assert!(answer.detail.contains("withdrawn"), "{answer:?}");
    // The entry is called what happened, not what was attempted.
    let entry = f.latest("request.cancelled");
    assert_eq!(entry.outcome, Some(Outcome::Cancelled));
    assert_eq!(f.field(&entry, "Broker said"), "410, the module withdrew it");
    assert!(f.of_kind("request.granted").is_empty());
}

#[tokio::test]
async fn deny_tells_the_broker_and_archives_the_scope() {
    let f = Fixture::new().await;
    f.pending_event("system:shutdown", now() + 300).await;
    f.broker.on("DELETE", EVENT, 200, "{}");

    f.core.refresh().await.expect("refresh");
    let answer = f.core.deny("ev1").await.expect("deny");
    assert_eq!(answer.outcome, Outcome::Denied);
    assert_eq!(answer.title, "Shut down");
    assert_eq!(f.broker.count("DELETE", EVENT), 1);
    let entry = f.latest("request.denied");
    assert_eq!((entry.outcome, entry.level), (Some(Outcome::Denied), Level::Bad));
    assert_eq!(entry.summary, "system:shutdown");
    assert_eq!(f.field(&entry, "Refused scope"), "system:shutdown");
}

// ---- unpair and push -------------------------------------------------------

#[tokio::test]
async fn unpair_proves_the_key_to_the_broker_then_drops_the_module() {
    let f = Fixture::new().await;
    let aid = f.pair().await;
    f.broker.on("POST", "/notify/session", 200, &f.hem.session());
    f.broker.on("POST", "/notify/subscribers/delete", 200, "{}");

    f.core.unpair(PID).await.expect("unpair");
    assert_eq!(f.broker.last("POST", "/notify/session").json()["aid"], aid);
    assert_eq!(f.hem.check_delete_body(&f.broker.last("POST", "/subscribers/delete").json()).expect("proof"), PID);
    assert!(f.core.modules().expect("modules").is_empty());
    let entry = f.latest("unpair.done");
    assert_eq!(entry.outcome, Some(Outcome::Unpaired));
    assert!(entry.summary.contains("the broker agreed"), "{entry:?}");
}

#[tokio::test]
async fn a_broker_that_will_not_unsubscribe_keeps_the_module_until_it_is_forgotten() {
    let f = Fixture::new().await;
    f.pair().await;
    f.broker.on("POST", "/notify/session", 200, &f.hem.session());
    f.broker.on("POST", "/notify/subscribers/delete", 404, "{}");

    // This is what the live broker does today, for the Manager as well.
    assert_eq!(view(f.core.unpair(PID).await.expect_err("404")).code, "expired");
    assert_eq!(f.core.modules().expect("modules").len(), 1, "the module stays until the person decides");

    let failed = f.latest("unpair.failed");
    assert_eq!(f.field(&failed, "Code"), "expired");
    assert!(f.field(&failed, "What it means").contains("Remove the phone in the Manager"), "{failed:?}");

    f.core.forget(PID).expect("forget");
    assert!(f.core.modules().expect("modules").is_empty());
    let local = f.latest("unpair.local");
    assert_eq!(local.level, Level::Bad);
    assert!(local.summary.contains("did not confirm"), "{local:?}");
}

#[tokio::test]
async fn a_broker_that_refuses_the_session_leaves_the_module_alone() {
    let f = Fixture::new().await;
    f.pair().await;
    f.broker.on("POST", "/notify/session", 401, "{}");

    assert_eq!(view(f.core.unpair(PID).await.expect_err("401")).code, "unauthorized");
    assert_eq!(f.core.modules().expect("modules").len(), 1);
    assert_eq!(f.broker.count("POST", "/subscribers/delete"), 0);
}

#[tokio::test]
async fn a_push_asks_for_a_refresh_only_when_an_event_is_waiting() {
    let f = Fixture::new().await;
    f.pair().await;

    assert!(f.core.push_payload(r#"{"event":{"id":"ev1"}}"#).expect("event push"));
    assert!(!f.core.push_payload(r#"{"pairing":{"status":"ACTIVE"}}"#).expect("pairing push"));
    assert!(f.core.push_payload("not json").is_err());

    // The Manager unpaired this phone: the module goes, with a note.
    assert!(!f.core.push_payload(&format!(r#"{{"pairing":{{"status":"DELETED","pid":"{PID}"}}}}"#)).expect("delete push"));
    assert!(f.core.modules().expect("modules").is_empty());
    let entry = f.latest("unpair.remote");
    assert_eq!((entry.outcome, entry.level), (Some(Outcome::Unpaired), Level::Bad));
    assert!(entry.summary.contains("the module unpaired this phone"), "{entry:?}");
}

#[tokio::test]
async fn a_new_push_token_is_offered_to_the_module_and_a_missing_endpoint_is_tolerated() {
    let f = Fixture::new().await;
    f.pair().await;
    f.broker.on("POST", "/notify/session", 200, &f.hem.session());
    // Today's backend has no token endpoint; tomorrow's answers 200.
    f.broker.on("POST", "/notify/subscribers/token", 404, "{}");
    f.broker.on("POST", "/notify/subscribers/token", 200, "{}");

    // The token from pairing is already on file: nothing to say.
    f.core.push_token_changed("fid-abc").await.expect("same token");
    assert_eq!(f.broker.count("POST", "/notify/session"), 0);

    f.core.push_token_changed("fid-new").await.expect("404 is not an error");
    assert_eq!(f.core.push_token().expect("token"), Some("fid-new".into()));

    f.core.push_token_changed("fid-newer").await.expect("token accepted");
    let (pid, fid) = f.hem.check_token_body(&f.broker.last("POST", "/subscribers/token").json()).expect("proof");
    assert_eq!((pid.as_str(), fid.as_str()), (PID, "fid-newer"));
    assert_eq!(f.broker.count("POST", "/subscribers/token"), 2);
    let entries = f.of_kind("push.token_changed");
    assert_eq!(entries.len(), 2);
    assert!(entries[1].summary.contains("404, the endpoint is not deployed yet"), "{:?}", entries[1]);
    assert_eq!(f.field(&entries[0], "New token"), "fid-newer");
}

#[tokio::test]
async fn every_pairing_makes_its_own_key() {
    let f = Fixture::new().await;
    let first = f.pair().await;

    // A second module, with its own long-term key and its own pid.
    let other = Hem::with_seeds(11, 13);
    let jwt = other.pairing_request("jti-two", Some(now() + 300));
    f.broker.only("GET", "/notify/pairing/", 200, &other.pairing_offer(&jwt));
    f.broker.only("POST", "/notify/pairing/", 200, &format!(r#"{{"pid":"{PID2}"}}"#));
    let preview = f.core.pair_scan(&other.qr_code(&f.link(), &jwt)).await.expect("scan");
    assert!(!preview.already_paired, "a different module is not the one already paired");
    let module = f.core.pair_confirm("lab", "fid-abc").await.expect("confirm");
    let request = PairingRequest::from_jwt(&jwt).expect("request");
    let reply: PairingReply = serde_json::from_value(f.broker.last("POST", "/notify/pairing/").json()).expect("reply");
    let second = other.check_pairing_reply(&request, &reply).expect("the second module accepts its own reply").aid;

    // One key per pairing: two modules never share the phone's key.
    assert_ne!(first, second, "the second pairing must not reuse the first key");
    assert_eq!(module.aid, second);
    let modules = f.core.modules().expect("modules");
    assert_eq!(modules.len(), 2);
    assert_eq!(modules.iter().filter(|m| m.aid == first).count(), 1);
    assert_eq!(modules.iter().filter(|m| m.aid == second).count(), 1);
    // The first module cannot verify what was signed for the second.
    assert!(f.hem.check_pairing_reply(&request, &reply).is_err(), "the keys are not interchangeable");

    // An event from the second module opens with the second module's key.
    let event = other.event(&second, JTI, "logger:get", now() + 300).expect("event");
    f.broker.only("POST", "/allbypid", 200, &format!(r#"{{"eventid":{{"ev1":["{PID2}"]}}}}"#));
    f.broker.only("GET", EVENT, 200, &serde_json::to_string(&event).expect("json"));
    let shown = f.core.refresh().await.expect("refresh");
    assert_eq!(shown.len(), 1);
    assert_eq!((shown[0].pid.as_str(), shown[0].scope.as_str()), (PID2, "logger:get"));
    assert_eq!(f.latest("pair.paired").fields.iter().filter(|x| x.label == "This phone's key" && x.value == second).count(), 1, "and the journal recorded which key was handed over");
}

// ---- the storage key -------------------------------------------------------

/// A storage key that behaves like the phone's: it can be bound to the person
/// (and then refused until they confirm), it can be lost, and forgetting it
/// leaves a different key behind. Cloning shares the same state, so a test can
/// flip a flag under a running core — or hand the same key to a second core,
/// which is what a restart looks like.
#[derive(Clone)]
struct FakeSecret {
    key: Arc<Mutex<[u8; 32]>>,
    bound: Arc<AtomicBool>,
    confirmed: Arc<AtomicBool>,
    lost: Arc<AtomicBool>,
    credential: bool,
}

impl FakeSecret {
    fn new(bound: bool, credential: bool) -> Self {
        Self {
            key: Arc::new(Mutex::new([7u8; 32])),
            bound: Arc::new(AtomicBool::new(bound)),
            confirmed: Arc::new(AtomicBool::new(false)),
            lost: Arc::new(AtomicBool::new(false)),
            credential,
        }
    }

    /// What the biometric prompt does for the real key: opens the window.
    fn confirm(&self) {
        self.confirmed.store(true, Ordering::Relaxed);
    }

    fn lose(&self) {
        self.lost.store(true, Ordering::Relaxed);
    }

    fn bound(&self) -> bool {
        self.bound.load(Ordering::Relaxed)
    }
}

impl SecretStore for FakeSecret {
    fn data_key(&self) -> Result<DataKey, StoreError> {
        if self.lost.load(Ordering::Relaxed) {
            return Err(StoreError::KeyLost);
        }
        if self.bound.load(Ordering::Relaxed) && !self.confirmed.load(Ordering::Relaxed) {
            return Err(StoreError::AuthRequired);
        }
        Ok(DataKey(*self.key.lock().expect("key")))
    }

    fn protection(&self) -> Protection {
        Protection { bound_to_user: self.bound(), credential: self.credential, strong_box: true, window_seconds: 30 }
    }

    fn bind_to_user(&self, bound: bool) -> Result<(), StoreError> {
        if bound && !self.credential {
            return Err(StoreError::NoCredential);
        }
        // Re-wrapping reads the key, so a stale confirmation refuses here too.
        self.data_key()?;
        self.bound.store(bound, Ordering::Relaxed);
        Ok(())
    }

    fn forget(&self) -> Result<(), StoreError> {
        self.lost.store(false, Ordering::Relaxed);
        self.bound.store(false, Ordering::Relaxed);
        *self.key.lock().expect("key") = [9u8; 32];
        Ok(())
    }
}

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("encedo-secret-{}-{name}", std::process::id()))
}

/// A core over `dir`, protected by `secret`. Called twice with the same secret
/// and directory, it is the app starting again.
fn core_with(secret: &FakeSecret, dir: &PathBuf) -> Core {
    Core::new(Box::new(secret.clone()), dir.join("store.bin"), NotifyClient::new("http://127.0.0.1:1"), DEVICE.to_string())
}

#[tokio::test]
async fn a_key_bound_to_the_person_keeps_the_storage_shut_until_they_confirm() {
    let dir = temp_dir("bound");
    let _ = std::fs::remove_dir_all(&dir);
    let secret = FakeSecret::new(true, true);
    let core = core_with(&secret, &dir);

    // Nothing is readable, and every command says so rather than pretending the
    // phone is empty.
    assert_eq!(view(core.open_storage().expect_err("bound")).code, "auth_required");
    assert!(!core.status().open);
    assert!(core.status().protection.bound_to_user);
    assert_eq!(view(core.modules().expect_err("locked")).code, "locked");
    assert_eq!(view(core.journal(None).expect_err("locked")).code, "locked");
    assert_eq!(view(core.settings().expect_err("locked")).code, "locked");
    assert_eq!(view(core.refresh().await.expect_err("locked")).code, "locked");

    // The confirmation happens on the Locked screen; then the key is released.
    secret.confirm();
    core.open_storage().expect("opens after a confirmation");
    assert!(core.status().open);
    core.opened("2.0.0-test").expect("first lines");
    assert!(core.modules().expect("modules").is_empty());
    assert_eq!(core.journal(None).expect("journal").first().map(|e| e.kind.clone()), Some("app.launched".into()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn a_storage_whose_key_is_gone_can_only_be_started_over() {
    let dir = temp_dir("lost");
    let _ = std::fs::remove_dir_all(&dir);
    let secret = FakeSecret::new(false, true);
    let core = core_with(&secret, &dir);
    core.open_storage().expect("opens");
    core.log_push(Some("Encedo".into()), Some("A request is waiting".into()), None, false).expect("something worth keeping");
    assert_eq!(core.journal(None).expect("journal").len(), 1);
    drop(core);

    // The phone comes back with the file but without its key: a restored backup,
    // or the screen lock was removed.
    secret.lose();
    let core = core_with(&secret, &dir);
    assert_eq!(view(core.open_storage().expect_err("no key")).code, "key_lost");
    assert!(!core.status().open);

    core.reset().expect("start over");
    assert!(core.status().open);
    assert!(core.modules().expect("modules").is_empty());
    let journal = core.journal(None).expect("journal");
    assert_eq!(journal.len(), 1, "the old entries went with the key: {journal:?}");
    assert_eq!(journal[0].kind, "app.storage_reset");
    assert_eq!(journal[0].level, Level::Bad);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn the_lock_setting_decides_whether_the_key_is_bound() {
    let dir = temp_dir("bind");
    let _ = std::fs::remove_dir_all(&dir);
    let secret = FakeSecret::new(false, true);
    let core = core_with(&secret, &dir);
    core.open_storage().expect("opens");
    let mut settings = core.settings().expect("settings");
    assert!(!secret.bound());

    settings.biometric_lock = true;
    core.set_settings(settings.clone()).expect("bind");
    assert!(secret.bound(), "turning the lock on binds the key to the person");
    let entry = core.journal(Some(Family::App)).expect("journal").into_iter().find(|e| e.kind == "settings.changed").expect("written down");
    assert!(entry.summary.contains("storage key bound to you"), "{entry:?}");

    // Unbinding reads the key, so it needs a confirmation of its own.
    settings.biometric_lock = false;
    assert_eq!(view(core.set_settings(settings.clone()).expect_err("stale")).code, "auth_required");
    assert!(core.settings().expect("settings").biometric_lock, "and the setting stays where it was");
    secret.confirm();
    core.set_settings(settings).expect("unbind");
    assert!(!secret.bound());

    // A phone with no screen lock cannot bind at all: asking for it says why,
    // while saving anything else is not held up by it.
    let bare_dir = temp_dir("nolock");
    let _ = std::fs::remove_dir_all(&bare_dir);
    let bare = core_with(&FakeSecret::new(false, false), &bare_dir);
    bare.open_storage().expect("opens");
    let mut want = bare.settings().expect("settings");
    want.biometric_lock = false;
    want.theme = "dark".into();
    bare.set_settings(want.clone()).expect("a theme is not a security setting");
    assert_eq!(bare.settings().expect("settings").theme, "dark");
    want.biometric_lock = true;
    assert_eq!(view(bare.set_settings(want).expect_err("no credential")).code, "no_credential");
    assert!(!bare.settings().expect("settings").biometric_lock);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&bare_dir);
}

// ---- the journal itself ----------------------------------------------------

#[tokio::test]
async fn the_journal_seals_every_answer_and_says_so() {
    let f = Fixture::new().await;
    f.pending_event("logger:get", now() + 300).await;
    f.broker.on("DELETE", EVENT, 200, "{}");
    f.core.refresh().await.expect("refresh");
    f.core.deny("ev1").await.expect("deny");

    let health = f.core.audit_health().expect("health");
    assert!(health.entries >= 4, "pair, scan, shown, denied: {health:?}");
    assert_eq!(health.pruned, 0);
    assert_eq!(health.broken_at, None, "the chain holds");
    // Every sealed entry is an answer, a pairing or a push; nothing else.
    for e in f.journal().iter().filter(|e| !e.seal.is_empty()) {
        assert!(e.family.is_audit(), "{e:?}");
    }
}

#[tokio::test]
async fn the_same_broker_check_over_and_over_is_one_entry() {
    let f = Fixture::new().await;
    f.pair().await;
    f.broker.on("POST", "/allbypid", 200, r#"{"eventid":[]}"#);

    for _ in 0..5 {
        f.core.refresh().await.expect("refresh");
    }
    let checks = f.of_kind("broker.checked");
    assert_eq!(checks.len(), 1, "five identical checks are one row");
    assert_eq!(checks[0].repeat, 5);
    assert!(checks[0].first_at.is_some(), "and it says when the first one was");
    assert_eq!(checks[0].summary, "nothing waiting");
    assert_eq!(checks[0].raw.as_deref(), Some(r#"{"eventid":[]}"#));
}

#[tokio::test]
async fn the_commentary_is_kept_apart_from_the_record() {
    let f = Fixture::new().await;
    f.pair().await;
    f.broker.on("POST", "/allbypid", 200, r#"{"eventid":[]}"#);
    f.core.refresh().await.expect("refresh");
    f.core.flush().expect("flush");

    let trace = f.core.journal(Some(Family::Trace)).expect("trace");
    assert!(trace.iter().any(|e| e.title.contains("allbypid")), "{:?}", trace.iter().map(|e| e.title.clone()).collect::<Vec<_>>());
    assert!(trace.iter().all(|e| e.seal.is_empty()), "the commentary is not evidence");
    assert!(f.journal().iter().all(|e| e.family != Family::Trace), "and it does not crowd the timeline");

    f.core.clear_trace().expect("clear");
    assert!(f.core.journal(Some(Family::Trace)).expect("trace").is_empty());
    assert_eq!(f.core.audit_health().expect("health").broken_at, None, "clearing the trail leaves the record whole");
}

#[tokio::test]
async fn the_webview_may_only_write_events_the_core_knows() {
    let f = Fixture::new().await;
    f.core.log_push(None, None, Some(r#"{"encedo":{"event":{"id":"g0O7U"}}}"#.into()), false).expect("push");
    let entry = f.latest("push.received");
    assert_eq!(entry.title, "Push received");
    assert_eq!(f.field(&entry, "Kind"), "data only, no notification text");
    assert_eq!(entry.raw.as_deref(), Some(r#"{"encedo":{"event":{"id":"g0O7U"}}}"#));
    assert!(!entry.seal.is_empty(), "a push is part of the record");

    f.core.log_push(Some("Encedo".into()), Some("A request is waiting".into()), None, true).expect("tap");
    assert_eq!(f.latest("push.tapped").title, "Encedo");

    f.core.log_app("app.unlocked", "Unlocked", "fingerprint").expect("lock event");
    assert_eq!(f.latest("app.unlocked").level, Level::Good);
    assert_eq!(view(f.core.log_app("whatever", "x", "y").expect_err("unknown kind")).code, "state");
}

#[tokio::test]
async fn changing_a_setting_is_written_down_once() {
    let f = Fixture::new().await;
    let mut settings = f.core.settings().expect("settings");
    settings.theme = "dark".into();
    settings.biometric_lock = false;
    f.core.set_settings(settings.clone()).expect("save");
    let entry = f.latest("settings.changed");
    assert!(entry.summary.contains("lock off") && entry.summary.contains("theme dark"), "{entry:?}");

    f.core.set_settings(settings).expect("save again");
    assert_eq!(f.of_kind("settings.changed").len(), 1, "saving the same settings says nothing");
}
