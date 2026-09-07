//! Encedo Authenticator core. Phase 1: an empty shell that only hosts the
//! webview. The protocol, storage and native integrations land in later phases;
//! the webview will only ever receive display data from here.

mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::app_info])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
