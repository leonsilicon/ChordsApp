use crate::input::{Key, KeyEvent};
use device_query::{DeviceQuery, DeviceState};
use keycode::KeyMappingCode;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

const OVERLAY_MODE_LETTER_1_PRESSED: u8 = 1 << 0;
const OVERLAY_MODE_LETTER_2_PRESSED: u8 = 1 << 1;
const OVERLAY_MODE_NUMBER_1_PRESSED: u8 = 1 << 2;
const OVERLAY_MODE_NUMBER_2_PRESSED: u8 = 1 << 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    None,
    Overlay,
    Chord,
}

impl From<AppMode> for u8 {
    fn from(mode: AppMode) -> Self {
        match mode {
            AppMode::None => 0,
            AppMode::Overlay => 2,
            AppMode::Chord => 3,
        }
    }
}

impl AppMode {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => AppMode::None,
            2 => AppMode::Overlay,
            3 => AppMode::Chord,
            _ => AppMode::None,
        }
    }
}

#[derive(Debug)]
pub struct AppModeStateMachine {
    device_state: Option<DeviceState>,
    mode: AtomicU8,
    overlay_mode_flags: AtomicU8,

    caps_lock_just_pressed: AtomicBool,
}

impl AppModeStateMachine {
    pub fn new(device_state: Option<DeviceState>) -> Self {
        Self {
            device_state,
            mode: AtomicU8::new(AppMode::None.into()),
            overlay_mode_flags: AtomicU8::new(0),
            caps_lock_just_pressed: AtomicBool::new(false),
        }
    }

    pub fn get_app_mode(&self) -> AppMode {
        AppMode::from_u8(self.mode.load(Ordering::Relaxed))
    }

    pub fn handle_event(&self, event: &KeyEvent) -> bool {
        let previous_mode = self.get_app_mode();
        log::debug!("Handling {:?} mode event: {:?}", event, previous_mode);
        let consumed = match previous_mode {
            AppMode::None => self.handle_none_mode_event(event),
            AppMode::Overlay => self.handle_overlay_mode_event(event),
            AppMode::Chord => self.handle_chord_mode_event(event),
        };
        let new_mode = self.get_app_mode();

        if previous_mode != new_mode {
            log::info!("Mode changed from {:?} to {:?}", previous_mode, new_mode);
        }

        consumed
    }

    // If no mode is active, we only consume the event if it's the key that activates a mode
    // For chord mode, this key is Caps Lock
    fn handle_none_mode_event(&self, event: &KeyEvent) -> bool {
        match event {
            // We always consume caps lock
            KeyEvent::Press(Key(KeyMappingCode::CapsLock)) => {
                self.caps_lock_just_pressed.store(true, Ordering::Relaxed);
                true
            }
            KeyEvent::Press(Key(KeyMappingCode::Space)) => {
                if self.caps_lock_just_pressed.swap(false, Ordering::Relaxed) {
                    self.mode.store(AppMode::Chord.into(), Ordering::Relaxed);
                    return true;
                }

                false
            }
            // We never consume any other events in None mode
            KeyEvent::Press(_) => {
                self.caps_lock_just_pressed.store(false, Ordering::Relaxed);
                false
            }
            _ => false,
        }
    }

    // In overlay mode, the user must type:
    // 1. Letter
    // 2. Letter
    // 3. Number
    // 4. Number
    // If any of these are false, the mode automatically exits
    // We always consume the event in overlay mode
    fn handle_overlay_mode_event(&self, event: &KeyEvent) -> bool {
        let overlay_mode_flags = self.overlay_mode_flags.load(Ordering::Relaxed);

        let KeyEvent::Press(ref key) = event else {
            // We consume release events
            return true;
        };

        // If the first letter hasn't been pressed yet
        if overlay_mode_flags & OVERLAY_MODE_LETTER_1_PRESSED == 0 {
            if key.is_letter() {
                self.overlay_mode_flags
                    .fetch_or(OVERLAY_MODE_LETTER_1_PRESSED, Ordering::Relaxed);
            } else {
                // If a non-letter is pressed, exit overlay mode
                self.mode.store(AppMode::None.into(), Ordering::Relaxed);
            }
        } else if overlay_mode_flags & OVERLAY_MODE_LETTER_2_PRESSED == 0 {
            if key.is_letter() {
                self.overlay_mode_flags
                    .fetch_or(OVERLAY_MODE_LETTER_2_PRESSED, Ordering::Relaxed);
            } else {
                // If a non-letter is pressed, exit overlay mode
                self.mode.store(AppMode::None.into(), Ordering::Relaxed);
            }
        } else if overlay_mode_flags & OVERLAY_MODE_NUMBER_1_PRESSED == 0 {
            if key.is_digit() {
                self.overlay_mode_flags
                    .fetch_or(OVERLAY_MODE_NUMBER_1_PRESSED, Ordering::Relaxed);
            } else {
                // If a non-number is pressed, exit overlay mode
                self.mode.store(AppMode::None.into(), Ordering::Relaxed);
            }
        } else if overlay_mode_flags & OVERLAY_MODE_NUMBER_2_PRESSED == 0 {
            // All four keys have been pressed, so we exit overlay mode
            self.mode.store(AppMode::None.into(), Ordering::Relaxed);
        }

        // We always consume in overlay mode
        true
    }

    // We always consume the event in chord mode
    fn handle_chord_mode_event(&self, event: &KeyEvent) -> bool {
        let modifiers = Key::modifiers();
        match event {
            KeyEvent::Release(Key(code)) => {
                if code == &KeyMappingCode::Space {
                    self.mode.store(AppMode::None.into(), Ordering::Relaxed);
                }

                if code == &KeyMappingCode::ShiftLeft || code == &KeyMappingCode::ShiftRight {
                    return false;
                }
            }
            KeyEvent::Press(key) => {
                // We don't consume modifier events
                if modifiers.contains(key) {
                    return false;
                }

                let Some(device_state) = &self.device_state else {
                     return false;
                };

                let device_keys = device_state.get_keys();
                let non_shift_modifiers = Key::non_shift_modifiers();
                if device_keys
                    .iter()
                    .copied()
                    .any(|key| non_shift_modifiers.contains(&key.into()))
                {
                    log::debug!(
                        "Ignoring event because the following modifiers were held down: {:?}",
                        device_keys
                    );
                    return false;
                }
            }
        };

        true
    }
}
