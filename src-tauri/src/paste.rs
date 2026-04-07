use enigo::{Enigo, Keyboard, Settings, Key, Direction};
use std::thread;
use std::time::Duration;

/// Copy text to clipboard and simulate Ctrl+V to paste into active field.
pub fn paste_text(text: &str) -> Result<(), String> {
    set_clipboard(text)?;

    // Small delay to ensure clipboard is ready
    thread::sleep(Duration::from_millis(50));

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

#[cfg(target_os = "windows")]
fn set_clipboard(text: &str) -> Result<(), String> {
    use std::process::Command;
    let mut child = Command::new("cmd")
        .args(["/C", &format!("echo {} | clip", text)])
        .spawn()
        .map_err(|e| e.to_string())?;
    child.wait().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn set_clipboard(text: &str) -> Result<(), String> {
    use std::process::Command;
    use std::io::Write;
    let mut child = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    child.stdin.as_mut().unwrap().write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    child.wait().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn get_clipboard() -> Result<String, String> {
    use std::process::Command;
    let output = Command::new("powershell")
        .args(["-Command", "Get-Clipboard"])
        .output()
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(not(target_os = "windows"))]
fn get_clipboard() -> Result<String, String> {
    use std::process::Command;
    let output = Command::new("xclip")
        .args(["-selection", "clipboard", "-o"])
        .output()
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
