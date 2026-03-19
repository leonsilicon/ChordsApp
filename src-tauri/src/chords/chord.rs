use crate::chords::shortcut::{press_shortcut, release_shortcut, Shortcut};
use crate::chords::{AppChordMapValue, AppChordsFile, AppChordsFileConfig, ChordFolder};
use crate::input::Key;
use anyhow::Result;
use rquickjs::{Context, Ctx, Function, Module, Object};
use rquickjs::runtime::{Runtime, UserDataError};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use gix::url::expand_path::with;
use tauri::AppHandle;
use crate::js::with_js;

#[derive(Debug, Clone)]
pub struct Chord {
    pub keys: Vec<Key>,
    pub name: String,
    pub shortcut: Option<Shortcut>,
    pub shell: Option<String>,
    pub js: Option<String>,
}

pub struct LoadedAppChords {
    pub global_chords_to_runtime_key: HashMap<Vec<Key>, String>,
    pub runtimes: HashMap<String, ChordRuntime>,
}

pub struct ChordRuntime {
    pub chords: HashMap<Vec<Key>, Chord>,
    // Needs to be an Arc so that the Lua runtime can access its latest value
    pub raw_chords: Arc<Mutex<HashMap<String, AppChordMapValue>>>,
    pub config: Option<AppChordsFileConfig>,
}

impl ChordRuntime {
    pub fn from_chords(chords: HashMap<Vec<Key>, Chord>) -> Result<Self> {
        let raw_chords = Arc::new(Mutex::new(HashMap::new()));
        let js_runtime = Runtime::new()?;
        Ok(Self {
            chords,
            raw_chords,
            config: None,
        })
    }

    // Doesn't resolve _config.extends
    pub fn from_file_shallow(chord_file: AppChordsFile) -> Result<Self> {
        let raw_chords = Arc::new(Mutex::new(chord_file.chords.clone()));
        let config = chord_file.config.clone();
        // We intentionally keep in global chords because they execute in this runtime
        let chords = chord_file.get_chords_shallow();

        let js_runtime = Runtime::new()?;
        let runtime = Self {
            raw_chords,
            config,
            chords,
        };

        let raw_chords = runtime.raw_chords.clone();

        Ok(runtime)
    }

    // We intentionally don't extend Lua init scripts so `chords.toml` can be better audited
    pub fn extend_runtime(&mut self, base: &Self) -> Result<()> {
        for (sequence, chord) in &base.chords {
            self.chords
                .entry(sequence.clone())
                .or_insert_with(|| chord.clone());
        }

        let mut raw_chords = self.raw_chords.lock().expect("poisoned lock");
        let base_raw_chords = base.raw_chords.lock().expect("poisoned lock");
        for (sequence, chord) in base_raw_chords.iter() {
            raw_chords
                .entry(sequence.clone())
                .or_insert_with(|| chord.clone());
        }

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
    pub fn from_folders(chord_folders: Vec<ChordFolder>) -> Result<Self> {
        let mut global_chords_to_runtime_key = HashMap::new();
        let mut app_runtime_map = HashMap::new();
        let mut app_config_map = HashMap::new();


        for chord_folder in chord_folders {
            log::debug!("Loading folder from root {:?}", chord_folder.root_dir);

            for (chord_file_path, file) in chord_folder.chords_files {
                let Some(application_id) = application_id_from_chords_path(Path::new(&chord_file_path))
                else {
                    log::warn!("Invalid chords path: {:?}", chord_file_path);
                    continue;
                };

                // Loading global chords into `global_chords`
                let chords = file.get_chords_shallow();
                for (sequence, chord) in &chords {
                    if sequence
                        .first()
                        .is_some_and(|c| !c.is_digit() && !c.is_letter())
                    {
                        log::debug!("Adding global chord for sequence: {:?}", sequence);
                        global_chords_to_runtime_key.insert(sequence.clone(), application_id.clone());
                    }
                }

                let config = file.config.clone();
                let app_chord_runtime = ChordRuntime::from_file_shallow(file)?;

                // Load all JS files as modules
                let js_files = &chord_folder.js_files;
                with_js(|ctx| -> Result<()> {
                    for (filepath, js) in js_files.iter() {
                        Module::declare(ctx.clone(), filepath, js.as_str())?;
                    }

                    Ok(())
                })?;

                log::debug!(
                    "Loaded {} initial chords for application ID {}",
                    app_chord_runtime.chords.len(),
                    application_id
                );
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
        }

        log::debug!("Loaded global chords: {:?}", global_chords_to_runtime_key.keys());
        Ok(LoadedAppChords {
            global_chords_to_runtime_key,
            runtimes: app_runtime_map,
        })
    }

    // No application = global chord
    pub fn get_chord_runtime(
        &self,
        sequence: &[Key],
        application_id: Option<String>,
    ) -> Option<&ChordRuntime> {
        if sequence
            .first()
            .is_some_and(|c| !c.is_digit() && !c.is_letter())
        {
            let Some(runtime_key) = self.global_chords_to_runtime_key.get(sequence) else {
                log::warn!("Invalid global chord sequence: {:?}", sequence);
                return None;
            };

            self.runtimes.get(runtime_key)
        } else {
            if let Some(app_id) = application_id {
                self.runtimes.get(&app_id)
            } else {
                None
            }
        }
    }
}

pub fn press_chord(handle: AppHandle, runtime: &ChordRuntime, chord: &Chord) -> Result<()> {
    log::debug!("Pressing chord: {:?}", chord);
    let shortcut = chord.shortcut.clone();
    let shell = chord.shell.clone();
    let js = chord.js.clone();

    // Prioritize shortcuts
    if let Some(shortcut) = shortcut {
        handle.clone().run_on_main_thread(move || {
            if let Err(e) = press_shortcut(shortcut.clone()) {
                log::error!("failed to press shortcut: {e}");
            }
        })?;
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
    } else if let Some(js_code) = js {
        log::debug!("Executing javascript: {}", js_code);
        if let Err(e) = with_js(|ctx| {
            ctx.eval::<(), _>(js_code)
        }) {
            log::error!("failed to execute javascript: {e}");
        }
    }

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
