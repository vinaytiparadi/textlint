use crate::app_filter;
use crate::clipboard;
use crate::corrections::CorrectionResult;
use crate::floating_panel::{self, ScreenPosition};
use crate::gemini;
use crate::settings::SettingsState;
use tauri::{AppHandle, Manager};

/// The main correction handler — called when the global shortcut is triggered.
/// Orchestrates: check disabled app → capture text → call API → handle result.
pub async fn handle_correction_trigger(app: &AppHandle) {
    println!("[TextLint] Correction trigger started...");

    // IMMEDIATELY capture cursor position before anything else
    let cursor_position = floating_panel::get_cursor_position();
    println!("[TextLint] Cursor captured at: {:?}", cursor_position);

    // Read current settings
    let (api_key, strictness, learn_mode, auto_apply, disabled_apps) = {
        let state = app.state::<SettingsState>();
        let settings = state.0.lock().unwrap();
        (
            settings.api_key.clone(),
            settings.strictness.clone(),
            settings.learn_mode,
            settings.auto_apply,
            settings.disabled_apps.clone(),
        )
    };

    // Check if API key is set
    if api_key.is_empty() {
        println!("[TextLint] ERROR: No API key configured!");
        show_error(app, "API key not configured. Right-click the tray icon → Settings to add your Gemini API key.", &cursor_position);
        return;
    }

    // Check if current app is disabled
    if app_filter::is_app_disabled(&disabled_apps) {
        println!("[TextLint] Skipped: foreground app is in disabled list");
        return;
    }

    let foreground = app_filter::get_foreground_app().unwrap_or_default();
    println!("[TextLint] Foreground app: {}", foreground);

    // Capture selected text via clipboard
    println!("[TextLint] Capturing selected text...");
    let (selected_text, original_clipboard) = match clipboard::capture_selected_text() {
        Ok(result) => result,
        Err(e) => {
            println!("[TextLint] ERROR: Failed to capture text: {}", e);
            show_error(
                app,
                &format!("Failed to capture text: {}", e),
                &cursor_position,
            );
            return;
        }
    };

    println!(
        "[TextLint] Captured text ({} chars): \"{}\"",
        selected_text.len(),
        if selected_text.len() > 80 {
            &selected_text[..80]
        } else {
            &selected_text
        }
    );

    // Call Gemini API
    println!("[TextLint] Calling Gemini API...");
    let response = match gemini::check_grammar(&api_key, &selected_text, &strictness).await {
        Ok(resp) => {
            println!(
                "[TextLint] API response received. Has changes: {}",
                resp.has_changes
            );
            resp
        }
        Err(e) => {
            println!("[TextLint] ERROR: Gemini API error: {}", e);
            clipboard::restore_clipboard(original_clipboard);
            show_error(app, &e, &cursor_position);
            return;
        }
    };

    let result = CorrectionResult::from_response(selected_text, response);

    if result.has_changes {
        println!("[TextLint] {} corrections found", result.num_corrections);

        if learn_mode {
            // In Learn Mode: show the panel instead of auto-pasting
            println!("[TextLint] Learn Mode ON: showing panel");
            clipboard::restore_clipboard(original_clipboard);
            show_panel_with_result(app, &result, &cursor_position, true);
        } else if auto_apply {
            // Auto-apply: paste corrected text
            println!("[TextLint] Auto-applying correction...");
            if let Err(e) = clipboard::apply_correction(&result.corrected_text, original_clipboard)
            {
                println!("[TextLint] ERROR: Failed to apply correction: {}", e);
                show_error(app, &e, &cursor_position);
                return;
            }
            println!("[TextLint] Correction applied successfully");
            // Just replaced the text, no explanation needed (Learn Mode OFF)
            show_info(
                app,
                "Fixed!",
                &format!(
                    "{} correction{} applied.",
                    result.num_corrections,
                    if result.num_corrections == 1 { "" } else { "s" }
                ),
                &cursor_position,
            );
        } else {
            // Not auto-apply and not learn mode: show panel for review (no explanations)
            clipboard::restore_clipboard(original_clipboard);
            show_panel_with_result(app, &result, &cursor_position, false);
        }
    } else {
        // No changes needed
        println!("[TextLint] No corrections needed - text looks good!");
        clipboard::restore_clipboard(original_clipboard);
        show_info(app, "Looks good!", "No changes needed.", &cursor_position);
    }
}

