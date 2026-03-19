use crate::chords::{Chord, Shortcut};
use crate::input::Key;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct AppChordsFile {
    pub config: Option<AppChordsFileConfig>,
    pub chords: HashMap<String, AppChordMapValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RawAppChordsFile {
    pub config: Option<AppChordsFileConfig>,
    pub chords: Option<HashMap<String, AppChordMapValue>>,
}

impl AppChordsFile {
    pub fn parse(content: &str) -> Result<Self> {
        let file = toml::from_str::<RawAppChordsFile>(content)?;
        Ok(Self {
            config: file.config,
            chords: file.chords.unwrap_or_default(),
        })
    }

    pub fn get_chords_shallow(&self) -> HashMap<Vec<Key>, Chord> {
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

            let Ok(keys) = Key::parse_sequence(sequence) else {
                log::warn!("Skipping invalid sequence for chord: {}", sequence);
                continue;
            };

            let Ok(shortcut) = entry
                .shortcut
                .as_ref()
                .map(|s| Shortcut::parse(s))
                .transpose() else {
                log::warn!("Skipping invalid shortcut for sequence: {}", sequence);
                continue;
            };

            let chord = Chord {
                keys: keys.clone(),
                name: entry.name.clone(),
                shortcut,
                shell: entry.shell.clone(),
                args: entry.args.clone(),
            };

            chords.insert(keys, chord);
        }

        chords
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppChordsFileConfig {
    pub name: Option<String>,
    pub extends: Option<String>,
    pub js: Option<AppChordsFileConfigJs>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppChordsFileConfigJs {
    pub module: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppChord {
    pub name: String,
    pub shortcut: Option<String>,
    pub shell: Option<String>,
    pub args: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum AppChordMapValue {
    Single(AppChord),
    Multiple(Vec<AppChord>),
}
