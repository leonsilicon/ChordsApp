use crate::input::Key;
use anyhow::Result;
use enigo::Keyboard;
use std::str::FromStr;

pub fn press_shortcut(shortcut: Shortcut) -> Result<()> {
    log::debug!("Executing shortcut: {:?}", shortcut);
    let mut settings = enigo::Settings::default();
    settings.event_source_user_data = Some(0xDEADBEEF);
    let mut enigo = enigo::Enigo::new(&settings)?;

    let mut last_key: Option<Key> = None;
    for chord in &shortcut.chords {
        for &key in &chord.keys {
            // Release the last key
            if let Some(last_key) = last_key {
                enigo.key(last_key.try_into()?, enigo::Direction::Release)?;
            }

            enigo.key(key.try_into()?, enigo::Direction::Press)?;
            last_key = Some(key);
        }
    }

    Ok(())
}

pub fn release_shortcut(shortcut: Shortcut) -> Result<()> {
    log::debug!("Executing shortcut: {:?}", shortcut);
    let mut settings = enigo::Settings::default();
    settings.event_source_user_data = Some(0xDEADBEEF);
    let mut enigo = enigo::Enigo::new(&settings)?;

    for chord in &shortcut.chords {
        for &key in &chord.keys {
            enigo.key(key.try_into()?, enigo::Direction::Release)?;
        }
    }

    Ok(())
}

/// Represents a parsed keyboard shortcut, e.g. "cmd+shift+n".
#[derive(Debug, Clone)]
pub struct Shortcut {
    pub chords: Vec<ShortcutChord>,
}

impl Shortcut {
    pub fn parse(shortcut_str: &str) -> Result<Self> {
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