/// Show the panel window with correction result, positioned using the captured cursor position
fn show_panel_with_result(
    app: &AppHandle,
    result: &CorrectionResult,
    cursor: &Option<ScreenPosition>,
    learn_mode: bool,
) {
    let default_cursor = ScreenPosition { x: 400, y: 400 };
    let cursor_pos = cursor.as_ref().unwrap_or(&default_cursor);
    let height = std::cmp::min(400, 180 + (result.num_corrections * 60) as i32);
    let pos = floating_panel::calculate_panel_position(cursor_pos, 360, height);

    println!(
        "[TextLint] Panel positioned at ({},{}) for cursor ({},{})",
        pos.x, pos.y, cursor_pos.x, cursor_pos.y
    );

    if let Some(panel_win) = app.get_webview_window("panel") {
        // Compact window: 360 wide, height depends on corrections count
        let height = std::cmp::min(400, 180 + (result.num_corrections * 60) as u32);
        let _ = panel_win.set_size(tauri::PhysicalSize::new(360u32, height));
        let _ = panel_win.set_position(tauri::PhysicalPosition::new(pos.x, pos.y));
        let _ = panel_win.show();
        let _ = panel_win.set_focus();
        let _ = panel_win.set_always_on_top(true);

        // Inject data directly via eval — 100% reliable, no event timing issues
        if let Ok(json) = serde_json::to_string(result) {
            let js = format!("window.showCorrections({}, {})", json, learn_mode);
            let _ = panel_win.eval(&js);
            println!("[TextLint] Panel data injected via eval");
        }
    }
}

/// Show "Looks good!" in the panel (no changes case)
fn show_info(app: &AppHandle, message: &str, subtitle: &str, cursor: &Option<ScreenPosition>) {
    let default_cursor = ScreenPosition { x: 400, y: 400 };
    let cursor_pos = cursor.as_ref().unwrap_or(&default_cursor);
    let pos = floating_panel::calculate_panel_position(cursor_pos, 320, 95);

    if let Some(panel_win) = app.get_webview_window("panel") {
        let _ = panel_win.set_size(tauri::PhysicalSize::new(320u32, 95u32));
        let _ = panel_win.set_position(tauri::PhysicalPosition::new(pos.x, pos.y));
        let _ = panel_win.show();
        let _ = panel_win.set_focus();
        let _ = panel_win.set_always_on_top(true);

        let msg_escaped = message.replace('\\', "\\\\").replace('"', "\\\"");
        let sub_escaped = subtitle.replace('\\', "\\\\").replace('"', "\\\"");
        let js = format!("window.showInfo(\"{}\", \"{}\")", msg_escaped, sub_escaped);
        let _ = panel_win.eval(&js);
    }
}

/// Show error in the panel
fn show_error(app: &AppHandle, message: &str, cursor: &Option<ScreenPosition>) {
    let default_cursor = ScreenPosition { x: 400, y: 400 };
    let cursor_pos = cursor.as_ref().unwrap_or(&default_cursor);
    let pos = floating_panel::calculate_panel_position(cursor_pos, 340, 130);

    if let Some(panel_win) = app.get_webview_window("panel") {
        let _ = panel_win.set_size(tauri::PhysicalSize::new(340u32, 130u32));
        let _ = panel_win.set_position(tauri::PhysicalPosition::new(pos.x, pos.y));
        let _ = panel_win.show();
        let _ = panel_win.set_focus();
        let _ = panel_win.set_always_on_top(true);

        let msg_escaped = message.replace('\\', "\\\\").replace('"', "\\\"");
        let js = format!("window.showError(\"{}\")", msg_escaped);
        let _ = panel_win.eval(&js);
    }
}

/// IPC command: manually trigger correction
#[tauri::command]
pub async fn trigger_correction(app: AppHandle) {
    handle_correction_trigger(&app).await;
}

/// IPC command: apply a specific correction (from the panel)
#[tauri::command]
pub fn apply_correction_text(text: String) -> Result<(), String> {
    clipboard::paste_text(&text)
}
