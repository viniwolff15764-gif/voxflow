mod audio;
mod config;
mod groq;
mod hotkey;
mod paste;
mod tray;

use config::AppConfig;
use std::process;
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};

struct AppState {
    config: Arc<Mutex<AppConfig>>,
}

#[tauri::command]
fn get_config(state: State<AppState>) -> AppConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn save_config_cmd(
    app: tauri::AppHandle,
    state: State<AppState>,
    new_config: AppConfig,
) -> Result<(), String> {
    config::save_config(&new_config)?;
    let hotkey = new_config.hotkey.clone();
    *state.config.lock().unwrap() = new_config;
    // Apply the new hotkey live — no restart needed.
    if let Err(e) = hotkey::reregister(&app, &hotkey) {
        eprintln!("Hotkey re-register failed: {}", e);
        return Err(format!("Atalho inválido: {}", e));
    }
    Ok(())
}

#[tauri::command]
fn drag_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.start_dragging().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn exit_app() {
    process::exit(0);
}

#[tauri::command]
fn resize_window(app: tauri::AppHandle, width: f64, height: f64) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window
            .set_size(tauri::LogicalSize::new(width, height))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn run() {
    let app_config = Arc::new(Mutex::new(config::load_config()));
    let config_for_setup = Arc::clone(&app_config);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
            // On macOS behave like a floating utility: no Dock icon, lives in the menu bar / tray.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Position the widget at the bottom-center of the screen (Wispr-style).
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(Some(monitor)) = window.current_monitor() {
                    let size = monitor.size();
                    let scale = monitor.scale_factor();
                    let win_w = 420.0;
                    let screen_w = size.width as f64 / scale;
                    let screen_h = size.height as f64 / scale;
                    let x = (screen_w - win_w) / 2.0;
                    let y = screen_h - 130.0;
                    let _ = window.set_position(tauri::PhysicalPosition::new(
                        (x * scale) as i32,
                        (y * scale) as i32,
                    ));
                }
            }

            // Global hotkey.
            if let Err(e) = hotkey::setup_hotkey(&app.handle(), Arc::clone(&config_for_setup)) {
                eprintln!("Hotkey setup failed: {}", e);
            }

            // Autostart.
            #[cfg(desktop)]
            {
                app.handle()
                    .plugin(tauri_plugin_autostart::init(
                        tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                        None,
                    ))
                    .ok();
            }

            // Tray icon.
            if let Err(e) = tray::setup_tray(&app.handle()) {
                eprintln!("Tray setup failed: {}", e);
            }

            Ok(())
        })
        .manage(AppState { config: app_config })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config_cmd,
            drag_window,
            resize_window,
            exit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
