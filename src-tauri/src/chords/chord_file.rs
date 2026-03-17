use crate::chords::{Chord, Shortcut};
use crate::input::Key;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppChordsFile {
    pub config: Option<AppChordsFileConfig>,
    pub chords: HashMap<String, AppChordMapValue>,
}

impl AppChordsFile {
    pub fn parse(content: &str) -> Result<Self> {
        Ok(toml::from_str(content)?)
    }

    pub fn get_chords_shallow(&self) -> Result<HashMap<Vec<Key>, Chord>> {
        let mut chords = HashMap::new();

        for (sequence, value) in &self.chords {
            let entry = match value {
                AppChordMapValue::Single(entry) => Some(entry),
                AppChordMapValue::Multiple(entries) => entries.first(),
            };

            let Some(entry) = entry else {
                log::warn!("Skipping invalid chord entry for sequence: {}", sequence);
                continue;
            };

            let keys = Key::parse_sequence(sequence)?;

            let chord = Chord {
                keys: keys.clone(),
                name: entry.name.clone(),
                shortcut: entry
                    .shortcut
                    .as_ref()
                    .map(|s| Shortcut::parse(s))
                    .transpose()?,
                shell: entry.shell.clone(),
                lua: entry.lua.clone()
            };

            chords.insert(keys, chord);
        }

        Ok(chords)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppChordsFileConfig {
    pub name: Option<String>,
    pub extends: Option<String>,
    pub lua: Option<AppChordsFileConfigLua>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppChordsFileConfigLua {
    pub init: Option<String>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppChord {
    pub name: String,
    pub shortcut: Option<String>,
    pub shell: Option<String>,
    pub lua: Option<String>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum AppChordMapValue {
    Single(AppChord),
    Multiple(Vec<AppChord>),
}
