//! Known-answer tests against `tests/vectors/v1.json`, generated from the v1
//! JavaScript (`node tests/vectors/gen.mjs` at the repo root).

use encedo_protocol::codec::{b64, b64_decode_32, pid_to_path};
use encedo_protocol::event::{allow, open_scope, Event};
use encedo_protocol::jwt::{sign_hs256, verify_hs256};
use encedo_protocol::keys::{shared_secret, KeyPair};
use encedo_protocol::pairing::{reply, PairingReply, PairingRequest};
use encedo_protocol::unpair::{delete_body, nonce_from_b64, token_body};
use encedo_protocol::Error;
use serde_json::Value;

fn vectors() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../tests/vectors/v1.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("run node tests/vectors/gen.mjs first")).unwrap()
}

fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v[k].as_str().unwrap_or_else(|| panic!("missing {k}"))
}
fn key32(v: &Value, k: &str) -> [u8; 32] {
    b64_decode_32(s(v, k)).unwrap()
}

#[test]
fn x25519_matches_axlsign() {
    let v = vectors();
    let seed = &v["x25519"][0];
    let kp = KeyPair::from_secret(key32(seed, "seed"));
    assert_eq!(b64(&kp.secret_bytes()), s(seed, "private_clamped"));
    assert_eq!(b64(&kp.public_bytes()), s(seed, "public"));
    for case in v["x25519"].as_array().unwrap().iter().skip(1) {
        let shared = shared_secret(&key32(case, "private"), &key32(case, "peer_public"));
        assert_eq!(b64(&shared), s(case, "shared"), "{}", s(case, "name"));
    }
}

#[test]
fn jwt_matches_v1() {
    let v = vectors();
    let case = &v["jwt"][0];
    let secret = key32(case, "secret");
    let payload = case["payload"].clone();
    let jwt = sign_hs256(&payload, &secret);
    assert_eq!(jwt, s(case, "jwt"));
    assert!(verify_hs256(&jwt, &secret));
    assert!(!verify_hs256(&jwt, &[0u8; 32]));
}

#[test]
fn pid_path_matches_v1() {
    for case in vectors()["pid"].as_array().unwrap() {
        assert_eq!(pid_to_path(s(case, "pid")), s(case, "pidx"));
    }
}

#[test]
fn pairing_reply_matches_v1() {
    let v = vectors();
    let p = &v["pairing"];
    let req: PairingRequest = serde_json::from_value(p["inputs"]["request_payload"].clone()).unwrap();
    let app = KeyPair::from_secret(key32(&p["inputs"], "aid_seed"));
    assert_eq!(b64(&app.public_bytes()), s(&p["derived"], "aid"));
    assert_eq!(b64(&app.secret_bytes()), s(&p["derived"], "aid_prv"));
    let out = reply(&req, &app, s(&p["inputs"], "label"), s(&p["inputs"], "fid")).unwrap();
    let expected: PairingReply = serde_json::from_value(p["expected"]["post_body"].clone()).unwrap();
    assert_eq!(out, expected);

    // The module's own challenge has no `eat`: v1 dropped the claim, so do we.
    let mut no_eat = p["inputs"]["request_payload"].clone();
    no_eat.as_object_mut().unwrap().remove("eat");
    no_eat["exp"] = serde_json::json!(4102444800u64);
    let req2: PairingRequest = serde_json::from_value(no_eat).unwrap();
    let out2 = reply(&req2, &app, "x", "y").unwrap();
    let body = encedo_protocol::jwt::payload_unverified(&out2.reply).unwrap();
    assert!(body.get("eat").is_none());
    assert_eq!(body.as_object().unwrap().keys().cloned().collect::<Vec<_>>(), ["jti", "iss", "aud", "epk", "label"]);
    assert!(!req2.expired(4102444799));
    assert!(req2.expired(4102444801));
}

#[test]
fn events_match_v1() {
    let v = vectors();
    for case in v["events"].as_array().unwrap() {
        let name = s(case, "name");
        let inp = &case["inputs"];
        let event: Event = serde_json::from_value(inp["event"].clone()).unwrap();
        let app = KeyPair::from_secret(key32(inp, "aid_prv"));
        let eid = key32(inp, "eid");
        let now = inp["now"].as_u64().unwrap();
        let opened = open_scope(&event, &app, &eid, now);
        if !case["expected"]["mac_valid"].as_bool().unwrap() {
            assert_eq!(opened, Err(Error::BadMac), "{name}");
            continue;
        }
        let opened = opened.unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(opened.scope, s(&case["expected"], "scope"), "{name}");
        assert_eq!(open_scope(&event, &app, &eid, event.exp + 1), Err(Error::Expired), "{name} expiry");

        let exp = &case["expected"]["reply"];
        let period = inp["period_min"].as_u64().unwrap() * 60;
        let out = allow(&event, &app, s(inp, "eid"), s(inp, "pid"), &opened.scope, inp["writable"].as_bool().unwrap(), now, period).unwrap();
        assert_eq!(out.scope, s(exp, "scope_sent"), "{name} scope");
        assert_eq!(out.exp, exp["exp"].as_u64().unwrap(), "{name} exp");
        assert_eq!(out.authreply, s(exp, "authreply"), "{name} authreply");
        assert_eq!(out.mac, s(exp, "mac"), "{name} mac");
    }
}

#[test]
fn unpair_and_token_match_v1() {
    let v = vectors();
    let u = &v["unpair"];
    let app = KeyPair::from_secret(key32(&u["inputs"], "aid_prv"));
    let nonce = nonce_from_b64(s(&u["inputs"], "nonce")).unwrap();
    let body = delete_body(s(&u["inputs"], "pid"), s(&u["inputs"], "epk"), &app, &nonce).unwrap();
    assert_eq!(serde_json::to_value(&body).unwrap(), u["expected"]["post_body"]);

    let t = &v["token_refresh"];
    let nonce = nonce_from_b64(s(&t["inputs"], "nonce")).unwrap();
    let body = token_body(s(&t["inputs"], "pid"), s(&t["inputs"], "epk"), &app, &nonce, s(&t["inputs"], "fid")).unwrap();
    assert_eq!(body.mac, s(&t["expected"], "mac"));
}
