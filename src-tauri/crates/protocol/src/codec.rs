//! Encodings shared by every flow: standard base64 for values, base64url for
//! JWT segments and for `pid` in URL paths.

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;

pub fn b64(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}

pub fn b64_decode(s: &str) -> Option<Vec<u8>> {
    STANDARD.decode(s.trim()).ok()
}

pub fn b64_decode_32(s: &str) -> Option<[u8; 32]> {
    b64_decode(s)?.try_into().ok()
}

pub fn b64url(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn b64url_decode(s: &str) -> Option<Vec<u8>> {
    // v1 tolerates both alphabets and missing padding when reading JWT segments.
    let s = s.replace('-', "+").replace('_', "/");
    let s = s.trim_end_matches('=');
    let padded = format!("{s}{}", "=".repeat((4 - s.len() % 4) % 4));
    STANDARD.decode(padded).ok()
}

/// `pid` as the server returns it (standard base64) → the form used in URL
/// paths: `/` → `_`, `+` → `-`, padding dropped. v1's `Base64EncodeUrl`.
pub fn pid_to_path(pid: &str) -> String {
    pid.replace('+', "-").replace('/', "_").trim_end_matches('=').to_string()
}
