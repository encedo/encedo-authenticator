//! The module side of the same protocol, for tests: what a HEM (and the broker
//! acting for it) puts on the wire, and what it checks when the phone answers.
//! Every function here mirrors one in this crate from the other end, so a round
//! trip proves both directions instead of only our own bytes.
//!
//! Never compiled into a phone build: the `hem` feature is switched on by
//! `[dev-dependencies]` in the app crate.

use serde_json::{json, Value};

use crate::codec::{b64, b64_decode, b64_decode_32};
use crate::event::{open_scope, seal_scope, Event, IpInfo};
use crate::jwt::{hmac_sha256, payload_unverified, sign_hs256, verify_hs256};
use crate::keys::KeyPair;
use crate::pairing::{PairingReply, PairingRequest};
use crate::{Error, Result};

/// What the module learns about a phone from `POST link`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedPhone {
    pub aid: String,
    pub label: String,
    pub fid: String,
}

/// What the module reads out of an `authreply`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Granted {
    pub scope: String,
    pub pid: String,
    pub iat: u64,
    pub exp: u64,
}

/// A module with its long-term key (`eid`) and the broker's ephemeral key
/// (`epk`) for one pairing or event.
pub struct Hem {
    pub keys: KeyPair,
    pub epk: KeyPair,
}

impl Hem {
    /// Fixed seeds, so a failing test can be replayed byte for byte.
    pub fn with_seeds(eid_seed: u8, epk_seed: u8) -> Self {
        Self { keys: KeyPair::from_secret([eid_seed; 32]), epk: KeyPair::from_secret([epk_seed; 32]) }
    }

    pub fn eid(&self) -> String {
        b64(&self.keys.public_bytes())
    }

    pub fn epk(&self) -> String {
        b64(&self.epk.public_bytes())
    }

    /// Where the issuer looks like it is, as the broker reports it.
    pub fn ipinfo() -> IpInfo {
        IpInfo { city: "Warsaw".into(), country: "PL".into(), ip: "203.0.113.7".into() }
    }

    // ---- pairing ----------------------------------------------------------

    /// The JWT served by `GET link`. The phone reads this payload without
    /// verifying it (it has no shared secret yet), exactly as v1 did, so the
    /// signing key here only has to be something the module knows.
    pub fn pairing_request(&self, jti: &str, exp: Option<u64>) -> String {
        let mut claims = json!({ "jti": jti });
        if let Some(exp) = exp {
            claims["exp"] = json!(exp);
        }
        claims["aud"] = json!(self.epk());
        claims["iss"] = json!(self.eid());
        sign_hs256(&claims, &self.keys.secret_bytes())
    }

    /// Same, with the claim named in `drop` left out: the phone must say which
    /// field is missing instead of "malformed".
    pub fn pairing_request_without(&self, jti: &str, drop: &str) -> String {
        let mut claims = json!({ "jti": jti, "aud": self.epk(), "iss": self.eid() });
        claims.as_object_mut().expect("object").remove(drop);
        sign_hs256(&claims, &self.keys.secret_bytes())
    }

    /// `{request, ipinfo_eid}`, the body of `GET link`.
    pub fn pairing_offer(&self, request: &str) -> String {
        json!({ "request": request, "ipinfo_eid": Self::ipinfo() }).to_string()
    }

    /// What the Manager prints into the QR code: the link, the promise of what
    /// it serves (`hash`) and who is asking.
    pub fn qr_code(&self, link: &str, request: &str) -> String {
        use sha2::{Digest, Sha256};
        json!({
            "link": link,
            "hash": b64(&Sha256::digest(request.as_bytes())),
            "user": "chris",
            "hostname": "hem.example.org",
            "email": "chris@example.org",
        })
        .to_string()
    }

    /// The checks the module runs on `POST link`: the echoed claims, the JWT
    /// signature under `X25519(aid_prv, eid)` and the MAC over `reply || fid`
    /// under `X25519(aid_prv, epk)`.
    pub fn check_pairing_reply(&self, request: &PairingRequest, r: &PairingReply) -> Result<PairedPhone> {
        let aid = b64_decode_32(&r.aid).ok_or(Error::Malformed("aid"))?;
        if !verify_hs256(&r.reply, &self.keys.shared(&aid)) {
            return Err(Error::BadMac);
        }
        let p = payload_unverified(&r.reply)?;
        for (field, want) in [("jti", request.jti.as_str()), ("iss", r.aid.as_str()), ("aud", &self.eid()), ("epk", &self.epk())] {
            if p.get(field).and_then(Value::as_str) != Some(want) {
                return Err(Error::MalformedField("pairing reply", field));
            }
        }
        if p.get("eat") != request.eat.as_ref() {
            return Err(Error::MalformedField("pairing reply", "eat"));
        }
        let mac = hmac_sha256(&self.epk.shared(&aid), format!("{}{}", r.reply, r.fid).as_bytes());
        if b64(&mac) != r.mac {
            return Err(Error::BadMac);
        }
        Ok(PairedPhone {
            aid: r.aid.clone(),
            label: p.get("label").and_then(Value::as_str).unwrap_or_default().to_string(),
            fid: r.fid.clone(),
        })
    }

