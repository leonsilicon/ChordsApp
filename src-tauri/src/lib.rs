use crate::chords::{ChordFolder, LoadedAppChords};
use crate::feature::{Chorder, ChorderIndicatorPanel};
use crate::input::{register_caps_lock_input_handler, register_key_event_input_grabber};
use anyhow::{Context, Result};
use frontmost::{start_nsrunloop, Detector};
use objc2_app_kit::NSWorkspace;
use parking_lot::deadlock;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_app::store::GlobalHotkeyStore;
pub use tauri_app::*;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_log::{Target, TargetKind};
use tauri_plugin_store::StoreExt;

mod chords;
mod constants;
mod feature;
mod git;
mod input;
mod mode;
mod sources;
mod tauri_app;

use tauri_nspanel::tauri_panel;
tauri_panel! {
    panel!(IndicatorPanel {
        config: {
            can_become_key_window: true,
            can_become_main_window: false,
            is_floating_panel: true,
            hides_on_deactivate: false
        }
    })
}

#[derive(Debug)]
struct Frontmost {
    frontmost: String,
    handle: AppHandle,
}

#[cfg(target_os = "macos")]
impl frontmost::app::FrontmostApp for Frontmost {
    fn set_frontmost(&mut self, new_value: &str) {
        self.frontmost = new_value.to_string();
        let context = self.handle.state::<AppContext>();
        context
            .frontmost_application_id
            .store(Arc::new(Some(new_value.to_string())));
    }

    fn update(&mut self) {
        println!("Application activated: {}", self.frontmost);
    }
}

#[cfg_attr(mobile, tauri_app::mobile_entry_point)]
pub fn run() {
    std::panic::set_hook(Box::new(|info| {
        let bt = std::backtrace::Backtrace::force_capture();

        eprintln!("PANIC: {info}");
        eprintln!("{bt}");

        log::error!("PANIC: {info}");
        log::error!("{bt}");
    }));

    // https://github.com/Narsil/rdev/issues/165#issuecomment-2907684547
    #[cfg(target_os = "macos")]
    rdev::set_is_main_thread(false);

    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(10));
        let deadlocks = deadlock::check_deadlock();
        if deadlocks.is_empty() {
            continue;
        }

        log::warn!("{} deadlocks detected", deadlocks.len());
        for (i, threads) in deadlocks.iter().enumerate() {
            log::warn!("Deadlock #{}", i);
            for t in threads {
                log::warn!("Thread Id {:#?}", t.thread_id());
                log::warn!("{:#?}", t.backtrace());
            }
        }
    });

    let log_plugin = tauri_plugin_log::Builder::new()
        .clear_targets()
        .level(log::LevelFilter::Debug)
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::LogDir {
                file_name: Some("chords".into()),
            }),
            Target::new(TargetKind::Webview),
        ])
        .build();

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            tauri_app::commands::list_git_repos,
            tauri_app::commands::add_git_repo_command,
            tauri_app::commands::sync_git_repo_command,
            tauri_app::commands::list_local_chord_folders_command,
            tauri_app::commands::pick_local_chord_folder_command,
            tauri_app::commands::add_local_chord_folder_command,
            tauri_app::commands::list_active_chords_command,
            tauri_app::commands::list_repo_chords_command,
            tauri_app::commands::list_local_chord_folder_chords_command,
            tauri_app::commands::list_global_shortcut_mappings_command,
            tauri_app::commands::remove_global_shortcut_mapping_command,
            tauri_app::commands::open_accessibility_settings,
            tauri_app::commands::open_input_monitoring_settings,
        ])
        .plugin(log_plugin)
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_single_instance::init(|handle, _args, _cwd| {
            if let Err(error) = tauri_app::settings::show_settings_window(handle.clone()) {
                log::error!("Failed to show settings window for existing instance: {error}");
            }
        }))
        .plugin(tauri_nspanel::init())
        .plugin({
            #[cfg(target_os = "macos")]
            {
                tauri_plugin_autostart::Builder::new()
                    .macos_launcher(tauri_plugin_autostart::MacosLauncher::LaunchAgent)
                    .args(["--autostart"])
                    .build()
            }
            #[cfg(not(target_os = "macos"))]
            {
                tauri_plugin_autostart::Builder::new()
                    .args(["--autostart"])
                    .build()
            }
        })
        .plugin(tauri_plugin_macos_permissions::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_user_input::init())
        .setup(|app| {
            if let Err(e) = setup(app) {
                log::error!("Failed to set up app:\n{:#?}", e);
                app.dialog()
                    .message(format!("Failed to start Chords:\n\n{e}"))
                    .title("Startup Error")
                    .blocking_show();

                std::process::exit(1);
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri_app application");
}

// https://github.com/orgs/tauri-apps/discussions/7596#discussioncomment-6718895
fn setup(app: &mut tauri::App) -> Result<()> {
    thread::spawn(|| {
        start_nsrunloop!();
    });

    let handle = app.handle().clone();
    let chorder = {
        let window = handle
            .get_webview_window(crate::constants::INDICATOR_WINDOW_LABEL)
            .ok_or(anyhow::anyhow!("chord indicator window not found"))?;
        Chorder::new(ChorderIndicatorPanel::from_window(window)?)
    };
    let bundled_app_chords = LoadedAppChords::from_folders(vec![ChordFolder::load_bundled()?])?;
    let context = AppContext::new(chorder, bundled_app_chords);
    // Setting the frontmost application immediately (the frontmost crate only detects changes)
    let workspace = NSWorkspace::sharedWorkspace();
    if let Some(application) = workspace.frontmostApplication() {
        if let Some(bundle_id) = application.bundleIdentifier() {
            context
                .frontmost_application_id
                .store(Arc::new(Some(bundle_id.to_string())));
        }
    }

    handle.manage(context);

    tauri_app::tray::create_tray(handle.clone()).context("failed to create tray")?;
    tauri_app::settings::configure_settings_window(handle.clone())?;

    let frontmost = Frontmost {
        frontmost: String::new(),
        handle: handle.clone(),
    };
    Detector::init(Box::new(frontmost));

    if let Err(error) = tauri_app::settings::hide_settings_window(handle.clone()) {
        log::error!("failed to hide settings window at startup: {error}");
    }

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

    {
        let handle = handle.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(e) = reload_loaded_app_chords(handle).await {
                log::error!("Failed to reload app chords:\n{:#?}", e);
            }
        });
    }

    Ok(())
}
