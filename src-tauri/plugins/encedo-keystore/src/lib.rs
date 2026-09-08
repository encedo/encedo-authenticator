//! Storage key protection. Android: an AES-256-GCM key generated inside the
//! Android Keystore (never exportable) wraps the 32-byte data key; the wrapped
//! blob lives in the app's files. Only Rust talks to this plugin.

#![cfg(mobile)]

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, PluginHandle, TauriPlugin},
    Manager, Runtime,
};

pub struct Keystore<R: Runtime>(PluginHandle<R>);

#[derive(Serialize)]
struct BytesArg {
    data: String,
}

#[derive(Deserialize)]
struct BytesOut {
    data: String,
}

impl<R: Runtime> Keystore<R> {
    pub fn wrap(&self, key: &[u8]) -> Result<Vec<u8>, String> {
        self.call("wrap", key)
    }

    pub fn unwrap(&self, blob: &[u8]) -> Result<Vec<u8>, String> {
        self.call("unwrap", blob)
    }

    fn call(&self, cmd: &str, bytes: &[u8]) -> Result<Vec<u8>, String> {
        #[cfg(target_os = "android")]
        {
            let out: BytesOut = self
                .0
                .run_mobile_plugin(cmd, BytesArg { data: STANDARD.encode(bytes) })
                .map_err(|e| e.to_string())?;
            STANDARD.decode(out.data).map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = (cmd, bytes);
            Err("keystore plugin not available on this platform yet".into())
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
            {
                let handle = api.register_android_plugin("com.encedo.keystore", "KeystorePlugin")?;
                app.manage(Keystore(handle));
            }
            #[cfg(not(target_os = "android"))]
            {
                let _ = (app, api);
            }
            Ok(())
        })
        .build()
}
