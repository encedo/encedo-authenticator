//! Pairing (v1 `parseQRCode` + `genReply`): the QR code carries a link, the
//! link returns a JWT whose payload names the manager (`iss` = eid) and the
//! broker's ephemeral key (`aud` = epk). The app answers with its own key pair.

use serde::{Deserialize, Serialize};

use crate::codec::{b64, b64_decode_32};
use crate::jwt::{hmac_sha256, payload_unverified, sign_hs256};
use crate::keys::KeyPair;
use crate::{Error, Result};

/// Payload of the pairing request JWT (`GET link` → `request`).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PairingRequest {
    pub jti: String,
    pub eat: u64,
    /// Manager public key, base64 (EncedoID).
    pub iss: String,
    /// Broker ephemeral public key for this pairing, base64.
    pub aud: String,
}

impl PairingRequest {
    pub fn from_jwt(token: &str) -> Result<Self> {
        let v = payload_unverified(token)?;
        serde_json::from_value(v).map_err(|_| Error::Malformed("pairing request payload"))
    }

    pub fn eid(&self) -> Result<[u8; 32]> {
        b64_decode_32(&self.iss).ok_or(Error::Malformed("eid"))
    }

    pub fn epk(&self) -> Result<[u8; 32]> {
        b64_decode_32(&self.aud).ok_or(Error::Malformed("epk"))
    }
}

#[derive(Serialize)]
struct ReplyClaims<'a> {
    jti: &'a str,
    eat: u64,
    iss: String,
    aud: &'a str,
    epk: &'a str,
    label: &'a str,
}

/// Body of `POST link` (accept) and `DELETE link` (refuse).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PairingReply {
    pub reply: String,
    pub fid: String,
    pub mac: String,
    pub aid: String,
}

/// Build the reply for `request` with the app's fresh key pair, `label`
/// (device name shown in the Manager) and `fid` (push token).
///
/// `reply = JWT(S1 = X25519(aid_prv, eid), {jti, eat, iss: aid, aud: eid, epk, label})`,
/// `mac = base64(HMAC-SHA256(reply || fid, X25519(aid_prv, epk)))`.
pub fn reply(request: &PairingRequest, app: &KeyPair, label: &str, fid: &str) -> Result<PairingReply> {
    let eid = request.eid()?;
    let epk = request.epk()?;
    let aid = b64(&app.public_bytes());
    let s1 = app.shared(&eid);
    let claims = ReplyClaims { jti: &request.jti, eat: request.eat, iss: aid.clone(), aud: &request.iss, epk: &request.aud, label };
    let jwt = sign_hs256(&claims, &s1);
    let s_epk = app.shared(&epk);
    let mac = b64(&hmac_sha256(&s_epk, format!("{jwt}{fid}").as_bytes()));
    Ok(PairingReply { reply: jwt, fid: fid.to_string(), mac, aid })
}
