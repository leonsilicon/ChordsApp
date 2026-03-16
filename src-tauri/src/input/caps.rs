use crate::input::handle_key_event;
use crate::input::{Key, KeyEvent};
use crate::AppContext;
use anyhow::Result;
use keycode::KeyMappingCode;
use std::os::raw::c_int;
use std::process::Command;
use std::sync::mpsc::channel;
use std::sync::{mpsc::Sender, OnceLock};
use tauri::{AppHandle, Manager};

static TX: OnceLock<Sender<bool>> = OnceLock::new();

extern "C" {
    fn start_caps_lock_listener(cb: extern "C" fn(c_int));
}

extern "C" fn caps_lock_changed(pressed: c_int) {
    log::debug!("caps_lock_changed: {}", pressed);
    if let Some(tx) = TX.get() {
        if let Err(e) = tx.send(pressed != 0) {
            log::error!("Failed to send caps lock changed event: {e}");
        }
    } else {
        log::error!("No tx found");
    }
}

pub fn register_caps_lock_input_handler(handle: AppHandle) -> Result<()> {
    log::info!("Registering caps lock handler");
    let (tx, rx) = channel();

    TX.set(tx)
        .map_err(|_| anyhow::anyhow!("failed to set tx"))?;
    remap_caps_to_no_action()?;

    std::thread::spawn(|| unsafe {
        start_caps_lock_listener(caps_lock_changed);
    });

    let handle = handle.clone();
    std::thread::spawn(move || {
        while let Ok(pressed) = rx.recv() {
            let context = handle.state::<AppContext>();
            if pressed {
                context
                    .key_event_state
                    .process_event(&KeyEvent::Press(Key(KeyMappingCode::CapsLock)));

                if let Err(e) = handle_key_event(
                    handle.clone(),
                    KeyEvent::Press(Key(KeyMappingCode::CapsLock)),
                ) {
                    log::error!("Failed to handle Caps Lock Press: {e}");
                }
            } else {
                context
                    .key_event_state
                    .process_event(&KeyEvent::Release(Key(KeyMappingCode::CapsLock)));

                if let Err(e) = handle_key_event(
                    handle.clone(),
                    KeyEvent::Release(Key(KeyMappingCode::CapsLock)),
                ) {
                    log::error!("Failed to handle Caps Lock Release: {e}");
                }
            }
        }
    });

    Ok(())
}

fn run_hidutil(args: &[&str]) -> Result<String> {
    let output = Command::new("/usr/bin/hidutil")
        .args(args)
        .output()
        .map_err(|e| anyhow::anyhow!("failed to launch hidutil: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(anyhow::anyhow!(
            "hidutil failed (status: {:?}): {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

extern "C" fn cleanup_c() {
    let json = r#"{"UserKeyMapping":[]}"#;

    match Command::new("/usr/bin/hidutil")
        .args(["property", "--set", json])
        .status()
    {
        Ok(status) if status.success() => {
            eprintln!("restored hidutil mapping");
        }
        Ok(status) => {
            eprintln!("hidutil restore failed with status: {status}");
        }
        Err(err) => {
            eprintln!("failed to launch hidutil: {err}");
        }
    }
}

pub fn remap_caps_to_no_action() -> Result<()> {
    unsafe {
        let rc = libc::atexit(cleanup_c);
        if rc != 0 {
            eprintln!("failed to register atexit handler: {rc}");
        }
    }

    // Caps Lock source usage: 0x700000039
    // Keeping the same destination value you were using for "No Action".
    let json = r#"{
        "UserKeyMapping": [
            {
                "HIDKeyboardModifierMappingSrc": 0x700000039,
                "HIDKeyboardModifierMappingDst": 0x0
            }
        ]
    }"#;

    run_hidutil(&["property", "--set", json]).map(|_| ())
}
