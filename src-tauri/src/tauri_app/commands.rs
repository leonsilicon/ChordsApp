fn open_system_settings(url: &str, permission_name: &str) {
    if let Err(error) = std::process::Command::new("open").arg(url).spawn() {
        log::error!("Failed to open {permission_name} settings: {error}");
    }
}

#[tauri::command]
pub fn open_accessibility_settings() {
    open_system_settings(
        "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
        "accessibility",
    );
}

#[tauri::command]
pub fn open_input_monitoring_settings() {
    open_system_settings(
        "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent",
        "input monitoring",
    );
}
