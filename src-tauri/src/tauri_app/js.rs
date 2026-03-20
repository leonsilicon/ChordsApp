use crate::tauri_app::js_chordsapp::ChordsappModule;
use include_json::include_json;
use rquickjs::async_with;
use rquickjs::class::{Trace, Tracer};
use rquickjs::{
    loader::{Loader, Resolver},
    module::Declared,
    AsyncContext, AsyncRuntime, Ctx, Error, Function, JsLifetime, Module, Object, Value,
};
use std::sync::LazyLock;
use std::{cell::RefCell, future::Future, pin::Pin};
use tauri::{
    async_runtime::{block_on, channel},
    AppHandle,
};

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
    llrt_resolver: llrt_modules::module::resolver::ModuleResolver,
}

impl ModuleResolver {
    pub fn new(llrt_resolver: llrt_modules::module::resolver::ModuleResolver) -> Self {
        Self { llrt_resolver }
    }
}

impl Resolver for ModuleResolver {
    fn resolve<'js>(&mut self, ctx: &Ctx<'js>, base: &str, name: &str) -> rquickjs::Result<String> {
        // `.` from `.js`
        if (name.contains(".") || name == "chordsapp") {
            return Ok(name.into());
        }

        self.llrt_resolver.resolve(ctx, base, name)
    }
}

#[derive(Debug, Default)]
struct ModuleLoader {
    llrt_loader: llrt_modules::module::loader::ModuleLoader,
}

impl ModuleLoader {
    pub fn new(llrt_loader: llrt_modules::module::loader::ModuleLoader) -> Self {
        Self { llrt_loader }
    }
}

fn get_module<'js>(ctx: &Ctx<'js>, name: &str) -> rquickjs::Result<Option<Module<'js, Declared>>> {
    println!("name: {}", name);
    let module = match name {
        "chordsapp" => Module::declare_def::<ChordsappModule, _>(ctx.clone(), "chordsapp"),
        _ => return Ok(None),
    };

    Some(module).transpose()
}

impl Loader for ModuleLoader {
    fn load<'js>(&mut self, ctx: &Ctx<'js>, name: &str) -> rquickjs::Result<Module<'js, Declared>> {
        let module = get_module(ctx, name)?;
        Ok(match module {
            Some(module) => module,
            None => self.llrt_loader.load(ctx, name)?,
        })
    }
}

async fn ensure_engine(handle: AppHandle) -> anyhow::Result<AsyncContext> {
    let existing = JS_ENGINE.with(|cell| cell.borrow().as_ref().map(|engine| engine.ctx.clone()));
    if let Some(ctx) = existing {
        return Ok(ctx);
    }

    let rt = AsyncRuntime::new()?;
    let module_builder = llrt_modules::module_builder::ModuleBuilder::default()
        .with_global(llrt_core::modules::embedded::init)
        .with_global(llrt_core::builtins_inspect::init);
    let (llrt_module_resolver, llrt_module_loader, global_attachment) = module_builder.build();
    let module_resolver = ModuleResolver::new(llrt_module_resolver);
    let resolver = (
        module_resolver,
        llrt_core::embedded::resolver::EmbeddedResolver,
        llrt_core::package::resolver::PackageResolver,
    );
    let module_loader = ModuleLoader::new(llrt_module_loader);
    let loader = (
        module_loader,
        llrt_core::embedded::loader::EmbeddedLoader,
        llrt_core::package::loader::PackageLoader,
    );

    rt.set_loader(resolver, loader).await;

    let context = AsyncContext::full(&rt).await?;
    async_with!(context => |ctx| {
        global_attachment.attach(&ctx)?;
        ctx.store_userdata(AppUserData { handle })?;

        Ok::<_, Error>(())
    })
    .await?;

    // Deno makes the app super slow
    // JS_WORKER.with(move |cell| {
    //     *cell.borrow_mut() = Some(main_worker);
    // });

    let out = context.clone();
    JS_ENGINE.with(|cell| {
        *cell.borrow_mut() = Some(JsEngine {
            _rt: rt,
            ctx: context,
        });
    });

    Ok(out)
}

type LocalBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;
type SendBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

unsafe fn uplift<'a, 'b, T>(fut: LocalBoxFuture<'a, T>) -> SendBoxFuture<'b, T> {
    std::mem::transmute(fut)
}

pub async fn with_js<F, R>(handle: AppHandle, f: F) -> anyhow::Result<R>
where
    F: Send + 'static + for<'js> FnOnce(Ctx<'js>) -> LocalBoxFuture<'js, anyhow::Result<R>>,
    R: Send + 'static,
{
    let (tx, mut rx) = channel(1);

    handle.clone().run_on_main_thread(move || {
        let result = block_on(async move {
            let async_ctx: AsyncContext = ensure_engine(handle).await?;

            async_ctx
                .async_with(|ctx| {
                    let fut = f(ctx.clone());
                    let fut = Box::pin(async move { fut.await });
                    unsafe { uplift(fut) }
                })
                .await
        });

        let _ = tx.try_send(result);
    })?;

    rx.recv()
        .await
        .ok_or_else(|| anyhow::anyhow!("main thread task dropped"))?
}

pub fn throw_any_js_error(ctx: Ctx<'_>, err: Error) -> Error {
    match err {
        Error::Exception => {
            let value = ctx.catch();

            if let Some(ex) = value.as_exception() {
                let name = ex
                    .get::<_, String>("name")
                    .unwrap_or_else(|_| "Error".into());
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
