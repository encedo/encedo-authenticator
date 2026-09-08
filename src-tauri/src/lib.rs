//! Encedo Authenticator core: the store, the broker client and the protocol
//! live here; the webview only ever receives display data and a push token.

mod commands;
pub mod core;
pub mod notify;
pub mod scope;
pub mod store;

use std::path::PathBuf;

use tauri::Manager;

use crate::core::Core;
use crate::notify::NotifyClient;
use crate::store::{SecretStore, Store};

/// Where the data key comes from on this platform.
fn secret_store(app: &tauri::AppHandle, data_dir: &std::path::Path) -> Box<dyn SecretStore> {
    #[cfg(target_os = "android")]
    {
        Box::new(android::KeystoreSecret { app: app.clone(), path: data_dir.join("store.key") })
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Box::new(store::DevFileSecret { path: data_dir.join("dev-store.key") })
    }
}

#[cfg(target_os = "android")]
mod android {
    use std::fs;
    use std::path::PathBuf;

    use rand_core::{OsRng, RngCore};
    use tauri_plugin_encedo_keystore::KeystoreExt;

    use crate::store::{DataKey, SecretStore, StoreError};

    /// The data key, wrapped by the Android Keystore, in `store.key`.
    pub struct KeystoreSecret {
        pub app: tauri::AppHandle,
        pub path: PathBuf,
    }

    impl SecretStore for KeystoreSecret {
        fn data_key(&self) -> Result<DataKey, StoreError> {
            let ks = self.app.keystore();
            if let Ok(blob) = fs::read(&self.path) {
                let key = ks.unwrap(&blob).map_err(StoreError::Key)?;
                let key: [u8; 32] = key.try_into().map_err(|_| StoreError::Corrupt)?;
                return Ok(DataKey(key));
            }
            let mut key = [0u8; 32];
            OsRng.fill_bytes(&mut key);
            let blob = ks.wrap(&key).map_err(StoreError::Key)?;
            if let Some(dir) = self.path.parent() {
                fs::create_dir_all(dir)?;
            }
            fs::write(&self.path, blob)?;
            Ok(DataKey(key))
        }
    }
}

fn device_name() -> String {
    #[cfg(target_os = "android")]
    {
        "Android phone".to_string()
    }
    #[cfg(target_os = "ios")]
    {
        "iPhone".to_string()
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        format!("{} desktop", std::env::consts::OS)
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(mobile)]
    let builder = builder
        .plugin(tauri_plugin_barcode_scanner::init())
        .plugin(tauri_plugin_encedo_push::init())
        .plugin(tauri_plugin_encedo_keystore::init());
    builder
        .setup(|app| {
            let data_dir: PathBuf = app.path().app_data_dir().expect("app data dir");
            let secrets = secret_store(app.handle(), &data_dir);
            let store = Store::open(data_dir.join("store.bin"), secrets.as_ref())?;
            let client = NotifyClient::new(std::env::var("ENCEDO_BROKER").as_deref().unwrap_or(notify::DEFAULT_BASE));
            app.manage(Core::new(store, client, device_name()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::settings_get,
            commands::settings_set,
            commands::modules_list,
            commands::archive_list,
            commands::pair_scan,
            commands::pair_confirm,
            commands::pair_refuse,
            commands::requests_refresh,
            commands::request_allow,
            commands::request_deny,
            commands::module_unpair,
            commands::push_payload,
            commands::push_token_changed,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
