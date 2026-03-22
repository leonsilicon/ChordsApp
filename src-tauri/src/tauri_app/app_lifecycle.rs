use crate::tauri_app::js::{format_js_error, with_js};
use anyhow::Result;
use rquickjs::{Ctx, Function, Persistent, Promise, Value};
use serde::Serialize;
use std::cell::RefCell;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
static HAS_LAUNCH_CALLBACKS: AtomicBool = AtomicBool::new(false);
static HAS_TERMINATE_CALLBACKS: AtomicBool = AtomicBool::new(false);

thread_local! {
    static APP_LIFECYCLE_CALLBACKS: RefCell<AppLifecycleCallbacks> =
        RefCell::new(AppLifecycleCallbacks::default());
}

#[derive(Default)]
struct AppLifecycleCallbacks {
    launch: Vec<AppLifecycleCallbackEntry>,
    terminate: Vec<AppLifecycleCallbackEntry>,
}

#[derive(Clone)]
struct AppLifecycleCallbackEntry {
    bundle_id: String,
    callback: Persistent<Function<'static>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservedApp {
    pub pid: i32,
    pub bundle_id: String,
}

pub fn init(handle: AppHandle) {
    let _ = APP_HANDLE.set(handle);

    #[cfg(target_os = "macos")]
    init_macos_observers();
}

pub fn register_app_launch_handler<'js>(
    ctx: Ctx<'js>,
    bundle_id: String,
    callback: Function<'js>,
) -> rquickjs::Result<()> {
    APP_LIFECYCLE_CALLBACKS.with(|callbacks| {
        callbacks.borrow_mut().launch.push(AppLifecycleCallbackEntry {
            bundle_id,
            callback: Persistent::save(&ctx, callback),
        });
    });
    HAS_LAUNCH_CALLBACKS.store(true, Ordering::SeqCst);

    Ok(())
}

pub fn register_app_terminate_handler<'js>(
    ctx: Ctx<'js>,
    bundle_id: String,
    callback: Function<'js>,
) -> rquickjs::Result<()> {
    APP_LIFECYCLE_CALLBACKS.with(|callbacks| {
        callbacks
            .borrow_mut()
            .terminate
            .push(AppLifecycleCallbackEntry {
                bundle_id,
                callback: Persistent::save(&ctx, callback),
            });
    });
    HAS_TERMINATE_CALLBACKS.store(true, Ordering::SeqCst);

    Ok(())
}

pub fn clear_callbacks() {
    APP_LIFECYCLE_CALLBACKS.with(|callbacks| {
        let mut callbacks = callbacks.borrow_mut();
        callbacks.launch.clear();
        callbacks.terminate.clear();
    });
    HAS_LAUNCH_CALLBACKS.store(false, Ordering::SeqCst);
    HAS_TERMINATE_CALLBACKS.store(false, Ordering::SeqCst);
}

pub fn dispatch_app_launch(app: ObservedApp) {
    if !HAS_LAUNCH_CALLBACKS.load(Ordering::SeqCst) {
        return;
    }

    let Some(handle) = APP_HANDLE.get().cloned() else {
        return;
    };

    tauri::async_runtime::spawn(async move {
        if let Err(error) = with_js(handle, move |ctx| Box::pin(invoke_launch_callbacks(ctx, app)))
            .await
        {
            log::error!("Failed to run app launch callbacks: {error}");
        }
    });
}

pub fn dispatch_app_terminate(app: ObservedApp) {
    if !HAS_TERMINATE_CALLBACKS.load(Ordering::SeqCst) {
        return;
    }

    let Some(handle) = APP_HANDLE.get().cloned() else {
        return;
    };

    tauri::async_runtime::spawn(async move {
        if let Err(error) =
            with_js(handle, move |ctx| Box::pin(invoke_terminate_callbacks(ctx, app))).await
        {
            log::error!("Failed to run app terminate callbacks: {error}");
        }
    });
}

async fn invoke_launch_callbacks<'js>(ctx: Ctx<'js>, app: ObservedApp) -> Result<()> {
    let callbacks = APP_LIFECYCLE_CALLBACKS.with(|callbacks| callbacks.borrow().launch.clone());

    for callback in callbacks {
        if callback.bundle_id != app.bundle_id {
            continue;
        }

        let callback = callback
            .callback
            .restore(&ctx)
            .map_err(|error| anyhow::anyhow!(format_js_error(ctx.clone(), error)))?;
        let js_app = rquickjs_serde::to_value(ctx.clone(), app.clone())
            .map_err(|error| anyhow::anyhow!("Failed to serialize app launch payload: {error}"))?;
        let result: Value<'js> = callback
            .call((js_app,))
            .map_err(|error| anyhow::anyhow!(format_js_error(ctx.clone(), error)))?;

        if let Err(error) = await_promise_if_needed(result).await {
            log::error!(
                "App launch callback promise rejected: {}",
                format_js_error(ctx.clone(), error)
            );
        }
    }

    Ok(())
}

