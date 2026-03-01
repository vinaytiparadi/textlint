use tauri::Emitter;
#[cfg(windows)]
use windows::Win32::Foundation::POINT;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
};

/// Position on screen
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScreenPosition {
    pub x: i32,
    pub y: i32,
}

const PANEL_WIDTH: i32 = 360;
const PANEL_HEIGHT: i32 = 300;
const PANEL_MARGIN: i32 = 12;

/// Get the current mouse cursor position.
#[cfg(windows)]
pub fn get_cursor_position() -> Option<ScreenPosition> {
    unsafe {
        let mut point = POINT::default();
        if GetCursorPos(&mut point).is_ok() {
            Some(ScreenPosition {
                x: point.x,
                y: point.y,
            })
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
pub fn get_cursor_position() -> Option<ScreenPosition> {
    None
}

/// Calculate where the panel should appear:
/// - Prefer ABOVE the cursor
/// - If not enough room above, place BELOW
/// - Keep within screen bounds horizontally
#[cfg(windows)]
pub fn calculate_panel_position(
    cursor: &ScreenPosition,
    width: i32,
    height: i32,
) -> ScreenPosition {
    let (screen_w, screen_h) =
        unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };

    // Try to place above the cursor
    let mut y = cursor.y - height - PANEL_MARGIN;
    if y < 0 {
        // Not enough room above, place below
        y = cursor.y + PANEL_MARGIN;
    }
    // Clamp to screen bottom
    if y + height > screen_h {
        y = screen_h - height - PANEL_MARGIN;
    }

    // Horizontal: center on cursor, clamp to screen
    let mut x = cursor.x - width / 2;
    if x < 0 {
        x = PANEL_MARGIN;
    }
    if x + width > screen_w {
        x = screen_w - width - PANEL_MARGIN;
    }

    ScreenPosition { x, y }
}

#[cfg(not(windows))]
pub fn calculate_panel_position(
    cursor: &ScreenPosition,
    _width: i32,
    height: i32,
) -> ScreenPosition {
    ScreenPosition {
        x: cursor.x,
        y: cursor.y.saturating_sub((height + PANEL_MARGIN) as usize) as i32,
    }
}

/// IPC command: get the position where the panel should appear
#[tauri::command]
pub fn get_panel_position() -> Option<ScreenPosition> {
    get_cursor_position()
}

/// IPC command: show the floating panel near the cursor with correction data
#[tauri::command]
pub async fn show_panel_at_cursor(
    app: tauri::AppHandle,
    corrections_json: String,
) -> Result<(), String> {
    use tauri::Manager;

    let cursor = get_cursor_position().unwrap_or(ScreenPosition { x: 400, y: 400 });
    let position = calculate_panel_position(&cursor, PANEL_WIDTH, PANEL_HEIGHT);

    println!(
        "[GrammarLens] Panel position: cursor=({},{}) panel=({},{})",
        cursor.x, cursor.y, position.x, position.y
    );

    if let Some(panel_window) = app.get_webview_window("panel") {
        let _ = panel_window.set_position(tauri::PhysicalPosition::new(position.x, position.y));
        let _ = panel_window.show();
        let _ = panel_window.set_focus();
    }

    let _ = app.emit(
        "panel-show",
        serde_json::json!({
            "x": position.x,
            "y": position.y,
            "corrections": corrections_json
        }),
    );

    Ok(())
}

/// IPC command: hide the floating panel
#[tauri::command]
pub async fn hide_panel(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;

    if let Some(panel_window) = app.get_webview_window("panel") {
        let _ = panel_window.hide();
    }

    Ok(())
}
