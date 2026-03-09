use crate::app_filter;
use crate::clipboard;
use crate::corrections::CorrectionResult;
use crate::floating_panel::{self, ScreenPosition};
use crate::gemini;
use crate::settings::{self, SettingsState};
use tauri::{AppHandle, Emitter, Manager};

/// The main correction handler — called when the global shortcut is triggered.
/// Orchestrates: check disabled app → capture text → call API → handle result.
pub async fn handle_correction_trigger(app: &AppHandle) {
    log::debug!("[TextLint] Correction trigger started...");

    // IMMEDIATELY capture cursor position before anything else
    let cursor_position = floating_panel::get_cursor_position();
    log::debug!("[TextLint] Cursor captured at: {:?}", cursor_position);

    // Read current settings
    let (api_key, strictness, learn_mode, auto_apply, enhance_writing, disabled_apps) = {
        let state = app.state::<SettingsState>();
        let settings = match state.0.lock() {
            Ok(s) => s,
            Err(poisoned) => {
                log::error!(
                    "[TextLint] Settings Mutex poisoned in handle_correction_trigger, recovering"
                );
                poisoned.into_inner()
            }
        };
        (
            settings::load_api_key(), // loaded from OS keyring, never from settings struct
            settings.strictness.clone(),
            settings.learn_mode,
            settings.auto_apply,
            settings.enhance_writing,
            settings.disabled_apps.clone(),
        )
    };

    // Check if API key is set
    if api_key.is_empty() {
        log::error!("[TextLint] ERROR: No API key configured!");
        show_error(app, "API key not configured. Right-click the tray icon → Settings to add your Gemini API key.", &cursor_position);
        return;
    }

    // Check if current app is disabled
    if app_filter::is_app_disabled(&disabled_apps) {
        log::info!("[TextLint] Skipped: foreground app is in disabled list");
        return;
    }

    let foreground = app_filter::get_foreground_app().unwrap_or_default();
    log::info!("[TextLint] Foreground app: {}", foreground);

    // Capture selected text via clipboard
    log::debug!("[TextLint] Capturing selected text...");
    let (selected_text, original_clipboard) = match clipboard::capture_selected_text() {
        Ok(result) => result,
        Err(e) => {
            log::error!("[TextLint] ERROR: Failed to capture text: {}", e);
            show_error(
                app,
                "Failed to capture text. Please try again.",
                &cursor_position,
            );
            return;
        }
    };

    log::info!("[TextLint] Captured text ({} chars)", selected_text.len());

    const MAX_TEXT_LENGTH: usize = 10_000;
    if selected_text.len() > MAX_TEXT_LENGTH {
        show_error(
            app,
            &format!(
                "Selected text is too long ({} chars). Please select up to {} characters at a time.",
                selected_text.len(),
                MAX_TEXT_LENGTH
            ),
            &cursor_position,
        );
        clipboard::restore_clipboard(original_clipboard);
        return;
    }

    let word_count = selected_text.split_whitespace().count();
    if word_count > 1_500 {
        show_error(
            app,
            &format!(
                "Selected text is too long ({} words). Please select up to 1500 words at a time.",
                word_count
            ),
            &cursor_position,
        );
        clipboard::restore_clipboard(original_clipboard);
        return;
    }

    // Call Gemini API
    log::debug!("[TextLint] Calling Gemini API...");
    let response =
        match gemini::check_grammar(&api_key, &selected_text, &strictness, enhance_writing).await {
            Ok(resp) => {
                log::debug!(
                    "[TextLint] API response received. Has changes: {}",
                    resp.has_changes
                );
                resp
            }
            Err(e) => {
                log::error!("[TextLint] ERROR: Gemini API error: {}", e);
                clipboard::restore_clipboard(original_clipboard);
                show_error(app, &e, &cursor_position);
                return;
            }
        };

    let result = CorrectionResult::from_response(selected_text, response);

    if result.has_changes {
        log::info!("[TextLint] {} corrections found", result.num_corrections);

        // Store correction securely in backend state
        if let Ok(mut pending) = app.state::<crate::PendingCorrectionState>().0.lock() {
            *pending = Some(result.corrected_text.clone());
        }

        if learn_mode {
            // In Learn Mode: show the panel instead of auto-pasting
            log::debug!("[TextLint] Learn Mode ON: showing panel");
            clipboard::restore_clipboard(original_clipboard);
            show_panel_with_result(app, &result, &cursor_position, true);
        } else if auto_apply {
            // Auto-apply: paste corrected text
            log::debug!("[TextLint] Auto-applying correction...");
            if let Err(e) = clipboard::apply_correction(&result.corrected_text, original_clipboard)
            {
                log::error!("[TextLint] ERROR: Failed to apply correction: {}", e);
                show_error(app, &e, &cursor_position);
                return;
            }
            log::info!("[TextLint] Correction applied successfully");
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
        log::info!("[TextLint] No corrections needed - text looks good!");
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

    log::debug!(
        "[TextLint] Panel positioned at ({},{}) for cursor ({},{})",
        pos.x,
        pos.y,
        cursor_pos.x,
        cursor_pos.y
    );

    if let Some(panel_win) = app.get_webview_window("panel") {
        // Compact window: 360 wide, height depends on corrections count
        let height = std::cmp::min(400, 180 + (result.num_corrections * 60) as u32);
        let _ = panel_win.set_size(tauri::PhysicalSize::new(360u32, height));
        let _ = panel_win.set_position(tauri::PhysicalPosition::new(pos.x, pos.y));
        let _ = panel_win.show();
        let _ = panel_win.set_focus();
        let _ = panel_win.set_always_on_top(true);

        #[derive(serde::Serialize, Clone)]
        struct CorrectionsPayload<'a> {
            result: &'a CorrectionResult,
            #[serde(rename = "learnMode")]
            learn_mode: bool,
        }

        let _ = panel_win.emit(
            "show-corrections",
            CorrectionsPayload { result, learn_mode },
        );
        log::debug!("[TextLint] Panel data emitted via event");
    }
}

/// Show "Looks good!" in the panel (no changes case)
fn show_info(app: &AppHandle, message: &str, subtitle: &str, cursor: &Option<ScreenPosition>) {
    let default_cursor = ScreenPosition { x: 400, y: 400 };
    let cursor_pos = cursor.as_ref().unwrap_or(&default_cursor);
    let pos = floating_panel::calculate_panel_position(cursor_pos, 320, 120);

    if let Some(panel_win) = app.get_webview_window("panel") {
        let _ = panel_win.set_size(tauri::PhysicalSize::new(320u32, 120u32));
        let _ = panel_win.set_position(tauri::PhysicalPosition::new(pos.x, pos.y));
        let _ = panel_win.show();
        let _ = panel_win.set_focus();
        let _ = panel_win.set_always_on_top(true);

        #[derive(serde::Serialize, Clone)]
        struct InfoPayload<'a> {
            message: &'a str,
            subtitle: &'a str,
        }
        let _ = panel_win.emit("show-info", InfoPayload { message, subtitle });
    }
}

