use crate::chords::shortcut::{press_shortcut, release_shortcut, Shortcut};
use crate::chords::{AppChordMapValue, AppChordsFile, AppChordsFileConfig, ChordFolder};
use crate::input::Key;
use crate::js::{format_js_error, with_js};
use anyhow::Result;
use rquickjs::function::Args;
use rquickjs::{Ctx, Function, IntoJs, Module, Object, Promise, Value};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;

#[derive(Debug, Clone)]
pub struct Chord {
    pub keys: Vec<Key>,
    pub name: String,
    pub shortcut: Option<Shortcut>,
    pub shell: Option<String>,
    // TODO: support non-string arguments
    pub args: Option<Vec<String>>,
}

pub struct LoadedAppChords {
    pub global_chords_to_runtime_key: HashMap<Vec<Key>, String>,
    pub runtimes: HashMap<String, ChordRuntime>,
}

// Each chord runtime is associated with a JS module which lives in-memory
// (similar to require.cache)
pub struct ChordRuntime {
    // Used as a unique module key
    pub path: String,

    pub chords: HashMap<Vec<Key>, Chord>,
    // Needs to be an Arc so the JS runtime can access its latest value
    pub raw_chords: Arc<Mutex<HashMap<String, AppChordMapValue>>>,
    pub config: Option<AppChordsFileConfig>,
}

#[derive(Debug, Clone)]
pub struct ChordPayload {
    pub chord: Chord,
    pub num_times: usize,
}

const GLOBAL_CHORD_RUNTIME_ID: &str = "__global__";

impl ChordRuntime {
    pub fn from_chords(path: String, chords: HashMap<Vec<Key>, Chord>) -> Result<Self> {
        let raw_chords = Arc::new(Mutex::new(HashMap::new()));
        Ok(Self {
            path,
            chords,
            raw_chords,
            config: None,
        })
    }

    // Doesn't resolve _config.extends
    pub fn from_file_shallow(path: String, chord_file: AppChordsFile) -> Result<Self> {
        let raw_chords = Arc::new(Mutex::new(chord_file.chords.clone()));
        let config = chord_file.config.clone();

        // We intentionally keep global chords because they execute in this runtime
        let chords = chord_file.get_chords_shallow();

        Ok(Self {
            path,
            raw_chords,
            config,
            chords,
        })
    }

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

    pub fn get_chord(&self, sequence: &[Key]) -> Option<ChordPayload> {
        let split_idx = sequence
            .iter()
            .position(|k| !k.is_digit())
            .unwrap_or(sequence.len());
        let (digit_keys, chord_keys) = sequence.split_at(split_idx);
        let num_times = if digit_keys.is_empty() {
            1
        } else {
            let digits: String = digit_keys.iter().filter_map(|k| k.to_char(false)).collect();
            let num_times = digits.parse::<usize>().unwrap_or(1);
            num_times
        };
        self.chords.get(chord_keys).map(|chord| ChordPayload {
            chord: chord.clone(),
            num_times,
        })
    }
}

