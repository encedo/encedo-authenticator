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
    /// What the requester says the access is for (`note` of `ext/request`, 1-128
    /// chars), passed on by the broker as plain text: shown, never trusted (DEV-512).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// The requester's demands (`ctx` of `ext/request`, 1-64 chars): `key=value`
    /// items separated by `;`, passed on by the broker like `note`. Read by the
    /// app (`uv=bio`, `uv=pin`, DEV-511); not verified by anything here, which is
    /// why the answer says what was actually done (`actx.uvm`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ctx: Option<String>,
}

/// How the person confirmed one approval, or what a requester demands for it
/// (`uv` in `ctx`, `uvm` in `actx`). Ordered: `Bio` satisfies `Pin`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Uv {
    None,
    /// The device PIN, pattern or password (or biometrics: the system prompt
    /// that allows both does not say which one was used).
    Pin,
    /// Biometrics only, the device credential not accepted.
    Bio,
}

impl Uv {
    pub fn as_str(self) -> &'static str {
        match self {
            Uv::None => "none",
            Uv::Pin => "pin",
            Uv::Bio => "bio",
        }
    }

    pub fn parse(s: &str) -> Option<Uv> {
        match s {
            "none" => Some(Uv::None),
            "pin" => Some(Uv::Pin),
            "bio" => Some(Uv::Bio),
            _ => None,
        }
    }

    /// What `ctx` demands of the approval: the strongest `uv=` item, or none.
    /// Items are `key=value` separated by `;`; anything not understood
    /// (other keys, other values, bare words like `oidc`) is ignored.
    pub fn wanted(ctx: Option<&str>) -> Uv {
        ctx.unwrap_or_default()
            .split(';')
            .filter_map(|item| item.split_once('='))
            .filter(|(k, _)| k.trim() == "uv")
            .filter_map(|(_, v)| match v.trim() {
                "bio" => Some(Uv::Bio),
                "pin" => Some(Uv::Pin),
                _ => None,
            })
            .max()
            .unwrap_or(Uv::None)
    }
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

/// The app's login context (DEV-512): how the person at this phone was
/// verified, under the authreply's MAC. The HEM copies it 1:1 into its token
/// and into the `auth` block of the key attestation (FW DEV-511) without
/// reading it; Encedo OIDC turns it into `amr` / `acr`. A statement of the
/// app, not a proof.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Actx {
    /// Format version.
    pub v: u8,
    /// `bio+pin`, `pin` or `none`: what unlocks this app.
    pub lock: &'static str,
    /// Seconds from the last user verification to `iat`; absent with `lock: none`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uva: Option<u64>,
    /// `android` or `ios`.
    pub aos: &'static str,
    /// Version and commit of this build, `2.0.0-dev.15+9bad376`.
    pub app: String,
    /// Where the storage wrapping key is: `sb`, `tee`, `sw` or `kc`; absent when unknown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hw: Option<&'static str>,
    /// How the person confirmed this one approval: `bio`, `pin` or `none`
    /// (DEV-511). `uva` says when the app was last unlocked; this says what
    /// happened for the answer itself.
    pub uvm: &'static str,
}

/// The longest `actx` the HEM copies (serialized, FW `AUTH_ACTX_MAX`); a longer
/// one would be dropped there without a word, so it is refused here.
pub const ACTX_MAX: usize = 128;

#[derive(Serialize)]
struct AllowClaims<'a> {
    aud: &'a str,
    jti: &'a str,
    exp: u64,
    iat: u64,
    pid: &'a str,
    iss: String,
    scope: String,
    /// The request's `ctx`, echoed as received (DEV-511): FW 1.3 takes the
    /// token's `ctx` from the authreply, not from the request it no longer has.
    #[serde(skip_serializing_if = "Option::is_none")]
    ctx: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actx: Option<&'a Actx>,
}

/// The longest `ctx` the HEM takes (`ext/request` and `ext/token`).
pub const CTX_MAX: usize = 64;

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

/// Grant `scope` (as decrypted) for `period_secs` from `now`, with `:rw` on
/// the drive value when `writable` (and stripped otherwise, as v1 does).
/// `actx` goes into the claims when given; without it the reply is byte for
/// byte what v1 sent.
#[allow(clippy::too_many_arguments)]
pub fn allow(event: &Event, app: &KeyPair, eid_b64: &str, pid: &str, scope: &str, writable: bool, now: u64, period_secs: u64, actx: Option<&Actx>) -> Result<AllowReply> {
    if let Some(a) = actx {
        if serde_json::to_string(a).map_err(|_| Error::Malformed("actx"))?.len() > ACTX_MAX {
            return Err(Error::Malformed("actx size"));
        }
    }
    let eid = b64_decode_32(eid_b64).ok_or(Error::Malformed("eid"))?;
    let epk = b64_decode_32(&event.epk).ok_or(Error::Malformed("epk"))?;
    // FW 1.3.x: `:rw` belongs to the storage value of a list (one at most), `#<meta>` stays at the end
    let meta = scope.find('#').map_or("", |i| &scope[i..]);
    let list = &scope[..scope.len() - meta.len()];
    let mut granted = list.split(' ').map(|v| grant_value(v, writable)).collect::<Vec<_>>().join(" ");
    granted.push_str(meta);
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
        // 1..64 as the HEM requires; anything else would be dropped there, so it is not sent
        ctx: event.ctx.as_deref().filter(|c| !c.is_empty() && c.len() <= CTX_MAX),
        actx,
    };
    let jwt = sign_hs256(&claims, &session.s1);
    let mac = b64(&hmac_sha256(&app.shared(&epk), jwt.as_bytes()));
    Ok(AllowReply { authreply: jwt, mac, scope: granted, exp })
}

/// One value as granted: a drive read-write when `writable`, otherwise without `:rw`.
fn grant_value(value: &str, writable: bool) -> String {
    let base = value.trim_end_matches(":rw");
    match writable && value.starts_with("storage:disk") {
        true => format!("{}:rw", base.trim_end_matches(":ro")),
        false => base.to_string(),
    }
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