/// Show error in the panel
fn show_error(app: &AppHandle, message: &str, cursor: &Option<ScreenPosition>) {
    let default_cursor = ScreenPosition { x: 400, y: 400 };
    let cursor_pos = cursor.as_ref().unwrap_or(&default_cursor);
    let pos = floating_panel::calculate_panel_position(cursor_pos, 340, 140);

    if let Some(panel_win) = app.get_webview_window("panel") {
        let _ = panel_win.set_size(tauri::PhysicalSize::new(340u32, 140u32));
        let _ = panel_win.set_position(tauri::PhysicalPosition::new(pos.x, pos.y));
        let _ = panel_win.show();
        let _ = panel_win.set_focus();
        let _ = panel_win.set_always_on_top(true);

        #[derive(serde::Serialize, Clone)]
        struct ErrorPayload<'a> {
            message: &'a str,
        }
        let _ = panel_win.emit("show-error", ErrorPayload { message });
    }
}

/// IPC command: manually trigger correction
#[tauri::command]
pub async fn trigger_correction(app: AppHandle) {
    handle_correction_trigger(&app).await;
}

/// IPC command: apply a specific correction (from the panel)
#[tauri::command]
pub fn apply_current_correction(app: AppHandle) -> Result<(), String> {
    let text_to_paste = match app.state::<crate::PendingCorrectionState>().0.lock() {
        Ok(mut pending) => pending.take(),
        Err(_) => None,
    };

    if let Some(text) = text_to_paste {
        clipboard::paste_text(&text)
    } else {
        Err("No pending correction found to apply.".to_string())
    }
}
