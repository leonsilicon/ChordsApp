use super::ChorderIndicatorPanel;
use crate::chords::{press_chord, release_chord, Chord};
use crate::input::Key;
use crate::{input::KeyEvent, AppContext};
use anyhow::Result;
use device_query::DeviceQuery;
use keycode::KeyMappingCode;
use observable_property::ObservableProperty;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use crate::mode::AppMode::Chord;

pub struct Chorder {
    pub state: ObservableProperty<Arc<ChorderState>>,
    panel: ChorderIndicatorPanel,
}

impl Chorder {
    pub fn new(panel: ChorderIndicatorPanel) -> Self {
        Self {
            state: ObservableProperty::new(Arc::new(ChorderState::new())),
            panel,
        }
    }

    pub fn ensure_active(&self, handle: AppHandle) -> Result<()> {
        self.panel.ensure_visible(handle)?;
        Ok(())
    }

    pub fn ensure_inactive(&self, handle: AppHandle) -> Result<()> {
        self.state.set(Arc::new(ChorderState::new()))?;
        self.panel.ensure_hidden(handle)?;
        Ok(())
    }

    // If `handle_key_event` is called, the state is guaranteed to be active
    pub fn handle_key_event(&self, handle: AppHandle, key_event: &KeyEvent) -> Result<()> {
        // Don't handle any modifier key events
        let modifiers = Key::modifiers();
        let (KeyEvent::Press(key) | KeyEvent::Release(key)) = key_event;
        if modifiers.contains(key) {
            log::debug!("Ignoring modifier key: {:?}", key);
            return Ok(());
        }

        let non_shift_modifiers = Key::non_shift_modifiers();
        let context = handle.state::<AppContext>();
        let Some(device_state) = &context.device_state else {
            log::debug!("no accessibility permissions");
            return Ok(());
        };
        let device_keys = device_state.get_keys();

        // If any non-Shift modifier keys are held down, do not handle the event, because it's
        // likely the user just wants to execute a regular shortcut
        if device_keys
            .iter()
            .copied()
            .any(|key| non_shift_modifiers.contains(&key.into()))
        {
            log::debug!(
                "Ignoring event because the following modifiers were held down: {:?}",
                device_keys
            );
            return Ok(());
        }

        match key_event {
            KeyEvent::Release(Key(code)) => {
                if let Some(pressed_chord) = &self.state.get()?.pressed_chord {
                    if code == &KeyMappingCode::CapsLock {
                        release_chord(handle, pressed_chord)?;
                    } else if pressed_chord.keys.last().is_some_and(|k| &k.0 == code) {
                        release_chord(handle, pressed_chord)?;
                    }
                }

                if code == &KeyMappingCode::Space {
                    self.state.set(Arc::new(ChorderState::new()))?;
                }

                Ok(())
            }

            // If the caps lock key is pressed, it means we should execute (and clear) the chord
            // currently in `key_buffer`, or if empty, execute the last chord
            KeyEvent::Press(Key(KeyMappingCode::CapsLock)) => {
                self.ensure_active(handle.clone())?;

                let context = handle.state::<AppContext>();
                let loaded_app_chords = context.loaded_app_chords.read();
                let state = self.state.get()?;
                let key_buffer = state.key_buffer.clone();

                // An empty `key_buffer` means we should execute the last executed chord
                if key_buffer.is_empty() {
                    // If there isn't an active chord, then do nothing
                    let Some(last_chord) = &state.active_chord else {
                        log::error!("Key buffer is empty and no chord is active");
                        return Ok(());
                    };

                    press_chord(handle.clone(), &last_chord)?;
                    self.state.set(Arc::new(ChorderState {
                        pressed_chord: state.active_chord.clone(),
                        key_buffer: vec![],
                        active_chord: state.active_chord.clone(),
                    }))?;

                    return Ok(());
                }

                // A non-empty key_buffer means we should execute the chord.
                log::debug!("Executing key_buffer {:?}", key_buffer);

                let chord_runtime = loaded_app_chords.get_chord_runtime(
                    &state.key_buffer,
                    context.frontmost_application_id.load().as_ref().clone(),
                );
                let Some(chord) = chord_runtime.get_chord(key_buffer) else {
                    // If the chord is the buffer is invalid, reset it
                    log::error!(
                        "Invalid chord: {:?} for application: {:?}",
                        state.key_buffer,
                        context.frontmost_application_id.load().as_ref().clone()
                    );
                    self.state.set(Arc::new(ChorderState {
                        key_buffer: vec![],
                        pressed_chord: None,
                        active_chord: None,
                    }))?;
                    return Ok(());
                };

                press_chord(handle.clone(), &chord_runtime, chord)?;
                self.state.set(Arc::new(ChorderState {
                    pressed_chord: Some(chord.clone()),
                    key_buffer: vec![],
                    active_chord: Some(chord),
                }))?;
                Ok(())
            }
            KeyEvent::Press(key) => {
                // Ignore space presses
                if key == &Key(KeyMappingCode::Space) {
                    return Ok(());
                }

                self.ensure_active(handle.clone())?;
                let is_shift_pressed = device_keys.contains(&device_query::Keycode::LShift)
                    || device_keys.contains(&device_query::Keycode::RShift);

                if is_shift_pressed {
                    self.handle_shifted_key_press(handle, key)
                } else {
                    self.handle_unshifted_key_press(key)
                }
            }
        }
    }

