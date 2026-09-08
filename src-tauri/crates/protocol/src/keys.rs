//! X25519 as `axlsign.generateKeyPair` / `axlsign.sharedKey` produce it: the
//! secret is 32 random bytes, clamped; the public key is the Montgomery
//! u-coordinate. `x25519-dalek` clamps on use, so feeding it the raw seed or the
//! clamped bytes gives the same result.

use rand_core::OsRng;
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct KeyPair {
    secret: [u8; 32],
    #[zeroize(skip)]
    public: [u8; 32],
}

impl KeyPair {
    /// Fresh key pair from the OS random source. Used once per pairing.
    pub fn generate() -> Self {
        let secret = StaticSecret::random_from_rng(OsRng);
        Self::from_secret(secret.to_bytes())
    }

    /// Key pair from 32 bytes: a stored `aid_prv`, or a seed in tests.
    pub fn from_secret(seed: [u8; 32]) -> Self {
        let secret = StaticSecret::from(seed);
        let public = PublicKey::from(&secret);
        Self { secret: clamp(seed), public: public.to_bytes() }
    }

    /// Clamped secret, as v1 stored it (`aid_prv`).
    pub fn secret_bytes(&self) -> [u8; 32] {
        self.secret
    }

    /// Public key bytes (`aid`).
    pub fn public_bytes(&self) -> [u8; 32] {
        self.public
    }

    pub fn shared(&self, peer_public: &[u8; 32]) -> [u8; 32] {
        shared_secret(&self.secret, peer_public)
    }
}

/// Raw X25519 shared secret, the 32 bytes v1 base64-encodes and then parses
/// straight back into an HMAC key.
pub fn shared_secret(secret: &[u8; 32], peer_public: &[u8; 32]) -> [u8; 32] {
    let s = StaticSecret::from(*secret);
    s.diffie_hellman(&PublicKey::from(*peer_public)).to_bytes()
}

fn clamp(mut k: [u8; 32]) -> [u8; 32] {
    k[0] &= 248;
    k[31] &= 127;
    k[31] |= 64;
    k
}
