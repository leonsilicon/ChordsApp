use crate::OverlayPanel;
use anyhow::Result;
use objc2_app_kit::NSWindowAnimationBehavior;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, WebviewWindow};
use tauri_nspanel::{CollectionBehavior, Panel, PanelLevel, StyleMask, WebviewWindowExt};

pub struct ClickerOverlayPanel {
    pub is_visible: Arc<AtomicBool>,
    panel: Arc<dyn Panel>,
}

impl ClickerOverlayPanel {
    pub fn from_window(window: WebviewWindow) -> Result<Self> {
        let _ = window.set_ignore_cursor_events(true);

        let panel = window.to_panel::<OverlayPanel>()?;
        panel.set_level(PanelLevel::ScreenSaver.into());
        panel.set_has_shadow(false);
        panel.set_transparent(true);
        panel.set_ignores_mouse_events(true);
        panel.set_style_mask(StyleMask::empty().nonactivating_panel().into());
        panel.set_floating_panel(true);
        panel.set_hides_on_deactivate(false);
        panel
            .as_panel()
            .setAnimationBehavior(NSWindowAnimationBehavior::None);
        panel.set_collection_behavior(
            CollectionBehavior::new()
                .can_join_all_spaces()
                .stationary()
                .full_screen_auxiliary()
                .ignores_cycle()
                .into(),
        );

        Ok(Self {
            is_visible: Arc::new(AtomicBool::new(false)),
            panel,
        })
    }

    fn show(&self, handle: AppHandle) -> Result<()> {
        log::debug!("Showing clicker panel");
        let is_visible = self.is_visible.clone();
        let panel = self.panel.clone();

        handle.clone().run_on_main_thread(move || {
            // Ensure that the panel is the correct size and position
            if let Ok(Some(monitor)) = handle.primary_monitor() {
                let position = monitor.position();
                let size = monitor.size();

                let native_panel = panel.as_panel();
                native_panel.setContentSize(tauri_nspanel::objc2_foundation::NSSize::new(
                    size.width as f64,
                    size.height as f64,
                ));
                native_panel.setFrameTopLeftPoint(tauri_nspanel::objc2_foundation::NSPoint::new(
                    position.x as f64,
                    (position.y + size.height as i32) as f64,
                ));
            };

            is_visible.store(true, Ordering::Relaxed);
            panel.show_and_make_key();
        })?;

        Ok(())
    }

    fn hide(&self, handle: AppHandle) -> Result<()> {
        log::debug!("Hiding clicker panel");
        let is_visible = self.is_visible.clone();
        let panel = self.panel.clone();

        handle.clone().run_on_main_thread(move || {
            is_visible.store(false, Ordering::Relaxed);
            panel.hide();
        })?;

        Ok(())
    }

    pub fn ensure_hidden(&self, handle: AppHandle) -> Result<()> {
        if self.is_visible.load(Ordering::Relaxed) {
            self.hide(handle)?;
        }

        Ok(())
    }

    pub fn ensure_visible(&self, handle: AppHandle) -> Result<()> {
        if !self.is_visible.load(Ordering::Relaxed) {
            self.show(handle)?;
        }

        Ok(())
    }
}
