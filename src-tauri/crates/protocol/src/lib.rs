//! The Encedo notify protocol, byte for byte as the v1 app spoke it, with the
//! checks v1 skipped (scope MAC, expiry) added. Pure Rust, no I/O: the HTTP
//! client and storage live in the app crate. Every function here is covered by
//! known-answer vectors generated from the v1 JavaScript (`tests/vectors/v1.json`).

pub mod codec;
pub mod event;
#[cfg(any(test, feature = "hem"))]
pub mod hem;
pub mod jwt;
pub mod keys;
pub mod pairing;
pub mod unpair;

pub use event::{AllowReply, Event, OpenedScope};
pub use keys::{shared_secret, KeyPair};
pub use pairing::{PairingReply, PairingRequest};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("malformed {0}")]
    Malformed(&'static str),
    #[error("malformed {0}: field {1} missing or not a string")]
    MalformedField(&'static str, &'static str),
    #[error("scope MAC does not match")]
    BadMac,
    #[error("request expired")]
    Expired,
    #[error("scope is not valid UTF-8")]
    NotUtf8,
}

pub type Result<T> = std::result::Result<T, Error>;
