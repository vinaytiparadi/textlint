#[cfg(windows)]
use windows::Win32::Foundation::CloseHandle;
#[cfg(windows)]
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
#[cfg(windows)]
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

/// Get the process name of the currently focused (foreground) window.
/// Returns the lowercase executable name (e.g., "chrome.exe", "notepad.exe").
#[cfg(windows)]
pub fn get_foreground_app() -> Option<String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0 == std::ptr::null_mut() {
            return None;
        }

        let mut process_id: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));

        if process_id == 0 {
            return None;
        }

        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()?;

        let mut buffer = [0u16; 1024];
        let mut size = buffer.len() as u32;

        let success = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut size,
        );

        let _ = CloseHandle(handle);

        if success.is_ok() {
            let path = String::from_utf16_lossy(&buffer[..size as usize]);
            // Extract just the filename from the full path
            let filename = path.rsplit('\\').next().unwrap_or(&path).to_lowercase();
            Some(filename)
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
pub fn get_foreground_app() -> Option<String> {
    None
}

/// Check if the current foreground app is in the disabled list
pub fn is_app_disabled(disabled_apps: &[String]) -> bool {
    if disabled_apps.is_empty() {
        return false;
    }

    match get_foreground_app() {
        Some(app_name) => disabled_apps
            .iter()
            .any(|disabled| disabled.to_lowercase() == app_name),
        None => false,
    }
}

/// IPC command: Get the name of the currently focused application
#[tauri::command]
pub fn get_current_app() -> Option<String> {
    get_foreground_app()
}

/// Get a sorted, deduplicated list of running process names (exe filenames).
/// Filters out common system/background processes to keep the list relevant.
#[cfg(windows)]
fn enumerate_running_processes() -> Vec<String> {
    use std::collections::BTreeSet;

    // System processes to hide from the user list
    const HIDDEN_PROCESSES: &[&str] = &[
        "system",
        "system idle process",
        "registry",
        "smss.exe",
        "csrss.exe",
        "wininit.exe",
        "services.exe",
        "lsass.exe",
        "svchost.exe",
        "fontdrvhost.exe",
        "dwm.exe",
        "conhost.exe",
        "sihost.exe",
        "taskhostw.exe",
        "ctfmon.exe",
        "dllhost.exe",
        "runtimebroker.exe",
        "searchhost.exe",
        "startmenuexperiencehost.exe",
        "shellexperiencehost.exe",
        "textinputhost.exe",
        "widgetservice.exe",
        "securityhealthservice.exe",
        "securityhealthsystray.exe",
        "spoolsv.exe",
        "wudfhost.exe",
        "dashost.exe",
        "audiodg.exe",
        "searchindexer.exe",
        "searchprotocolhost.exe",
        "searchfilterhost.exe",
        "wmiprvse.exe",
        "sgrmbroker.exe",
        "memorystatus.exe",
        "applicationframehost.exe",
        "systemsettings.exe",
        "lockapp.exe",
        "crashpad_handler.exe",
        "backgroundtaskhost.exe",
        "gamebarpresencewriter.exe",
        "textlint.exe",
    ];

    let mut apps = BTreeSet::new();

    unsafe {
        let snapshot = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            Ok(h) => h,
            Err(_) => return Vec::new(),
        };

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let name_len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..name_len]).to_lowercase();

                if !name.is_empty() && !HIDDEN_PROCESSES.contains(&name.as_str()) {
                    apps.insert(name);
                }

                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
    }

    apps.into_iter().collect()
}

#[cfg(not(windows))]
fn enumerate_running_processes() -> Vec<String> {
    Vec::new()
}

/// IPC command: Get list of running applications
#[tauri::command]
pub fn get_running_apps() -> Vec<String> {
    enumerate_running_processes()
}
