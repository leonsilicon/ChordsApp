use crate::input::{register_caps_lock_input_handler, register_key_event_input_grabber};
use crate::{initialize_app_context, tauri_app, Frontmost};
use anyhow::{Context, Result};
use frontmost::{start_nsrunloop, Detector};
use std::thread;
use tauri::App;

pub fn setup_app(app: &mut App) -> Result<()> {
    let handle = app.handle();

    // We want to initialize this as early as possible because code usually assumes that context is available.
    initialize_app_context(handle.clone()).context("failed to initialize app context")?;

    let frontmost = Frontmost {
        frontmost: String::new(),
        handle: handle.clone(),
    };
    Detector::init(Box::new(frontmost));

    thread::spawn(|| {
        start_nsrunloop!();
    });

    tauri_app::tray::create_tray(handle.clone()).context("failed to create tray")?;

    tauri_app::settings::configure_settings_window(handle.clone())
        .context("failed to configure settings window")?;

    {
        let handle = handle.clone();
        tauri::async_runtime::spawn(async move {
            let has_permission =
                tauri_plugin_macos_permissions::check_input_monitoring_permission().await;
            if has_permission {
                log::info!("Input monitoring permission granted, registering caps lock listener");
                if let Err(e) = register_caps_lock_input_handler(handle.clone()) {
                    log::error!("Failed to handle caps lock input: {e}");
                }
            } else {
                log::warn!("Input monitoring permission not granted, skipping caps lock listener");
            }
        });
    }

    {
        let handle = handle.clone();
        tauri::async_runtime::spawn(async move {
            let has_permission =
                tauri_plugin_macos_permissions::check_accessibility_permission().await;
            if has_permission {
                log::info!("Accessibility permission granted, registering grab listener");
                register_key_event_input_grabber(handle.clone());
            } else {
                log::warn!("Accessibility permission not granted, skipping grab listener");
            }
        });
    }

    if let Err(error) = tauri_app::settings::hide_settings_window(handle.clone()) {
        log::error!("failed to hide settings window at startup: {error}");
    }

    Ok(())
}
