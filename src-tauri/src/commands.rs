use crate::desktop::{notify, AppState};
use chrono::Utc;
use serde::Serialize;
use std::{io::Read, sync::Arc};
use tauri::{Manager, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;
use todo_core::*;

#[derive(Debug, Serialize)]
pub struct CommandError {
    code: &'static str,
    message: String,
}
impl From<AppError> for CommandError {
    fn from(e: AppError) -> Self {
        Self {
            code: match &e {
                AppError::Validation(_) => "validation",
                AppError::NotFound => "not_found",
                AppError::Database(_) => "database",
                AppError::Io(_) => "io",
                AppError::Json(_) => "invalid_data",
            },
            message: e.to_string(),
        }
    }
}
fn platform_error(message: impl ToString) -> CommandError {
    CommandError {
        code: "platform",
        message: message.to_string(),
    }
}
#[tauri::command]
pub fn open_card(app: tauri::AppHandle) -> Result<(), CommandError> {
    let card = app
        .get_webview_window("card")
        .ok_or_else(|| platform_error("小卡片窗口不可用"))?;
    card.show().map_err(platform_error)?;
    card.unminimize().map_err(platform_error)?;
    card.set_focus().map_err(platform_error)
}
#[tauri::command]
pub fn hide_card(app: tauri::AppHandle) -> Result<(), CommandError> {
    app.get_webview_window("card")
        .ok_or_else(|| platform_error("小卡片窗口不可用"))?
        .hide()
        .map_err(platform_error)
}
#[tauri::command]
pub fn open_main(app: tauri::AppHandle) {
    crate::desktop::show(&app);
}
#[tauri::command]
pub async fn edit_in_main(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), CommandError> {
    use tauri::Emitter;
    let task = with_store(state.inner().clone(), move |store| store.get_task(&id)).await?;
    crate::desktop::show(&app);
    app.emit_to("main", "edit_task", task)
        .map_err(platform_error)
}
#[tauri::command]
pub fn card_pin(state: State<'_, Arc<AppState>>) -> Result<bool, CommandError> {
    Ok(*state
        .card_pinned
        .lock()
        .map_err(|_| platform_error("小卡片状态不可用"))?)
}
#[tauri::command]
pub fn set_card_pin(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    value: bool,
) -> Result<bool, CommandError> {
    use std::io::Write;
    let mut previous = state
        .card_pinned
        .lock()
        .map_err(|_| platform_error("小卡片状态不可用"))?;
    let card = app
        .get_webview_window("card")
        .ok_or_else(|| platform_error("小卡片窗口不可用"))?;
    card.set_always_on_top(value).map_err(platform_error)?;
    if let Err(error) = atomic_write_with(&state.data_directory.join("card.json"), |file| {
        file.write_all(if value { b"true" } else { b"false" })
    }) {
        let _ = card.set_always_on_top(*previous);
        return Err(error.into());
    }
    *previous = value;
    Ok(value)
}
async fn with_store<T: Send + 'static>(
    state: Arc<AppState>,
    action: impl FnOnce(&mut Store) -> AppResult<T> + Send + 'static,
) -> Result<T, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut store = state
            .store
            .lock()
            .map_err(|_| invalid("数据库服务不可用，请重新启动程序"))?;
        action(&mut store)
    })
    .await
    .map_err(platform_error)?
    .map_err(Into::into)
}
#[tauri::command]
pub async fn snapshot(
    state: State<'_, Arc<AppState>>,
    query: Query,
) -> Result<Snapshot, CommandError> {
    let mut result = with_store(state.inner().clone(), move |store| {
        store.snapshot(&query, Utc::now())
    })
    .await?;
    result.scheduler_error = state
        .scheduler_error
        .lock()
        .map_err(|_| platform_error("后台提醒状态不可用"))?
        .clone();
    Ok(result)
}
#[tauri::command]
pub async fn save_task(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    id: Option<String>,
    draft: TaskDraft,
    scope: String,
) -> Result<Task, CommandError> {
    let task = with_store(state.inner().clone(), move |store| {
        store.save_task(id.as_deref(), draft, &scope, Utc::now())
    })
    .await?;
    state.changed(&app);
    Ok(task)
}
#[tauri::command]
pub async fn set_completed(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    id: String,
    value: bool,
) -> Result<(), CommandError> {
    with_store(state.inner().clone(), move |store| {
        store.set_completed(&id, value, Utc::now()).map(|_| ())
    })
    .await?;
    state.changed(&app);
    Ok(())
}
#[tauri::command]
pub async fn set_deleted(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    id: String,
    value: bool,
) -> Result<(), CommandError> {
    with_store(state.inner().clone(), move |store| {
        store.set_deleted(&id, value, Utc::now())
    })
    .await?;
    state.changed(&app);
    Ok(())
}
#[tauri::command]
pub async fn mark_inbox_read(state: State<'_, Arc<AppState>>) -> Result<(), CommandError> {
    with_store(state.inner().clone(), |store| store.mark_inbox_read()).await
}
#[tauri::command]
pub async fn update_settings(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    settings: Settings,
) -> Result<(), CommandError> {
    settings.validate()?;
    let app_for_worker = app.clone();
    with_store(state.inner().clone(), move |store| {
        let old = store.settings()?;
        let launcher = app_for_worker.autolaunch();
        let previous = launcher.is_enabled().map_err(|e| invalid(&e.to_string()))?;
        if settings.autostart != previous {
            if settings.autostart {
                launcher.enable()
            } else {
                launcher.disable()
            }
            .map_err(|e| invalid(&format!("修改登录启动失败：{e}")))?;
        }
        if let Err(error) = store.update_settings(settings, Utc::now()) {
            if previous {
                let _ = launcher.enable();
            } else {
                let _ = launcher.disable();
            }
            let _ = old;
            return Err(error);
        }
        Ok(())
    })
    .await?;
    state.changed(&app);
    Ok(())
}
#[tauri::command]
pub async fn test_notification(app: tauri::AppHandle) -> Result<(), CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        notify(
            &app,
            &NotificationBatch {
                id: "test".into(),
                title: "拾序提醒已准备好".into(),
                body: "这是一次测试通知。关闭窗口后，拾序仍会在托盘中提醒你。".into(),
                task_ids: vec![],
                created_at: Utc::now(),
                submitted: false,
                error: None,
                retry_exhausted: false,
                read: true,
            },
        )
    })
    .await
    .map_err(platform_error)?
    .map_err(Into::into)
}
#[tauri::command]
pub async fn export_backup(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    kind: String,
) -> Result<Option<String>, CommandError> {
    if !["json", "sqlite"].contains(&kind.as_str()) {
        return Err(platform_error("备份格式无效"));
    }
    let handle = app.clone();
    let extension = if kind == "json" { "json" } else { "db" };
    let file = tauri::async_runtime::spawn_blocking(move || {
        handle
            .dialog()
            .file()
            .set_title("导出拾序备份")
            .add_filter("拾序备份", &[extension])
            .set_file_name(format!(
                "LocalTodo-{}.{}",
                Utc::now().format("%Y%m%d-%H%M%S"),
                extension
            ))
            .blocking_save_file()
    })
    .await
    .map_err(platform_error)?;
    let Some(file) = file else { return Ok(None) };
    let path = file.into_path().map_err(platform_error)?;
    let result = path.display().to_string();
    let protected = state.data_directory.clone();
    with_store(state.inner().clone(), move |store| {
        if path.starts_with(&protected) {
            return Err(invalid(
                "请将导出文件保存到应用数据目录以外，避免覆盖当前数据",
            ));
        }
        if kind == "sqlite" {
            store.backup_sqlite(&path)
        } else {
            use std::io::Write;
            let json = store.export_json()?;
            atomic_write_with(&path, |file| file.write_all(json.as_bytes()))
        }
    })
    .await?;
    Ok(Some(result))
}
#[tauri::command]
pub async fn restore_backup(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<Option<usize>, CommandError> {
    let handle = app.clone();
    let file = tauri::async_runtime::spawn_blocking(move || {
        handle
            .dialog()
            .file()
            .set_title("恢复拾序JSON备份")
            .add_filter("拾序JSON备份", &["json"])
            .blocking_pick_file()
    })
    .await
    .map_err(platform_error)?;
    let Some(file) = file else { return Ok(None) };
    let path = file.into_path().map_err(platform_error)?;
    let count = with_store(state.inner().clone(), move |store| {
        let mut json = String::new();
        std::fs::File::open(path)?
            .take(100 * 1024 * 1024 + 1)
            .read_to_string(&mut json)?;
        // Restoring task data must not silently enable startup from an imported preference.
        let mut value: serde_json::Value = serde_json::from_str(&json)?;
        if let Some(settings) = value.get_mut("settings").and_then(|v| v.as_object_mut()) {
            settings.insert(
                "autostart".into(),
                serde_json::Value::Bool(store.settings()?.autostart),
            );
        }
        store.restore_json(&serde_json::to_string(&value)?, Utc::now())
    })
    .await?;
    state.changed(&app);
    Ok(Some(count))
}