fn runtime_id_from_chords_path(file_path: &Path) -> Option<String> {
    if file_path.file_name()? != "macos.toml" {
        return None;
    }

    let application_path = file_path.parent()?.strip_prefix("chords").ok()?;
    if application_path.as_os_str().is_empty() {
        return Some(GLOBAL_CHORD_RUNTIME_ID.to_string());
    }

    Some(
        application_path
            .iter()
            .map(|component| component.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("."),
    )
}

fn is_global_chord_sequence(sequence: &[Key]) -> bool {
    sequence
        .first()
        .is_some_and(|key| !key.is_digit() && !key.is_letter())
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
                log::debug!(
                    "Starting to load chords file from path {:?}",
                    chord_file_path
                );

                let Some(runtime_id) = runtime_id_from_chords_path(Path::new(&chord_file_path))
                else {
                    log::warn!("Invalid chords path: {:?}", chord_file_path);
                    continue;
                };

                let chords = file.get_chords_shallow();
                for sequence in chords.keys() {
                    if is_global_chord_sequence(sequence) {
                        log::debug!("Adding global chord for sequence: {:?}", sequence);
                        global_chords_to_runtime_key.insert(sequence.clone(), runtime_id.clone());
                    }
                }

                let config = file.config.clone();
                let app_chord_runtime = ChordRuntime::from_file_shallow(chord_file_path, file)?;

                log::debug!(
                    "Loaded {} initial chords for runtime {}",
                    app_chord_runtime.chords.len(),
                    runtime_id
                );
                app_runtime_map.insert(runtime_id.clone(), app_chord_runtime);
                app_config_map.insert(runtime_id, config);
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

        log::debug!(
            "Loaded global chords: {:?}",
            global_chords_to_runtime_key.keys()
        );

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
        if is_global_chord_sequence(sequence) {
            let Some(runtime_key) = self.global_chords_to_runtime_key.get(sequence) else {
                log::warn!("Invalid global chord sequence: {:?}", sequence);
                return None;
            };

            self.runtimes.get(runtime_key)
        } else {
            application_id.and_then(|app_id| self.runtimes.get(&app_id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{runtime_id_from_chords_path, GLOBAL_CHORD_RUNTIME_ID};
    use std::path::Path;

    #[test]
    fn maps_root_macos_file_to_global_runtime() {
        assert_eq!(
            runtime_id_from_chords_path(Path::new("chords/macos.toml")).as_deref(),
            Some(GLOBAL_CHORD_RUNTIME_ID)
        );
    }

    #[test]
    fn maps_nested_macos_file_to_bundle_identifier_style_runtime() {
        assert_eq!(
            runtime_id_from_chords_path(Path::new("chords/com/apple/finder/macos.toml")).as_deref(),
            Some("com.apple.finder")
        );
    }

    #[test]
    fn rejects_non_macos_toml_paths() {
        assert_eq!(
            runtime_id_from_chords_path(Path::new("chords/com/apple/finder/chords.toml")),
            None
        );
    }
}

fn press_shortcut_on_main_thread(
    handle: AppHandle,
    shortcut: Shortcut,
    num_times: usize,
) -> Result<()> {
    handle.run_on_main_thread(move || {
        if let Err(e) = press_shortcut(shortcut.clone(), num_times) {
            log::error!("failed to press shortcut: {e}");
        }
    })?;

    Ok(())
}

fn release_shortcut_on_main_thread(handle: AppHandle, shortcut: Shortcut) -> Result<()> {
    handle.run_on_main_thread(move || {
        if let Err(e) = release_shortcut(shortcut.clone()) {
            log::error!("failed to release shortcut: {e}");
        }
    })?;

    Ok(())
}

fn run_shell_command_in_background(shell: String) {
    std::thread::spawn(move || run_shell_command(shell));
}

fn run_shell_command(shell: String) {
    let mut command = Command::new("sh");
    command.arg("-c").arg(&shell);
    log::debug!("Running shell command: {:?}", command);

    match command.output() {
        Ok(output) => log_shell_output(&shell, output),
        Err(e) => {
            log::error!("failed to run shell command `{shell}`: {e}");
        }
    }
}

fn log_shell_output(shell: &str, output: std::process::Output) {
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

fn invoke_js_chord_in_background(
    handle: AppHandle,
    module_path: String,
    args: Vec<String>,
    num_times: usize,
) {
    tauri::async_runtime::spawn(async move {
        if let Err(e) = with_js(handle.clone(), move |ctx| {
            Box::pin(call_js_default_export(ctx, module_path, args, num_times))
        })
        .await
        {
            log::error!("press_chord failed: {}", e);
        }
    });
}

async fn call_js_default_export<'js>(
    ctx: Ctx<'js>,
    module_path: String,
    args: Vec<String>,
    num_times: usize,
) -> anyhow::Result<()> {
    for _ in 0..num_times {
        let Some(namespace) = import_js_namespace(ctx.clone(), &module_path).await else {
            return Ok(());
        };

        let Some(default_function) = get_default_export_function(ctx.clone(), &namespace).await
        else {
            return Ok(());
        };

        let Some(js_args) = convert_js_args(&ctx, args.clone()) else {
            return Ok(());
        };

        log::debug!("Calling default function with arguments: {:?}", js_args);

        let result = match call_function_with_values(ctx.clone(), default_function, js_args) {
            Ok(value) => value,
            Err(e) => {
                log::error!(
                    "Failed to call default function: {}",
                    format_js_error(ctx.clone(), e)
                );
                return Ok(());
            }
        };

        log::debug!("Return value: {:?}", result);

        match await_promise_if_needed(ctx.clone(), result).await {
            Ok(awaited) => {
                log::debug!("Promise awaited: {:?}", awaited);
            }
            Err(e) => {
                log::error!(
                    "Default function promise rejected: {}",
                    format_js_error(ctx.clone(), e)
                );
            }
        }
    }

    Ok(())
}

async fn import_js_namespace<'js>(ctx: Ctx<'js>, module_path: &str) -> Option<Object<'js>> {
    let import_promise = match Module::import(&ctx, module_path.to_string()) {
        Ok(import_promise) => import_promise,
        Err(e) => {
            log::error!(
                "Failed to start importing JS module: {}",
                format_js_error(ctx.clone(), e)
            );
            return None;
        }
    };

    match import_promise.into_future::<Object>().await {
        Ok(namespace) => Some(namespace),
        Err(e) => {
            log::error!(
                "Failed to import JS module: {}",
                format_js_error(ctx.clone(), e)
            );
            None
        }
    }
}

async fn get_default_export_function<'js>(
    ctx: Ctx<'js>,
    namespace: &Object<'js>,
) -> Option<Function<'js>> {
    let default: Value<'js> = match namespace.get("default") {
        Ok(default) => default,
        Err(e) => {
            log::error!(
                "Failed to get default export: {}",
                format_js_error(ctx.clone(), e)
            );
            return None;
        }
    };

    log::debug!("Default export: {:?}", default);
    let resolved: Value<'js> = if let Some(promise) = default.as_promise().cloned() {
        match promise.into_future::<Value<'js>>().await {
            Ok(value) => value,
            Err(e) => {
                log::error!(
                    "Failed to resolve default export promise: {}",
                    format_js_error(ctx.clone(), e)
                );
                return None;
            }
        }
    } else {
        default
    };

    let Some(function) = resolved.as_function().cloned() else {
        log::error!(
            "Default export did not resolve to a function: {:?}",
            resolved
        );
        return None;
    };

    Some(function)
}

