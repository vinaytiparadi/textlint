#[cfg(windows)]
use windows::Win32::Foundation::CloseHandle;
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
