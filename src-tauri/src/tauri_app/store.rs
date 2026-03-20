use std::collections::HashMap;
use tauri_plugin_store::Store;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use tauri::Wry;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GlobalHotkeyStoreEntry {
    pub bundle_id: String,
    pub hotkey_id: String,
}


#[derive(Clone)]
pub struct GlobalHotkeyStore {
    pub store: Arc<Store<Wry>>,
}

impl GlobalHotkeyStore {
    pub fn new(store: Arc<Store<Wry>>) -> Self {
        Self { store }
    }

    pub fn get_all(&self) -> HashMap<String, GlobalHotkeyStoreEntry> {
        self.store
            .entries()
            .into_iter()
            .filter_map(|(k, v)| {
                serde_json::from_value::<GlobalHotkeyStoreEntry>(v.clone())
                    .ok()
                    .map(|entry| (k.to_string(), entry))
            })
            .collect()
    }

    pub fn set(&self, shortcut: &str, entry: GlobalHotkeyStoreEntry) {
        let value = serde_json::to_value(entry).unwrap();
        self.store.set(shortcut, value);
    }

    pub fn remove(&self, shortcut: &str) {
        self.store.delete(shortcut);
    }
}
