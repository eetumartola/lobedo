use std::collections::HashMap;
#[cfg(target_arch = "wasm32")]
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

#[cfg(target_arch = "wasm32")]
use crate::graph::NodeId;
#[cfg(target_arch = "wasm32")]
use crate::progress::{current_progress_context, ProgressEvent, ProgressSink};

static ASSET_STORE: OnceLock<Mutex<HashMap<String, Vec<u8>>>> = OnceLock::new();
static URL_STORE: OnceLock<Mutex<HashMap<String, Vec<u8>>>> = OnceLock::new();
#[cfg(target_arch = "wasm32")]
static URL_PENDING: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
#[cfg(target_arch = "wasm32")]
static URL_PROGRESS: OnceLock<Mutex<HashMap<String, UrlProgressEntry>>> = OnceLock::new();
static NEXT_ASSET_ID: AtomicUsize = AtomicUsize::new(1);
static URL_REVISION: AtomicUsize = AtomicUsize::new(0);

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;
#[cfg(target_arch = "wasm32")]
use web_sys::Response;

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
    if path.starts_with("mem://") {
        let store = ASSET_STORE.get_or_init(|| Mutex::new(HashMap::new()));
        return store
            .lock()
            .expect("asset store lock")
            .get(path)
            .cloned();
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
    if let Some(data) = URL_STORE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("url store lock")
        .get(path)
        .cloned()
    {
        return Some(data);
    }
    let response = ureq::get(path).call().ok()?;
    let mut reader = response.into_reader();
    let mut data = Vec::new();
    use std::io::Read;
    reader.read_to_end(&mut data).ok()?;
    URL_STORE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("url store lock")
        .insert(path.to_string(), data.clone());
    URL_REVISION.fetch_add(1, Ordering::Relaxed);
    Some(data)
}

#[cfg(target_arch = "wasm32")]
fn load_url_bytes(path: &str) -> Option<Vec<u8>> {
    if let Some(data) = URL_STORE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("url store lock")
        .get(path)
        .cloned()
    {
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
        URL_STORE
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .expect("url store lock")
            .insert(path.clone(), bytes);
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
        (listener.sink)(ProgressEvent::Start { node: listener.node });
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
        (listener.sink)(ProgressEvent::Finish { node: listener.node });
    }
}

#[cfg(target_arch = "wasm32")]
fn defer_progress_start(node: NodeId, sink: ProgressSink) {
    wasm_bindgen_futures::spawn_local(async move {
        let _ = JsFuture::from(js_sys::Promise::resolve(&JsValue::NULL)).await;
        (sink)(ProgressEvent::Start { node });
    });
}
