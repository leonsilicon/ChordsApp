use crate::AppContext;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

pub fn post_manage_setup_clicker_frontend_sync(handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let context = handle.state::<AppContext>();

        let handle = handle.clone();
        if let Err(e) = context.clicker.state.subscribe(Arc::new(move |_, state| {
            let state = &**state;

            // Send the updated UI state to the frontend by emitting an event.
            // This emits a "change" event to all windows (including overlay) with the new state.
            if let Err(e) = handle.emit("onStateChange", state) {
                log::error!("Failed to emit change event to frontend: {e}");
            }
        })) {
            log::error!("Failed to subscribe to clicker state: {e}");
        };
    });
}
