//! Local storage: one JSON document (settings, paired modules, archive)
//! encrypted with AES-256-GCM into a single file, written atomically. The
//! 32-byte data key comes from a [`SecretStore`]: the Android Keystore on the
//! phone, a plain file in the app data dir on desktop (development only).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

const MAGIC: &[u8; 4] = b"ENA2";
const FORMAT_VERSION: u8 = 1;
const ARCHIVE_CAP: usize = 500;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("storage io: {0}")]
    Io(#[from] std::io::Error),
    #[error("storage file is not ours or is damaged")]
    Corrupt,
    #[error("storage key unavailable: {0}")]
    Key(String),
    #[error("storage json: {0}")]
    Json(#[from] serde_json::Error),
}

/// Where the data key lives. Implementations must return the same 32 bytes
/// on every call for the lifetime of the installation.
pub trait SecretStore: Send + Sync {
    fn data_key(&self) -> Result<DataKey, StoreError>;
}

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct DataKey(pub [u8; 32]);

/// Development only: the key sits next to the data, readable by the user.
/// Fine for `cargo tauri dev` on a laptop, never for a phone build.
pub struct DevFileSecret {
    pub path: PathBuf,
}

impl SecretStore for DevFileSecret {
    fn data_key(&self) -> Result<DataKey, StoreError> {
        if let Ok(bytes) = fs::read(&self.path) {
            let k: [u8; 32] = bytes.try_into().map_err(|_| StoreError::Corrupt)?;
            return Ok(DataKey(k));
        }
        let mut k = [0u8; 32];
        OsRng.fill_bytes(&mut k);
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        write_private(&self.path, &k)?;
        Ok(DataKey(k))
    }
}

// ---- data model ------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleRecord {
    pub pid: String,
    pub eid: String,
    pub aid: String,
    pub aid_prv: String,
    pub label: String,
    pub host: String,
    pub user: String,
    pub email: String,
    /// Paired at, unix seconds.
    pub iat: u64,
    #[serde(default)]
    pub last_used: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Granted,
    Denied,
    Expired,
    Cancelled,
    Rejected,
    Error,
    Paired,
    Unpaired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArchiveEntry {
    pub id: String,
    pub pid: String,
    pub title: String,
    pub detail: String,
    pub outcome: Outcome,
    /// Unix seconds.
    pub at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settings {
    pub biometric_lock: bool,
    pub lock_on_background: bool,
    pub theme: String,
    pub onboarded: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { biometric_lock: true, lock_on_background: true, theme: "system".into(), onboarded: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Data {
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub modules: Vec<ModuleRecord>,
    #[serde(default)]
    pub archive: Vec<ArchiveEntry>,
    /// Push token last sent to the broker, to notice rotations.
    #[serde(default)]
    pub push_token: Option<String>,
    /// Event ids already answered or discarded, so a stale `allbypid` does not
    /// show them again. Pruned to the last 200.
    #[serde(default)]
    pub handled_events: Vec<String>,
}

impl Data {
    pub fn module(&self, pid: &str) -> Option<&ModuleRecord> {
        self.modules.iter().find(|m| m.pid == pid)
    }

    pub fn archive_push(&mut self, entry: ArchiveEntry) {
        self.archive.insert(0, entry);
        self.archive.truncate(ARCHIVE_CAP);
    }

    pub fn mark_handled(&mut self, event_id: &str) {
        if !self.handled_events.iter().any(|e| e == event_id) {
            self.handled_events.push(event_id.to_string());
            if self.handled_events.len() > 200 {
                let drop = self.handled_events.len() - 200;
                self.handled_events.drain(..drop);
            }
        }
    }
}

// ---- encrypted file --------------------------------------------------------

pub struct Store {
    path: PathBuf,
    key: DataKey,
    pub data: Data,
}

impl Store {
    /// Open (or create) the store at `path` with the key from `secrets`.
    pub fn open(path: PathBuf, secrets: &dyn SecretStore) -> Result<Self, StoreError> {
        let key = secrets.data_key()?;
        let data = match fs::read(&path) {
            Ok(bytes) => decrypt(&key.0, &bytes)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Data::default(),
            Err(e) => return Err(e.into()),
        };
        Ok(Self { path, key, data })
    }

    /// Encrypt and write atomically: temp file in the same directory, then rename.
    pub fn save(&self) -> Result<(), StoreError> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let bytes = encrypt(&self.key.0, &self.data)?;
        let tmp = self.path.with_extension("tmp");
        write_private(&tmp, &bytes)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    /// Mutate and persist in one step.
    pub fn update<T>(&mut self, f: impl FnOnce(&mut Data) -> T) -> Result<T, StoreError> {
        let out = f(&mut self.data);
        self.save()?;
        Ok(out)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn encrypt(key: &[u8; 32], data: &Data) -> Result<Vec<u8>, StoreError> {
    let plain = serde_json::to_vec(data)?;
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let cipher = Aes256Gcm::new(key.into());
    let aad = [FORMAT_VERSION];
    let ct = cipher
        .encrypt(Nonce::from_slice(&nonce), Payload { msg: &plain, aad: &aad })
        .map_err(|_| StoreError::Corrupt)?;
    let mut out = Vec::with_capacity(4 + 1 + 12 + ct.len());
    out.extend_from_slice(MAGIC);
    out.push(FORMAT_VERSION);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

fn decrypt(key: &[u8; 32], bytes: &[u8]) -> Result<Data, StoreError> {
    if bytes.len() < 4 + 1 + 12 + 16 || &bytes[..4] != MAGIC {
        return Err(StoreError::Corrupt);
    }
    let version = bytes[4];
    if version != FORMAT_VERSION {
        return Err(StoreError::Corrupt);
    }
    let nonce = &bytes[5..17];
    let ct = &bytes[17..];
    let cipher = Aes256Gcm::new(key.into());
    let plain = cipher
        .decrypt(Nonce::from_slice(nonce), Payload { msg: ct, aad: &[version] })
        .map_err(|_| StoreError::Corrupt)?;
    Ok(serde_json::from_slice(&plain)?)
}

fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path)?;
    f.write_all(bytes)?;
    f.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_atomic_file() {
        let dir = std::env::temp_dir().join(format!("encedo-store-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let secrets = DevFileSecret { path: dir.join("dev.key") };
        let mut s = Store::open(dir.join("store.bin"), &secrets).unwrap();
        assert_eq!(s.data, Data::default());
        s.update(|d| {
            d.settings.onboarded = true;
            d.modules.push(ModuleRecord { pid: "p".into(), eid: "e".into(), aid: "a".into(), aid_prv: "s".into(), label: "L".into(), host: "h".into(), user: "u".into(), email: "".into(), iat: 1, last_used: None });
            d.archive_push(ArchiveEntry { id: "1".into(), pid: "p".into(), title: "t".into(), detail: "".into(), outcome: Outcome::Granted, at: 2 });
        })
        .unwrap();
        let again = Store::open(dir.join("store.bin"), &secrets).unwrap();
        assert_eq!(again.data, s.data);
        assert!(!dir.join("store.bin.tmp").exists());
        // Wrong key: refused, not garbage.
        let other = DevFileSecret { path: dir.join("other.key") };
        assert!(matches!(Store::open(dir.join("store.bin"), &other), Err(StoreError::Corrupt)));
        let _ = fs::remove_dir_all(&dir);
    }
}
