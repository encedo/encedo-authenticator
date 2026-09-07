use serde::Serialize;

/// Static application metadata shown on the About screen.
#[derive(Serialize)]
pub struct AppInfo {
    pub version: &'static str,
    pub platform: &'static str,
}

#[tauri::command]
pub fn app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
    }
}
