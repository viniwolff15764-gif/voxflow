use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub groq_api_key: String,
    pub hotkey: String,
    pub language: String,
    pub auto_paste: bool,
    pub command_mode: bool,
    pub command_prefix: String,
    pub llm_model: String,
    pub opacity: f64,
    pub autostart: bool,
    pub window_x: f64,
    pub window_y: f64,
    pub window_width: f64,
    pub window_height: f64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            groq_api_key: String::new(),
            hotkey: "CapsLock".to_string(),
            language: "pt".to_string(),
            auto_paste: true,
            command_mode: false,
            command_prefix: "comando".to_string(),
            llm_model: "llama-3.3-70b-versatile".to_string(),
            opacity: 0.8,
            autostart: true,
            window_x: 100.0,
            window_y: 100.0,
            window_width: 350.0,
            window_height: 200.0,
        }
    }
}

fn config_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("voxflow");
    fs::create_dir_all(&config_dir).ok();
    config_dir.join("config.json")
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => {
            let config = AppConfig::default();
            save_config(&config).ok();
            config
        }
    }
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}
