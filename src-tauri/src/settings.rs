use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const STORE_FILENAME: &str = "settings.json";
const KEYRING_SERVICE: &str = "textlint";
const KEYRING_USER: &str = "gemini-api-key";

/// All user-configurable settings (api_key is NOT stored here — it lives in the OS keyring)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
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

/// Settings response that includes the api_key fetched separately from the keyring.
/// Used only for IPC return values — never persisted to disk as a whole.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettingsWithKey {
    #[serde(flatten)]
    pub settings: AppSettings,
    pub api_key: String,
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

// ===========================
// Keyring helpers
// ===========================

/// Load the Gemini API key from the OS keyring (Windows Credential Manager).
pub fn load_api_key() -> String {
    match keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        Ok(entry) => match entry.get_password() {
            Ok(key) => {
                eprintln!(
                    "[TextLint] API key loaded from keyring ({} chars)",
                    key.len()
                );
                key
            }
            Err(keyring::Error::NoEntry) => {
                eprintln!("[TextLint] No API key found in keyring");
                String::new()
            }
            Err(e) => {
                eprintln!("[TextLint] keyring get_password error: {}", e);
                String::new()
            }
        },
        Err(e) => {
            eprintln!("[TextLint] keyring Entry::new error (load): {}", e);
            String::new()
        }
    }
}

/// Save the Gemini API key to the OS keyring.
/// If `key` is empty, the credential is deleted.
pub fn save_api_key(key: &str) -> Result<(), String> {
    eprintln!("[TextLint] save_api_key called ({} chars)", key.len());
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(|e| {
        eprintln!("[TextLint] keyring Entry::new error (save): {}", e);
        e.to_string()
    })?;

    if key.is_empty() {
        let _ = entry.delete_password();
        eprintln!("[TextLint] API key deleted from keyring");
        return Ok(());
    }

    entry.set_password(key).map_err(|e| {
        eprintln!("[TextLint] keyring set_password error: {}", e);
        e.to_string()
    })?;
    eprintln!("[TextLint] API key saved to keyring successfully");
    Ok(())
}

// ===========================
// Settings load / save
// ===========================

/// Load settings from the Tauri store, or return defaults.
/// Also performs a one-time migration of any api_key found in the old JSON store to the keyring.
pub fn load_settings(app: &AppHandle) -> AppSettings {
    let store = match app.store(STORE_FILENAME) {
        Ok(s) => s,
        Err(_) => return AppSettings::default(),
    };

    // --- One-time migration: move api_key from JSON to keyring ---
    if let Some(raw) = store.get("settings") {
        if let Some(old_key) = raw.get("api_key").and_then(|v| v.as_str()) {
            if !old_key.is_empty() {
                log::info!("[TextLint] Migrating API key from JSON store to OS keyring");
                if let Err(e) = save_api_key(old_key) {
                    log::error!("[TextLint] Failed to migrate API key to keyring: {}", e);
                }
                // Re-save settings without the api_key field
                if let Ok(mut obj) = serde_json::from_value::<
                    serde_json::Map<String, serde_json::Value>,
                >(raw.clone())
                {
                    obj.remove("api_key");
                    store.set("settings", serde_json::Value::Object(obj));
                    let _ = store.save();
                }
            }
        }
    }
    // --- End migration ---

    match store.get("settings") {
        Some(value) => serde_json::from_value(value.clone()).unwrap_or_default(),
        None => AppSettings::default(),
    }
}

/// Save settings to the Tauri store (api_key is NOT included).
pub fn save_settings_to_store(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let store = app.store(STORE_FILENAME).map_err(|e| e.to_string())?;
    let value = serde_json::to_value(settings).map_err(|e| e.to_string())?;
    store.set("settings", value);
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

// ===========================
// IPC commands
// ===========================

/// IPC command: get current settings + api_key from keyring
#[tauri::command]
pub fn get_settings(state: tauri::State<'_, SettingsState>) -> AppSettingsWithKey {
    let settings = state.0.lock().unwrap().clone();
    let api_key = load_api_key();
    AppSettingsWithKey { settings, api_key }
}

/// IPC command: save settings + api_key to keyring
#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: tauri::State<'_, SettingsState>,
    settings: AppSettings,
    api_key: String,
) -> Result<(), String> {
    save_api_key(&api_key)?;
    save_settings_to_store(&app, &settings)?;
    let mut current = state.0.lock().unwrap();
    *current = settings;
    Ok(())
}
