use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

static ASSET_STORE: OnceLock<Mutex<HashMap<String, Vec<u8>>>> = OnceLock::new();
static NEXT_ASSET_ID: AtomicUsize = AtomicUsize::new(1);

pub fn store_bytes(name: String, data: Vec<u8>) -> String {
    let id = NEXT_ASSET_ID.fetch_add(1, Ordering::Relaxed);
    let key = if name.trim().is_empty() {
        format!("mem://{}", id)
    } else {
        format!("mem://{}::{}", id, name)
    };
    let store = ASSET_STORE.get_or_init(|| Mutex::new(HashMap::new()));
    store.lock().expect("asset store lock").insert(key.clone(), data);
    key
}

pub fn load_bytes(path: &str) -> Option<Vec<u8>> {
    if !path.starts_with("mem://") {
        return None;
    }
    let store = ASSET_STORE.get_or_init(|| Mutex::new(HashMap::new()));
    store
        .lock()
        .expect("asset store lock")
        .get(path)
        .cloned()
}
