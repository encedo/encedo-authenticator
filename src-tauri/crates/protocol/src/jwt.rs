//! JWT HS256 exactly as `jwt_generate_hs256` in v1 base.js wrote it: header
//! `{"ecdh":"x25519","alg":"HS256","typ":"JWT"}` in that order, payload fields
//! in the order given, base64url without padding, signature over
//! `head "." body` with the raw shared secret as the HMAC key.

use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::Sha256;

use crate::codec::{b64url, b64url_decode};
use crate::{Error, Result};

type HmacSha256 = Hmac<Sha256>;

#[derive(Serialize)]
struct Header {
    ecdh: &'static str,
    alg: &'static str,
    typ: &'static str,
}

/// Sign `payload` (any `Serialize`, fields serialised in declaration order).
pub fn sign_hs256<T: Serialize>(payload: &T, secret: &[u8]) -> String {
    let header = Header { ecdh: "x25519", alg: "HS256", typ: "JWT" };
    let head = b64url(serde_json::to_string(&header).expect("static header").as_bytes());
    let body = b64url(serde_json::to_string(payload).expect("serialisable payload").as_bytes());
    let signing_input = format!("{head}.{body}");
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(signing_input.as_bytes());
    let sig = b64url(&mac.finalize().into_bytes());
    format!("{signing_input}.{sig}")
}

/// The payload of a JWT without checking its signature. v1 reads the pairing
/// request this way because the app has no shared secret yet at that point.
pub fn payload_unverified(token: &str) -> Result<serde_json::Value> {
    let body = token.split('.').nth(1).ok_or(Error::Malformed("jwt"))?;
    let raw = b64url_decode(body).ok_or(Error::Malformed("jwt payload base64"))?;
    serde_json::from_slice(&raw).map_err(|_| Error::Malformed("jwt payload json"))
}

/// Verify an HS256 signature with `secret`, constant time. Not used by v1
/// (the broker never signs towards the app), kept for the [v2] token endpoint.
pub fn verify_hs256(token: &str, secret: &[u8]) -> bool {
    let mut parts = token.rsplitn(2, '.');
    let (Some(sig), Some(signing_input)) = (parts.next(), parts.next()) else { return false };
    let Some(sig) = b64url_decode(sig) else { return false };
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(signing_input.as_bytes());
    mac.verify_slice(&sig).is_ok()
}

pub(crate) fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(data);
    mac.finalize().into_bytes().into()
}
