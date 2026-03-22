use crate::tauri_app::app_lifecycle::{
    init as init_app_lifecycle, register_app_launch_handler, register_app_terminate_handler,
};
use crate::chords::{press_shortcut, release_shortcut, Shortcut};
use crate::constants::GLOBAL_HOTKEYS_POOL;
use crate::js::{throw_js_error, AppUserData};
use crate::store::{GlobalHotkeyStore, GlobalHotkeyStoreEntry};
use rquickjs::module::{Declarations, Exports, ModuleDef};
use rquickjs::{Ctx, Function};
use std::collections::HashSet;
use tauri_plugin_store::StoreExt;

pub struct ChordsappModule;

fn on_app_launch<'js>(
    ctx: Ctx<'js>,
    bundle_id: String,
    callback: Function<'js>,
) -> rquickjs::Result<()> {
    register_app_launch_handler(ctx, bundle_id, callback)
}

fn on_app_terminate<'js>(
    ctx: Ctx<'js>,
    bundle_id: String,
    callback: Function<'js>,
) -> rquickjs::Result<()> {
    register_app_terminate_handler(ctx, bundle_id, callback)
}

impl ModuleDef for ChordsappModule {
    fn declare(declare: &Declarations) -> rquickjs::Result<()> {
        declare.declare("press")?;
        declare.declare("release")?;
        declare.declare("tap")?;
        declare.declare("getGlobalHotkey")?;
        declare.declare("registerGlobalHotkey")?;
        declare.declare("onAppLaunch")?;
        declare.declare("onAppTerminate")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> rquickjs::Result<()> {
        let userdata = ctx.userdata::<AppUserData>().unwrap();
        let handle = &userdata.handle;
        init_app_lifecycle(handle.clone());

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

        let global_hotkeys_store =
            GlobalHotkeyStore::new(handle.store("global-hotkeys.json").map_err(|err| {
                throw_js_error(
                    ctx.clone(),
                    format!("failed to open global hotkeys store: {err}"),
                )
            })?);

        let register_global_hotkey_store = global_hotkeys_store.clone();
        let register_global_hotkey = Function::new(
            ctx.clone(),
            move |_ctx: Ctx<'_>,
                  bundle_id: String,
                  hotkey_id: String|
                  -> rquickjs::Result<Option<String>> {
                let all = register_global_hotkey_store.entries();

                // idempotent: if this hotkey is already registered, return the existing shortcut
                if let Some(existing) = all.iter().find_map(|(shortcut, entry)| {
                    (entry.bundle_id == bundle_id && entry.hotkey_id == hotkey_id)
                        .then_some(shortcut.clone())
                }) {
                    return Ok(Some(existing));
                }

                let used: HashSet<String> = all.into_keys().collect();

                let Some(next) = GLOBAL_HOTKEYS_POOL
                    .iter()
                    .find(|shortcut| !used.contains(&shortcut.serialize()))
                    .cloned()
                else {
                    return Ok(None);
                };

                let shortcut = next.serialize();

                register_global_hotkey_store.set(
                    &shortcut,
                    GlobalHotkeyStoreEntry {
                        bundle_id,
                        hotkey_id,
                    },
                );

                Ok(Some(shortcut))
            },
        )?
        .with_name("registerGlobalHotkey")?;
        exports.export("registerGlobalHotkey", register_global_hotkey)?;

        let get_global_hotkey_store = global_hotkeys_store.clone();
        let get_global_hotkey =
            Function::new(
                ctx.clone(),
                move |_ctx: Ctx<'_>,
                      bundle_id: String,
                      hotkey_id: String|
                      -> rquickjs::Result<Option<String>> {
                    let shortcut = get_global_hotkey_store.entries().into_iter().find_map(
                        |(shortcut, entry)| {
                            (entry.bundle_id == bundle_id && entry.hotkey_id == hotkey_id)
                                .then_some(shortcut)
                        },
                    );

                    Ok(shortcut)
                },
            )?
            .with_name("getGlobalHotkey")?;
        exports.export("getGlobalHotkey", get_global_hotkey)?;

        let on_app_launch = Function::new(ctx.clone(), on_app_launch)?.with_name("onAppLaunch")?;
        exports.export("onAppLaunch", on_app_launch)?;

        let on_app_terminate =
            Function::new(ctx.clone(), on_app_terminate)?.with_name("onAppTerminate")?;
        exports.export("onAppTerminate", on_app_terminate)?;

        Ok(())
    }
}
