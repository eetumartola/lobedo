use std::collections::HashMap;
#[cfg(target_arch = "wasm32")]
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

#[cfg(target_arch = "wasm32")]
use crate::graph::NodeId;
#[cfg(target_arch = "wasm32")]
use crate::progress::{current_progress_context, ProgressEvent, ProgressSink};

const MAX_ASSET_STORE_ENTRIES: usize = 256;
const MAX_ASSET_STORE_BYTES: usize = 256 * 1024 * 1024;
const MAX_URL_STORE_ENTRIES: usize = 128;
const MAX_URL_STORE_BYTES: usize = 512 * 1024 * 1024;

static ASSET_STORE: OnceLock<Mutex<ByteCacheState>> = OnceLock::new();
static URL_STORE: OnceLock<Mutex<ByteCacheState>> = OnceLock::new();
#[cfg(target_arch = "wasm32")]
static URL_PENDING: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
#[cfg(target_arch = "wasm32")]
static URL_PROGRESS: OnceLock<Mutex<HashMap<String, UrlProgressEntry>>> = OnceLock::new();
static NEXT_ASSET_ID: AtomicUsize = AtomicUsize::new(1);
static URL_REVISION: AtomicUsize = AtomicUsize::new(0);
static CACHE_TICK: AtomicUsize = AtomicUsize::new(1);

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;
#[cfg(target_arch = "wasm32")]
use web_sys::Response;

#[derive(Default)]
struct ByteCacheState {
    entries: HashMap<String, ByteCacheEntry>,
    total_bytes: usize,
}

struct ByteCacheEntry {
    data: Vec<u8>,
    last_access: usize,
}

fn next_cache_tick() -> usize {
    CACHE_TICK.fetch_add(1, Ordering::Relaxed)
}

fn insert_cached_bytes(
    state: &mut ByteCacheState,
    key: String,
    data: Vec<u8>,
    max_entries: usize,
    max_bytes: usize,
) {
    let last_access = next_cache_tick();
    if let Some(old) = state.entries.remove(&key) {
        state.total_bytes = state.total_bytes.saturating_sub(old.data.len());
    }
    state.total_bytes = state.total_bytes.saturating_add(data.len());
    state
        .entries
        .insert(key.clone(), ByteCacheEntry { data, last_access });
    trim_cached_bytes(state, max_entries, max_bytes, Some(key.as_str()));
}

fn get_cached_bytes(state: &mut ByteCacheState, key: &str) -> Option<Vec<u8>> {
    let entry = state.entries.get_mut(key)?;
    entry.last_access = next_cache_tick();
    Some(entry.data.clone())
}

fn trim_cached_bytes(
    state: &mut ByteCacheState,
    max_entries: usize,
    max_bytes: usize,
    protected: Option<&str>,
) {
    while (state.entries.len() > max_entries || state.total_bytes > max_bytes)
        && state.entries.len() > 1
    {
        let oldest_key = state
            .entries
            .iter()
            .filter(|(key, _)| Some(key.as_str()) != protected)
            .min_by_key(|(_, entry)| entry.last_access)
            .map(|(key, _)| key.clone());
        let Some(oldest_key) = oldest_key else {
            break;
        };
        if let Some(old) = state.entries.remove(&oldest_key) {
            state.total_bytes = state.total_bytes.saturating_sub(old.data.len());
        }
    }
}

pub fn store_bytes(name: String, data: Vec<u8>) -> String {
    let id = NEXT_ASSET_ID.fetch_add(1, Ordering::Relaxed);
    let key = if name.trim().is_empty() {
        format!("mem://{id}")
    } else {
        format!("mem://{id}::{name}")
    };
    let store = ASSET_STORE.get_or_init(|| Mutex::new(ByteCacheState::default()));
    let mut guard = store.lock().expect("asset store lock");
    insert_cached_bytes(
        &mut guard,
        key.clone(),
        data,
        MAX_ASSET_STORE_ENTRIES,
        MAX_ASSET_STORE_BYTES,
    );
    key
}