    // ---- events -----------------------------------------------------------

    /// An event for a paired phone, `scope` sealed under the session key.
    pub fn event(&self, aid_b64: &str, jti: &str, scope: &str, exp: u64) -> Result<Event> {
        let aid = b64_decode_32(aid_b64).ok_or(Error::Malformed("aid"))?;
        Ok(Event {
            jti: jti.to_string(),
            scope: seal_scope(&self.keys, &aid, jti, scope)?,
            epk: self.epk(),
            exp,
            ipinfo_eid: Some(Self::ipinfo()),
        })
    }

    /// The same event with the last byte of the scope MAC flipped: what a
    /// tampered or mis-keyed event looks like to the phone.
    pub fn event_with_bad_mac(&self, aid_b64: &str, jti: &str, scope: &str, exp: u64) -> Result<Event> {
        let mut ev = self.event(aid_b64, jti, scope, exp)?;
        let mut raw = b64_decode(ev.scope.trim_start_matches('A')).ok_or(Error::Malformed("scope base64"))?;
        let last = raw.len() - 1;
        raw[last] ^= 0x01;
        ev.scope = format!("A{}", b64(&raw));
        Ok(ev)
    }

    /// `POST /notify/event/data/{event}/{pidx}` as the module reads it: the MAC
    /// over the JWT under `X25519(aid_prv, epk)`, the JWT signature under the
    /// pairing secret, and the scope sealed inside it.
    pub fn check_allow(&self, aid_b64: &str, jti: &str, body: &Value) -> Result<Granted> {
        let aid = b64_decode_32(aid_b64).ok_or(Error::Malformed("aid"))?;
        let authreply = body.get("authreply").and_then(Value::as_str).ok_or(Error::MalformedField("allow", "authreply"))?;
        let mac = body.get("mac").and_then(Value::as_str).ok_or(Error::MalformedField("allow", "mac"))?;
        if b64(&hmac_sha256(&self.epk.shared(&aid), authreply.as_bytes())) != mac {
            return Err(Error::BadMac);
        }
        if !verify_hs256(authreply, &self.keys.shared(&aid)) {
            return Err(Error::BadMac);
        }
        let p = payload_unverified(authreply)?;
        for (field, want) in [("jti", jti), ("aud", &self.eid()), ("iss", aid_b64)] {
            if p.get(field).and_then(Value::as_str) != Some(want) {
                return Err(Error::MalformedField("allow", field));
            }
        }
        let sealed = p.get("scope").and_then(Value::as_str).ok_or(Error::MalformedField("allow", "scope"))?;
        // The granted scope travels sealed under the same session key; opening it
        // needs no expiry check, so pass an event that cannot be expired.
        let holder = Event { jti: jti.to_string(), scope: sealed.to_string(), epk: String::new(), exp: u64::MAX, ipinfo_eid: None };
        let scope = open_scope(&holder, &self.keys, &aid, 0)?.scope;
        Ok(Granted {
            scope,
            pid: p.get("pid").and_then(Value::as_str).unwrap_or_default().to_string(),
            iat: p.get("iat").and_then(Value::as_u64).unwrap_or_default(),
            exp: p.get("exp").and_then(Value::as_u64).unwrap_or_default(),
        })
    }

    // ---- session, unpair, token -------------------------------------------

    /// `POST /notify/session` hands the phone this ephemeral key.
    pub fn session(&self) -> String {
        json!({ "epk": self.epk() }).to_string()
    }

    /// `POST /notify/subscribers/delete`: the phone proves it holds `aid_prv`
    /// with a MAC over the nonce under `X25519(aid_prv, epk)`.
    pub fn check_delete_body(&self, body: &Value) -> Result<String> {
        let (pid, nonce, aid) = self.nonce_mac_parts(body)?;
        if b64(&hmac_sha256(&self.epk.shared(&aid), &nonce)) != body["mac"].as_str().unwrap_or_default() {
            return Err(Error::BadMac);
        }
        Ok(pid)
    }

