use crate::chords::shortcut::{press_shortcut, release_shortcut, Shortcut};
use crate::chords::ChordFolder;
use crate::input::Key;
use anyhow::Result;
use std::collections::HashMap;
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

impl LoadedAppChords {
    pub fn from_folder(chord_folder: ChordFolder) -> Result<Self> {
        // Loads all the chords from all included TOML files and parses them into struct Chord
        let mut global_chords = HashMap::new();
        let mut app_chords_files = HashMap::new();

        let mut chords_by_application_id = HashMap::new();
        for (file_path, file) in chord_folder.files_map {
            let Some(application_id) = Path::new(&file_path)
                .parent()
                .map(|p| p.to_string_lossy().replace("/", "."))
            else {
                continue;
            };

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
                }
            }

            for (sequence, chord) in chords {
                app_chords.insert(sequence.clone(), chord);
            }

            chords_by_application_id.insert(application_id.clone(), app_chords);
            app_chords_files.insert(application_id.clone(), file);
        }

        // Handle inheritance of chords by extending parent chords into children, if specified
        for (application_id, file) in &app_chords_files {
            if let Some(config) = &file.config {
                if let Some(ref parent_id) = config.extends {
                    let parent_chords_opt = chords_by_application_id.get(parent_id).cloned();
                    if let (Some(parent_chords), Some(child_chords)) = (
                        parent_chords_opt,
                        chords_by_application_id.get_mut(application_id),
                    ) {
                        // Insert parent chords only if not already present in child
                        for (chord_string, chord) in parent_chords {
                            child_chords
                                .entry(chord_string.clone())
                                .or_insert(chord.clone());
                        }
                    }
                }
            }
        }

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
