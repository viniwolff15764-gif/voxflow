use enigo::{Enigo, Keyboard, Settings, Key, Direction};
use std::thread;
use std::time::Duration;

/// Copy text to clipboard and simulate Ctrl+V to paste into active field.
pub fn paste_text(text: &str) -> Result<(), String> {
    set_clipboard(text)?;

    // Small delay to ensure clipboard is ready
    thread::sleep(Duration::from_millis(80));

    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| format!("Enigo init: {}", e))?;

    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| format!("Key press: {}", e))?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| format!("Key click: {}", e))?;
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| format!("Key release: {}", e))?;

    Ok(())
}

/// Read currently selected text by simulating Ctrl+C and reading clipboard.
pub fn get_selected_text() -> Result<String, String> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| format!("Enigo init: {}", e))?;

    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| format!("Key press: {}", e))?;
    enigo
        .key(Key::Unicode('c'), Direction::Click)
        .map_err(|e| format!("Key click: {}", e))?;
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| format!("Key release: {}", e))?;

    thread::sleep(Duration::from_millis(100));

    get_clipboard()
}

fn set_clipboard(text: &str) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard init: {}", e))?;
    clipboard.set_text(text).map_err(|e| format!("Clipboard set: {}", e))?;
    Ok(())
}

fn get_clipboard() -> Result<String, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard init: {}", e))?;
    clipboard.get_text().map_err(|e| format!("Clipboard get: {}", e))
}
