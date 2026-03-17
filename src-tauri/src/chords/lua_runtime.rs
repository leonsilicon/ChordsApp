use mlua::{Lua, LuaOptions, StdLib};
use crate::chords::{press_shortcut, release_shortcut, Shortcut};
use anyhow::Result;

pub struct ChordLuaRuntime {
    pub lua: Lua,
    pub lua_init_scripts: Vec<String>,
}

impl ChordLuaRuntime {
    pub fn new(init_scripts: Vec<String>) -> Result<Self> {
        unsafe {
            let lua = Lua::unsafe_new_with(
                StdLib::ALL,
                LuaOptions::default()
            );
            let globals = lua.globals();

            globals.set("press", lua.create_function(|_, key: String| {
                press_shortcut(parse_shortcut(&key)?).map_err(lua_err)
            })?)?;

            globals.set("release", lua.create_function(|_, key: String| {
                release_shortcut(parse_shortcut(&key)?).map_err(lua_err)
            })?)?;

            globals.set("tap", lua.create_function(|_, key: String| {
                let shortcut = parse_shortcut(&key)?;
                press_shortcut(shortcut.clone()).map_err(lua_err)?;
                release_shortcut(shortcut).map_err(lua_err)
            })?)?;

            for init_script in &init_scripts {
                lua.load(init_script).exec()?;
            }

            Ok(Self { lua, lua_init_scripts: init_scripts })
        }
    }
}

fn lua_err(msg: impl std::fmt::Debug) -> mlua::Error {
    mlua::Error::RuntimeError(format!("{msg:?}"))
}

fn parse_shortcut(s: &str) -> mlua::Result<Shortcut> {
    Shortcut::parse(s).map_err(|_| mlua::Error::RuntimeError(format!("unknown key: {s}")))
}
