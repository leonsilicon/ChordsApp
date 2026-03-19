use crate::input::Key;
use anyhow::Result;
use keycode::KeyMappingCode;
use std::str::FromStr;

pub fn press_shortcut(shortcut: Shortcut) -> Result<()> {
    log::debug!("Pressing shortcut: {:?}", shortcut);
    apply_actions(press_actions(&shortcut))?;

    Ok(())
}

pub fn release_shortcut(shortcut: Shortcut) -> Result<()> {
    log::debug!("Releasing shortcut: {:?}", shortcut);
    apply_actions(release_actions(&shortcut))?;

    Ok(())
}

// We use `rdev` for simulate instead of Enigo because rdev sends actual keypresses
// instead of enigo's input injection (this works better in some apps, e.g. cmd+1 in IntelliJ)
fn apply_actions(actions: Vec<ShortcutAction>) -> Result<()> {
    for action in actions {
        match action {
            ShortcutAction::Press(key, suppress_shift) => {
                rdev::simulate(&rdev::EventType::KeyPress(key.try_into()?), suppress_shift)?;
            }
            ShortcutAction::Release(key, suppress_shift) => {
                rdev::simulate(
                    &rdev::EventType::KeyRelease(key.try_into()?),
                    suppress_shift,
                )?;
            }
        }
    }

    Ok(())
}

fn press_actions(shortcut: &Shortcut) -> Vec<ShortcutAction> {
    let mut actions = Vec::new();
    let has_shift = shortcut.chords.iter().any(|chord| {
        chord.keys.iter().any(|key| {
            matches!(
                key,
                Key(KeyMappingCode::ShiftLeft) | Key(KeyMappingCode::ShiftRight)
            )
        })
    });
    let suppress_shift = !has_shift;
    log::debug!("Has Shift: {}", has_shift);

    for (index, chord) in shortcut.chords.iter().enumerate() {
        for &key in &chord.keys {
            actions.push(ShortcutAction::Press(key, suppress_shift));
        }

        if index + 1 != shortcut.chords.len() {
            for &key in chord.keys.iter().rev() {
                actions.push(ShortcutAction::Release(key, suppress_shift));
            }
        }
    }

    actions
}

fn release_actions(shortcut: &Shortcut) -> Vec<ShortcutAction> {
    let has_shift = shortcut.chords.iter().any(|chord| {
        chord.keys.iter().any(|key| {
            matches!(
                key,
                Key(KeyMappingCode::ShiftLeft) | Key(KeyMappingCode::ShiftRight)
            )
        })
    });
    let suppress_shift = !has_shift;

    shortcut
        .chords
        .last()
        .into_iter()
        .flat_map(|chord| {
            chord
                .keys
                .iter()
                .rev()
                .copied()
                .map(|k| ShortcutAction::Release(k, suppress_shift))
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShortcutAction {
    Press(Key, bool),
    Release(Key, bool),
}

/// Represents a parsed keyboard shortcut, e.g. "cmd+shift+n".
#[derive(Debug, Clone)]
pub struct Shortcut {
    pub chords: Vec<ShortcutChord>,
}

impl Shortcut {
    pub fn parse(shortcut_str: &str) -> Result<Self> {
        Key::modifiers();
        let mut chords = Vec::new();
        for chord in shortcut_str.split(' ') {
            let mut keys = Vec::new();
            for key_name in chord.split('+') {
                if let Ok(key) = Key::from_str(key_name) {
                    keys.push(key);
                } else {
                    return Err(anyhow::anyhow!(
                        "Failed to parse shortcut: {}",
                        shortcut_str
                    ));
                }
            }
            chords.push(ShortcutChord { keys });
        }

        Ok(Shortcut { chords })
    }
}

#[derive(Debug, Clone)]
pub struct ShortcutChord {
    pub keys: Vec<Key>,
}
