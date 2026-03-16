use crate::input::Key;
use crate::AppContext;
use crate::{input::handler::handle_key_event, mode::AppModeStateMachine};
use bitflags::bitflags;
use keycode::KeyMappingCode;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::mpsc::channel;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[derive(Debug)]
pub enum KeyEvent {
    Press(Key),
    Release(Key),
}

pub fn register_key_event_input_grabber(handle: AppHandle) {
    let (tx, rx) = channel::<KeyEvent>();

    {
        let handle = handle.clone();
        // Spawning the handler in a separate thread to keep the key grabber callback as fast as possible
        std::thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                if let Err(e) = handle_key_event(handle.clone(), event) {
                    log::error!("Failed to handle key event: {e}");
                }
            }
        });
    }

    let callback = move |event: rdev::Event| -> Option<rdev::Event> {
        // Synthetic, skip processing
        if event.source_user_data == 0xDEADBEEF {
            return Some(event);
        }

        let context = handle.state::<AppContext>();
        let key_event = match event.event_type {
            rdev::EventType::KeyPress(key) => Key::try_from(key)
                .map(|mapped_key| KeyEvent::Press(mapped_key))
                .ok(),
            rdev::EventType::KeyRelease(key) => Key::try_from(key)
                .map(|mapped_key| KeyEvent::Release(mapped_key))
                .ok(),
            _ => return Some(event),
        };

        let Some(key_event) = key_event else {
            return Some(event);
        };

        let action = context.key_event_state.process_event(&key_event);
        if let Err(e) = tx.send(key_event) {
            log::error!("Failed to send key event: {e}");
        }

        match action {
            EventAction::Consume => None,
            _ => Some(event),
        }
    };

    if let Err(error) = rdev::grab(callback) {
        println!("Error: {:?}", error)
    }
}

bitflags! {
  pub struct Modifiers: u16 {
      const LEFT_SHIFT      = 1 << 0;
      const RIGHT_SHIFT    = 1 << 1;
      const LEFT_CONTROL    = 1 << 2;
      const RIGHT_CONTROL   = 1 << 3;
      const LEFT_OPTION     = 1 << 4;
      const RIGHT_OPTION    = 1 << 5;
      const LEFT_COMMAND    = 1 << 6;
      const RIGHT_COMMAND   = 1 << 7;
      const FUNCTION        = 1 << 8;
  }
}

pub struct KeyEventState {
    app_mode_state_machine: Arc<AppModeStateMachine>,

    // Modifier Flags
    pub modifier_flags: AtomicU16,
}

#[derive(Debug, PartialEq)]
pub enum EventAction {
    Consume,
    Forward,
}

impl KeyEventState {
    pub fn new(app_mode_state_machine: Arc<AppModeStateMachine>) -> Self {
        Self {
            app_mode_state_machine,
            modifier_flags: AtomicU16::new(0),
        }
    }

    pub fn process_event(&self, event: &KeyEvent) -> EventAction {
        self.update_modifier_flags(&event);

        log::debug!("Processing event: {:?}", event);
        let consumed = self.app_mode_state_machine.handle_event(event);

        if consumed {
            log::debug!("Consuming event: {:?}", event);
            EventAction::Consume
        } else {
            EventAction::Forward
        }
    }

    pub fn get_modifier_flags(&self) -> Modifiers {
        Modifiers::from_bits_truncate(self.modifier_flags.load(Ordering::Relaxed))
    }

    fn modifier_key_to_flag(key: &Key) -> Option<Modifiers> {
        let flag = match key.0 {
            KeyMappingCode::ShiftLeft => Modifiers::LEFT_SHIFT,
            KeyMappingCode::ShiftRight => Modifiers::RIGHT_SHIFT,
            KeyMappingCode::ControlLeft => Modifiers::LEFT_CONTROL,
            KeyMappingCode::ControlRight => Modifiers::RIGHT_CONTROL,
            KeyMappingCode::AltLeft => Modifiers::LEFT_OPTION,
            KeyMappingCode::AltRight => Modifiers::RIGHT_OPTION,
            KeyMappingCode::MetaLeft => Modifiers::LEFT_COMMAND,
            KeyMappingCode::MetaRight => Modifiers::RIGHT_COMMAND,
            KeyMappingCode::Fn => Modifiers::FUNCTION,
            _ => return None,
        };

        Some(flag)
    }

    fn update_modifier_flags(&self, event: &KeyEvent) {
        match event {
            KeyEvent::Press(key) => {
                if let Some(flag) = Self::modifier_key_to_flag(key) {
                    self.modifier_flags.fetch_or(flag.bits(), Ordering::Relaxed);
                }
            }
            KeyEvent::Release(key) => {
                if let Some(flag) = Self::modifier_key_to_flag(key) {
                    self.modifier_flags
                        .fetch_and(!flag.bits(), Ordering::Relaxed);
                }
            }
        }
    }
}
