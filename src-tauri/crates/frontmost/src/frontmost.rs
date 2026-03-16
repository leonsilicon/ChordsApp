use crate::app::FrontmostApp;
use objc2::declare::ClassBuilder;
use objc2::runtime::{NSObject, Sel};
use objc2::{msg_send, sel, ClassType};
use objc2_app_kit::{
    NSRunningApplication, NSWorkspace, NSWorkspaceApplicationKey,
    NSWorkspaceDidActivateApplicationNotification,
};
use objc2_foundation::NSNotification;
use std::sync::{OnceLock, RwLock};

#[macro_export]
macro_rules! start_nsrunloop {
    () => {
        use objc2_foundation::NSRunLoop;
        let current_run_loop = NSRunLoop::currentRunLoop();
        current_run_loop.run();
    };
}

static APP_INSTANCE: OnceLock<RwLock<Box<dyn FrontmostApp + Send + Sync>>> = OnceLock::new();

pub struct Detector;

impl Detector {
    // initialize Detector object with the `init()` function by
    // passing in the callback function that will be triggered upon switching the frontmost app
    pub fn init(app: Box<dyn FrontmostApp + Send + Sync>) {
        if APP_INSTANCE.set(RwLock::new(app)).is_err() {
            log::warn!("Frontmost detector already initialized");
            return;
        }
        // this is the Observer object that we'll be using
        // using ClassBuilder since I wasn't able to figure out
        // how to register external methods with the `define_class!` macro
        let mut builder = ClassBuilder::new(c"AppObserver", NSObject::class())
            .expect("a class with name AppObserver likely already exists.");

        // defining the external methods
        unsafe extern "C" fn init(this: *mut NSObject, _sel: Sel) -> *mut NSObject {
            let this: *mut NSObject = msg_send![super(this, NSObject::class()), init];
            this
        }
        unsafe extern "C" fn application_activated(
            _this: *mut NSObject,
            _sel: Sel,
            notification: *mut NSNotification,
        ) {
            unsafe {
                let Some(notification) = notification.as_ref() else {
                    log::warn!("Ignoring null application activation notification");
                    return;
                };

                let Some(user_info) = notification.userInfo() else {
                    log::warn!(
                        "Ignoring NSWorkspaceDidActivateApplicationNotification without userInfo"
                    );
                    return;
                };

                let Some(object) = user_info.objectForKey(NSWorkspaceApplicationKey) else {
                    log::warn!(
                        "Ignoring NSWorkspaceDidActivateApplicationNotification without NSWorkspaceApplicationKey"
                    );
                    return;
                };

                let Some(running_app) = object.downcast_ref::<NSRunningApplication>() else {
                    log::warn!(
                        "Ignoring NSWorkspaceDidActivateApplicationNotification with an unexpected application object"
                    );
                    return;
                };

                let Some(bundle_identifier) = running_app.bundleIdentifier() else {
                    log::warn!(
                        "Ignoring NSWorkspaceDidActivateApplicationNotification without a bundle identifier"
                    );
                    return;
                };

                let bundle_identifier = bundle_identifier.to_string();

                if let Some(app_ref) = APP_INSTANCE.get() {
                    match app_ref.write() {
                        Ok(mut app) => {
                            app.set_frontmost(&bundle_identifier);
                            app.update();
                        }
                        Err(error) => {
                            log::error!("Failed to lock frontmost app state: {error}");
                        }
                    }
                }
            }
        }

        unsafe {
            builder.add_method(
                sel!(init),
                init as unsafe extern "C" fn(*mut NSObject, Sel) -> *mut NSObject,
            );
            builder.add_method(
                sel!(applicationActivated:),
                application_activated
                    as unsafe extern "C" fn(*mut NSObject, Sel, *mut NSNotification),
            );
        }

        // register new AppObserver class to the Objective-C runtime
        let app_observer_class = builder.register();

        // add Observer to the notification center
        unsafe {
            let observer: *mut NSObject = msg_send![app_observer_class, alloc];
            let observer: *mut NSObject = msg_send![observer, init];

            let workspace = NSWorkspace::sharedWorkspace();
            let notification_center = workspace.notificationCenter();

            notification_center.addObserver_selector_name_object(
                &*(observer as *const NSObject),
                sel!(applicationActivated:),
                Some(NSWorkspaceDidActivateApplicationNotification),
                None,
            );
        }
    }
}
