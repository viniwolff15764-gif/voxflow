use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;

// On macOS the copy/paste modifier is Command (Meta); everywhere else it's Control.
#[cfg(target_os = "macos")]
const MOD_KEY: Key = Key::Meta;
#[cfg(not(target_os = "macos"))]
const MOD_KEY: Key = Key::Control;

/// Put text on the clipboard and simulate the paste shortcut into the active field.
/// The text is left on the clipboard afterwards, so ⌘V still works as a fallback.
pub fn paste_text(text: &str) -> Result<(), String> {
    set_clipboard(text)?;
    thread::sleep(Duration::from_millis(120));
    press_combo('v')
}

/// Just put the text on the clipboard (used when we can't auto-paste).
pub fn copy_only(text: &str) -> Result<(), String> {
    set_clipboard(text)
}

/// Read currently selected text by simulating copy and reading the clipboard.
pub fn get_selected_text() -> Result<String, String> {
    press_combo('c')?;
    thread::sleep(Duration::from_millis(120));
    get_clipboard()
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
