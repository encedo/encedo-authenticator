//! Encedo Authenticator core: the store, the broker client and the protocol
//! live here; the webview only ever receives display data and a push token.

mod commands;
pub mod core;
pub mod diag;
pub mod legacy;
pub mod notify;
pub mod scope;
pub mod store;
pub mod update;

use std::path::PathBuf;

use tauri::Manager;

use crate::core::Core;
use crate::notify::NotifyClient;
use crate::store::SecretStore;

/// Where the data key comes from on this platform.
fn secret_store(app: &tauri::AppHandle, data_dir: &std::path::Path) -> Box<dyn SecretStore> {
    #[cfg(mobile)]
    {
        Box::new(mobile_secret::KeystoreSecret { app: app.clone(), path: data_dir.join("store.key") })
    }
    #[cfg(not(mobile))]
    {
        let _ = app;
        Box::new(store::DevFileSecret { path: data_dir.join("dev-store.key") })
    }
}

#[cfg(mobile)]
mod mobile_secret {
    use std::fs;
    use std::path::PathBuf;

    use rand_core::{OsRng, RngCore};
    use tauri_plugin_encedo_keystore::{KeystoreError, KeystoreExt};

    use crate::store::{DataKey, Protection, SecretStore, StoreError};

    /// The data key, wrapped by the platform's key store (Android Keystore, iOS
    /// Keychain), in `store.key`. A fresh install is wrapped plain, because the
    /// store has to exist before anybody can be asked anything; the first run
    /// binds it to the person as soon as the lock is confirmed.
    pub struct KeystoreSecret {
        pub app: tauri::AppHandle,
        pub path: PathBuf,
    }

    /// The plugin's codes decide what the app does next.
    fn to_store_error(e: KeystoreError) -> StoreError {
        match e.code.as_str() {
            "auth_required" => StoreError::AuthRequired,
            "key_lost" => StoreError::KeyLost,
            "no_credential" => StoreError::NoCredential,
            _ => StoreError::Key(e.message),
        }
    }

    impl KeystoreSecret {
        fn read_key(&self) -> Result<Option<[u8; 32]>, StoreError> {
            let Ok(blob) = fs::read(&self.path) else { return Ok(None) };
            let key = self.app.keystore().unwrap(&blob).map_err(to_store_error)?;
            let key: [u8; 32] = key.try_into().map_err(|_| StoreError::Corrupt)?;
            Ok(Some(key))
        }

        fn write_key(&self, key: &[u8; 32], bound: bool) -> Result<(), StoreError> {
            let blob = self.app.keystore().wrap(key, bound).map_err(to_store_error)?;
            if let Some(dir) = self.path.parent() {
                fs::create_dir_all(dir)?;
            }
            fs::write(&self.path, blob)?;
            Ok(())
        }
    }

    impl SecretStore for KeystoreSecret {
        fn data_key(&self) -> Result<DataKey, StoreError> {
            if let Some(key) = self.read_key()? {
                return Ok(DataKey(key));
            }
            let mut key = [0u8; 32];
            OsRng.fill_bytes(&mut key);
            self.write_key(&key, false)?;
            Ok(DataKey(key))
        }

        fn protection(&self) -> Protection {
            let ks = self.app.keystore();
            let bound = fs::read(&self.path).ok().and_then(|blob| ks.bound_to_user(&blob).ok()).unwrap_or(false);
            match ks.protection() {
                Ok(p) => Protection { bound_to_user: bound, credential: p.credential, strong_box: p.strong_box, window_seconds: p.window_seconds },
                Err(_) => Protection { bound_to_user: bound, ..Protection::default() },
            }
        }

        /// Read the key under its current protection and write it back under the
        /// other one. Both halves may need a fresh confirmation, which is why the
        /// screens ask before calling this.
        fn bind_to_user(&self, bound: bool) -> Result<(), StoreError> {
            if self.protection().bound_to_user == bound {
                return Ok(());
            }
            if bound && !self.protection().credential {
                return Err(StoreError::NoCredential);
            }
            let key = self.read_key()?.ok_or(StoreError::KeyLost)?;
            self.write_key(&key, bound)
        }

        fn forget(&self) -> Result<(), StoreError> {
            match fs::remove_file(&self.path) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e.into()),
            }
        }
    }
}

fn device_name(app: &tauri::AppHandle) -> String {
    #[cfg(mobile)]
    {
        use tauri_plugin_encedo_keystore::KeystoreExt;
        // v1 sent `device.model + " (" + device.platform + ")"`; keep the shape.
        let fallback = if cfg!(target_os = "ios") { "iPhone" } else { "Android phone" };
        let platform = if cfg!(target_os = "ios") { "iOS" } else { "Android" };
        let model = app.keystore().device_name().unwrap_or_else(|_| fallback.to_string());
        format!("{model} ({platform})")
    }
    #[cfg(not(mobile))]
    {
        let _ = app;
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
        .plugin(tauri_plugin_encedo_keystore::init())
        .plugin(tauri_plugin_biometric::init());
    #[cfg(target_os = "android")]
    let builder = builder.plugin(tauri_plugin_encedo_update::init());
    builder
        .setup(|app| {
            let data_dir: PathBuf = app.path().app_data_dir().expect("app data dir");
            let secrets = secret_store(app.handle(), &data_dir);
            let client = NotifyClient::new(std::env::var("ENCEDO_BROKER").as_deref().unwrap_or(notify::DEFAULT_BASE));
            let name = device_name(app.handle());
            let core = Core::new(secrets, data_dir.join("store.bin"), client, name);
            // A key bound to the person cannot be used before the Locked screen has
            // asked, so a refusal here is expected: the webview opens the storage
            // once it has a confirmation. Anything else is reported as it happens.
            match core.open_storage() {
                Ok(()) => {
                    let leftovers = legacy::sweep_v1(&data_dir);
                    if !leftovers.is_empty() {
                        let _ = core.note_v1_removed(&leftovers);
                    }
                    let _ = core.note_launched(&app.package_info().version.to_string());
                    // One write per launch, so the first line of the session survives
                    // even if the app is killed rather than sent to the background.
                    let _ = core.flush();
                }
                Err(e) => eprintln!("[encedo] storage not open at launch: {e}"),
            }
            app.manage(core);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::trace,
            commands::update_status,
            commands::update_start,
            commands::update_simulate,
            commands::store_status,
            commands::store_open,
            commands::store_reset,
            commands::settings_get,
            commands::settings_set,
            commands::modules_list,
            commands::log_list,
            commands::log_verify,
            commands::log_clear_trace,
            commands::log_push,
            commands::log_app,
            commands::log_flush,
            commands::pair_scan,
            commands::pair_confirm,
            commands::pair_refuse,
            commands::requests_refresh,
            commands::last_refresh,
            commands::diag_log,
            commands::diag_clear,
            commands::module_forget,
            commands::request_allow,
            commands::request_deny,
            commands::module_unpair,
            commands::push_payload,
            commands::push_token_changed,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
