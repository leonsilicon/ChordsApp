use crate::input::KeyEvent;
use crate::mode::AppMode;
use crate::AppContext;
use anyhow::Result;
use tauri::{AppHandle, Manager};

pub fn handle_key_event(handle: AppHandle, key_event: KeyEvent) -> Result<()> {
    let context = handle.state::<AppContext>();
    let app_mode = context.get_app_mode();

    match app_mode {
        AppMode::Chord => {
            context.clicker.ensure_inactive(handle.clone())?;
            context
                .chorder
                .handle_key_event(handle.clone(), &key_event)?;
        }
        AppMode::Overlay => {
            context.chorder.ensure_inactive(handle.clone())?;
            context
                .clicker
                .handle_key_event(handle.clone(), &key_event)?;
        }
        AppMode::None => {
            context.clicker.ensure_inactive(handle.clone())?;
            context.chorder.ensure_inactive(handle.clone())?;
        }
    }

    Ok(())
}
