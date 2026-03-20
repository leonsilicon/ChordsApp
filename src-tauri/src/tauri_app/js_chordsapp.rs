use crate::chords::{press_shortcut, release_shortcut, Shortcut};
use crate::js::throw_js_error;
use rquickjs::module::{Declarations, Exports, ModuleDef};
use rquickjs::{Ctx, Function, Value};

pub struct ChordsappModule;

impl ModuleDef for ChordsappModule {
    fn declare(declare: &Declarations) -> rquickjs::Result<()> {
        declare.declare("press")?;
        declare.declare("release")?;
        declare.declare("tap")?;
        declare.declare("getGlobalHotkey")?;
        declare.declare("registerGlobalHotkey")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> rquickjs::Result<()> {
        let press = Function::new(
            ctx.clone(),
            |ctx: Ctx<'_>, key: String| -> rquickjs::Result<()> {
                let shortcut = Shortcut::parse(&key).map_err(|err| {
                    throw_js_error(ctx.clone(), format!("Invalid shortcut {key:?}: {err}"))
                })?;

                press_shortcut(shortcut, 1).map_err(|err| {
                    throw_js_error(ctx.clone(), format!("press({key:?}) failed: {err}"))
                })?;

                Ok(())
            },
        )?
        .with_name("press")?;
        exports.export("press", press)?;

        let release = Function::new(
            ctx.clone(),
            |ctx: Ctx<'_>, key: String| -> rquickjs::Result<()> {
                let shortcut = Shortcut::parse(&key).map_err(|err| {
                    throw_js_error(ctx.clone(), format!("Invalid shortcut {key:?}: {err}"))
                })?;

                release_shortcut(shortcut).map_err(|err| {
                    throw_js_error(ctx.clone(), format!("release({key:?}) failed: {err}"))
                })?;

                Ok(())
            },
        )?
        .with_name("release")?;
        exports.export("release", release)?;

        let tap = Function::new(
            ctx.clone(),
            |ctx: Ctx<'_>, key: String| -> rquickjs::Result<()> {
                let shortcut = Shortcut::parse(&key).map_err(|err| {
                    crate::tauri_app::js::throw_js_error(
                        ctx.clone(),
                        format!("Invalid shortcut {key:?}: {err}"),
                    )
                })?;

                press_shortcut(shortcut.clone(), 1).map_err(|err| {
                    crate::tauri_app::js::throw_js_error(
                        ctx.clone(),
                        format!("tap({key:?}) press failed: {err}"),
                    )
                })?;

                release_shortcut(shortcut).map_err(|err| {
                    crate::tauri_app::js::throw_js_error(
                        ctx.clone(),
                        format!("tap({key:?}) release failed: {err}"),
                    )
                })?;

                Ok(())
            },
        )?
        .with_name("tap")?;
        exports.export("tap", tap)?;

        let register_global_hotkey = Function::new(
            ctx.clone(),
            |ctx: Ctx<'_>, bundle_id: String, hotkey_id: String| -> rquickjs::Result<()> { Ok(()) },
        )?
        .with_name("registerGlobalHotkey")?;
        exports.export("registerGlobalHotkey", register_global_hotkey)?;

        let get_global_hotkey = Function::new(
            ctx.clone(),
            |ctx: Ctx<'_>, bundle_id: String, hotkey_id: String| -> rquickjs::Result<()> {
                Ok(())
            },
        )?
        .with_name("getGlobalHotkey")?;
        exports.export("getGlobalHotkey", get_global_hotkey)?;

        Ok(())
    }
}
