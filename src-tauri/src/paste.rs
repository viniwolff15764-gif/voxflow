use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;

// On macOS the copy/paste modifier is Command (Meta); everywhere else it's Control.
#[cfg(target_os = "macos")]
const MOD_KEY: Key = Key::Meta;
#[cfg(not(target_os = "macos"))]
const MOD_KEY: Key = Key::Control;

/// Copy text to clipboard and simulate the paste shortcut into the active field.
/// Preserves whatever the user already had on the clipboard.
pub fn paste_text(text: &str) -> Result<(), String> {
    let previous = get_clipboard().ok();

    set_clipboard(text)?;

    // Small delay to ensure clipboard is ready before pasting.
    thread::sleep(Duration::from_millis(90));

    press_combo('v')?;

    // Restore the user's previous clipboard after the paste lands.
    if let Some(prev) = previous {
        thread::sleep(Duration::from_millis(150));
        let _ = set_clipboard(&prev);
    }

    Ok(())
}

/// Read currently selected text by simulating copy and reading the clipboard.
/// Restores the previous clipboard afterwards.
pub fn get_selected_text() -> Result<String, String> {
    let previous = get_clipboard().ok();

    press_combo('c')?;
    thread::sleep(Duration::from_millis(120));

    let selected = get_clipboard()?;

    if let Some(prev) = previous {
        let _ = set_clipboard(&prev);
    }

    Ok(selected)
}

fn press_combo(letter: char) -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| format!("Enigo init: {}", e))?;
    enigo
        .key(MOD_KEY, Direction::Press)
        .map_err(|e| format!("Key press: {}", e))?;
    enigo
        .key(Key::Unicode(letter), Direction::Click)
        .map_err(|e| format!("Key click: {}", e))?;
    enigo
        .key(MOD_KEY, Direction::Release)
        .map_err(|e| format!("Key release: {}", e))?;
    Ok(())
}

fn set_clipboard(text: &str) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard init: {}", e))?;
    clipboard
        .set_text(text)
        .map_err(|e| format!("Clipboard set: {}", e))?;
    Ok(())
}

fn get_clipboard() -> Result<String, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard init: {}", e))?;
    clipboard
        .get_text()
        .map_err(|e| format!("Clipboard get: {}", e))
}