async fn invoke_terminate_callbacks<'js>(ctx: Ctx<'js>, app: ObservedApp) -> Result<()> {
    let callbacks = APP_LIFECYCLE_CALLBACKS.with(|callbacks| callbacks.borrow().terminate.clone());

    for callback in callbacks {
        if callback.bundle_id != app.bundle_id {
            continue;
        }

        let callback = callback
            .callback
            .restore(&ctx)
            .map_err(|error| anyhow::anyhow!(format_js_error(ctx.clone(), error)))?;
        let js_app = rquickjs_serde::to_value(ctx.clone(), app.clone()).map_err(|error| {
            anyhow::anyhow!("Failed to serialize app terminate payload: {error}")
        })?;
        let result: Value<'js> = callback
            .call((js_app,))
            .map_err(|error| anyhow::anyhow!(format_js_error(ctx.clone(), error)))?;

        if let Err(error) = await_promise_if_needed(result).await {
            log::error!(
                "App terminate callback promise rejected: {}",
                format_js_error(ctx.clone(), error)
            );
        }
    }

    Ok(())
}

async fn await_promise_if_needed<'js>(result: Value<'js>) -> rquickjs::Result<()> {
    if !result.is_promise() {
        return Ok(());
    }

    let promise = Promise::from_value(result)?;
    promise.into_future::<Value<'js>>().await.map(|_| ())
}

#[cfg(target_os = "macos")]
mod macos {
    use super::{ObservedApp, dispatch_app_launch, dispatch_app_terminate};
    use block2::RcBlock;
    use core::ptr::NonNull;
    use objc2::MainThreadMarker;
    use objc2::runtime::AnyObject;
    use objc2_app_kit::{
        NSRunningApplication, NSWorkspace, NSWorkspaceApplicationKey,
        NSWorkspaceDidLaunchApplicationNotification,
        NSWorkspaceDidTerminateApplicationNotification,
    };
    use objc2_foundation::NSNotification;
    use std::sync::OnceLock;

    static OBSERVERS_INITIALIZED: OnceLock<()> = OnceLock::new();

    pub fn init_macos_observers() {
        if OBSERVERS_INITIALIZED.set(()).is_err() {
            return;
        }

        let _main_thread =
            MainThreadMarker::new().expect("app lifecycle observers must initialize on the main thread");

        let workspace = NSWorkspace::sharedWorkspace();
        let center = workspace.notificationCenter();

        let launch_block = Box::leak(Box::new(RcBlock::new(|notification| {
            if let Some(app) = observed_app_from_notification(notification) {
                dispatch_app_launch(app);
            }
        })));
        let terminate_block = Box::leak(Box::new(RcBlock::new(|notification| {
            if let Some(app) = observed_app_from_notification(notification) {
                dispatch_app_terminate(app);
            }
        })));

        let launch_observer = unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(NSWorkspaceDidLaunchApplicationNotification),
                None::<&AnyObject>,
                None,
                launch_block,
            )
        };
        let terminate_observer = unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(NSWorkspaceDidTerminateApplicationNotification),
                None::<&AnyObject>,
                None,
                terminate_block,
            )
        };

        let _ = Box::leak(Box::new(launch_observer));
        let _ = Box::leak(Box::new(terminate_observer));
    }

    fn observed_app_from_notification(notification: NonNull<NSNotification>) -> Option<ObservedApp> {
        let notification = unsafe { notification.as_ref() };

        let Some(user_info) = notification.userInfo() else {
            return None;
        };

        let application_key = unsafe { NSWorkspaceApplicationKey };
        let Some(app_obj) = user_info.objectForKey(application_key) else {
            return None;
        };

        let Ok(app) = app_obj.downcast::<NSRunningApplication>() else {
            return None;
        };

        let Some(bundle_id) = app.bundleIdentifier() else {
            return None;
        };

        let pid = app.processIdentifier();
        if pid <= 0 {
            return None;
        }

        Some(ObservedApp {
            pid,
            bundle_id: bundle_id.to_string(),
        })
    }
}

#[cfg(target_os = "macos")]
use macos::init_macos_observers;
