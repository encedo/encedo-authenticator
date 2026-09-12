//! An access request (v1 `executeEvent`): the broker hands over
//! `{jti, scope, epk, exp}`; the scope is encrypted with a session key derived
//! from the pairing secret and `jti`. The app decrypts, checks the MAC ([v2]),
//! shows the request, and answers with a re-encrypted scope inside a JWT.

use aes::Aes128;
use cbc::cipher::block_padding::Pkcs7;
use cbc::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::codec::{b64, b64_decode, b64_decode_32};
use crate::jwt::{hmac_sha256, sign_hs256};
use crate::keys::KeyPair;
use crate::{Error, Result};

type Enc = cbc::Encryptor<Aes128>;
type Dec = cbc::Decryptor<Aes128>;

/// `GET /notify/event/data/{event}/{pidx}`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Event {
    pub jti: String,
    pub scope: String,
    pub epk: String,
    pub exp: u64,
    #[serde(default)]
    pub ipinfo_eid: Option<IpInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Default)]
pub struct IpInfo {
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub ip: String,
}

/// Session material for one event. Zeroed on drop.
#[derive(Zeroize, ZeroizeOnDrop)]
struct Session {
    s1: [u8; 32],
    master: [u8; 32],
}

impl Session {
    fn derive(app: &KeyPair, eid: &[u8; 32], jti: &str) -> Result<Self> {
        let jti_bytes = b64_decode(jti).ok_or(Error::Malformed("jti"))?;
        let s1 = app.shared(eid);
        let master = hmac_sha256(&s1, &jti_bytes);
        Ok(Self { s1, master })
    }
    fn key(&self) -> [u8; 16] {
        self.master[..16].try_into().unwrap()
    }
    fn iv(&self) -> [u8; 16] {
        self.master[16..].try_into().unwrap()
    }
    fn seal(&self, scope: &str) -> String {
        let ct = Enc::new(&self.key().into(), &self.iv().into()).encrypt_padded_vec_mut::<Pkcs7>(scope.as_bytes());
        let mac = hmac_sha256(&self.master, scope.as_bytes());
        let mut out = ct;
        out.extend_from_slice(&mac);
        format!("A{}", b64(&out))
    }
}

/// The decrypted, verified scope of an event, ready to display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedScope {
    pub scope: String,
}

/// Decrypt and verify the scope of `event` for the pairing (`app`, `eid`).
/// Fails with [`Error::Expired`] when `now > exp` (checked first, as v1 meant
/// to), [`Error::BadMac`] when the MAC does not match (v1 only logged it).
pub fn open_scope(event: &Event, app: &KeyPair, eid: &[u8; 32], now: u64) -> Result<OpenedScope> {
    if now > event.exp {
        return Err(Error::Expired);
    }
    let raw = event.scope.strip_prefix('A').ok_or(Error::Malformed("scope algorithm"))?;
    let raw = b64_decode(raw).ok_or(Error::Malformed("scope base64"))?;
    if raw.len() < 32 + 16 {
        return Err(Error::Malformed("scope length"));
    }
    let (ct, mac) = raw.split_at(raw.len() - 32);
    let session = Session::derive(app, eid, &event.jti)?;
    let plain = Dec::new(&session.key().into(), &session.iv().into())
        .decrypt_padded_vec_mut::<Pkcs7>(ct)
        .map_err(|_| Error::Malformed("scope padding"))?;
    let expected = hmac_sha256(&session.master, &plain);
    if expected.ct_eq(mac).unwrap_u8() != 1 {
        return Err(Error::BadMac);
    }
    let scope = String::from_utf8(plain).map_err(|_| Error::NotUtf8)?;
    Ok(OpenedScope { scope })
}

#[derive(Serialize)]
struct AllowClaims<'a> {
    aud: &'a str,
    jti: &'a str,
    exp: u64,
    iat: u64,
    pid: &'a str,
    iss: String,
    scope: String,
}

/// Body of `POST /notify/event/data/{event}/{pidx}`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AllowReply {
    pub authreply: String,
    pub mac: String,
    /// The scope actually granted (with or without `:rw`), for the archive.
    #[serde(skip)]
    pub scope: String,
    #[serde(skip)]
    pub exp: u64,
}

/// Grant `scope` (as decrypted) for `period_secs` from `now`, with `:rw`
/// appended when `writable` (and stripped otherwise, as v1 does).
pub fn allow(event: &Event, app: &KeyPair, eid_b64: &str, pid: &str, scope: &str, writable: bool, now: u64, period_secs: u64) -> Result<AllowReply> {
    let eid = b64_decode_32(eid_b64).ok_or(Error::Malformed("eid"))?;
    let epk = b64_decode_32(&event.epk).ok_or(Error::Malformed("epk"))?;
    let mut granted = scope.replace(":rw", "");
    if writable {
        granted.push_str(":rw");
    }
    let session = Session::derive(app, &eid, &event.jti)?;
    let exp = now + period_secs;
    let claims = AllowClaims {
        aud: eid_b64,
        jti: &event.jti,
        exp,
        iat: now,
        pid,
        iss: b64(&app.public_bytes()),
        scope: session.seal(&granted),
    };
    let jwt = sign_hs256(&claims, &session.s1);
    let mac = b64(&hmac_sha256(&app.shared(&epk), jwt.as_bytes()));
    Ok(AllowReply { authreply: jwt, mac, scope: granted, exp })
}

/// Access periods offered by the UI, in seconds. v1: 15 min default, 1 h, 8 h, 24 h.
pub const PERIODS: [u64; 4] = [15 * 60, 60 * 60, 8 * 60 * 60, 24 * 60 * 60];

/// Seal `scope` the way a module does when it issues an event: the other end of
/// [`open_scope`], under the same session key. Only for the `hem` simulator and
/// this crate's own tests; a phone never seals an event.
#[cfg(any(test, feature = "hem"))]
pub fn seal_scope(module: &KeyPair, phone_public: &[u8; 32], jti: &str, scope: &str) -> Result<String> {
    Ok(Session::derive(module, phone_public, jti)?.seal(scope))
}
