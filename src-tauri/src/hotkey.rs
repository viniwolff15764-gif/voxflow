use crate::audio::AudioRecorder;
use crate::config::AppConfig;
use crate::groq;
use crate::paste;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

struct RecordingState {
    is_recording: bool,
    recorder: AudioRecorder,
    /// Live-preview text built from the small chunks while the user talks.
    preview_chunks: Vec<String>,
}

type SharedState = Arc<Mutex<RecordingState>>;
type SharedConfig = Arc<Mutex<AppConfig>>;

pub fn setup_hotkey(app: &AppHandle, config: SharedConfig) -> Result<(), String> {
    let recording_state: SharedState = Arc::new(Mutex::new(RecordingState {
        is_recording: false,
        recorder: AudioRecorder::new()?,
        preview_chunks: Vec::new(),
    }));

    let state_for_handler = Arc::clone(&recording_state);
    let config_for_handler = Arc::clone(&config);
    let app_handle = app.clone();

    app.plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |_app, _shortcut, event| {
                let state = Arc::clone(&state_for_handler);
                let config = Arc::clone(&config_for_handler);
                let handle = app_handle.clone();

                let hold_to_talk = config.lock().unwrap().hold_to_talk;

                match event.state() {
                    ShortcutState::Pressed => {
                        if hold_to_talk {
                            start_recording(state, config, handle);
                        } else {
                            // Toggle mode: first press starts, second press stops.
                            let recording = state.lock().unwrap().is_recording;
                            if recording {
                                stop_and_process(state, config, handle);
                            } else {
                                start_recording(state, config, handle);
                            }
                        }
                    }
                    ShortcutState::Released => {
                        if hold_to_talk {
                            stop_and_process(state, config, handle);
                        }
                        // Toggle mode ignores release.
                    }
                }
            })
            .build(),
    )
    .map_err(|e| format!("Failed to init global shortcut: {}", e))?;

    // Register hotkey from config.
    let hotkey_str = config.lock().unwrap().hotkey.clone();
    app.global_shortcut()
        .register(parse_hotkey(&hotkey_str))
        .map_err(|e| format!("Failed to register hotkey '{}': {}", hotkey_str, e))?;

    Ok(())
}

/// Re-register the global shortcut at runtime (so changing it in Settings
/// takes effect without restarting the app).
pub fn reregister(app: &AppHandle, hotkey: &str) -> Result<(), String> {
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| e.to_string())?;
    app.global_shortcut()
        .register(parse_hotkey(hotkey))
        .map_err(|e| format!("Failed to register hotkey '{}': {}", hotkey, e))
}

fn start_recording(state: SharedState, config: SharedConfig, handle: AppHandle) {
    // Require an API key first.
    {
        let cfg = config.lock().unwrap();
        if cfg.groq_api_key.trim().is_empty() {
            let _ = handle.emit(
                "recording-error",
                "Sem API Key! Clique ⚙ e cole sua Groq API Key.".to_string(),
            );
            return;
        }
    }

    let mut s = state.lock().unwrap();
    if s.is_recording {
        return;
    }
    s.is_recording = true;
    s.preview_chunks.clear();

    if let Err(e) = s.recorder.start() {
        eprintln!("Failed to start recording: {}", e);
        let _ = handle.emit("recording-error", e);
        s.is_recording = false;
        return;
    }
    drop(s);

    let _ = handle.emit("recording-started", ());

    // Background live-preview loop: transcribe small chunks just for on-screen feedback.
    let chunk_state = Arc::clone(&state);
    let chunk_config = Arc::clone(&config);
    let chunk_handle = handle.clone();

    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(1800)).await;

            if !chunk_state.lock().unwrap().is_recording {
                break;
            }

            let wav = chunk_state.lock().unwrap().recorder.take_preview_chunk();
            if let Some(data) = wav {
                let (api_key, language, model) = {
                    let c = chunk_config.lock().unwrap();
                    (c.groq_api_key.clone(), c.language.clone(), c.whisper_model.clone())
                };
                match groq::transcribe(&api_key, data, &language, &model).await {
                    Ok(text) if !text.trim().is_empty() => {
                        let preview = {
                            let mut st = chunk_state.lock().unwrap();
                            st.preview_chunks.push(text.trim().to_string());
                            st.preview_chunks.join(" ")
                        };
                        let _ = chunk_handle.emit("transcription-update", &preview);
                    }
                    Ok(_) => {}
                    Err(e) => eprintln!("Preview transcription error: {}", e),
                }
            }
        }
    });
}

fn stop_and_process(state: SharedState, config: SharedConfig, handle: AppHandle) {
    // Stop the audio stream synchronously on this (handler) thread and grab the
    // full recording, then do the network work in the background.
    let (full_audio, preview_text) = {
        let mut s = state.lock().unwrap();
        if !s.is_recording {
            return;
        }
        s.is_recording = false;
        s.recorder.stop();
        (s.recorder.take_all(), s.preview_chunks.join(" "))
    };

    let cfg = config.lock().unwrap().clone();

    tauri::async_runtime::spawn(async move {
        // Accurate final pass: transcribe the WHOLE recording in one request.
        let mut full_text = preview_text.trim().to_string();
        if let Some(data) = full_audio {
            match groq::transcribe(&cfg.groq_api_key, data, &cfg.language, &cfg.whisper_model).await {
                Ok(text) if !text.trim().is_empty() => full_text = text.trim().to_string(),
                Ok(_) => {}
                Err(e) => eprintln!("Final transcription error: {}", e),
            }
        }

        if full_text.is_empty() {
            let _ = handle.emit("recording-stopped", "");
            return;
        }

        // Command mode: "<prefix> <instruction>" rewrites the selected text.
        let prefix = cfg.command_prefix.to_lowercase();
        let lower = full_text.to_lowercase();

        if cfg.command_mode && !prefix.is_empty() && lower.starts_with(&prefix) {
            let instruction = full_text[prefix.len()..].trim().to_string();
            let _ = handle.emit("command-processing", &instruction);

            match paste::get_selected_text() {
                Ok(selected) => {
                    match groq::chat_command(
                        &cfg.groq_api_key,
                        &cfg.llm_model,
                        &instruction,
                        &selected,
                    )
                    .await
                    {
                        Ok(result) => {
                            let _ = paste::paste_text(&result);
                            let _ = handle.emit("command-complete", &result);
                        }
                        Err(e) => {
                            let _ = handle.emit("recording-error", e);
                        }
                    }
                }
                Err(e) => {
                    let _ = handle.emit("recording-error", e);
                }
            }
        } else {
            // Normal dictation.
            if cfg.auto_paste {
                if let Err(e) = paste::paste_text(&full_text) {
                    eprintln!("Paste error: {}", e);
                }
            }
            let _ = handle.emit("recording-stopped", &full_text);
        }
    });
}

fn parse_hotkey(key: &str) -> Shortcut {
    match key {
        "F7" => Shortcut::new(None, Code::F7),
        "F8" => Shortcut::new(None, Code::F8),
        "F9" => Shortcut::new(None, Code::F9),
        "F10" => Shortcut::new(None, Code::F10),
        "Ctrl+Shift+Space" => Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space),
        "Ctrl+Shift+V" => Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyV),
        "Alt+Space" => Shortcut::new(Some(Modifiers::ALT), Code::Space),
        "Ctrl+Alt+Space" => Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space),
        "Cmd+Shift+Space" => Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Space),
        "Cmd+Shift+V" => Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyV),
        _ => Shortcut::new(None, Code::F9), // fallback
    }
}
