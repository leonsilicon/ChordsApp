use crate::chords::{Chord, ChordMap, Shortcut};
use crate::input::Key;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct RawAppChordsFile {
    #[serde(rename = "_config")]
    pub config: Option<AppChordsFileConfig>,

    // Needs to be toml::Value because otherwise parsing will fail
    #[serde(flatten)]
    pub chords: HashMap<String, toml::Value>,
}

#[derive(Debug, Serialize)]
pub struct AppChordsFile {
    #[serde(rename = "_config")]
    pub config: Option<AppChordsFileConfig>,

    #[serde(flatten)]
    pub chords: HashMap<String, AppChordMapValue>,
}

impl AppChordsFile {
    pub fn parse(content: &str) -> Result<Self> {
        let parsed: RawAppChordsFile = toml::from_str(content)?;

        let mut chords = HashMap::new();
        for (key, value) in parsed.chords {
            match value.try_into() {
                Ok(chord) => {
                    chords.insert(key, chord);
                }
                Err(error) => {
                    log::warn!("Skipping invalid chord entry {}: {}", key, error);
                }
            }
        }

        Ok(AppChordsFile {
            config: parsed.config,
            chords,
        })
    }

    pub fn get_chord_map(&self) -> Result<ChordMap> {
        let mut chords = HashMap::new();

        for (sequence, value) in &self.chords {
            let entry = match value {
                AppChordMapValue::Single(entry) => Some(entry),
                AppChordMapValue::Multiple(entries) => entries.first(),
            };

            let Some(entry) = entry else { continue };

            let keys = Key::parse_sequence(sequence)?;

            let chord = Chord {
                keys: keys.clone(),
                name: entry.name.clone(),
                command: entry.command.clone(),
                shortcut: entry
                    .shortcut
                    .as_ref()
                    .map(|s| Shortcut::parse(s))
                    .transpose()?,
                shell: entry.shell.clone(),
            };

            chords.insert(keys, chord);
        }

        Ok(chords)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppChordsFileConfig {
    pub name: Option<String>,
    pub extends: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppChord {
    pub name: String,
    pub command: Option<String>,
    pub shortcut: Option<String>,
    pub shell: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum AppChordMapValue {
    Single(AppChord),
    Multiple(Vec<AppChord>),
}
