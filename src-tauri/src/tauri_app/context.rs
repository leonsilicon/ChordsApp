use crate::chords::{ChordFolder, LoadedAppChords};
use crate::feature::Chorder;
use crate::git::load_all_app_chords;
use crate::{
    feature::ChorderIndicatorPanel,
    input::KeyEventState,
    mode::{AppMode, AppModeStateMachine},
};
use anyhow::{Context, Result};
use arc_swap::ArcSwap;
use device_query::DeviceState;
use keycode::KeyMappingCode::*;
use objc2_app_kit::NSWorkspace;
use parking_lot::RwLock;
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

pub fn initialize_app_context(app: AppHandle) -> Result<()> {
    let chorder = {
        let window = app
            .get_webview_window(crate::constants::INDICATOR_WINDOW_LABEL)
            .ok_or(anyhow::anyhow!("chord indicator window not found"))?;
        Chorder::new(ChorderIndicatorPanel::from_window(window)?)
    };
    let bundled_app_chords = LoadedAppChords::from_folders(vec![ChordFolder::load_bundled()?])?;

    let context = AppContext::new(chorder, bundled_app_chords);

    // Setting the frontmost application immediately (the frontmost crate only detects changes)
    let workspace = NSWorkspace::sharedWorkspace();
    if let Some(application) = workspace.frontmostApplication() {
        if let Some(bundle_id) = application.bundleIdentifier() {
            context
                .frontmost_application_id
                .store(Arc::new(Some(bundle_id.to_string())));
        }
    }

    app.manage(context);
    reload_loaded_app_chords(&app).context("failed to reload app chords")?;

    Ok(())
}

pub fn reload_loaded_app_chords(app: &AppHandle) -> Result<()> {
    let context = app.state::<AppContext>();
    context.chorder.ensure_inactive(app.clone())?;

    let loaded_chords = load_all_app_chords(app)?;
    log::debug!(
        "Loaded app chords: {:?}",
        loaded_chords.app_runtime_map.keys()
    );
    *context.loaded_app_chords.write() = loaded_chords;

    Ok(())
}

pub fn list_active_chords(app: &AppHandle) -> Result<Vec<ActiveChordInfo>> {
    let context = app.state::<AppContext>();
    let loaded_app_chords = context.loaded_app_chords.read();
    Ok(list_loaded_chords(&loaded_app_chords))
}

pub fn list_loaded_chords(loaded_app_chords: &LoadedAppChords) -> Vec<ActiveChordInfo> {
    let mut chords = Vec::new();
    let mut seen = HashSet::new();

    for chord in loaded_app_chords.global_runtime.chords.values() {
        let item = ActiveChordInfo {
            scope: "Global".to_string(),
            scope_kind: "global".to_string(),
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

    for (application_id, runtime) in &loaded_app_chords.app_runtime_map {
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

    if let Some(lua) = &chord.lua {
        return format!("Lua: {lua}");
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
