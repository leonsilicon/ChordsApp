use crate::chords::Shortcut;
use crate::input::{Key, KeyCombination, KeyCombinationModifiers};
use anyhow::{bail, Context};
use jsonc_parser::{JsonValue, ParseOptions};
use std::str::FromStr;
use std::sync::LazyLock;

pub const SETTINGS_MENU_ID: &str = "settings";
pub const QUIT_MENU_ID: &str = "quit";
pub const INDICATOR_WINDOW_LABEL: &str = "indicator";

pub static GLOBAL_HOTKEYS_POOL: LazyLock<Vec<KeyCombination>> =
    LazyLock::new(|| load_hotkeys().expect("failed to load GLOBAL_HOTKEYS_POOL"));

fn load_hotkeys() -> anyhow::Result<Vec<KeyCombination>> {
    let data = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../data/global-hotkey-pool.jsonc"
    ));

    let parsed = jsonc_parser::parse_to_value(data, &ParseOptions::default())
        .context("failed to parse jsonc")?;

    let array = match parsed {
        Some(JsonValue::Array(arr)) => arr,
        _ => bail!("expected top-level array"),
    };

    let mut result = Vec::new();

    for (i, item) in array.into_iter().enumerate() {
        let object = match item {
            JsonValue::Object(obj) => obj,
            _ => bail!("item {i} is not an object"),
        };

        let key = object
            .get_string("key")
            .context(format!("item {i}: missing 'key'"))?
            .to_string();

        let modifiers_obj = object
            .get_object("mod")
            .context(format!("item {i}: missing 'mod'"))?;

        let parse_flag = |k: &str| -> anyhow::Result<bool> {
            let val = modifiers_obj
                .get_number(k)
                .with_context(|| format!("item {i}: missing 'mod.{k}'"))?;

            Ok(val == "1")
        };

        let modifiers = KeyCombinationModifiers {
            meta: parse_flag("m")?,
            ctrl: parse_flag("c")?,
            alt: parse_flag("a")?,
            shift: parse_flag("s")?,
        };

        result.push(KeyCombination {
            key: Key::from_str(&key)?,
            modifiers,
        });
    }

    Ok(result)
}
