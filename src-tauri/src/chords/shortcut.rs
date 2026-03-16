use crate::input::Key;
use anyhow::Result;
use enigo::Keyboard;
use std::str::FromStr;

pub fn press_shortcut(shortcut: Shortcut) -> Result<()> {
    log::debug!("Pressing shortcut: {:?}", shortcut);
    let mut enigo = new_enigo()?;
    apply_actions(&mut enigo, press_actions(&shortcut))?;

    Ok(())
}

pub fn release_shortcut(shortcut: Shortcut) -> Result<()> {
    log::debug!("Releasing shortcut: {:?}", shortcut);
    let mut enigo = new_enigo()?;
    apply_actions(&mut enigo, release_actions(&shortcut))?;

    Ok(())
}

fn new_enigo() -> Result<enigo::Enigo> {
    let mut settings = enigo::Settings::default();
    settings.event_source_user_data = Some(0xDEADBEEF);
    Ok(enigo::Enigo::new(&settings)?)
}

fn apply_actions(enigo: &mut enigo::Enigo, actions: Vec<ShortcutAction>) -> Result<()> {
    for action in actions {
        match action {
            ShortcutAction::Press(key) => enigo.key(key.try_into()?, enigo::Direction::Press)?,
            ShortcutAction::Release(key) => {
                enigo.key(key.try_into()?, enigo::Direction::Release)?
            }
        }
    }

    Ok(())
}

fn press_actions(shortcut: &Shortcut) -> Vec<ShortcutAction> {
    let mut actions = Vec::new();

    for (index, chord) in shortcut.chords.iter().enumerate() {
        for &key in &chord.keys {
            actions.push(ShortcutAction::Press(key));
        }

        if index + 1 != shortcut.chords.len() {
            for &key in chord.keys.iter().rev() {
                actions.push(ShortcutAction::Release(key));
            }
        }
    }

    actions
}

fn release_actions(shortcut: &Shortcut) -> Vec<ShortcutAction> {
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
                .map(ShortcutAction::Release)
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShortcutAction {
    Press(Key),
    Release(Key),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn press_single_chord_keeps_it_held() {
        let shortcut = Shortcut::parse("cmd+c").unwrap();

        assert_eq!(
            press_actions(&shortcut),
            vec![
                ShortcutAction::Press(Key::from_str("cmd").unwrap()),
                ShortcutAction::Press(Key::from_str("c").unwrap()),
            ]
        );
    }

    #[test]
    fn press_multi_chord_releases_intermediate_chords_only() {
        let shortcut = Shortcut::parse("cmd+k cmd+c").unwrap();

        assert_eq!(
            press_actions(&shortcut),
            vec![
                ShortcutAction::Press(Key::from_str("cmd").unwrap()),
                ShortcutAction::Press(Key::from_str("k").unwrap()),
                ShortcutAction::Release(Key::from_str("k").unwrap()),
                ShortcutAction::Release(Key::from_str("cmd").unwrap()),
                ShortcutAction::Press(Key::from_str("cmd").unwrap()),
                ShortcutAction::Press(Key::from_str("c").unwrap()),
            ]
        );
    }

    #[test]
    fn release_only_releases_last_chord() {
        let shortcut = Shortcut::parse("cmd+k cmd+c").unwrap();

        assert_eq!(
            release_actions(&shortcut),
            vec![
                ShortcutAction::Release(Key::from_str("c").unwrap()),
                ShortcutAction::Release(Key::from_str("cmd").unwrap()),
            ]
        );
    }
}
