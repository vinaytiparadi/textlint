mod app_filter;
mod clipboard;
mod corrections;
mod floating_panel;
mod gemini;
mod settings;
mod shortcuts;

use settings::{load_api_key, load_settings, SettingsState};
use std::sync::Mutex;
use tauri::{
    menu::{CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    Emitter, Manager,
};

pub struct PendingCorrectionState(pub Mutex<Option<String>>);
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        log::debug!("[TextLint] Shortcut triggered!");
                        let app_handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            shortcuts::handle_correction_trigger(&app_handle).await;
                        });
                    }
                })
                .build(),
        )
        .setup(|app| {
            // Enable auto-start on reboot
            let _ = app.autolaunch().enable();

            // Load settings
            let settings = load_settings(&app.handle());
            log::info!(
                "[TextLint] Settings loaded. API key set: {}",
                !load_api_key().is_empty()
            );

            // Store settings in app state
            app.manage(SettingsState(Mutex::new(settings.clone())));

            // Set up system tray
            setup_tray(app)?;

            // Register the global shortcut keybinding
            let shortcut: Shortcut = settings.shortcut.parse().unwrap_or_else(|_| {
                "CmdOrCtrl+Alt+G"
                    .parse()
                    .expect("hardcoded fallback shortcut must be valid")
            });
            app.global_shortcut().register(shortcut)?;
            log::info!(
                "[TextLint] Global shortcut registered: {}",
                settings.shortcut
            );

            // Create the panel window (hidden initially)
            create_panel_window(app)?;

            log::info!("[TextLint] App started successfully. Waiting in system tray...");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            settings::get_settings,
            settings::save_settings,
            shortcuts::trigger_correction,
            shortcuts::apply_current_correction,
            app_filter::get_current_app,
            app_filter::get_running_apps,
            floating_panel::get_panel_position,
            floating_panel::show_panel_at_cursor,
            floating_panel::hide_panel,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Set up the system tray icon and menu
fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let settings_item = MenuItemBuilder::with_id("settings", "Settings").build(app)?;
    let learn_mode_item = {
        let state = app.state::<SettingsState>();
        let settings = match state.0.lock() {
            Ok(s) => s,
            Err(poisoned) => {
                log::error!("[TextLint] Settings Mutex poisoned in setup_tray, recovering");
                poisoned.into_inner()
            }
        };
        CheckMenuItemBuilder::with_id("learn_mode", "Learn Mode")
            .checked(settings.learn_mode)
            .build(app)?
    };
    let separator = tauri::menu::PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItemBuilder::with_id("quit", "Quit TextLint").build(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[&settings_item, &learn_mode_item, &separator, &quit_item])
        .build()?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("TextLint - Grammar Correction")
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "settings" => {
                open_settings_window(app);
            }
            "learn_mode" => {
                toggle_learn_mode(app);
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

/// Open the settings window
fn open_settings_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }

    let _settings_window = tauri::WebviewWindowBuilder::new(
        app,
        "settings",
        tauri::WebviewUrl::App("settings.html".into()),
    )
    .title("TextLint Settings")
    .inner_size(660.0, 720.0)
    .resizable(true)
    .center()
    .build();
}

/// Toggle Learn Mode from the tray menu
fn toggle_learn_mode(app: &tauri::AppHandle) {
    let state = app.state::<SettingsState>();
    let mut settings = match state.0.lock() {
        Ok(s) => s,
        Err(poisoned) => {
            log::error!("[TextLint] Settings Mutex poisoned in toggle_learn_mode, recovering");
            poisoned.into_inner()
        }
    };
    settings.learn_mode = !settings.learn_mode;
    let new_value = settings.learn_mode;
    let settings_clone = settings.clone();
    drop(settings);

    // Persist
    let _ = settings::save_settings_to_store(app, &settings_clone);

    // Notify frontend
    let _ = app.emit("learn-mode-changed", new_value);
    log::info!("[TextLint] Learn Mode toggled: {}", new_value);
}

/// Create the floating panel window (hidden initially)
fn create_panel_window(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let _panel_window =
        tauri::WebviewWindowBuilder::new(app, "panel", tauri::WebviewUrl::App("panel.html".into()))
            .title("")
            .inner_size(360.0, 240.0)
            .decorations(false)
            .always_on_top(true)
            .visible(false)
            .skip_taskbar(true)
            .build()?;

    Ok(())
}
