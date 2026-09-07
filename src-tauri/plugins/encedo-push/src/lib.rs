//! Encedo push plugin. On Android it registers the Kotlin `PushPlugin`, which
//! owns the Firebase Messaging service; every command and event is handled
//! there. iOS gets its own implementation in a later phase.

#![cfg(mobile)]

use tauri::{
    plugin::{Builder, PluginHandle, TauriPlugin},
    Manager, Runtime,
};

/// Handle to the native side; kept so Rust can call into it later
/// (for example to hand the token to the protocol module).
pub struct Push<R: Runtime>(#[allow(dead_code)] PluginHandle<R>);

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("encedo-push")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                let handle = api.register_android_plugin("com.encedo.push", "PushPlugin")?;
                app.manage(Push(handle));
            }
            #[cfg(not(target_os = "android"))]
            {
                let _ = (app, api);
            }
            Ok(())
        })
        .build()
}
