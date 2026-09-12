//! Storage key protection. Android: two AES-256-GCM keys generated inside the
//! Android Keystore (never exportable, StrongBox where the phone has it, both
//! refusing to work while the screen is locked) wrap the 32-byte data key. One
//! is plain; the other only works within half a minute of the person confirming
//! who they are, which is what keeps a rooted phone from using the key behind
//! their back. The wrapped blob lives in the app's files and says which key made
//! it. Only Rust talks to this plugin.

#![cfg(mobile)]

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, PluginHandle, TauriPlugin},
    Manager, Runtime,
};

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_encedo_keystore);

pub struct Keystore<R: Runtime>(PluginHandle<R>);

#[derive(Serialize)]
struct BytesArg {
    data: String,
    /// Wrap under the key bound to the person.
    auth: bool,
}

#[derive(Deserialize)]
struct BytesOut {
    data: String,
}

#[derive(Deserialize)]
struct NameOut {
    name: String,
}

#[derive(Deserialize)]
struct BoundOut {
    bound: bool,
}

/// What this phone can protect the key with.
#[derive(Debug, Clone, Deserialize)]
pub struct Protection {
    /// There is a screen lock, so a key can be bound to the person.
    pub credential: bool,
    /// The key sits in a separate secure element, not only in the TEE.
    #[serde(rename = "strongBox")]
    pub strong_box: bool,
    /// How long a confirmation authorises the key for.
    #[serde(rename = "windowSeconds")]
    pub window_seconds: u32,
}

/// The plugin answers with `code: message`; the code decides what the app does
/// next, so it is kept apart from the sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeystoreError {
    pub code: String,
    pub message: String,
}

impl KeystoreError {
    fn parse(raw: String) -> Self {
        match raw.split_once(": ") {
            Some((code, message)) if !code.contains(' ') => Self { code: code.to_string(), message: message.to_string() },
            _ => Self { code: "keystore".into(), message: raw },
        }
    }
}

impl std::fmt::Display for KeystoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl<R: Runtime> Keystore<R> {
    /// Manufacturer and model on Android, the user's device name on iOS.
    pub fn device_name(&self) -> Result<String, String> {
        #[cfg(mobile)]
        {
            let out: NameOut = self.0.run_mobile_plugin("deviceName", ()).map_err(|e| e.to_string())?;
            Ok(out.name)
        }
        #[cfg(not(mobile))]
        {
            Err("not available".into())
        }
    }

    /// What this phone can protect the key with.
    pub fn protection(&self) -> Result<Protection, KeystoreError> {
        #[cfg(mobile)]
        {
            self.0.run_mobile_plugin("protection", ()).map_err(|e| KeystoreError::parse(e.to_string()))
        }
        #[cfg(not(mobile))]
        {
            Err(KeystoreError { code: "keystore".into(), message: "not available on this platform".into() })
        }
    }

    /// `auth`: wrap under the key that only works right after a confirmation.
    pub fn wrap(&self, key: &[u8], auth: bool) -> Result<Vec<u8>, KeystoreError> {
        self.call("wrap", key, auth)
    }

    pub fn unwrap(&self, blob: &[u8]) -> Result<Vec<u8>, KeystoreError> {
        self.call("unwrap", blob, false)
    }

    /// Whether this blob was wrapped under the key bound to the person. Reads the
    /// blob's own header, so it needs no key and cannot fail on a locked phone.
    pub fn bound_to_user(&self, blob: &[u8]) -> Result<bool, KeystoreError> {
        #[cfg(mobile)]
        {
            let out: BoundOut = self
                .0
                .run_mobile_plugin("boundToUser", BytesArg { data: STANDARD.encode(blob), auth: false })
                .map_err(|e| KeystoreError::parse(e.to_string()))?;
            Ok(out.bound)
        }
        #[cfg(not(mobile))]
        {
            let _ = blob;
            Ok(false)
        }
    }

    fn call(&self, cmd: &str, bytes: &[u8], auth: bool) -> Result<Vec<u8>, KeystoreError> {
        #[cfg(mobile)]
        {
            let out: BytesOut = self
                .0
                .run_mobile_plugin(cmd, BytesArg { data: STANDARD.encode(bytes), auth })
                .map_err(|e| KeystoreError::parse(e.to_string()))?;
            STANDARD.decode(out.data).map_err(|e| KeystoreError { code: "keystore".into(), message: e.to_string() })
        }
        #[cfg(not(mobile))]
        {
            let _ = (cmd, bytes, auth);
            Err(KeystoreError { code: "keystore".into(), message: "keystore plugin not available on this platform".into() })
        }
    }
}

pub trait KeystoreExt<R: Runtime> {
    fn keystore(&self) -> &Keystore<R>;
}

impl<R: Runtime, T: Manager<R>> KeystoreExt<R> for T {
    fn keystore(&self) -> &Keystore<R> {
        self.state::<Keystore<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("encedo-keystore")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            let handle = api.register_android_plugin("com.encedo.keystore", "KeystorePlugin")?;
            #[cfg(target_os = "ios")]
            let handle = api.register_ios_plugin(init_plugin_encedo_keystore)?;
            app.manage(Keystore(handle));
            Ok(())
        })
        .build()
}
