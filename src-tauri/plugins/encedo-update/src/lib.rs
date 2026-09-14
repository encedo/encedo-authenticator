//! Play In-App Updates: whether Play has a newer release for this phone, how
//! urgent the publisher said it is, and the flow that installs it.
//!
//! Android only. Apple offers nothing of the kind, so on iOS this plugin does
//! not exist and the app falls back to its own version check once it is in the
//! App Store.

#![cfg(target_os = "android")]

use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, PluginHandle, TauriPlugin},
    Manager, Runtime,
};

/// What Play answered. `priority` is the number set on the release in the Play
/// Console (0-5); `stale_days` is how long this phone has been behind, or -1
/// when Play does not say.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PlayUpdate {
    #[serde(default)]
    pub available: bool,
    #[serde(default, rename = "inProgress")]
    pub in_progress: bool,
    #[serde(default, rename = "versionCode")]
    pub version_code: i64,
    #[serde(default)]
    pub priority: i32,
    #[serde(default, rename = "staleDays")]
    pub stale_days: i64,
    #[serde(default, rename = "immediateAllowed")]
    pub immediate_allowed: bool,
    /// The build this phone is running, as Play counts it.
    #[serde(default, rename = "currentVersionCode")]
    pub current_version_code: i64,
    /// False for a sideloaded build: Play will not update it in place.
    #[serde(default, rename = "installedFromPlay")]
    pub installed_from_play: bool,
    /// Play could not be asked (no services, no network). Not a failure of the
    /// app, so it comes back as a state rather than an error.
    #[serde(default)]
    pub error: Option<String>,
}

pub struct Updates<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> Updates<R> {
    pub fn check(&self) -> Result<PlayUpdate, String> {
        self.0.run_mobile_plugin("check", ()).map_err(|e| e.to_string())
    }

    /// Play's own full-screen update, the one a person cannot walk away from.
    pub fn start(&self) -> Result<(), String> {
        self.0.run_mobile_plugin::<()>("start", ()).map_err(|e| e.to_string())
    }

    /// The store page, for a build Play cannot update in place.
    pub fn open_store(&self) -> Result<(), String> {
        self.0.run_mobile_plugin::<()>("openStore", ()).map_err(|e| e.to_string())
    }
}

pub trait UpdatesExt<R: Runtime> {
    fn updates(&self) -> &Updates<R>;
}

impl<R: Runtime, T: Manager<R>> UpdatesExt<R> for T {
    fn updates(&self) -> &Updates<R> {
        self.state::<Updates<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("encedo-update")
        .setup(|app, api| {
            let handle = api.register_android_plugin("com.encedo.update", "UpdatePlugin")?;
            app.manage(Updates(handle));
            Ok(())
        })
        .build()
}
