mod audio;
mod config;
mod groq;
mod hotkey;
mod paste;
mod tray;

use config::AppConfig;
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
fn save_config_cmd(state: State<AppState>, new_config: AppConfig) -> Result<(), String> {
    config::save_config(&new_config)?;
    *state.config.lock().unwrap() = new_config;
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
            // Position widget in bottom-right corner
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(monitor) = window.current_monitor() {
                    if let Some(monitor) = monitor {
                        let size = monitor.size();
                        let scale = monitor.scale_factor();
                        let x = (size.width as f64 / scale) - 300.0;
                        let y = (size.height as f64 / scale) - 80.0;
                        let _ = window.set_position(tauri::PhysicalPosition::new(
                            (x * scale) as i32,
                            (y * scale) as i32,
                        ));
                    }
                }
            }

            // Setup hotkey
            if let Err(e) = hotkey::setup_hotkey(&app.handle(), Arc::clone(&config_for_setup)) {
                eprintln!("Hotkey setup failed: {}", e);
            }

            // Setup autostart
            #[cfg(desktop)]
            {
                app.handle()
                    .plugin(tauri_plugin_autostart::init(
                        tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                        None,
                    ))
                    .ok();
            }

            if let Err(e) = tray::setup_tray(&app.handle()) {
                eprintln!("Tray setup failed: {}", e);
            }

            Ok(())
        })
        .manage(AppState {
            config: app_config,
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config_cmd,
            drag_window,
            resize_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
