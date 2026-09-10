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

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_encedo_keystore);

pub struct Keystore<R: Runtime>(PluginHandle<R>);

#[derive(Serialize)]
struct BytesArg {
    data: String,
}

#[derive(Deserialize)]
struct BytesOut {
    data: String,
}

#[derive(Deserialize)]
struct NameOut {
    name: String,
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

    pub fn wrap(&self, key: &[u8]) -> Result<Vec<u8>, String> {
        self.call("wrap", key)
    }

    pub fn unwrap(&self, blob: &[u8]) -> Result<Vec<u8>, String> {
        self.call("unwrap", blob)
    }

    fn call(&self, cmd: &str, bytes: &[u8]) -> Result<Vec<u8>, String> {
        #[cfg(mobile)]
        {
            let out: BytesOut = self
                .0
                .run_mobile_plugin(cmd, BytesArg { data: STANDARD.encode(bytes) })
                .map_err(|e| e.to_string())?;
            STANDARD.decode(out.data).map_err(|e| e.to_string())
        }
        #[cfg(not(mobile))]
        {
            let _ = (cmd, bytes);
            Err("keystore plugin not available on this platform".into())
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
