use crate::git::{add_git_repo, discover_git_repos, load_repo_chords, sync_git_repo, GitRepoInfo};
use crate::sources::{
    add_local_chord_folder, list_local_chord_folders, load_local_chord_folder_chords,
    pick_local_chord_folder, LocalChordFolderInfo,
};
use crate::tauri_app::context::{
    list_active_chords, list_loaded_chords, reload_loaded_app_chords, ActiveChordInfo,
};
use crate::tauri_app::store::GlobalHotkeyStore;
use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalShortcutMappingInfo {
    pub shortcut: String,
    pub bundle_id: String,
    pub hotkey_id: String,
}

fn global_hotkeys_store(app: &AppHandle) -> Result<GlobalHotkeyStore, String> {
    app.store("global-hotkeys.json")
        .map(GlobalHotkeyStore::new)
        .map_err(|error| format!("failed to open global hotkeys store: {error}"))
}

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
    discover_git_repos(app).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn add_git_repo_command(app: AppHandle, repo: String) -> Result<GitRepoInfo, String> {
    let repo_info = add_git_repo(app.clone(), &repo).map_err(|error| error.to_string())?;
    reload_loaded_app_chords(app)
        .await
        .map_err(|error| error.to_string())?;
    Ok(repo_info)
}

#[tauri::command]
pub async fn sync_git_repo_command(app: AppHandle, repo: String) -> Result<GitRepoInfo, String> {
    let repo_info = sync_git_repo(app.clone(), &repo).map_err(|error| error.to_string())?;
    reload_loaded_app_chords(app)
        .await
        .map_err(|error| error.to_string())?;
    Ok(repo_info)
}

#[tauri::command]
pub fn list_local_chord_folders_command(
    app: AppHandle,
) -> Result<Vec<LocalChordFolderInfo>, String> {
    list_local_chord_folders(app).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn pick_local_chord_folder_command(app: AppHandle) -> Result<Option<String>, String> {
    pick_local_chord_folder(app).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn add_local_chord_folder_command(
    app: AppHandle,
    path: String,
) -> Result<LocalChordFolderInfo, String> {
    let folder_info =
        add_local_chord_folder(app.clone(), &path).map_err(|error| error.to_string())?;
    reload_loaded_app_chords(app)
        .await
        .map_err(|error| error.to_string())?;
    Ok(folder_info)
}

#[tauri::command]
pub fn list_active_chords_command(app: AppHandle) -> Result<Vec<ActiveChordInfo>, String> {
    list_active_chords(app).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_repo_chords_command(
    app: AppHandle,
    repo: String,
) -> Result<Vec<ActiveChordInfo>, String> {
    let loaded_chords = load_repo_chords(app, &repo).map_err(|error| error.to_string())?;
    Ok(list_loaded_chords(&loaded_chords))
}

#[tauri::command]
pub fn list_local_chord_folder_chords_command(
    app: AppHandle,
    path: String,
) -> Result<Vec<ActiveChordInfo>, String> {
    let loaded_chords =
        load_local_chord_folder_chords(app, &path).map_err(|error| error.to_string())?;
    Ok(list_loaded_chords(&loaded_chords))
}

#[tauri::command]
pub fn list_global_shortcut_mappings_command(
    app: AppHandle,
) -> Result<Vec<GlobalShortcutMappingInfo>, String> {
    let store = global_hotkeys_store(&app)?;
    let mut mappings = store
        .entries()
        .into_iter()
        .map(|(shortcut, entry)| GlobalShortcutMappingInfo {
            shortcut,
            bundle_id: entry.bundle_id,
            hotkey_id: entry.hotkey_id,
        })
        .collect::<Vec<_>>();

    mappings.sort_by(|left, right| {
        left.bundle_id
            .cmp(&right.bundle_id)
            .then(left.hotkey_id.cmp(&right.hotkey_id))
            .then(left.shortcut.cmp(&right.shortcut))
    });

    Ok(mappings)
}

#[tauri::command]
pub fn remove_global_shortcut_mapping_command(
    app: AppHandle,
    shortcut: String,
) -> Result<(), String> {
    let trimmed_shortcut = shortcut.trim();
    if trimmed_shortcut.is_empty() {
        return Err("shortcut cannot be empty".to_string());
    }

    let store = global_hotkeys_store(&app)?;
    store.remove(trimmed_shortcut);
    Ok(())
}
