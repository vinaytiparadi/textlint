use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const STORE_FILENAME: &str = "settings.json";

/// All user-configurable settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Gemini API key
    pub api_key: String,
    /// Keyboard shortcut string (e.g., "CmdOrCtrl+Alt+G")
    pub shortcut: String,
    /// Whether Learn Mode is on (shows expanded panel instead of auto-paste)
    pub learn_mode: bool,
    /// Whether the floating icon is shown
    pub show_floating_icon: bool,
    /// Whether to auto-apply corrections when Learn Mode is off
    pub auto_apply: bool,
    /// Correction strictness level
    pub strictness: Strictness,
    /// Whether to also enhance writing quality (word choice, clarity, flow)
    pub enhance_writing: bool,
    /// Toast notification duration in seconds
    pub toast_duration: u32,
    /// Theme preference
    pub theme: Theme,
    /// Launch on Windows startup
    pub launch_on_startup: bool,
    /// List of disabled app process names (lowercase)
    pub disabled_apps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Strictness {
    Relaxed,
    Balanced,
    Strict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
    System,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            api_key: String::new(),
            shortcut: "CmdOrCtrl+Alt+G".to_string(),
            learn_mode: false,
            show_floating_icon: true,
            auto_apply: true,
            strictness: Strictness::Balanced,
            enhance_writing: false,
            toast_duration: 3,
            theme: Theme::System,
            launch_on_startup: true,
            disabled_apps: Vec::new(),
        }
    }
}

/// Thread-safe settings state
pub struct SettingsState(pub Mutex<AppSettings>);

/// Load settings from the Tauri store, or return defaults
pub fn load_settings(app: &AppHandle) -> AppSettings {
    let store = match app.store(STORE_FILENAME) {
        Ok(s) => s,
        Err(_) => return AppSettings::default(),
    };

    match store.get("settings") {
        Some(value) => serde_json::from_value(value.clone()).unwrap_or_default(),
        None => AppSettings::default(),
    }
}

/// Save settings to the Tauri store
pub fn save_settings_to_store(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let store = app.store(STORE_FILENAME).map_err(|e| e.to_string())?;
    let value = serde_json::to_value(settings).map_err(|e| e.to_string())?;
    store.set("settings", value);
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

/// IPC command: get current settings
#[tauri::command]
pub fn get_settings(state: tauri::State<'_, SettingsState>) -> AppSettings {
    state.0.lock().unwrap().clone()
}

/// IPC command: save settings
#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: tauri::State<'_, SettingsState>,
    settings: AppSettings,
) -> Result<(), String> {
    save_settings_to_store(&app, &settings)?;
    let mut current = state.0.lock().unwrap();
    *current = settings;
    Ok(())
}
