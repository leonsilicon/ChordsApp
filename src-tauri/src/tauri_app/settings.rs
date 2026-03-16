use anyhow::Result;
use tauri::{AppHandle, Manager, WebviewWindow};

pub fn get_settings_window(handle: AppHandle) -> Result<WebviewWindow> {
    handle
        .get_webview_window("settings")
        .ok_or(anyhow::anyhow!("settings window not found"))
}

pub fn configure_settings_window(handle: AppHandle) -> Result<()> {
    let window = get_settings_window(handle)?;
    window.set_always_on_top(true)?;
    window.set_skip_taskbar(true)?;
    Ok(())
}

pub fn show_settings_window(handle: AppHandle) -> Result<()> {
    let window = get_settings_window(handle)?;
    window.show()?;
    window.unminimize()?;
    window.set_focus()?;
    Ok(())
}

pub fn hide_settings_window(handle: AppHandle) -> Result<()> {
    let window = get_settings_window(handle)?;
    window.hide()?;
    Ok(())
}
