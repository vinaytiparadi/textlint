use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;

/// Save the current clipboard text content, returning it
pub fn save_clipboard() -> Option<String> {
    let mut clipboard = Clipboard::new().ok()?;
    clipboard.get_text().ok()
}

/// Restore clipboard text content
pub fn restore_clipboard(content: Option<String>) {
    if let Some(text) = content {
        if let Ok(mut clipboard) = Clipboard::new() {
            let _ = clipboard.set_text(text);
        }
    }
}

/// Simulate Ctrl+C to copy selected text, then read from clipboard
pub fn copy_selected_text() -> Result<String, String> {
    // Small delay to let any modifiers release
    thread::sleep(Duration::from_millis(50));

    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| format!("Enigo init error: {}", e))?;

    // Press Ctrl+C
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| format!("Key error: {}", e))?;
    enigo
        .key(Key::Unicode('c'), Direction::Click)
        .map_err(|e| format!("Key error: {}", e))?;
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| format!("Key error: {}", e))?;

    // Wait for system clipboard to update
    thread::sleep(Duration::from_millis(150));

    // Read from clipboard
    let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
    clipboard
        .get_text()
        .map_err(|e| format!("Clipboard read error: {}", e))
}

/// Write corrected text to clipboard and simulate Ctrl+V to paste
pub fn paste_text(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
    clipboard
        .set_text(text.to_string())
        .map_err(|e| format!("Clipboard write error: {}", e))?;

    // Small delay before pasting
    thread::sleep(Duration::from_millis(50));

    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| format!("Enigo init error: {}", e))?;

    // Press Ctrl+V
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| format!("Key error: {}", e))?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| format!("Key error: {}", e))?;
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| format!("Key error: {}", e))?;

    // Wait for paste to complete
    thread::sleep(Duration::from_millis(100));

    Ok(())
}

/// Execute the full clipboard-based correction flow:
/// 1. Save current clipboard
/// 2. Copy selected text (Ctrl+C)
/// 3. Read clipboard to get selected text
/// 4. Return the selected text and saved clipboard for later restoration
pub fn capture_selected_text() -> Result<(String, Option<String>), String> {
    // Save original clipboard content
    let original_clipboard = save_clipboard();

    // Copy selected text
    let selected_text = copy_selected_text()?;

    if selected_text.trim().is_empty() {
        // Restore original clipboard
        restore_clipboard(original_clipboard);
        return Err("No text selected".to_string());
    }

    Ok((selected_text, original_clipboard))
}

/// Paste corrected text and restore original clipboard
pub fn apply_correction(
    corrected_text: &str,
    original_clipboard: Option<String>,
) -> Result<(), String> {
    paste_text(corrected_text)?;

    // Brief delay then restore clipboard
    thread::sleep(Duration::from_millis(200));
    restore_clipboard(original_clipboard);

    Ok(())
}
