use crate::chords::LoadedAppChords;
use crate::feature::Chorder;
use crate::git::load_all_chord_folders;
use crate::js::{format_js_error, with_js};
use crate::{
    input::KeyEventState,
    mode::{AppMode, AppModeStateMachine},
};
use anyhow::Result;
use arc_swap::ArcSwap;
use device_query::DeviceState;
use keycode::KeyMappingCode::*;
use parking_lot::RwLock;
use rquickjs::Module;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveChordInfo {
    pub scope: String,
    pub scope_kind: String,
    pub sequence: String,
    pub name: String,
    pub action: String,
}

pub struct AppContext {
    pub chorder: Chorder,

    pub device_state: Option<DeviceState>,
    pub loaded_app_chords: RwLock<LoadedAppChords>,
    pub frontmost_application_id: ArcSwap<Option<String>>,
    pub key_event_state: KeyEventState,

    // Not a mutex since it uses Atomics
    app_mode_state_machine: Arc<AppModeStateMachine>,
}

impl AppContext {
    pub fn new(chorder: Chorder, bundled_app_chords: LoadedAppChords) -> Self {
        let device_state = if macos_accessibility_client::accessibility::application_is_trusted() {
            Some(DeviceState {})
        } else {
            None
        };

        let app_mode_state_machine = Arc::new(AppModeStateMachine::new(device_state.clone()));

        Self {
            device_state,
            frontmost_application_id: ArcSwap::new(Arc::new(None)),
            key_event_state: KeyEventState::new(app_mode_state_machine.clone()),
            loaded_app_chords: RwLock::new(bundled_app_chords),
            app_mode_state_machine,
            chorder,
        }
    }

    pub fn get_app_mode(&self) -> AppMode {
        self.app_mode_state_machine.get_app_mode()
    }

    pub fn is_shift_pressed(&self) -> bool {
        self.app_mode_state_machine
            .is_shift_pressed
            .load(Ordering::SeqCst)
    }
}

pub async fn initialize_app_context(handle: AppHandle) -> Result<()> {
    Ok(())
}

// Also evaluates JavaScript
pub async fn reload_loaded_app_chords(app: AppHandle) -> Result<()> {
    let context = app.state::<AppContext>();
    context.chorder.ensure_inactive(app.clone())?;

    // Load all JS files as modules
    let chord_folders = load_all_chord_folders(app.clone())?;

    // Load all JS files as modules, but keep `chord_folders` so we can use it later.
    for chord_folder in &chord_folders {
        let js_files = chord_folder.js_files.clone();

        with_js(app.clone(), move |ctx| {
            Box::pin(async move {
                for (filepath, js) in js_files {
                    let module = match Module::declare(ctx.clone(), filepath.clone(), js) {
                        Ok(m) => {
                            log::debug!("Declared module {}", filepath);
                            m
                        }
                        Err(e) => {
                            log::error!(
                                "Failed to declare JS module {}: {}",
                                filepath,
                                format_js_error(ctx.clone(), e)
                            );
                            continue;
                        }
                    };

                    let (_evaluated, promise) = match module.eval() {
                        Ok(v) => v,
                        Err(e) => {
                            log::error!(
                                "Failed to start evaluating JS module {}: {}",
                                filepath,
                                format_js_error(ctx.clone(), e)
                            );
                            continue;
                        }
                    };

                    if let Err(e) = promise.into_future::<()>().await {
                        log::error!(
                            "Failed to evaluate JS module {}: {}",
                            filepath,
                            format_js_error(ctx.clone(), e)
                        );
                    }
                }

                Ok(())
            })
        })
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    }

    let loaded_chords = LoadedAppChords::from_folders(chord_folders)?;
    // We should only load `macos.toml` modules AFTER the js files have been loaded
    load_chord_files_runtime_modules(app.clone(), &loaded_chords).await;

    log::debug!("Loaded chord files: {:?}", loaded_chords.runtimes.keys());
    *context.loaded_app_chords.write() = loaded_chords;

    Ok(())
}

