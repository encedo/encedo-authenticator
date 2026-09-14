//! Tauri commands: thin wrappers over [`crate::core::Core`]. Everything that
//! crosses to the webview is a view type; keys never do.

use serde::Serialize;
use tauri::State;

use crate::core::{AnswerView, Core, ErrorView, ModuleView, PairingPreview, RefreshReport, RequestView, StoreStatus};
use crate::store::{AuditHealth, Family, LogEntry, Settings};
use crate::update::UpdateStatus;

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

/// Ask Play what it has and fold it into what this phone already knew. Answers
/// on every platform; off Android, and for a build Play does not manage, the
/// verdict is whatever the phone remembers.
#[tauri::command]
pub fn update_status(app: tauri::AppHandle, core: State<'_, Core>) -> UpdateStatus {
    let _ = &app;
    #[cfg(target_os = "android")]
    {
        use tauri_plugin_encedo_update::UpdatesExt;
        return match app.updates().check() {
            Ok(p) => {
                let answer = crate::update::PlayAnswer {
                    available: p.available,
                    version_code: p.version_code,
                    priority: p.priority,
                    stale_days: p.stale_days,
                    can_update_in_app: p.installed_from_play && p.immediate_allowed,
                };
                core.update_seen(&answer, p.current_version_code, p.error, false)
            }
            Err(e) => {
                let mut status = core.update_known(0);
                status.note = Some(e);
                status
            }
        };
    }
    #[cfg(not(target_os = "android"))]
    core.update_known(i64::MAX)
}

/// Hand the person to Play: its own full-screen update where that is possible,
/// the store page where it is not.
#[tauri::command]
pub fn update_start(app: tauri::AppHandle, core: State<'_, Core>) -> Res<()> {
    let _ = (&app, &core);
    #[cfg(target_os = "android")]
    {
        use tauri_plugin_encedo_update::UpdatesExt;
        let updates = app.updates();
        return match updates.start() {
            Ok(()) => {
                let _ = core.update_started("Play's own update flow");
                Ok(())
            }
            Err(flow) => match updates.open_store() {
                Ok(()) => {
                    let _ = core.update_started(&format!("the store page ({flow})"));
                    Ok(())
                }
                Err(e) => Err(ErrorView { code: "update".into(), message: e }),
            },
        };
    }
    #[cfg(not(target_os = "android"))]
    Err(ErrorView { code: "update".into(), message: "there is no update flow on this platform".into() })
}

/// Development builds only: pretend Play said something, to see the screens.
#[tauri::command]
pub fn update_simulate(app: tauri::AppHandle, core: State<'_, Core>, level: String) -> Res<UpdateStatus> {
    if !app.package_info().version.to_string().contains("-dev.") {
        return Err(ErrorView { code: "state".into(), message: "only a development build can pretend".into() });
    }
    // The real build number, so the screens read like the real thing rather than
    // "this build 0, needed 1".
    #[cfg(target_os = "android")]
    let current = {
        use tauri_plugin_encedo_update::UpdatesExt;
        app.updates().check().map(|p| p.current_version_code).unwrap_or(0)
    };
    #[cfg(not(target_os = "android"))]
    let current = 0;

    // A pretence starts from nothing: without this, a blocking verdict from the
    // last press would outrank whatever is being asked for now — correct in the
    // field, useless for looking at screens.
    core.update_forget();
    if level != "critical" && level != "recommended" {
        return Ok(core.update_known(i64::MAX));
    }
    let answer = match level.as_str() {
        "critical" => crate::update::PlayAnswer { available: true, version_code: current + 1, priority: 5, stale_days: 2, can_update_in_app: false },
        _ => crate::update::PlayAnswer { available: true, version_code: current + 1, priority: 2, stale_days: 3, can_update_in_app: false },
    };
    Ok(core.update_seen(&answer, current, Some("pretended in a development build".into()), true))
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