    // If an unshifted key is pressed, we append it to the key buffer, which always clears
    // our `active_chord`
    fn handle_unshifted_key_press(&self, key: &Key) -> Result<()> {
        let state = self.state.get()?;
        let mut next_key_buffer = state.key_buffer.clone();
        next_key_buffer.push(key.clone());
        log::debug!("New key buffer: {:?}", next_key_buffer);
        self.state.set(Arc::new(ChorderState {
            key_buffer: next_key_buffer,
            pressed_chord: None,
            active_chord: None,
        }))?;
        Ok(())
    }

    // If shift is pressed, it means the user is trying to execute a chord.
    // If a chord is executed, we always reset `key_buffer`.
    fn handle_shifted_key_press(&self, handle: AppHandle, key: &Key) -> Result<()> {
        let context = handle.state::<AppContext>();
        let state = self.state.get()?;
        let key_buffer = state.key_buffer.clone();

        let sequence = {
            // If key_buffer is empty (i.e. we just activated a chord), we should use that chord to
            // determine our sequence
            if key_buffer.is_empty() {
                let Some(active_chord) = &state.active_chord else {
                    // If `key_buffer` and `active_chord` is empty, then we do nothing
                    log::error!("No chord active");
                    return Ok(());
                };

                let mut new_chord = active_chord.keys.clone();
                new_chord.pop();
                new_chord.push(key.clone());
                new_chord
            }
            // If `key_buffer` is non-empty, we should run the chord `key_buffer` + key
            else {
                let mut sequence = key_buffer.clone();
                sequence.push(key.clone());
                sequence
            }
        };

        let frontmost_application_id = context.frontmost_application_id.load().as_ref().clone();
        let chord = {
            let loaded_app_chords = context.loaded_app_chords.read();
            loaded_app_chords.get_chord(&sequence, frontmost_application_id)
        };
        let Some(chord) = chord else {
            // We don't change the state for an invalid sequence
            log::debug!("Invalid sequence {:?}", sequence);
            return Ok(());
        };

        log::debug!("Pressing chord: {:?}", chord);
        press_chord(handle, &chord)?;
        self.state.set(Arc::new(ChorderState {
            // We always clear the key_buffer if a chord is pressed
            key_buffer: vec![],
            pressed_chord: Some(chord.clone()),
            active_chord: Some(chord),
        }))?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ChorderState {
    // The key buffer represents the pending letters for a not-yet created chord.
    // When a chord is executed, the key buffer is always cleared.
    pub key_buffer: Vec<Key>,

    // The chord currently being pressed down.
    pub pressed_chord: Option<Chord>,

    // The chord that is "active"
    pub active_chord: Option<Chord>,
}

impl ChorderState {
    pub fn new() -> Self {
        Self {
            key_buffer: Vec::new(),
            pressed_chord: None,
            active_chord: None,
        }
    }
}
