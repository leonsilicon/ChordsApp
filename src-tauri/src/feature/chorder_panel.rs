use crate::IndicatorPanel;
use anyhow::Result;
use objc2_app_kit::NSWindowAnimationBehavior;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, WebviewWindow};
use tauri_nspanel::{CollectionBehavior, Panel, PanelLevel, StyleMask, WebviewWindowExt};

const INDICATOR_WIDTH: u32 = 240;
const INDICATOR_HEIGHT: u32 = 44;
const INDICATOR_TOP_INSET: i32 = 38;

pub struct ChorderIndicatorPanel {
    pub is_visible: Arc<AtomicBool>,
    pub panel: Arc<dyn Panel>,
}

impl ChorderIndicatorPanel {
    pub fn from_window(window: WebviewWindow) -> Result<Self> {
        let _ = window.set_ignore_cursor_events(true);

        let panel = window.to_panel::<IndicatorPanel>()?;
        panel.set_level(PanelLevel::ScreenSaver.into());
        panel.set_has_shadow(false);
        panel.set_opaque(true);
        panel.set_transparent(false);
        panel.set_ignores_mouse_events(true);
        panel.set_becomes_key_only_if_needed(true);
        panel.set_style_mask(StyleMask::empty().borderless().nonactivating_panel().into());
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
        return Ok(());
        log::debug!("Showing chorder panel");
        let panel = self.panel.clone();
        let is_visible = self.is_visible.clone();

        handle.clone().run_on_main_thread(move || {
            if let Ok(Some(monitor)) = handle.primary_monitor() {
                let position = monitor.position();
                let size = monitor.size();
                let x = position.x + ((size.width.saturating_sub(INDICATOR_WIDTH)) / 2) as i32;
                let y = position.y + size.height as i32 - INDICATOR_TOP_INSET;

                let native_panel = panel.as_panel();
                native_panel.setContentSize(tauri_nspanel::objc2_foundation::NSSize::new(
                    INDICATOR_WIDTH as f64,
                    INDICATOR_HEIGHT as f64,
                ));
                native_panel.setFrameTopLeftPoint(tauri_nspanel::objc2_foundation::NSPoint::new(
                    x as f64, y as f64,
                ));
            }

            panel.show_and_make_key();
            panel.order_front_regardless();
            is_visible.store(true, Ordering::Relaxed);
        })?;

        Ok(())
    }

    fn hide(&self, handle: AppHandle) -> Result<()> {
        log::debug!("Hiding chorder panel");
        let is_visible = self.is_visible.clone();
        let panel = self.panel.clone();

        handle.clone().run_on_main_thread(move || {
            panel.hide();
            is_visible.store(false, Ordering::Relaxed);
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
