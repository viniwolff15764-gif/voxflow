mod audio;
mod config;
mod groq;
mod paste;

use config::AppConfig;
use std::sync::Mutex;
use tauri::State;

struct AppState {
    config: Mutex<AppConfig>,
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

pub fn run() {
    let app_config = config::load_config();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            config: Mutex::new(app_config),
        })
        .invoke_handler(tauri::generate_handler![get_config, save_config_cmd])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
