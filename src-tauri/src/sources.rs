use crate::chords::{ChordFolder, LoadedAppChords};
use crate::git;
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_store::{Store, StoreExt};

const CHORD_SOURCES_STORE_PATH: &str = "chord-sources.json";
const LOCAL_FOLDERS_KEY: &str = "localFolders";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalChordFolderInfo {
    pub name: String,
    pub local_path: String,
}

fn sources_store(app: &AppHandle) -> Result<Arc<Store<tauri::Wry>>> {
    app.store(CHORD_SOURCES_STORE_PATH)
        .context("failed to open chord sources store")
}

fn read_local_folder_paths(app: &AppHandle) -> Result<Vec<String>> {
    let store = sources_store(app)?;
    let Some(value) = store.get(LOCAL_FOLDERS_KEY) else {
        return Ok(Vec::new());
    };

    serde_json::from_value(value).context("failed to parse local chord folder list")
}

fn write_local_folder_paths(app: &AppHandle, paths: &[String]) -> Result<()> {
    let store = sources_store(app)?;
    let value =
        serde_json::to_value(paths).context("failed to serialize local chord folder list")?;
    store.set(LOCAL_FOLDERS_KEY, value);
    Ok(())
}

fn local_folder_info_from_path(path: PathBuf) -> LocalChordFolderInfo {
    let display_path = path.display().to_string();
    let name = path
        .file_name()
        .and_then(|segment| segment.to_str())
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_string())
        .unwrap_or_else(|| display_path.clone());

    LocalChordFolderInfo {
        name,
        local_path: display_path,
    }
}

fn canonicalize_local_folder_path(path: &str) -> Result<PathBuf> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        anyhow::bail!("Folder path cannot be empty");
    }

    let canonical_path =
        std::fs::canonicalize(trimmed).context(format!("failed to access folder {trimmed}"))?;
    if !canonical_path.is_dir() {
        anyhow::bail!("{trimmed} is not a folder");
    }

    Ok(canonical_path)
}

fn load_local_chord_folder(folder_path: &str) -> Result<ChordFolder> {
    let canonical_path = canonicalize_local_folder_path(folder_path)?;
    ChordFolder::load_from_local_folder(&canonical_path)
}

pub fn list_local_chord_folders(app: AppHandle) -> Result<Vec<LocalChordFolderInfo>> {
    let mut folders = read_local_folder_paths(&app)?
        .into_iter()
        .map(PathBuf::from)
        .map(local_folder_info_from_path)
        .collect::<Vec<_>>();

    folders.sort_by(|left, right| left.local_path.cmp(&right.local_path));
    Ok(folders)
}

pub fn pick_local_chord_folder(app: AppHandle) -> Result<Option<String>> {
    Ok(app
        .dialog()
        .file()
        .set_title("Select Local Chord Folder")
        .blocking_pick_folder()
        .and_then(|folder_path| folder_path.into_path().ok())
        .map(|folder_path| folder_path.display().to_string()))
}

pub fn add_local_chord_folder(app: AppHandle, folder_path: &str) -> Result<LocalChordFolderInfo> {
    let canonical_path = canonicalize_local_folder_path(folder_path)?;
    let canonical_path_string = canonical_path.display().to_string();
    let mut local_folder_paths = read_local_folder_paths(&app)?;

    if !local_folder_paths.contains(&canonical_path_string) {
        local_folder_paths.push(canonical_path_string);
        local_folder_paths.sort();
        write_local_folder_paths(&app, &local_folder_paths)?;
    }

    Ok(local_folder_info_from_path(canonical_path))
}

pub fn load_local_chord_folder_chords(
    _app: AppHandle,
    folder_path: &str,
) -> Result<LoadedAppChords> {
    let chord_folder = load_local_chord_folder(folder_path)?;
    LoadedAppChords::from_folders(vec![chord_folder])
}

pub fn load_all_chord_folders(app: AppHandle) -> Result<Vec<ChordFolder>> {
    let mut chord_folders = vec![ChordFolder::load_bundled()?];
    chord_folders.extend(git::load_all_chord_folders(app.clone())?);

    for folder in list_local_chord_folders(app)? {
        match ChordFolder::load_from_local_folder(Path::new(&folder.local_path)) {
            Ok(local_folder) => chord_folders.push(local_folder),
            Err(error) => {
                log::warn!("Skipping local folder {}: {error}", folder.local_path);
            }
        }
    }

    Ok(chord_folders)
}
