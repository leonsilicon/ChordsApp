use rquickjs::{loader::{BuiltinLoader, BuiltinResolver, Loader, Resolver}, module::Declared, AsyncContext, AsyncRuntime, Ctx, Error, Function, JsLifetime, Module, Object, Value};
use std::{cell::RefCell, future::Future, pin::Pin};
use rquickjs::class::{Trace, Tracer};
use tauri::{
    async_runtime::{block_on, channel},
    AppHandle,
};
use include_json::include_json;
use minijinja::context;
use std::sync::LazyLock;
use rquickjs::module::{Declarations, Exports, ModuleDef};
use crate::tauri_app::js_chordsapp::ChordsappModule;

const BUILTIN_MODULES: LazyLock<Vec<String>> = LazyLock::new(|| {
  let json: serde_json::Value = include_json!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../data/builtin-modules.json"
  ));
  serde_json::from_value(json).expect("failed to parse builtin-modules.json")
});

struct JsEngine {
    // Keep the runtime alive for as long as the context exists.
    _rt: AsyncRuntime,
    ctx: AsyncContext,
}

thread_local! {
    static JS_ENGINE: RefCell<Option<JsEngine>> = RefCell::new(None);
    // static JS_WORKER: RefCell<Option<MainWorker>> = RefCell::new(None);
}

pub struct AppUserData {
    pub handle: AppHandle,
}

// This tells rquickjs "this type does not contain JS references"
unsafe impl<'js> JsLifetime<'js> for AppUserData {
    type Changed<'to> = AppUserData;
}

// Usually safe because AppHandle doesn't hold JS values
impl<'js> Trace<'js> for AppUserData {
    fn trace(&self, _tracer: Tracer<'_, 'js>) {}
}

#[derive(Debug, Default)]
struct ModuleResolver {
    builtin_resolver: BuiltinResolver,
}

impl Resolver for ModuleResolver {
    fn resolve<'js>(&mut self, ctx: &Ctx<'js>, base: &str, name: &str) -> rquickjs::Result<String> {
      let name = name.trim_start_matches("node:").trim_end_matches("/");
      if BUILTIN_MODULES.contains(&name.to_string()) {
        return Ok(name.into());
      }

      match name {
        "chordsapp" => Ok("chordsapp".into()),
        _ => self.builtin_resolver.resolve(ctx, base, name),
      }
    }
}

#[derive(Debug, Default)]
struct ModuleLoader {
    builtin_loader: BuiltinLoader,
}


pub struct ModuleModule;

impl ModuleDef for ModuleModule {
    fn declare(declare: &Declarations) -> rquickjs::Result<()> {
        declare.declare("createRequire")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> rquickjs::Result<()> {
        let global = ctx.globals();
        let create_require: Value<'js> = global.get("createRequire")?;
        exports.export("createRequire", create_require)?;
        Ok(())
    }
}

fn get_module<'js>(ctx: &Ctx<'js>, name: &str) -> rquickjs::Result<Option<Module<'js, Declared>>> {
    println!("name: {}", name);
    let name = name.trim_start_matches("node:").trim_end_matches("/");
    let module = match name {
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
        "console" => {
            Module::declare_def::<llrt_console::ConsoleModule, _>(ctx.clone(), "console")
        }
        "buffer" => Module::declare_def::<llrt_buffer::BufferModule, _>(ctx.clone(), "buffer"),
        "module" => Module::declare_def::<ModuleModule, _>(ctx.clone(), "module"),
        "chordsapp" => Module::declare_def::<ChordsappModule, _>( ctx.clone(), "chordsapp"),
        // "crypto" => Module::declare_def::<llrt_crypto::CryptoModule, _>(ctx.clone(), "crypto"),
        _ => return Ok(None)
    };

    Some(module).transpose()
}

impl Loader for ModuleLoader {
    fn load<'js>(&mut self, ctx: &Ctx<'js>, name: &str) -> rquickjs::Result<Module<'js, Declared>> {
        let module = get_module(ctx, name)?;
        Ok(match module {
            Some(module) => module,
            None => self.builtin_loader.load(ctx, name)?
        })
    }
}

async fn ensure_engine(handle: AppHandle) -> Result<AsyncContext, String> {
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

    ctx.with(|ctx| init_globals(ctx, handle.clone()))
        .await
        .map_err(|err| format_js_error_fallback(err))?;

    let out = ctx.clone();

    // Deno makes the app super slow
    // JS_WORKER.with(move |cell| {
    //     *cell.borrow_mut() = Some(main_worker);
    // });

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

    handle.clone()
        .run_on_main_thread(move || {
            let result = block_on(async move {
                let async_ctx: AsyncContext = ensure_engine(handle).await?;

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

pub fn throw_any_js_error(ctx: Ctx<'_>, err: Error) -> Error {
    match err {
        Error::Exception => {
            let value = ctx.catch();

            if let Some(ex) = value.as_exception() {
                let name = ex.get::<_, String>("name").unwrap_or_else(|_| "Error".into());
                let message = ex
                    .get::<_, String>("message")
                    .unwrap_or_else(|_| "Unknown JS error".into());
                let stack = ex
                    .get::<_, String>("stack")
                    .unwrap_or_else(|_| "No stack".into());

                throw_js_error(ctx, format!("{name}: {message}\n{stack}"))
            } else {
                let rendered = ctx
                    .json_stringify(value.clone())
                    .ok()
                    .flatten()
                    .and_then(|s| s.to_string().ok())
                    .unwrap_or_else(|| "<non-serializable thrown value>".to_string());

                throw_js_error(ctx, format!("Thrown JS value: {rendered}"))
            }
        }
        _ => throw_js_error(ctx, err.to_string()),
    }
}

pub fn throw_js_error(ctx: Ctx<'_>, message: impl Into<String>) -> Error {

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

fn init_globals(ctx: Ctx<'_>, handle: AppHandle) -> rquickjs::Result<()> {
    llrt_process::init(&ctx)?;
    llrt_console::init(&ctx)?;
    llrt_buffer::init(&ctx)?;

    let quickjs_require_template = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../data/quickjs-require.jinja.js"));
    let env = minijinja::Environment::new();
    let quickjs_require_js = env.render_str(
        quickjs_require_template,
        context!(builtinModules => &*BUILTIN_MODULES),
    ).map_err(|err| throw_js_error(ctx.clone(), err.to_string()))?;
    log::debug!("Evaluating quickjs-require.js:\n{}", quickjs_require_js);
    if let Err(err) = Module::evaluate(ctx.clone(), "require", quickjs_require_js.as_bytes()) {
        log::error!("Failed to evaluate quickjs-require.js: {}", format_js_error(ctx.clone(), err));
    }

    ctx.store_userdata(AppUserData { handle })?;

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
