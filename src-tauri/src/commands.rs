//! Tauri commands: thin wrappers over [`crate::core::Core`]. Everything that
//! crosses to the webview is a view type; keys never do.

use serde::Serialize;
use tauri::State;

use crate::core::{AnswerView, Core, ErrorView, ModuleView, PairingPreview, RefreshReport, RequestView};
use crate::store::{ArchiveEntry, Settings};

type Res<T> = Result<T, ErrorView>;

fn map<T>(r: Result<T, crate::core::CoreError>) -> Res<T> {
    r.map_err(ErrorView::from)
}

#[derive(Serialize)]
pub struct AppInfo {
    pub version: String,
    pub platform: &'static str,
    pub broker: String,
}

#[tauri::command]
pub fn app_info(app: tauri::AppHandle) -> AppInfo {
    // tauri.conf version, so a dev build reports 2.0.0-dev.N rather than the crate's 2.0.0.
    AppInfo { version: app.package_info().version.to_string(), platform: std::env::consts::OS, broker: crate::notify::DEFAULT_BASE.into() }
}

/// Development aid: a line in logcat (RustStdoutStderr) from the webview,
/// which has no console of its own in a release build.
#[tauri::command]
pub fn trace(msg: String) {
    eprintln!("[encedo] {msg}");
}

#[tauri::command]
pub fn settings_get(core: State<'_, Core>) -> Res<Settings> {
    map(core.settings())
}

#[tauri::command]
pub fn settings_set(core: State<'_, Core>, settings: Settings) -> Res<()> {
    map(core.set_settings(settings))
}

#[tauri::command]
pub fn modules_list(core: State<'_, Core>) -> Res<Vec<ModuleView>> {
    map(core.modules())
}

#[tauri::command]
pub fn archive_list(core: State<'_, Core>) -> Res<Vec<ArchiveEntry>> {
    map(core.archive_list())
}

#[tauri::command]
pub async fn pair_scan(core: State<'_, Core>, raw: String) -> Res<PairingPreview> {
    map(core.pair_scan(&raw).await)
}

#[tauri::command]
pub async fn pair_confirm(core: State<'_, Core>, label: String, fid: Option<String>) -> Res<ModuleView> {
    let fid = fid.or(map(core.push_token())?).unwrap_or_else(|| "no-push-token".into());
    map(core.pair_confirm(&label, &fid).await)
}

#[tauri::command]
pub async fn pair_refuse(core: State<'_, Core>, fid: Option<String>) -> Res<()> {
    let fid = fid.or(map(core.push_token())?).unwrap_or_else(|| "no-push-token".into());
    map(core.pair_refuse(&fid).await)
}

#[tauri::command]
pub fn diag_log() -> Vec<String> {
    crate::diag::lines()
}

#[tauri::command]
pub fn diag_clear() {
    crate::diag::clear()
}

/// Remove a module from this phone only, when the broker would not let go.
#[tauri::command]
pub fn module_forget(core: State<'_, Core>, pid: String) -> Res<()> {
    map(core.forget(&pid))
}

#[tauri::command]
pub fn last_refresh(core: State<'_, Core>) -> RefreshReport {
    core.last_refresh()
}

#[tauri::command]
pub async fn requests_refresh(core: State<'_, Core>) -> Res<Vec<RequestView>> {
    map(core.refresh().await)
}

#[tauri::command]
pub async fn request_allow(core: State<'_, Core>, id: String, period_secs: u64, writable: bool) -> Res<AnswerView> {
    map(core.allow(&id, period_secs, writable).await)
}

#[tauri::command]
pub async fn request_deny(core: State<'_, Core>, id: String) -> Res<AnswerView> {
    map(core.deny(&id).await)
}

#[tauri::command]
pub async fn module_unpair(core: State<'_, Core>, pid: String) -> Res<()> {
    map(core.unpair(&pid).await)
}

#[tauri::command]
pub fn push_payload(core: State<'_, Core>, encedo: String) -> Res<bool> {
    map(core.push_payload(&encedo))
}

#[tauri::command]
pub async fn push_token_changed(core: State<'_, Core>, token: String) -> Res<()> {
    map(core.push_token_changed(&token).await)
}
