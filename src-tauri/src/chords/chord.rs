use crate::chords::shortcut::{press_shortcut, release_shortcut, Shortcut};
use crate::chords::{AppChordsFile, ChordFolder};
use crate::input::Key;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;
use tauri::AppHandle;

#[derive(Debug, Clone)]
pub struct Chord {
    pub keys: Vec<Key>,
    pub name: String,
    pub command: Option<String>,
    pub shortcut: Option<Shortcut>,
    pub shell: Option<String>,
}

pub struct LoadedAppChords {
    pub global_chords: ChordMap,
    pub app_specific_chords: HashMap<String, ChordMap>,
}

pub type ChordMap = HashMap<Vec<Key>, Chord>;

fn application_id_from_chords_path(file_path: &Path) -> Option<String> {
    let application_path = file_path.parent()?;
    if application_path.as_os_str().is_empty() {
        return None;
    }

    Some(
        application_path
            .iter()
            .map(|component| component.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("."),
    )
}

impl LoadedAppChords {
    pub fn from_folder(chord_folder: ChordFolder) -> Result<Self> {
        // Loads all the chords from all included TOML files and parses them into struct Chord
        let mut global_chords = HashMap::new();
        let mut app_chords_files = HashMap::new();
        let mut direct_app_chords = HashMap::new();
        for (file_path, file) in chord_folder.files_map {
            let application_id = application_id_from_chords_path(Path::new(&file_path));

            log::debug!("Loading chords from file {:?}", file_path);

            let mut app_chords = HashMap::new();

            let chords = match file.get_chord_map() {
                Ok(chords) => chords,
                Err(error) => {
                    log::warn!("File {:?} contains invalid chords: {:?}", file, error);
                    continue;
                }
            };

            // For each global chord, insert it into the global_chords map
            for (sequence, chord) in &chords {
                if sequence
                    .first()
                    .is_some_and(|c| !c.is_digit() && !c.is_letter())
                {
                    global_chords.insert(sequence.clone(), chord.clone());
                } else {
                    app_chords.insert(sequence.clone(), chord.clone());
                }
            }

            if let Some(application_id) = application_id {
                direct_app_chords.insert(application_id.clone(), app_chords);
                app_chords_files.insert(application_id, file);
            }
        }

        let mut resolved_chords = HashMap::new();
        for application_id in app_chords_files.keys() {
            resolve_app_chords(
                application_id,
                &app_chords_files,
                &direct_app_chords,
                &mut resolved_chords,
                &mut HashSet::new(),
            );
        }

        let chords_by_application_id = resolved_chords
            .into_iter()
            .filter(|(_, chords)| !chords.is_empty())
            .collect();

        Ok(LoadedAppChords {
            global_chords,
            app_specific_chords: chords_by_application_id,
        })
    }

    // No application = global chord
    pub fn get_chord(&self, sequence: &[Key], application_id: Option<String>) -> Option<Chord> {
        // Prefer app chord, fall back to global
        let chord = if let Some(app_id) = application_id {
            self.app_specific_chords
                .get(&app_id)
                .and_then(|app_chord_map| app_chord_map.get(sequence))
                .or_else(|| self.global_chords.get(sequence))
        } else {
            self.global_chords.get(sequence)
        };

        chord.cloned()
    }
}

pub fn press_chord(handle: AppHandle, chord: &Chord) -> anyhow::Result<()> {
    let shortcut = chord.shortcut.clone();
    let shell = chord.shell.clone();
    handle.clone().run_on_main_thread(move || {
        if let Some(shell) = shell {
            std::thread::spawn(move || {
                let mut command = Command::new("sh");
                command.arg("-c").arg(&shell);
                log::debug!("Running shell command: {:?}", command);

                match command.output() {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                        let exit_code = output.status.code();

                        if output.status.success() {
                            log::debug!(
                                "shell command succeeded with exit code {:?}: {}",
                                exit_code,
                                shell
                            );
                        } else {
                            log::error!(
                                "shell command failed with exit code {:?}: {}",
                                exit_code,
                                shell
                            );
                        }

                        if !stdout.is_empty() {
                            log::debug!("shell stdout: {stdout}");
                        }

                        if !stderr.is_empty() {
                            log::debug!("shell stderr: {stderr}");
                        }
                    }
                    Err(e) => {
                        log::error!("failed to run shell command `{shell}`: {e}");
                    }
                }
            });
        } else {
            if let Some(shortcut) = shortcut {
                if let Err(e) = press_shortcut(shortcut.clone()) {
                    log::error!("failed to press shortcut: {e}");
                }
            } else {
                log::error!("no shortcut to execute");
            }
        }
    })?;

    Ok(())
}

