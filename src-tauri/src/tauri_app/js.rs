use crate::chords::{press_shortcut, release_shortcut, Shortcut};
use rquickjs::{Context, Ctx, Error, Function, Module, Object, Runtime, Value};
use rquickjs::loader::{BuiltinLoader, BuiltinResolver, Loader, Resolver};
use rquickjs::module::Declared;
use std::collections::HashMap;

thread_local! {
    static JS: std::cell::RefCell<Option<(Runtime, Context)>> =
        std::cell::RefCell::new(None);
}

#[derive(Debug, Default)]
struct ModuleResolver {
    builtin_resolver: BuiltinResolver
}

impl Resolver for ModuleResolver {
    fn resolve<'js>(&mut self, ctx: &Ctx<'js>, base: &str, name: &str) -> rquickjs::Result<String> {
        match name {
            "fs" => Ok("fs".into()),
            "os" => Ok("os".into()),
            "util" => Ok("util".into()),
            "child_process" => Ok("child_process".into()),
            "process" => Ok("process".into()),
            "path" => Ok("path".into()),
            _ => self.builtin_resolver.resolve(ctx, base, name),
        }
    }
}

#[derive(Debug, Default)]
struct ModuleLoader {
    builtin_loader: BuiltinLoader
}

impl Loader for ModuleLoader {
    fn load<'js>(&mut self, ctx: &Ctx<'js>, name: &str) -> rquickjs::Result<Module<'js, Declared>> {
        match name {
            "fs" => Module::declare_def::<llrt_fs::FsModule, _>(ctx.clone(), "fs"),
            "os" => Module::declare_def::<llrt_os::OsModule, _>(ctx.clone(), "os"),
            "util" => Module::declare_def::<llrt_util::UtilModule, _>(ctx.clone(), "util"),
            "child_process" => Module::declare_def::<llrt_child_process::ChildProcessModule, _>(ctx.clone(), "child_process"),
            "process" => Module::declare_def::<llrt_process::ProcessModule, _>(ctx.clone(), "process"),
            "path" => Module::declare_def::<llrt_path::PathModule, _>(ctx.clone(), "path"),
            _ => self.builtin_loader.load(ctx, name),
        }
    }
}

pub fn with_js<F, R>(f: F) -> R
where
    F: FnOnce(&Ctx) -> R,
{
    JS.with(|cell| {
        let mut opt = cell.borrow_mut();

        if opt.is_none() {
            let rt = Runtime::new().unwrap();
            let ctx = Context::full(&rt).unwrap();
            let loader = ModuleLoader::default();
            let resolver = ModuleResolver::default();
            rt.set_loader(resolver, loader);

            ctx.with(|ctx| {
                init_globals(ctx).unwrap();
            });

            *opt = Some((rt, ctx));
        }

        let (_, ctx) = opt.as_ref().unwrap();
        ctx.with(|ctx| f(&ctx))
    })
}

fn throw_js_error(ctx: Ctx<'_>, message: impl Into<String>) -> Error {
    let message = message.into();

    // Build a real JS Error object so QuickJS can attach a stack.
    let thrown = (|| -> rquickjs::Result<Value<'_>> {
        let error_ctor: Function<'_> = ctx.globals().get("Error")?;
        error_ctor.call((message.as_str(),))
    })();

    match thrown {
        Ok(err_value) => ctx.throw(err_value),
        Err(_) => Error::new_into_js_message("Rust", "JavaScript", message),
    }
}

fn init_globals(ctx: Ctx<'_>) -> rquickjs::Result<()> {
    let globals = ctx.globals();

    // press
    {
        let press = Function::new(ctx.clone(), |ctx: Ctx<'_>, key: String| -> rquickjs::Result<()> {
            let shortcut = Shortcut::parse(&key).map_err(|err| {
                throw_js_error(ctx.clone(), format!("Invalid shortcut {key:?}: {err}"))
            })?;

            press_shortcut(shortcut).map_err(|err| {
                throw_js_error(ctx.clone(), format!("press({key:?}) failed: {err}"))
            })?;

            Ok(())
        })?
            .with_name("press")?;

        globals.set("press", press)?;
    }

    // release
    {
        let release =
            Function::new(ctx.clone(), |ctx: Ctx<'_>, key: String| -> rquickjs::Result<()> {
                let shortcut = Shortcut::parse(&key).map_err(|err| {
                    throw_js_error(ctx.clone(), format!("Invalid shortcut {key:?}: {err}"))
                })?;

                release_shortcut(shortcut).map_err(|err| {
                    throw_js_error(ctx.clone(), format!("release({key:?}) failed: {err}"))
                })?;

                Ok(())
            })?
                .with_name("release")?;

        globals.set("release", release)?;
    }

    // tap
    {
        let tap = Function::new(ctx.clone(), |ctx: Ctx<'_>, key: String| -> rquickjs::Result<()> {
            let shortcut = Shortcut::parse(&key).map_err(|err| {
                throw_js_error(ctx.clone(), format!("Invalid shortcut {key:?}: {err}"))
            })?;

            press_shortcut(shortcut.clone()).map_err(|err| {
                throw_js_error(ctx.clone(), format!("tap({key:?}) press failed: {err}"))
            })?;

            release_shortcut(shortcut).map_err(|err| {
                throw_js_error(ctx.clone(), format!("tap({key:?}) release failed: {err}"))
            })?;

            Ok(())
        })?
            .with_name("tap")?;

        globals.set("tap", tap)?;
    }

    Ok(())
}

pub fn format_js_error(ctx: Ctx, err: Error) -> String {
    match err {
        Error::Exception => {
            let exception: Value = ctx.catch();

            if let Ok(obj) = Object::from_value(exception.clone()) {
                let message: Option<String> = obj.get("message").ok();
                let stack: Option<String> = obj.get("stack").ok();

                match (message, stack) {
                    (Some(msg), Some(stack)) => format!("{}\n{}", msg, stack),
                    (Some(msg), None) => msg,
                    _ => format!("{:?}", exception),
                }
            } else {
                format!("{:?}", exception)
            }
        }
        _ => err.to_string(),
    }
}