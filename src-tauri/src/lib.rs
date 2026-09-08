//! Encedo Authenticator core. Phase 1: the shell that hosts the webview plus
//! the two native integrations that carry the most risk, the QR scanner and
//! FCM registration. The protocol and storage land in later phases; the webview
//! only ever receives display data and a push token from here.

mod commands;
pub mod notify;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(mobile)]
    let builder = builder
        .plugin(tauri_plugin_barcode_scanner::init())
        .plugin(tauri_plugin_encedo_push::init());
    builder
        .invoke_handler(tauri::generate_handler![commands::app_info])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
