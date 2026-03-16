use crate::tauri_app::context::{add_git_repo, discover_git_repos, sync_git_repo, GitRepoInfo};
use tauri::AppHandle;

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

#[tauri::command]
pub fn list_git_repos(app: AppHandle) -> Result<Vec<GitRepoInfo>, String> {
    discover_git_repos(&app).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn add_git_repo_command(app: AppHandle, repo: String) -> Result<GitRepoInfo, String> {
    add_git_repo(&app, &repo).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn sync_git_repo_command(app: AppHandle, repo: String) -> Result<GitRepoInfo, String> {
    sync_git_repo(&app, &repo).map_err(|error| error.to_string())
}
