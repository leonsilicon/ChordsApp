use crate::constants::{ QUIT_MENU_ID, SETTINGS_MENU_ID };
use crate::tauri_app::settings::show_settings_window;
use crate::AppContext;
use tauri::Manager;
use tauri::{
    menu::MenuBuilder,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle,
};

pub fn create_tray(handle: AppHandle) -> tauri::Result<()> {
    let menu = MenuBuilder::new(&handle)
        .text(SETTINGS_MENU_ID, "Settings")
        .separator()
        .text(QUIT_MENU_ID, "Quit")
        .build()?;

    let mut tray = TrayIconBuilder::with_id("chords-tray")
        .menu(&menu)
        .tooltip("Chords")
        .show_menu_on_left_click(false)
        .on_menu_event(|handle, event| {
            match event.id().as_ref() {
                SETTINGS_MENU_ID => {
                    if let Err(e) = show_settings_window(handle.clone()) {
                        log::error!("Failed to show settings window: {e}");
                    }
                }
                QUIT_MENU_ID => {
                    handle.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Err(error) = show_settings_window(tray.app_handle().clone()) {
                    log::error!("Failed to show settings window from tray click: {error}");
                }
            }
        });

    if let Some(icon) = handle.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }

    tray.build(&handle)?;
    Ok(())
}
