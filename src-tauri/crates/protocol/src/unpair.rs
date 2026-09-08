//! Unpair from the app (v1 `unpairDevice`) and the [v2] push-token refresh,
//! both authenticated the same way: `POST /notify/session {aid}` yields a fresh
//! `epk`, the app proves possession of `aid_prv` with an HMAC under
//! `X25519(aid_prv, epk)` over a random nonce.

use serde::Serialize;

use crate::codec::{b64, b64_decode, b64_decode_32};
use crate::jwt::hmac_sha256;
use crate::keys::KeyPair;
use crate::{Error, Result};

/// Body of `POST /notify/subscribers/delete`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DeleteBody {
    pub pid: String,
    pub epk: String,
    pub nonce: String,
    pub mac: String,
    pub aid: String,
}

/// `mac = base64(HMAC-SHA256(nonce_bytes, X25519(aid_prv, epk)))`.
pub fn delete_body(pid: &str, epk_b64: &str, app: &KeyPair, nonce: &[u8; 32]) -> Result<DeleteBody> {
    let epk = b64_decode_32(epk_b64).ok_or(Error::Malformed("epk"))?;
    let mac = b64(&hmac_sha256(&app.shared(&epk), nonce));
    Ok(DeleteBody { pid: pid.to_string(), epk: epk_b64.to_string(), nonce: b64(nonce), mac, aid: b64(&app.public_bytes()) })
}

/// Body of the [v2] `POST /notify/subscribers/token`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TokenBody {
    pub pid: String,
    pub aid: String,
    pub fid: String,
    pub nonce: String,
    pub mac: String,
}

/// `mac = base64(HMAC-SHA256(nonce_bytes || utf8(fid), X25519(aid_prv, epk)))`.
pub fn token_body(pid: &str, epk_b64: &str, app: &KeyPair, nonce: &[u8; 32], fid: &str) -> Result<TokenBody> {
    let epk = b64_decode_32(epk_b64).ok_or(Error::Malformed("epk"))?;
    let mut msg = nonce.to_vec();
    msg.extend_from_slice(fid.as_bytes());
    let mac = b64(&hmac_sha256(&app.shared(&epk), &msg));
    Ok(TokenBody { pid: pid.to_string(), aid: b64(&app.public_bytes()), fid: fid.to_string(), nonce: b64(nonce), mac })
}

/// Same, with the nonce given as base64 (as stored in a vector).
pub fn nonce_from_b64(s: &str) -> Result<[u8; 32]> {
    b64_decode(s).and_then(|v| v.try_into().ok()).ok_or(Error::Malformed("nonce"))
}
