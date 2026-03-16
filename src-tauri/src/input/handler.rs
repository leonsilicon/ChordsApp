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
            context
                .chorder
                .handle_key_event(handle.clone(), &key_event)?;
        }
        AppMode::None => {
            context.chorder.ensure_inactive(handle.clone())?;
        }
    }

    Ok(())
}
