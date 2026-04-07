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
    transcribed_chunks: Vec<String>,
}

pub fn setup_hotkey(app: &AppHandle, config: Arc<Mutex<AppConfig>>) -> Result<(), String> {
    let recording_state = Arc::new(Mutex::new(RecordingState {
        is_recording: false,
        recorder: AudioRecorder::new()?,
        transcribed_chunks: Vec::new(),
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

                match event.state() {
                    ShortcutState::Pressed => {
                        let mut s = state.lock().unwrap();
                        if s.is_recording {
                            return; // Already recording, ignore repeat
                        }
                        s.is_recording = true;
                        s.transcribed_chunks.clear();

                        if let Err(e) = s.recorder.start() {
                            eprintln!("Failed to start recording: {}", e);
                            let _ = handle.emit("recording-error", e);
                            s.is_recording = false;
                            return;
                        }

                        let _ = handle.emit("recording-started", ());

                        // Start chunk sender in background
                        let chunk_state = Arc::clone(&state);
                        let chunk_config = Arc::clone(&config);
                        let chunk_handle = handle.clone();

                        tauri::async_runtime::spawn(async move {
                            loop {
                                tokio::time::sleep(Duration::from_secs(2)).await;

                                let is_recording = chunk_state.lock().unwrap().is_recording;
                                if !is_recording {
                                    break;
                                }

                                let wav_data = chunk_state.lock().unwrap().recorder.take_chunk();
                                if let Some(data) = wav_data {
                                    let api_key = chunk_config.lock().unwrap().groq_api_key.clone();
                                    let language = chunk_config.lock().unwrap().language.clone();

                                    match groq::transcribe(&api_key, data, &language).await {
                                        Ok(text) => {
                                            if !text.trim().is_empty() {
                                                chunk_state
                                                    .lock()
                                                    .unwrap()
                                                    .transcribed_chunks
                                                    .push(text.clone());
                                                let full_text: String = chunk_state
                                                    .lock()
                                                    .unwrap()
                                                    .transcribed_chunks
                                                    .join(" ");
                                                let _ =
                                                    chunk_handle.emit("transcription-update", &full_text);
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("Transcription error: {}", e);
                                            let _ = chunk_handle.emit("recording-error", e);
                                        }
                                    }
                                }
                            }
                        });
                    }
                    ShortcutState::Released => {
                        let config = config.lock().unwrap().clone();
                        let handle = handle.clone();
                        let state = Arc::clone(&state);

                        tauri::async_runtime::spawn(async move {
                            // Stop recording
                            let wav_data = {
                                let mut s = state.lock().unwrap();
                                if !s.is_recording {
                                    return;
                                }
                                s.is_recording = false;
                                s.recorder.stop();
                                s.recorder.take_all()
                            };

                            // Transcribe final chunk
                            if let Some(data) = wav_data {
                                match groq::transcribe(
                                    &config.groq_api_key,
                                    data,
                                    &config.language,
                                )
                                .await
                                {
                                    Ok(text) => {
                                        if !text.trim().is_empty() {
                                            state
                                                .lock()
                                                .unwrap()
                                                .transcribed_chunks
                                                .push(text);
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Final transcription error: {}", e);
                                    }
                                }
                            }

                            let full_text: String =
                                state.lock().unwrap().transcribed_chunks.join(" ");

                            if full_text.trim().is_empty() {
                                let _ = handle.emit("recording-stopped", "");
                                return;
                            }

                            // Check for command mode
                            let prefix = config.command_prefix.to_lowercase();
                            let lower = full_text.trim().to_lowercase();

                            if config.command_mode && lower.starts_with(&prefix) {
                                let instruction =
                                    full_text.trim()[prefix.len()..].trim().to_string();
                                let _ = handle.emit("command-processing", &instruction);

                                match paste::get_selected_text() {
                                    Ok(selected) => {
                                        match groq::chat_command(
                                            &config.groq_api_key,
                                            &config.llm_model,
                                            &instruction,
                                            &selected,
                                        )
                                        .await
                                        {
                                            Ok(result) => {
                                                let _ = paste::paste_text(&result);
                                                let _ =
                                                    handle.emit("command-complete", &result);
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
                                // Normal dictation — paste text
                                if config.auto_paste {
                                    if let Err(e) = paste::paste_text(&full_text) {
                                        eprintln!("Paste error: {}", e);
                                    }
                                }
                                let _ = handle.emit("recording-stopped", &full_text);
                            }
                        });
                    }
                }
            })
            .build(),
    )
    .map_err(|e| format!("Failed to init global shortcut: {}", e))?;

    // Register hotkey from config
    let hotkey_str = config.lock().unwrap().hotkey.clone();
    let shortcut = parse_hotkey(&hotkey_str);
    app.global_shortcut()
        .register(shortcut)
        .map_err(|e| format!("Failed to register hotkey '{}': {}", hotkey_str, e))?;

    Ok(())
}

fn parse_hotkey(key: &str) -> Shortcut {
    match key {
        "F8" => Shortcut::new(None, Code::F8),
        "F9" => Shortcut::new(None, Code::F9),
        "F10" => Shortcut::new(None, Code::F10),
        "Ctrl+Shift+Space" => Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space),
        "Ctrl+Shift+V" => Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyV),
        "Alt+Space" => Shortcut::new(Some(Modifiers::ALT), Code::Space),
        "Ctrl+Alt+Space" => Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space),
        _ => Shortcut::new(None, Code::F9), // fallback
    }
}
