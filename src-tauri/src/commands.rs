//! Tauri commands: thin wrappers over [`crate::core::Core`]. Everything that
//! crosses to the webview is a view type; keys never do.

use serde::Serialize;
use tauri::State;

use crate::core::{AnswerView, Core, ErrorView, ModuleView, PairingPreview, RefreshReport, RequestView, StoreStatus};
use crate::store::{AuditHealth, Family, LogEntry, Settings};

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

/// Is the storage open, and what protects its key. The only command that works
/// before the storage is open, because the webview needs it to decide what to show.
#[tauri::command]
pub fn store_status(core: State<'_, Core>) -> StoreStatus {
    core.status()
}

/// Open the storage. Call it after the person has confirmed who they are: a key
/// bound to them is refused until then (`auth_required`), and a key that is gone
/// says so (`key_lost`).
#[tauri::command]
pub fn store_open(app: tauri::AppHandle, core: State<'_, Core>) -> Res<StoreStatus> {
    let first = !core.is_open();
    map(core.open_storage())?;
    if first {
        let _ = core.opened(&app.package_info().version.to_string());
    }
    Ok(core.status())
}

/// Throw away a store whose key is gone and start clean. Everything paired
/// before this is lost; the journal says so as its first line.
#[tauri::command]
pub fn store_reset(app: tauri::AppHandle, core: State<'_, Core>) -> Res<StoreStatus> {
    map(core.reset())?;
    let _ = core.opened(&app.package_info().version.to_string());
    Ok(core.status())
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

/// The journal, newest first. `family` is one of the names [`Family`] serialises
/// to; without it, everything except the running commentary.
#[tauri::command]
pub fn log_list(core: State<'_, Core>, family: Option<String>) -> Res<Vec<LogEntry>> {
    let family = match family.as_deref() {
        None | Some("") | Some("all") => None,
        Some(name) => Some(serde_json::from_value::<Family>(serde_json::Value::String(name.into())).map_err(|_| ErrorView { code: "state".into(), message: format!("no such part of the log: {name}") })?),
    };
    map(core.journal(family))
}

/// Whether the sealed chain of answers and pairings still holds.
#[tauri::command]
pub fn log_verify(core: State<'_, Core>) -> Res<AuditHealth> {
    map(core.audit_health())
}

/// Drop the running commentary. Sealed entries stay.
#[tauri::command]
pub fn log_clear_trace(core: State<'_, Core>) -> Res<()> {
    map(core.clear_trace())
}

/// A push arrived or was tapped: only the webview sees the notification text.
#[tauri::command]
pub fn log_push(core: State<'_, Core>, title: Option<String>, body: Option<String>, data: Option<String>, tapped: bool) -> Res<()> {
    map(core.log_push(title, body, data, tapped))
}

/// Lifecycle and lock events, which only the webview can see.
#[tauri::command]
pub fn log_app(core: State<'_, Core>, kind: String, title: String, summary: String) -> Res<()> {
    map(core.log_app(&kind, &title, &summary))
}

/// Going to the background: write out what the commentary has collected.
#[tauri::command]
pub fn log_flush(core: State<'_, Core>) -> Res<()> {
    map(core.flush())
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
