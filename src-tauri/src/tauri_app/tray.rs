use crate::constants::{QUIT_MENU_ID, RELOAD_CONFIGS_MENU_ID, SETTINGS_MENU_ID};
use crate::tauri_app::context::reload_loaded_app_chords;
use crate::tauri_app::settings::show_settings_window;
use tauri::{menu::MenuBuilder, tray::TrayIconBuilder, AppHandle};

pub fn create_tray(handle: AppHandle) -> tauri::Result<()> {
    let menu = MenuBuilder::new(&handle)
        .text(SETTINGS_MENU_ID, "Show Settings")
        .text(RELOAD_CONFIGS_MENU_ID, "Reload Configs")
        .separator()
        .text(QUIT_MENU_ID, "Quit")
        .build()?;

    let mut tray = TrayIconBuilder::with_id("chords-tray")
        .menu(&menu)
        .tooltip("Chords")
        .show_menu_on_left_click(true)
        .on_menu_event(|handle, event| match event.id().as_ref() {
            SETTINGS_MENU_ID => {
                if let Err(e) = show_settings_window(handle.clone()) {
                    log::error!("Failed to show settings window: {e}");
                }
            }
            RELOAD_CONFIGS_MENU_ID => {
                let handle = handle.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = reload_loaded_app_chords(handle).await {
                        log::error!("Failed to reload configs: {error}");
                    }
                });
            }
            QUIT_MENU_ID => {
                handle.exit(0);
            }
            _ => {}
        });

    if let Some(icon) = handle.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }

    tray.build(&handle)?;
    Ok(())
}
