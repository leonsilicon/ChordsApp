use mlua::{Lua, LuaOptions, StdLib};
use crate::chords::{press_shortcut, release_shortcut, AppChordMapValue, AppChordsFile, Shortcut};
use anyhow::Result;
use fast_radix_trie::StringRadixSet;
use bracoxide::explode;
use gix::hashtable::HashMap;

pub struct ChordLuaRuntime {
    pub lua_init_scripts: Vec<String>,
}

impl ChordLuaRuntime {
    pub fn new(init_scripts: Vec<String>, chords_file: AppChordsFile) -> Result<Self> {
        unsafe {

            Ok(Self { lua, lua_init_scripts: init_scripts })
        }
    }
}

fn lua_err(msg: impl std::fmt::Debug) -> mlua::Error {
    mlua::Error::RuntimeError(format!("{msg:?}"))
}



fn add_lua_loader(lua: &Lua, base_dir: String) -> Result<()> {
    let globals = lua.globals();
    let package: Table = globals.get("package")?;
    let searchers: Table = package.get("searchers")?;

    // insert at position 1 (highest priority)
    let loader = lua.create_function(move |lua, module_name: String| {
        let module_path = module_name.replace(".", "/");

        let candidates = [
            format!("{}/{}.lua", base_dir, module_path),
            format!("{}/{}/init.lua", base_dir, module_path),
        ];

        for path in candidates {
            if let Ok(code) = std::fs::read_to_string(&path) {
                let chunk = lua.load(&code).set_name(&path)?;
                let func = chunk.into_function()?;
                return Ok(Value::Function(func));
            }
        }

        // return error string (Lua expects this)
        Ok(Value::String(lua.create_string(&format!(
            "\n\tno module '{}' in {}",
            module_name, base_dir
        ))?))
    })?;

    // shift existing searchers down
    let len = searchers.raw_len();
    for i in (1..=len).rev() {
        let val: Value = searchers.raw_get(i)?;
        searchers.raw_set(i + 1, val)?;
    }

    searchers.raw_set(1, loader)?;

    Ok(())
}

