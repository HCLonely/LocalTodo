use chrono::Utc;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tauri_plugin_notification::NotificationExt;
use todo_core::{invalid, AppResult, NotificationBatch, Store};

pub struct AppState {
    pub store: Mutex<Store>,
    pub wake: (Mutex<bool>, Condvar),
    pub scheduler_error: Mutex<Option<String>>,
}
impl AppState {
    pub fn changed(&self, app: &tauri::AppHandle) {
        let _ = app.emit("tasks_changed", ());
        if let Ok(mut flag) = self.wake.0.lock() {
            *flag = true;
            self.wake.1.notify_one();
        }
    }
}
pub fn notify(app: &tauri::AppHandle, b: &NotificationBatch) -> AppResult<()> {
    app.notification()
        .builder()
        .title(&b.title)
        .body(&b.body)
        .show()
        .map_err(|e| invalid(&format!("系统通知提交失败：{e}")))
}
fn show(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
pub fn run() {
    let context = tauri::generate_context!();
    #[cfg(debug_assertions)]
    let context = {
        let mut context = context;
        if std::env::var_os("LOCALTODO_TEST_DATA_DIR").is_some() {
            context.config_mut().app.windows[0].additional_browser_args = Some("--remote-debugging-port=9222 --disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection".into());
            context.config_mut().app.windows[0].data_directory =
                std::env::var_os("LOCALTODO_TEST_DATA_DIR")
                    .map(std::path::PathBuf::from)
                    .map(|p| p.join("webview"));
        }
        context
    };
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show(app)))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--background"]),
        ))
        .invoke_handler(tauri::generate_handler![
            crate::commands::snapshot,
            crate::commands::save_task,
            crate::commands::set_completed,
            crate::commands::set_deleted,
            crate::commands::mark_inbox_read,
            crate::commands::update_settings,
            crate::commands::test_notification,
            crate::commands::export_backup,
            crate::commands::restore_backup
        ])
        .setup(|app| {
            let directory = app.path().app_data_dir()?;
            #[cfg(debug_assertions)]
            let directory = std::env::var_os("LOCALTODO_TEST_DATA_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or(directory);
            std::fs::create_dir_all(&directory)?;
            let state = Arc::new(AppState {
                store: Mutex::new(Store::open(&directory.join("todo.db"))?),
                wake: (Mutex::new(false), Condvar::new()),
                scheduler_error: Mutex::new(None),
            });
            app.manage(state.clone());
            let open = MenuItem::with_id(app, "open", "打开拾序", true, None::<&str>)?;
            let new = MenuItem::with_id(app, "new", "新建任务", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出（暂停提醒）", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &new, &quit])?;
            let mut tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("拾序 · 本地任务")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show(app),
                    "new" => {
                        show(app);
                        let _ = app.emit("create_task", ());
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        show(tray.app_handle());
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;
            if std::env::args().any(|a| a == "--background") {
                if let Some(window) = app.get_webview_window("main") {
                    window.hide()?;
                }
            }
            let handle = app.handle().clone();
            std::thread::Builder::new()
                .name("todo-reminders".into())
                .spawn(move || loop {
                    let mut delay = 30_000;
                    match state.store.lock() {
                        Ok(mut store) => {
                            let now = Utc::now();
                            let result = (|| -> AppResult<()> {
                                let report =
                                    store.background_tick(now, |batch| notify(&handle, batch))?;
                                delay = report.next_wake_millis;
                                Ok(())
                            })();
                            if let Err(error) = result {
                                eprintln!("Background Todo error: {error}");
                                if let Ok(mut health) = state.scheduler_error.lock() {
                                    *health = Some(error.to_string());
                                }
                                let _ = handle.emit("background_error", error.to_string());
                            } else if let Ok(mut health) = state.scheduler_error.lock() {
                                if health.take().is_some() {
                                    let _ = handle.emit("background_recovered", ());
                                }
                            }
                        }
                        Err(_) => break,
                    }
                    let _ = handle.emit("tasks_changed", ());
                    let _ = handle.emit("reminders_changed", ());
                    if let Ok(mut flag) = state.wake.0.lock() {
                        if !*flag {
                            match state
                                .wake
                                .1
                                .wait_timeout(flag, Duration::from_millis(delay))
                            {
                                Ok((next, _)) => flag = next,
                                Err(_) => break,
                            }
                        }
                        *flag = false;
                    }
                })?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(context);
    match app {
        Ok(app) => app.run(|_, _| {}),
        Err(error) => {
            // GUI builds have no console; surface startup/migration failures with a native dialog.
            eprintln!("Unable to start LocalTodo: {error}");
            #[cfg(windows)]
            {
                let _=std::process::Command::new("powershell.exe").args(["-NoProfile","-NonInteractive","-Command","Add-Type -AssemblyName PresentationFramework; [System.Windows.MessageBox]::Show($env:LOCALTODO_STARTUP_ERROR, '拾序启动失败')"]).env("LOCALTODO_STARTUP_ERROR",format!("无法启动，请检查应用数据目录的权限或备份后恢复数据。\n{error}")).creation_flags_hidden().status();
            }
        }
    }
}
#[cfg(windows)]
trait HiddenCommand {
    fn creation_flags_hidden(&mut self) -> &mut Self;
}
#[cfg(windows)]
impl HiddenCommand for std::process::Command {
    fn creation_flags_hidden(&mut self) -> &mut Self {
        use std::os::windows::process::CommandExt;
        self.creation_flags(0x08000000)
    }
}
