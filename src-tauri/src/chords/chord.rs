use crate::chords::shortcut::{press_shortcut, release_shortcut, Shortcut};
use crate::chords::{AppChordsFile, AppChordsFileConfig, AppChordsFileConfigLua, ChordFolder, ChordLuaRuntime};
use crate::input::Key;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;
use tauri::AppHandle;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Chord {
    pub keys: Vec<Key>,
    pub name: String,
    pub command: Option<String>,
    pub shortcut: Option<Shortcut>,
    pub shell: Option<String>,
    pub lua: Option<String>
}

pub struct LoadedAppChords {
    pub global_runtime: ChordRuntime,
    pub app_runtime_map: HashMap<String, ChordRuntime>,
}

pub struct ChordRuntime {
    pub chords: HashMap<Vec<Key>, Chord>,
    pub lua_runtime: Option<Arc<ChordLuaRuntime>>
}

impl ChordRuntime {
    pub fn from_chords_no_lua(chords: HashMap<Vec<Key>, Chord>) -> Self {
        Self {
            chords,
            lua_runtime: None,
        }
    }

    // Doesn't resolve _config.extends
    pub fn from_file_shallow(chord_file: AppChordsFile) -> Result<Self> {
        let mut chords = chord_file.get_chords_shallow()?;
        // Filters out global chords
        chords.retain(|sequence, _| {
            sequence
                .first()
                .is_some_and(|c| c.is_digit() || c.is_letter())
        });

        // TODO: lua runtime should be none if no _config.lua
        let lua_init_scripts = chord_file
            .config
            .as_ref()
            .and_then(|AppChordsFileConfig { lua, .. }| lua.as_ref())
            .and_then(|lua_config| lua_config.init.clone())
            .into_iter()
            .collect::<Vec<_>>();
        let lua_runtime = ChordLuaRuntime::new(lua_init_scripts)?;

        Ok(Self { chords, lua_runtime: Some(Arc::new(lua_runtime)) })
    }

    pub fn extend_runtime(&mut self, base: &Self) -> Result<()> {
        for (sequence, chord) in &base.chords {
            self.chords
                .entry(sequence.clone())
                .or_insert_with(|| chord.clone());
        }

        let mut lua_init_scripts = base
            .lua_runtime
            .as_ref()
            .map(|r| r.lua_init_scripts.clone())
            .unwrap_or_default();

        if let Some(r) = &self.lua_runtime {
            lua_init_scripts.extend(r.lua_init_scripts.clone());
        }

        self.lua_runtime = Some(Arc::new(ChordLuaRuntime::new(lua_init_scripts.clone())?));

        Ok(())
    }

    pub fn get_chord(&self, sequence: &[Key]) -> Option<&Chord> {
        self.chords.get(sequence)
    }
}

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

fn resolve_runtime_extends(
    application_id: &str,
    app_runtime_map: &mut HashMap<String, ChordRuntime>,
    app_config_map: &HashMap<String, Option<AppChordsFileConfig>>,
    resolved: &mut HashSet<String>,
    resolving: &mut HashSet<String>,
) -> Result<()> {
    if resolved.contains(application_id) {
        return Ok(());
    }

    if !resolving.insert(application_id.to_string()) {
        log::warn!("Circular extends detected for application ID: {application_id}");
        return Ok(());
    }

    let extends = app_config_map
        .get(application_id)
        .and_then(|config| config.as_ref())
        .and_then(|config| config.extends.clone());

    if let Some(base_application_id) = extends {
        if app_runtime_map.contains_key(&base_application_id) {
            resolve_runtime_extends(
                &base_application_id,
                app_runtime_map,
                app_config_map,
                resolved,
                resolving,
            )?;

            let Some(mut app_runtime) = app_runtime_map.remove(application_id) else {
                resolving.remove(application_id);
                return Ok(());
            };

            if let Some(base_runtime) = app_runtime_map.get(&base_application_id) {
                app_runtime.extend_runtime(base_runtime)?;
            }

            app_runtime_map.insert(application_id.to_string(), app_runtime);
        } else {
            log::warn!(
                "Invalid extends for application ID {application_id}: {base_application_id}"
            );
        }
    }

    resolving.remove(application_id);
    resolved.insert(application_id.to_string());

    Ok(())
}

impl LoadedAppChords {
    pub fn from_folder(chord_folder: ChordFolder) -> Result<Self> {
        let mut global_chords = HashMap::new();
        let mut app_runtime_map = HashMap::new();
        let mut app_config_map = HashMap::new();

        for (file_path, file) in chord_folder.files_map {
            let Some(application_id) = application_id_from_chords_path(Path::new(&file_path)) else {
                log::error!("Invalid application ID for file {:?}", file_path);
                continue;
            };

            // Loading global chords into `global_chords`
            let chords = file.get_chords_shallow()?;
            for (sequence, chord) in &chords {
                if sequence.first().is_some_and(|c| !c.is_digit() && !c.is_letter()) {
                    global_chords.insert(sequence.clone(), chord.clone());
                }
            }

            let config = file.config.clone();
            let app_chord_runtime = ChordRuntime::from_file_shallow(file)?;
            app_runtime_map.insert(application_id.clone(), app_chord_runtime);
            app_config_map.insert(application_id, config);
        }

        let application_ids = app_runtime_map.keys().cloned().collect::<Vec<_>>();
        let mut resolved = HashSet::new();
        let mut resolving = HashSet::new();

        for application_id in application_ids {
            resolve_runtime_extends(
                &application_id,
                &mut app_runtime_map,
                &app_config_map,
                &mut resolved,
                &mut resolving,
            )?;
        }

        Ok(LoadedAppChords {
            global_runtime: ChordRuntime::from_chords_no_lua(global_chords),
            app_runtime_map,
        })
    }

    // No application = global chord
    pub fn get_chord_runtime(&self, sequence: &[Key], application_id: Option<String>) -> &ChordRuntime {
        if sequence.first().is_some_and(|c| !c.is_digit() && !c.is_letter()) {
            return &self.global_runtime;
        }

        let chord_runtime = if let Some(app_id) = application_id {
            self.app_runtime_map
                .get(&app_id).unwrap_or(&self.global_runtime)
        } else {
            &self.global_runtime
        };

        chord_runtime
    }
}

pub fn press_chord(handle: AppHandle, runtime: &ChordRuntime, chord: &Chord) -> Result<()> {
    let shortcut = chord.shortcut.clone();
    let shell = chord.shell.clone();
    let lua = chord.lua.clone();
    let lua_runtime = runtime.lua_runtime.clone();
    handle.clone().run_on_main_thread(move || {
        // Prioritize shortcuts
        if let Some(shortcut) = shortcut {
            if let Err(e) = press_shortcut(shortcut.clone()) {
                log::error!("failed to press shortcut: {e}");
            }
        } else if let Some(shell) = shell {
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
        } else if let Some(lua) = lua {
            let Some(lua_runtime) = lua_runtime.as_ref() else {
                log::error!("missing lua runtime");
                return;
            };
            if let Err(e) = lua_runtime.lua.load(lua).exec() {
                log::error!("failed to execute lua: {e}");
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
