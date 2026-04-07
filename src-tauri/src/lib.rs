mod audio;
mod config;
mod groq;
mod hotkey;
mod paste;
mod tray;

use config::AppConfig;
use std::sync::{Arc, Mutex};
use tauri::State;

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
fn open_settings(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    if let Some(window) = app.get_webview_window("settings") {
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    WebviewWindowBuilder::new(&app, "settings", WebviewUrl::App("settings.html".into()))
        .title("VoxFlow — Configurações")
        .inner_size(380.0, 520.0)
        .resizable(false)
        .center()
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn run() {
    let app_config = Arc::new(Mutex::new(config::load_config()));

    let config_for_setup = Arc::clone(&app_config);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
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
        .invoke_handler(tauri::generate_handler![get_config, save_config_cmd, open_settings])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