// Load all the modules specified in the config.js.module of the `macos.toml` files.
pub async fn load_chord_files_runtime_modules(
    handle: AppHandle,
    loaded_app_chords: &LoadedAppChords,
) {
    for (bundle_id, runtime) in loaded_app_chords.runtimes.iter() {
        let handle = handle.clone();

        let Some(js) = runtime.config.as_ref().and_then(|c| c.js.as_ref()) else {
            continue;
        };

        let Some(content) = js.module.clone() else {
            continue;
        };

        let path = runtime.path.clone();
        let raw_chords = runtime.raw_chords.lock().unwrap().clone();
        let bundle_id = bundle_id.clone();

        tauri::async_runtime::spawn(async move {
            let path_ = path.clone();
            let result = with_js(handle, move |ctx| {
                Box::pin(async move {
                    let module = match Module::declare(ctx.clone(), path.clone(), content) {
                        Ok(m) => m,
                        Err(e) => {
                            log::error!(
                                "Failed to declare module {}: {}",
                                path,
                                format_js_error(ctx.clone(), e)
                            );
                            return Ok(());
                        }
                    };

                    let chords = match rquickjs_serde::to_value(ctx.clone(), raw_chords) {
                        Ok(value) => value,
                        Err(e) => {
                            log::error!("Failed to serialize chords");
                            return Ok(());
                        }
                    };

                    let chords_obj = match chords.into_object() {
                        Some(value) => value,
                        None => {
                            log::error!("Failed to convert chords to object");
                            return Ok(());
                        }
                    };

                    let meta = match module.meta() {
                        Ok(meta) => meta,
                        Err(e) => {
                            log::error!("Failed to get import.meta for module {}", path);
                            return Ok(())
                        }
                    };

                    if let Err(e) = meta.set("chords", chords_obj) {
                        log::error!(
                            "Failed to set `import.meta.chords` for module {}: {}",
                            path,
                            format_js_error(ctx.clone(), e)
                        );
                        return Ok(());
                    }

                    if let Err(e) = meta.set("bundleId", bundle_id) {
                        log::error!(
                            "Failed to set `import.meta.bundleId` for module {}: {}",
                            path,
                            format_js_error(ctx.clone(), e)
                        );
                        return Ok(());
                    }

                    let (_evaluated, promise) = match module.eval() {
                        Ok(v) => v,
                        Err(e) => {
                            log::error!(
                                "Failed to start evaluating module {}: {}",
                                path,
                                format_js_error(ctx.clone(), e)
                            );
                            return Ok(());
                        }
                    };

                    if let Err(e) = promise.into_future::<()>().await {
                        log::error!(
                            "Failed to evaluate module {}: {}",
                            path,
                            format_js_error(ctx.clone(), e)
                        );
                    }

                    Ok(())
                })
            })
            .await;

            if let Err(err) = result {
                log::error!("load_module failed for {}: {}", path_, err);
            }
        });
    }
}

pub fn list_active_chords(app: AppHandle) -> Result<Vec<ActiveChordInfo>> {
    let context = app.state::<AppContext>();
    let loaded_app_chords = context.loaded_app_chords.read();
    Ok(list_loaded_chords(&loaded_app_chords))
}

pub fn list_loaded_chords(loaded_app_chords: &LoadedAppChords) -> Vec<ActiveChordInfo> {
    let mut chords = Vec::new();
    let mut seen = HashSet::new();

    for (application_id, runtime) in &loaded_app_chords.runtimes {
        for chord in runtime.chords.values() {
            let item = ActiveChordInfo {
                scope: application_id.clone(),
                scope_kind: "app".to_string(),
                sequence: format_sequence(&chord.keys),
                name: chord.name.clone(),
                action: format_action(chord),
            };
            let fingerprint = format!(
                "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
                item.scope_kind, item.scope, item.sequence, item.name, item.action
            );
            if seen.insert(fingerprint) {
                chords.push(item);
            }
        }
    }

    chords.sort_by(|left, right| {
        left.scope_kind
            .cmp(&right.scope_kind)
            .then(left.scope.cmp(&right.scope))
            .then(left.sequence.cmp(&right.sequence))
            .then(left.name.cmp(&right.name))
    });

    chords
}
fn format_action(chord: &crate::chords::Chord) -> String {
    if let Some(shortcut) = &chord.shortcut {
        return format!("Shortcut: {}", format_shortcut(shortcut));
    }

    if let Some(shell) = &chord.shell {
        return format!("Shell: {shell}");
    }

    if let Some(args) = &chord.args {
        return format!("Args: {:?}", args);
    }

    "No action".to_string()
}

fn format_shortcut(shortcut: &crate::chords::Shortcut) -> String {
    shortcut
        .chords
        .iter()
        .map(|chord| {
            chord
                .keys
                .iter()
                .map(|key| format_key(*key))
                .collect::<Vec<_>>()
                .join(" + ")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_sequence(keys: &[crate::input::Key]) -> String {
    keys.iter()
        .map(|key| {
            key.to_char(false)
                .map(|ch| ch.to_ascii_uppercase().to_string())
                .unwrap_or_else(|| format_key(*key))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_key(key: crate::input::Key) -> String {
    if let Some(ch) = key.to_char(false) {
        return ch.to_ascii_uppercase().to_string();
    }

    match key.0 {
        ShiftLeft | ShiftRight => "Shift".to_string(),
        ControlLeft | ControlRight => "Ctrl".to_string(),
        MetaLeft | MetaRight => "Cmd".to_string(),
        AltLeft | AltRight => "Alt".to_string(),
        CapsLock => "Caps Lock".to_string(),
        Space => "Space".to_string(),
        Enter => "Enter".to_string(),
        Tab => "Tab".to_string(),
        Escape => "Esc".to_string(),
        ArrowUp => "Up".to_string(),
        ArrowDown => "Down".to_string(),
        ArrowLeft => "Left".to_string(),
        ArrowRight => "Right".to_string(),
        Backspace => "Backspace".to_string(),
        Delete => "Delete".to_string(),
        Home => "Home".to_string(),
        End => "End".to_string(),
        PageUp => "Page Up".to_string(),
        PageDown => "Page Down".to_string(),
        Fn => "Fn".to_string(),
        other => format!("{other:?}"),
    }
}
