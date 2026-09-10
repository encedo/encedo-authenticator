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
#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_encedo_push);

pub struct Push<R: Runtime>(#[allow(dead_code)] PluginHandle<R>);

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("encedo-push")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            let handle = api.register_android_plugin("com.encedo.push", "PushPlugin")?;
            #[cfg(target_os = "ios")]
            let handle = api.register_ios_plugin(init_plugin_encedo_push)?;
            app.manage(Push(handle));
            Ok(())
        })
        .build()
}