fn convert_js_args<'js>(ctx: &Ctx<'js>, args: Vec<String>) -> Option<Vec<Value<'js>>> {
    match args
        .into_iter()
        .map(|arg| arg.into_js(ctx))
        .collect::<rquickjs::Result<_>>()
    {
        Ok(args) => Some(args),
        Err(e) => {
            log::error!(
                "Failed to convert arguments: {}",
                format_js_error(ctx.clone(), e)
            );
            None
        }
    }
}

fn call_function_with_values<'js>(
    ctx: Ctx<'js>,
    function: Function<'js>,
    values: Vec<Value<'js>>,
) -> rquickjs::Result<Value<'js>> {
    let mut args_builder = Args::new(ctx, values.len());

    for value in values {
        args_builder.push_arg(value)?;
    }

    function.call_arg(args_builder)
}

async fn await_promise_if_needed<'js>(ctx: Ctx<'js>, result: Value<'js>) -> rquickjs::Result<()> {
    if !result.is_promise() {
        return Ok(());
    }

    let promise = match Promise::from_value(result) {
        Ok(promise) => promise,
        Err(e) => {
            log::error!(
                "Function returned something marked as promise, but it could not be converted: {}",
                format_js_error(ctx.clone(), e)
            );
            return Ok(());
        }
    };

    let result = promise.into_future::<Value>().await.map(|_| ());
    log::debug!("Promise result: {:?}", result);
    result
}

pub fn press_chord(
    handle: AppHandle,
    runtime: &ChordRuntime,
    chord_payload: &ChordPayload,
) -> Result<()> {
    log::debug!("Pressing chord: {:?}", chord_payload);

    if let Some(shortcut) = chord_payload.chord.shortcut.clone() {
        return press_shortcut_on_main_thread(handle, shortcut, chord_payload.num_times);
    }

    if let Some(shell) = chord_payload.chord.shell.clone() {
        run_shell_command_in_background(shell);
        return Ok(());
    }

    if let Some(args) = chord_payload.chord.args.clone() {
        invoke_js_chord_in_background(handle, runtime.path.clone(), args, chord_payload.num_times);
    }

    Ok(())
}

pub fn release_chord(handle: AppHandle, chord: &Chord) -> Result<()> {
    if let Some(shortcut) = chord.shortcut.clone() {
        release_shortcut_on_main_thread(handle, shortcut)?;
    }

    Ok(())
}
