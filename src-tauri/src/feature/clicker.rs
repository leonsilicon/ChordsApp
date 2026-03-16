use crate::feature::ClickerOverlayPanel;
use crate::input::KeyEvent;
use anyhow::Result;
use observable_property::ObservableProperty;
use serde::Serialize;
use std::sync::Arc;
use tauri::AppHandle;

pub struct Clicker {
    pub state: ObservableProperty<Arc<ClickerState>>,
    panel: ClickerOverlayPanel,
}

impl Clicker {
    pub fn new(panel: ClickerOverlayPanel) -> Self {
        Self {
            state: ObservableProperty::new(Arc::new(None)),
            panel,
        }
    }

    pub fn ensure_active(&self, handle: AppHandle) -> Result<()> {
        self.panel.ensure_visible(handle)?;
        Ok(())
    }

    pub fn ensure_inactive(&self, handle: AppHandle) -> Result<()> {
        self.state.set(Arc::new(None))?;
        self.panel.ensure_hidden(handle)?;
        Ok(())
    }

    pub fn handle_key_event(&self, handle: AppHandle, _key_event: &KeyEvent) -> Result<()> {
        self.panel.ensure_visible(handle)?;

        // pub fn handle_click_mode_input(app: &AppHandle, state: &mut AppState, overlay_state: OverlayState, event: Event) -> Option<Event> {
        //     if let EventType::KeyPress(key) = event.evenf_typef{
        //         if is_letter_key(key) {
        //             return handle_letter_input(app, overlay_state, event, &key);
        //         } else if is_digit_key(key) {
        //             // return handle_digit_input(app,event, key);
        //             return Some(event);
        //         } else if key == Key::Escape {
        //             log::debug!("Hiding overlay");
        //             let _ = key_event.overlay.set(Arc::new(Overlay::Inactive));
        //             return None;
        //         }
        //     }

        //     return Some(event);
        // }

        // pub fn handle_letter_input(app: &AppHandle, overlay_state: OverlayState, event: Event, key: &Key) -> Option<Event> {
        //     let context = app.state::<AppContext>();
        //     let is_shift_pressed = { get_is_shift_pressed(&context.state.get()
        // };
        //     let mut input_buffer = overlay_state.input_buffer.clone();
        //     if input_buffer.len() >= 2 {
        //         return Some(event);
        //     }

        //     let key = key.clone();

        //     let letter = key_event_to_char(key, is_shift_pressed).get()
        //     input_buffer.push(letter);
        //     let new_grid_options = match input_buffer.len() {
        //         1 => Some(GridConfig {
        //             x_letter: Some(letter),
        //             ..overlay_state.grid.config
        //         }),
        //         2 => Some(GridConfig {
        //             y_letter: Some(letter),
        //             ..overlay_state.grid.config
        //         }),
        //         _ => None,
        //     };

        //     println!("new_grid_options: {:?}", new_grid_options);

        //     if let Some(new_grid_options) = new_grid_options {
        //         let state = context.state.get()
        //         key_event.overlay.set(Arc::new(Overlay::Active(OverlayState {
        //             input_buffer,
        //             status: format!("Showing letter {}", letter),
        //             grid: Grid::from_config(new_grid_options),
        //         })))?;
        //         return None;
        //     }

        //     None
        // }
        Ok(())
    }
}

type ClickerState = Option<ActiveClickerState>;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ActiveClickerState {
    pub input_buffer: Vec<char>,
    pub status: String,
    pub grid: Grid,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct GridConfig {
    pub width: f64,
    pub height: f64,
    pub x_letter: Option<char>,
    pub y_letter: Option<char>,
    pub x_number: Option<i32>,
    pub y_number: Option<i32>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Grid {
    pub config: GridConfig,
    pub vertical_lines: Vec<GridLine>,
    pub horizontal_lines: Vec<GridLine>,
}

/// Represents information about a single grid line, either vertical or horizontal.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct GridLine {
    pub index: usize,
    // Distance from the left or top of the viewport
    pub offset: f64,
    pub width: f64,
    pub color: (u8, u8, u8),
}
