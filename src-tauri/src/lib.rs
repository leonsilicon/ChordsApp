use frontmost::app::FrontmostApp;
use parking_lot::deadlock;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager};
pub use tauri_app::*;
use tauri_nspanel::tauri_panel;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_log::{Target, TargetKind};

mod chords;
mod constants;
mod feature;
mod git;
mod input;
mod mode;
mod tauri_app;

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

impl FrontmostApp for Frontmost {
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
        .invoke_handler(tauri::generate_handler![
            tauri_app::commands::list_git_repos,
            tauri_app::commands::add_git_repo_command,
            tauri_app::commands::sync_git_repo_command,
            tauri_app::commands::list_active_chords_command,
            tauri_app::commands::list_repo_chords_command,
            tauri_app::commands::open_accessibility_settings,
            tauri_app::commands::open_input_monitoring_settings,
        ])
        .plugin(log_plugin)
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
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_user_input::init())
        .setup(|app| {
            if let Err(e) = tauri_app::setup::setup_app(app) {
                log::error!("Failed to set up app:\n{:#?}", e);

                app.handle()
                    .dialog()
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