fn resolve_app_chords(
    application_id: &str,
    app_chords_files: &HashMap<String, AppChordsFile>,
    direct_app_chords: &HashMap<String, ChordMap>,
    resolved_chords: &mut HashMap<String, ChordMap>,
    visiting: &mut HashSet<String>,
) -> ChordMap {
    if let Some(chords) = resolved_chords.get(application_id) {
        return chords.clone();
    }

    if !visiting.insert(application_id.to_string()) {
        log::warn!(
            "Detected circular _config.extends chain while loading chords for {}",
            application_id
        );
        return direct_app_chords
            .get(application_id)
            .cloned()
            .unwrap_or_default();
    }

    let mut merged_chords = HashMap::new();

    if let Some(parent_id) = app_chords_files
        .get(application_id)
        .and_then(|file| file.config.as_ref())
        .and_then(|config| config.extends.as_deref())
    {
        if app_chords_files.contains_key(parent_id) {
            merged_chords = resolve_app_chords(
                parent_id,
                app_chords_files,
                direct_app_chords,
                resolved_chords,
                visiting,
            );
        } else {
            log::warn!(
                "Application {} extends missing parent {}",
                application_id,
                parent_id
            );
        }
    }

    if let Some(child_chords) = direct_app_chords.get(application_id) {
        for (sequence, chord) in child_chords {
            merged_chords.insert(sequence.clone(), chord.clone());
        }
    }

    visiting.remove(application_id);
    resolved_chords.insert(application_id.to_string(), merged_chords.clone());
    merged_chords
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chords::AppChordsFile;
    use std::collections::HashMap;

    #[test]
    fn derives_application_id_from_app_path() {
        let application_id =
            application_id_from_chords_path(Path::new("com/apple/finder/chords.toml"));

        assert_eq!(application_id.as_deref(), Some("com.apple.finder"));
    }

    #[test]
    fn global_chords_file_has_no_application_id() {
        let application_id = application_id_from_chords_path(Path::new("chords.toml"));

        assert_eq!(application_id, None);
    }

    #[test]
    fn extends_only_child_inherits_parent_chords() {
        let chord_folder = ChordFolder {
            files_map: HashMap::from([
                (
                    "com/jetbrains/intellij/chords.toml".to_string(),
                    AppChordsFile::parse(
                        r#"
                        a = { name = "Parent chord" }
                        "#,
                    )
                    .unwrap(),
                ),
                (
                    "com/jetbrains/rubymine/chords.toml".to_string(),
                    AppChordsFile::parse(
                        r#"
                        _config = { extends = "com.jetbrains.intellij" }
                        "#,
                    )
                    .unwrap(),
                ),
            ]),
        };

        let loaded = LoadedAppChords::from_folder(chord_folder).unwrap();
        let inherited = loaded.get_chord(
            &Key::parse_sequence("a").unwrap(),
            Some("com.jetbrains.rubymine".to_string()),
        );

        assert!(inherited.is_some());
        assert!(loaded
            .app_specific_chords
            .contains_key("com.jetbrains.rubymine"));
    }

    #[test]
    fn chained_extends_inherits_through_intermediate_app() {
        let chord_folder = ChordFolder {
            files_map: HashMap::from([
                (
                    "com/jetbrains/intellij/chords.toml".to_string(),
                    AppChordsFile::parse(
                        r#"
                        a = { name = "Grandparent chord" }
                        "#,
                    )
                    .unwrap(),
                ),
                (
                    "com/jetbrains/idea/chords.toml".to_string(),
                    AppChordsFile::parse(
                        r#"
                        _config = { extends = "com.jetbrains.intellij" }
                        "#,
                    )
                    .unwrap(),
                ),
                (
                    "com/jetbrains/rubymine/chords.toml".to_string(),
                    AppChordsFile::parse(
                        r#"
                        _config = { extends = "com.jetbrains.idea" }
                        "#,
                    )
                    .unwrap(),
                ),
            ]),
        };

        let loaded = LoadedAppChords::from_folder(chord_folder).unwrap();
        let inherited = loaded.get_chord(
            &Key::parse_sequence("a").unwrap(),
            Some("com.jetbrains.rubymine".to_string()),
        );

        assert!(inherited.is_some());
    }
}

pub fn release_chord(handle: AppHandle, chord: &Chord) -> anyhow::Result<()> {
    let shortcut = chord.shortcut.clone();
    handle.clone().run_on_main_thread(move || {
        if let Some(shortcut) = shortcut {
            if let Err(e) = release_shortcut(shortcut.clone()) {
                log::error!("failed to release shortcut: {e}");
            }
        }
    })?;

    Ok(())
}