    /// `POST /notify/subscribers/token`: same proof, over nonce and the new token.
    pub fn check_token_body(&self, body: &Value) -> Result<(String, String)> {
        let (pid, nonce, aid) = self.nonce_mac_parts(body)?;
        let fid = body.get("fid").and_then(Value::as_str).ok_or(Error::MalformedField("token", "fid"))?;
        let mut msg = nonce;
        msg.extend_from_slice(fid.as_bytes());
        if b64(&hmac_sha256(&self.epk.shared(&aid), &msg)) != body["mac"].as_str().unwrap_or_default() {
            return Err(Error::BadMac);
        }
        Ok((pid, fid.to_string()))
    }

    fn nonce_mac_parts(&self, body: &Value) -> Result<(String, Vec<u8>, [u8; 32])> {
        let pid = body.get("pid").and_then(Value::as_str).ok_or(Error::MalformedField("proof", "pid"))?;
        let aid = body.get("aid").and_then(Value::as_str).ok_or(Error::MalformedField("proof", "aid"))?;
        let nonce = body.get("nonce").and_then(Value::as_str).ok_or(Error::MalformedField("proof", "nonce"))?;
        let nonce = b64_decode(nonce).ok_or(Error::Malformed("nonce"))?;
        if nonce.len() != 32 {
            return Err(Error::Malformed("nonce length"));
        }
        Ok((pid.to_string(), nonce, b64_decode_32(aid).ok_or(Error::Malformed("aid"))?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{allow, PERIODS};
    use crate::pairing::reply;
    use crate::unpair::{delete_body, token_body};

    /// One pairing, one event, one grant: both ends of every step agree.
    #[test]
    fn round_trip_pairing_event_allow() {
        let hem = Hem::with_seeds(7, 9);
        let phone = KeyPair::from_secret([3u8; 32]);
        let aid = b64(&phone.public_bytes());

        let request = PairingRequest::from_jwt(&hem.pairing_request("jti-1", Some(2_000_000_000))).unwrap();
        assert_eq!(request.iss, hem.eid());
        let r = reply(&request, &phone, "Galaxy S24 (Android)", "fid-abc").unwrap();
        let paired = hem.check_pairing_reply(&request, &r).unwrap();
        assert_eq!(paired, PairedPhone { aid: aid.clone(), label: "Galaxy S24 (Android)".into(), fid: "fid-abc".into() });

        let ev = hem.event(&aid, "amNRPXl5", "storage:disk:home", 2_000_000_000).unwrap();
        let opened = open_scope(&ev, &phone, &b64_decode_32(&hem.eid()).unwrap(), 1_000).unwrap();
        assert_eq!(opened.scope, "storage:disk:home");

        let reply = allow(&ev, &phone, &hem.eid(), "pid-1", &opened.scope, true, 1_000, PERIODS[1]).unwrap();
        let body = serde_json::to_value(&reply).unwrap();
        let granted = hem.check_allow(&aid, "amNRPXl5", &body).unwrap();
        assert_eq!(granted, Granted { scope: "storage:disk:home:rw".into(), pid: "pid-1".into(), iat: 1_000, exp: 1_000 + PERIODS[1] });
    }

    #[test]
    fn tampered_scope_and_wrong_phone_are_caught() {
        let hem = Hem::with_seeds(7, 9);
        let phone = KeyPair::from_secret([3u8; 32]);
        let aid = b64(&phone.public_bytes());
        let eid = b64_decode_32(&hem.eid()).unwrap();

        let bad = hem.event_with_bad_mac(&aid, "amNRPXl5", "logger:get", 2_000_000_000).unwrap();
        assert_eq!(open_scope(&bad, &phone, &eid, 1_000), Err(Error::BadMac));

        // Another phone's reply must not pass as this one's.
        let other = KeyPair::from_secret([4u8; 32]);
        let request = PairingRequest::from_jwt(&hem.pairing_request("jti-1", None)).unwrap();
        let mut r = reply(&request, &other, "other", "fid").unwrap();
        r.aid = aid;
        assert_eq!(hem.check_pairing_reply(&request, &r), Err(Error::BadMac));
    }

    #[test]
    fn nonce_proofs_verify() {
        let hem = Hem::with_seeds(7, 9);
        let phone = KeyPair::from_secret([3u8; 32]);
        let nonce = [5u8; 32];
        let del = serde_json::to_value(delete_body("pid-1", &hem.epk(), &phone, &nonce).unwrap()).unwrap();
        assert_eq!(hem.check_delete_body(&del).unwrap(), "pid-1");
        let tok = serde_json::to_value(token_body("pid-1", &hem.epk(), &phone, &nonce, "fid-new").unwrap()).unwrap();
        assert_eq!(hem.check_token_body(&tok).unwrap(), ("pid-1".into(), "fid-new".into()));
        let mut forged = del;
        forged["mac"] = json!(b64(&[0u8; 32]));
        assert_eq!(hem.check_delete_body(&forged), Err(Error::BadMac));
    }
}
