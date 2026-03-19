use crate::chords::{press_shortcut, release_shortcut, Shortcut};
use rquickjs::{
    loader::{BuiltinLoader, BuiltinResolver, Loader, Resolver},
    module::Declared,
    AsyncContext, AsyncRuntime, Ctx, Error, Function, Module, Object, Value,
};
use std::{cell::RefCell, future::Future, pin::Pin};
use tauri::{
    async_runtime::{block_on, channel},
    AppHandle,
};

struct JsEngine {
    // Keep the runtime alive for as long as the context exists.
    _rt: AsyncRuntime,
    ctx: AsyncContext,
}

thread_local! {
    static JS_ENGINE: RefCell<Option<JsEngine>> = RefCell::new(None);
}

#[derive(Debug, Default)]
struct ModuleResolver {
    builtin_resolver: BuiltinResolver,
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
            "console" => Ok("console".into()),
            "buffer" => Ok("buffer".into()),
            "crypto" => Ok("crypto".into())
            _ => Ok(name.into()),
            // _ => self.builtin_resolver.resolve(ctx, base, name),
        }
    }
}

#[derive(Debug, Default)]
struct ModuleLoader {
    builtin_loader: BuiltinLoader,
}

impl Loader for ModuleLoader {
    fn load<'js>(&mut self, ctx: &Ctx<'js>, name: &str) -> rquickjs::Result<Module<'js, Declared>> {
        match name {
            "fs" => Module::declare_def::<llrt_fs::FsModule, _>(ctx.clone(), "fs"),
            "os" => Module::declare_def::<llrt_os::OsModule, _>(ctx.clone(), "os"),
            "util" => Module::declare_def::<llrt_util::UtilModule, _>(ctx.clone(), "util"),
            "child_process" => Module::declare_def::<llrt_child_process::ChildProcessModule, _>(
                ctx.clone(),
                "child_process",
            ),
            "process" => {
                Module::declare_def::<llrt_process::ProcessModule, _>(ctx.clone(), "process")
            }
            "path" => Module::declare_def::<llrt_path::PathModule, _>(ctx.clone(), "path"),
            "console" => Module::declare_def::<llrt_console::ConsoleModule, _>(ctx.clone(), "console"),
            "buffer" => Module::declare_def::<llrt_buffer::BufferModule, _>(ctx.clone(), "buffer"),
            "crypto" => Module::declare_def::<llrt_crypto::CryptoModule, _>(ctx.clone(), "crypto"),
            _ => self.builtin_loader.load(ctx, name),
        }
    }
}

async fn ensure_engine() -> Result<AsyncContext, String> {
    let existing = JS_ENGINE.with(|cell| cell.borrow().as_ref().map(|engine| engine.ctx.clone()));
    if let Some(ctx) = existing {
        return Ok(ctx);
    }

    let rt = AsyncRuntime::new().map_err(|err| err.to_string())?;
    rt.set_loader(ModuleResolver::default(), ModuleLoader::default())
        .await;

    let ctx = AsyncContext::full(&rt)
        .await
        .map_err(|err| err.to_string())?;

    ctx.with(init_globals)
        .await
        .map_err(|err| format_js_error_fallback(err))?;

    let out = ctx.clone();

    JS_ENGINE.with(|cell| {
        *cell.borrow_mut() = Some(JsEngine { _rt: rt, ctx });
    });

    Ok(out)
}

type LocalBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;
type SendBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

unsafe fn uplift<'a, 'b, T>(fut: LocalBoxFuture<'a, T>) -> SendBoxFuture<'b, T> {
    std::mem::transmute(fut)
}

pub async fn with_js<F, R>(handle: AppHandle, f: F) -> Result<R, String>
where
    F: Send + 'static + for<'js> FnOnce(Ctx<'js>) -> LocalBoxFuture<'js, rquickjs::Result<R>>,
    R: Send + 'static,
{
    let (tx, mut rx) = channel(1);

    handle
        .run_on_main_thread(move || {
            let result = block_on(async move {
                let async_ctx: AsyncContext = ensure_engine().await?;

                async_ctx
                    .async_with(|ctx| {
                        let fut = f(ctx.clone());

                        let fut =
                            Box::pin(async move { fut.await.map_err(|e| format_js_error(ctx, e)) });

                        unsafe { uplift(fut) }
                    })
                    .await
            });

            let _ = tx.try_send(result);
        })
        .map_err(|e| e.to_string())?;

    rx.recv()
        .await
        .ok_or_else(|| "main thread task dropped".to_string())?
}

fn throw_js_error(ctx: Ctx<'_>, message: impl Into<String>) -> Error {
    let message = message.into();

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
    llrt_process::init(&ctx)?;
    llrt_console::init(&ctx)?;
    llrt_buffer::init(&ctx)?;

    {
        let press = Function::new(
            ctx.clone(),
            |ctx: Ctx<'_>, key: String| -> rquickjs::Result<()> {
                let shortcut = Shortcut::parse(&key).map_err(|err| {
                    throw_js_error(ctx.clone(), format!("Invalid shortcut {key:?}: {err}"))
                })?;

                press_shortcut(shortcut).map_err(|err| {
                    throw_js_error(ctx.clone(), format!("press({key:?}) failed: {err}"))
                })?;

                Ok(())
            },
        )?
        .with_name("press")?;

        globals.set("press", press)?;
    }

    {
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

        globals.set("release", release)?;
    }

    {
        let tap = Function::new(
            ctx.clone(),
            |ctx: Ctx<'_>, key: String| -> rquickjs::Result<()> {
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
            },
        )?
        .with_name("tap")?;

        globals.set("tap", tap)?;
    }

    Ok(())
}

pub fn format_js_error(ctx: Ctx<'_>, err: Error) -> String {
    match err {
        Error::Exception => {
            let exception: Value<'_> = ctx.catch();

            if let Ok(obj) = Object::from_value(exception.clone()) {
                let message: Option<String> = obj.get("message").ok();
                let stack: Option<String> = obj.get("stack").ok();

                match (message, stack) {
                    (Some(msg), Some(stack)) => format!("{msg}\n{stack}"),
                    (Some(msg), None) => msg,
                    _ => format!("{exception:?}"),
                }
            } else {
                format!("{exception:?}")
            }
        }
        _ => err.to_string(),
    }
}

fn format_js_error_fallback(err: Error) -> String {
    err.to_string()
}
