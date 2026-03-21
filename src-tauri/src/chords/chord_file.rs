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
                .transpose()
            else {
                log::warn!("Skipping invalid shortcut for sequence: {}", sequence);
                continue;
            };

            let Ok(js) = entry.js_invocation() else {
                log::warn!(
                    "Skipping invalid JS action configuration for sequence: {}",
                    sequence
                );
                continue;
            };

            let chord = Chord {
                keys: keys.clone(),
                name: entry.name.clone(),
                shortcut,
                shell: entry.shell.clone(),
                js,
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
    #[serde(default, flatten)]
    pub extra: HashMap<String, toml::Value>,
}

impl AppChord {
    fn js_invocation(&self) -> Result<Option<crate::chords::ChordJsInvocation>> {
        let mut invocation = self
            .args
            .clone()
            .map(|args| crate::chords::ChordJsInvocation {
                export_name: None,
                args,
            });

        for (key, value) in &self.extra {
            let Some(export_name) = key.strip_suffix(":args") else {
                continue;
            };

            if export_name.is_empty() {
                anyhow::bail!("Invalid JS export key: {key}");
            }

            let args = parse_string_args(key, value)?;
            let next_invocation = crate::chords::ChordJsInvocation {
                export_name: Some(export_name.to_string()),
                args,
            };

            if invocation.replace(next_invocation).is_some() {
                anyhow::bail!("Multiple JS invocation targets configured");
            }
        }

        Ok(invocation)
    }
}

fn parse_string_args(key: &str, value: &toml::Value) -> Result<Vec<String>> {
    let toml::Value::Array(items) = value else {
        anyhow::bail!("{key} must be an array");
    };

    items
        .iter()
        .map(|item| match item {
            toml::Value::String(value) => Ok(value.clone()),
            _ => anyhow::bail!("{key} must contain only strings"),
        })
        .collect()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum AppChordMapValue {
    Single(AppChord),
    Multiple(Vec<AppChord>),
}

#[cfg(test)]
mod tests {
    use super::{AppChord, AppChordsFile};
    use crate::chords::ChordJsInvocation;

    #[test]
    fn parses_default_export_args() {
        let chord = AppChord {
            name: "Test".to_string(),
            shortcut: None,
            shell: None,
            args: Some(vec!["one".to_string(), "two".to_string()]),
            extra: Default::default(),
        };

        assert_eq!(
            chord.js_invocation().unwrap(),
            Some(ChordJsInvocation {
                export_name: None,
                args: vec!["one".to_string(), "two".to_string()],
            })
        );
    }

    #[test]
    fn parses_named_export_args() {
        let file = AppChordsFile::parse(
            r#"
[chords]
a = { name = "Menu", 'menu:args' = ["View", "Columns"] }
"#,
        )
        .unwrap();

        let entry = match file.chords.get("a").unwrap() {
            super::AppChordMapValue::Single(entry) => entry,
            super::AppChordMapValue::Multiple(_) => unreachable!(),
        };

        assert_eq!(
            entry.js_invocation().unwrap(),
            Some(ChordJsInvocation {
                export_name: Some("menu".to_string()),
                args: vec!["View".to_string(), "Columns".to_string()],
            })
        );
    }

    #[test]
    fn rejects_multiple_js_invocation_targets() {
        let file = AppChordsFile::parse(
            r#"
[chords]
a = { name = "Conflict", args = ["default"], 'menu:args' = ["View"] }
"#,
        )
        .unwrap();

        let entry = match file.chords.get("a").unwrap() {
            super::AppChordMapValue::Single(entry) => entry,
            super::AppChordMapValue::Multiple(_) => unreachable!(),
        };

        assert!(entry.js_invocation().is_err());
    }
}