pub fn store_bytes_with_key(key: String, data: Vec<u8>) -> String {
    let key = if key.starts_with("mem://") {
        key
    } else {
        format!("mem://{key}")
    };
    let store = ASSET_STORE.get_or_init(|| Mutex::new(ByteCacheState::default()));
    let mut guard = store.lock().expect("asset store lock");
    insert_cached_bytes(
        &mut guard,
        key.clone(),
        data,
        MAX_ASSET_STORE_ENTRIES,
        MAX_ASSET_STORE_BYTES,
    );
    key
}

pub fn load_bytes(path: &str) -> Option<Vec<u8>> {
    if path.starts_with("mem://") {
        let store = ASSET_STORE.get_or_init(|| Mutex::new(ByteCacheState::default()));
        let mut guard = store.lock().expect("asset store lock");
        return get_cached_bytes(&mut guard, path);
    }
    if is_url(path) {
        return load_url_bytes(path);
    }
    None
}

pub fn is_url(path: &str) -> bool {
    let trimmed = path.trim();
    let lower = trimmed.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

pub fn url_revision() -> usize {
    URL_REVISION.load(Ordering::Relaxed)
}

#[cfg(not(target_arch = "wasm32"))]
fn load_url_bytes(path: &str) -> Option<Vec<u8>> {
    let store = URL_STORE.get_or_init(|| Mutex::new(ByteCacheState::default()));
    if let Some(data) = get_cached_bytes(&mut store.lock().expect("url store lock"), path) {
        return Some(data);
    }
    let response = ureq::get(path).call().ok()?;
    let mut reader = response.into_reader();
    let mut data = Vec::new();
    use std::io::Read;
    reader.read_to_end(&mut data).ok()?;
    let mut guard = store.lock().expect("url store lock");
    insert_cached_bytes(
        &mut guard,
        path.to_string(),
        data.clone(),
        MAX_URL_STORE_ENTRIES,
        MAX_URL_STORE_BYTES,
    );
    URL_REVISION.fetch_add(1, Ordering::Relaxed);
    Some(data)
}

#[cfg(target_arch = "wasm32")]
fn load_url_bytes(path: &str) -> Option<Vec<u8>> {
    let store = URL_STORE.get_or_init(|| Mutex::new(ByteCacheState::default()));
    if let Some(data) = get_cached_bytes(&mut store.lock().expect("url store lock"), path) {
        return Some(data);
    }
    register_url_progress(path);
    let pending = URL_PENDING.get_or_init(|| Mutex::new(HashSet::new()));
    let mut pending_guard = pending.lock().expect("url pending lock");
    if pending_guard.insert(path.to_string()) {
        start_url_fetch(path.to_string());
    }
    None
}

#[cfg(target_arch = "wasm32")]
fn start_url_fetch(path: String) {
    wasm_bindgen_futures::spawn_local(async move {
        begin_url_progress(&path);
        let Some(window) = web_sys::window() else {
            clear_pending(&path);
            finish_url_progress(&path);
            return;
        };
        let resp_value = match JsFuture::from(window.fetch_with_str(&path)).await {
            Ok(val) => val,
            Err(_) => {
                clear_pending(&path);
                finish_url_progress(&path);
                return;
            }
        };
        let resp: Response = match resp_value.dyn_into() {
            Ok(resp) => resp,
            Err(_) => {
                clear_pending(&path);
                finish_url_progress(&path);
                return;
            }
        };
        let buffer_promise = match resp.array_buffer() {
            Ok(buf) => buf,
            Err(_) => {
                clear_pending(&path);
                finish_url_progress(&path);
                return;
            }
        };
        let buffer = match JsFuture::from(buffer_promise).await {
            Ok(buf) => buf,
            Err(_) => {
                clear_pending(&path);
                finish_url_progress(&path);
                return;
            }
        };
        let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
        let store = URL_STORE.get_or_init(|| Mutex::new(ByteCacheState::default()));
        let mut guard = store.lock().expect("url store lock");
        insert_cached_bytes(
            &mut guard,
            path.clone(),
            bytes,
            MAX_URL_STORE_ENTRIES,
            MAX_URL_STORE_BYTES,
        );
        URL_REVISION.fetch_add(1, Ordering::Relaxed);
        clear_pending(&path);
        finish_url_progress(&path);
    });
}

#[cfg(target_arch = "wasm32")]
fn clear_pending(path: &str) {
    if let Some(pending) = URL_PENDING.get() {
        pending.lock().expect("url pending lock").remove(path);
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
struct UrlProgressListener {
    node: NodeId,
    sink: ProgressSink,
}

#[cfg(target_arch = "wasm32")]
struct UrlProgressEntry {
    started: bool,
    listeners: Vec<UrlProgressListener>,
}

#[cfg(target_arch = "wasm32")]
fn register_url_progress(path: &str) {
    let Some((node, sink)) = current_progress_context() else {
        return;
    };
    let map = URL_PROGRESS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = map.lock().expect("url progress lock");
    let entry = map
        .entry(path.to_string())
        .or_insert_with(|| UrlProgressEntry {
            started: false,
            listeners: Vec::new(),
        });
    if entry.listeners.iter().any(|listener| listener.node == node) {
        return;
    }
    entry.listeners.push(UrlProgressListener {
        node,
        sink: sink.clone(),
    });
    if entry.started {
        defer_progress_start(node, sink);
    }
}

#[cfg(target_arch = "wasm32")]
fn begin_url_progress(path: &str) {
    let map = URL_PROGRESS.get_or_init(|| Mutex::new(HashMap::new()));
    let listeners = {
        let mut map = map.lock().expect("url progress lock");
        let entry = map
            .entry(path.to_string())
            .or_insert_with(|| UrlProgressEntry {
                started: false,
                listeners: Vec::new(),
            });
        entry.started = true;
        entry.listeners.clone()
    };
    for listener in listeners {
        (listener.sink)(ProgressEvent::Start {
            node: listener.node,
        });
    }
}

#[cfg(target_arch = "wasm32")]
fn finish_url_progress(path: &str) {
    let map = URL_PROGRESS.get_or_init(|| Mutex::new(HashMap::new()));
    let listeners = map
        .lock()
        .expect("url progress lock")
        .remove(path)
        .map(|entry| entry.listeners)
        .unwrap_or_default();
    for listener in listeners {
        (listener.sink)(ProgressEvent::Finish {
            node: listener.node,
        });
    }
}

#[cfg(target_arch = "wasm32")]
fn defer_progress_start(node: NodeId, sink: ProgressSink) {
    wasm_bindgen_futures::spawn_local(async move {
        let _ = JsFuture::from(js_sys::Promise::resolve(&JsValue::NULL)).await;
        (sink)(ProgressEvent::Start { node });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_cache_evicts_oldest_entry_limit() {
        let mut state = ByteCacheState::default();
        insert_cached_bytes(&mut state, "a".to_string(), vec![1], 2, 1024);
        insert_cached_bytes(&mut state, "b".to_string(), vec![2], 2, 1024);
        insert_cached_bytes(&mut state, "c".to_string(), vec![3], 2, 1024);

        assert!(!state.entries.contains_key("a"));
        assert!(state.entries.contains_key("b"));
        assert!(state.entries.contains_key("c"));
    }

    #[test]
    fn byte_cache_get_refreshes_lru_order() {
        let mut state = ByteCacheState::default();
        insert_cached_bytes(&mut state, "a".to_string(), vec![1], 2, 1024);
        insert_cached_bytes(&mut state, "b".to_string(), vec![2], 2, 1024);

        let cached = get_cached_bytes(&mut state, "a");
        assert_eq!(cached, Some(vec![1]));

        insert_cached_bytes(&mut state, "c".to_string(), vec![3], 2, 1024);
        assert!(state.entries.contains_key("a"));
        assert!(!state.entries.contains_key("b"));
        assert!(state.entries.contains_key("c"));
    }

    #[test]
    fn byte_cache_evicts_oldest_byte_limit() {
        let mut state = ByteCacheState::default();
        insert_cached_bytes(&mut state, "a".to_string(), vec![0; 6], 8, 10);
        insert_cached_bytes(&mut state, "b".to_string(), vec![0; 6], 8, 10);

        assert!(!state.entries.contains_key("a"));
        assert!(state.entries.contains_key("b"));
        assert_eq!(state.total_bytes, 6);
    }
}
