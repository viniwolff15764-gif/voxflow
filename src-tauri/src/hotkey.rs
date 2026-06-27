use crate::audio::AudioRecorder;
use crate::config::AppConfig;
use crate::groq;
use crate::paste;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

struct RecordingState {
    is_recording: bool,
    recorder: AudioRecorder,
}

type SharedState = Arc<Mutex<RecordingState>>;
type SharedConfig = Arc<Mutex<AppConfig>>;

/// Run a closure on the macOS/Windows main thread and wait for its result.
/// Clipboard (NSPasteboard) and key simulation MUST run on the main thread on
/// macOS, otherwise the app crashes. Called from background async tasks.
fn on_main<T, F>(handle: &AppHandle, f: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    let _ = handle.run_on_main_thread(move || {
        let _ = tx.send(f());
    });
    rx.recv().unwrap()
}

pub fn setup_hotkey(app: &AppHandle, config: SharedConfig) -> Result<(), String> {
    let recording_state: SharedState = Arc::new(Mutex::new(RecordingState {
        is_recording: false,
        recorder: AudioRecorder::new()?,
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
                    }
                }
            })
            .build(),
    )
    .map_err(|e| format!("Failed to init global shortcut: {}", e))?;

    let hotkey_str = config.lock().unwrap().hotkey.clone();
    app.global_shortcut()
        .register(parse_hotkey(&hotkey_str))
        .map_err(|e| format!("Failed to register hotkey '{}': {}", hotkey_str, e))?;

    Ok(())
}

/// Re-register the global shortcut at runtime (Settings change, no restart).
pub fn reregister(app: &AppHandle, hotkey: &str) -> Result<(), String> {
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| e.to_string())?;
    app.global_shortcut()
        .register(parse_hotkey(hotkey))
        .map_err(|e| format!("Failed to register hotkey '{}': {}", hotkey, e))
}

fn start_recording(state: SharedState, config: SharedConfig, handle: AppHandle) {
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

    if let Err(e) = s.recorder.start() {
        eprintln!("Failed to start recording: {}", e);
        let _ = handle.emit("recording-error", e);
        s.is_recording = false;
        return;
    }
    drop(s);

    let _ = handle.emit("recording-started", ());

    // Live loudness loop → drives the waveform animation in the UI.
    let level_state = Arc::clone(&state);
    let level_handle = handle.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(60)).await;
            let s = level_state.lock().unwrap();
            if !s.is_recording {
                break;
            }
            let level = s.recorder.peak_level();
            drop(s);
            let _ = level_handle.emit("audio-level", level);
        }
    });
}

fn stop_and_process(state: SharedState, config: SharedConfig, handle: AppHandle) {
    // Stop the stream and grab the full recording synchronously.
    let (full_audio, peak) = {
        let mut s = state.lock().unwrap();
        if !s.is_recording {
            return;
        }
        s.is_recording = false;
        s.recorder.stop();
        let peak = s.recorder.overall_peak();
        (s.recorder.take_all(), peak)
    };

    // Near-silence almost always means the microphone permission was denied
    // (macOS hands back only zeros). Tell the user instead of sending silence
    // to Whisper, which would hallucinate a short phrase.
    if peak < 0.012 {
        let _ = handle.emit(
            "recording-error",
            "Não ouvi áudio. Ative o Microfone em Ajustes → Privacidade e Segurança → Microfone."
                .to_string(),
        );
        return;
    }

    let cfg = config.lock().unwrap().clone();
    let _ = handle.emit("processing", ());

    tauri::async_runtime::spawn(async move {
        let data = match full_audio {
            Some(d) => d,
            None => {
                let _ = handle.emit("recording-stopped", "");
                return;
            }
        };

        let full_text =
            match groq::transcribe(&cfg.groq_api_key, data, &cfg.language, &cfg.whisper_model).await {
                Ok(t) => t.trim().to_string(),
                Err(e) => {
                    let _ = handle.emit("recording-error", e);
                    return;
                }
            };

        if full_text.is_empty() {
            let _ = handle.emit("recording-stopped", "");
            return;
        }

        // Command mode: "<prefix> <instruction>" rewrites the selected text.
        let prefix = cfg.command_prefix.to_lowercase();
        if cfg.command_mode && !prefix.is_empty() && full_text.to_lowercase().starts_with(&prefix) {
            let instruction = full_text[prefix.len()..].trim().to_string();
            let _ = handle.emit("command-processing", &instruction);

            let h = handle.clone();
            let selected = on_main(&handle, move || paste::get_selected_text());
            match selected {
                Ok(selected) => {
                    match groq::chat_command(&cfg.groq_api_key, &cfg.llm_model, &instruction, &selected)
                        .await
                    {
                        Ok(result) => {
                            let r = result.clone();
                            let _ = on_main(&h, move || paste::paste_text(&r));
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
            // Normal dictation — paste on the main thread (required on macOS).
            if cfg.auto_paste {
                let t = full_text.clone();
                let _ = on_main(&handle, move || paste::paste_text(&t));
            }
            let _ = handle.emit("recording-stopped", &full_text);
        }
    });
}

fn parse_hotkey(key: &str) -> Shortcut {
    match key {
        "F6" => Shortcut::new(None, Code::F6),
        "F7" => Shortcut::new(None, Code::F7),
        "F8" => Shortcut::new(None, Code::F8),
        "F9" => Shortcut::new(None, Code::F9),
        "F10" => Shortcut::new(None, Code::F10),
        "Ctrl+Shift+Space" => Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space),
        "Ctrl+Shift+D" => Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyD),
        "Ctrl+Alt+Space" => Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space),
        "Cmd+Shift+Space" => Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Space),
        "Cmd+Shift+D" => Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyD),
        "Cmd+Ctrl+Space" => Shortcut::new(Some(Modifiers::SUPER | Modifiers::CONTROL), Code::Space),
        _ => Shortcut::new(None, Code::F9),
    }
}
