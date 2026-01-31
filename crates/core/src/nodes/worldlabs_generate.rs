use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};

use serde::Deserialize;
use serde_json::{json, Value};

use crate::assets;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::nodes::geometry_out;
use crate::param_spec::{ParamPathKind, ParamSpec};
use crate::splat::{load_splat_ply_with_mode, load_splat_spz_with_mode, SplatGeo, SplatLoadMode};

use tracing::warn;

#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

#[cfg(not(target_arch = "wasm32"))]
use base64::Engine;

pub const NAME: &str = "WorldLabs Generate";

const API_BASE_URL: &str = "https://api.worldlabs.ai/marble/v1";
const DEFAULT_MODE: i32 = 0;
const DEFAULT_MODEL: i32 = 1;
const DEFAULT_AUTO_ENHANCE: bool = true;
const DEFAULT_IS_PANO: bool = false;
const DEFAULT_SEED: i32 = -1;
const DEFAULT_REQUEST_TOKEN: i32 = 0;

const REQUEST_SNAPSHOT_KEY: &str = "request_snapshot";
const REQUEST_TOKEN_KEY: &str = "request_token";

const POLL_INTERVAL_SECS: u64 = 5;
const MAX_WAIT_SECS: u64 = 900;

#[derive(Clone)]
struct WorldLabsCacheEntry {
    result: WorldLabsCachedResult,
}

#[derive(Clone)]
enum WorldLabsCachedResult {
    Ok(WorldLabsAsset),
    Err(String),
}

#[derive(Clone)]
enum WorldLabsAsset {
    Ply(String),
    Spz(String),
}

static WORLDLABS_CACHE: OnceLock<Mutex<HashMap<String, WorldLabsCacheEntry>>> = OnceLock::new();

#[derive(Deserialize)]
struct GenerateWorldResponse {
    operation_id: String,
    error: Option<OperationError>,
}

#[derive(Deserialize)]
struct GetOperationResponse {
    done: bool,
    error: Option<OperationError>,
    response: Option<Value>,
}

#[derive(Deserialize)]
struct OperationError {
    message: Option<String>,
    code: Option<i64>,
}

#[derive(Deserialize)]
struct WorldLabsRequestSnapshot {
    mode: i32,
    text_prompt: String,
    image_path: String,
    auto_enhance: bool,
    is_pano: bool,
    model: i32,
    seed: i32,
    tags: String,
    display_name: String,
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Sources".to_string(),
        inputs: Vec::new(),
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            ("mode".to_string(), ParamValue::Int(DEFAULT_MODE)),
            ("text_prompt".to_string(), ParamValue::String(String::new())),
            ("image_path".to_string(), ParamValue::String(String::new())),
            ("auto_enhance".to_string(), ParamValue::Bool(DEFAULT_AUTO_ENHANCE)),
            ("is_pano".to_string(), ParamValue::Bool(DEFAULT_IS_PANO)),
            ("model".to_string(), ParamValue::Int(DEFAULT_MODEL)),
            ("seed".to_string(), ParamValue::Int(DEFAULT_SEED)),
            ("tags".to_string(), ParamValue::String(String::new())),
            ("display_name".to_string(), ParamValue::String(String::new())),
            (
                REQUEST_SNAPSHOT_KEY.to_string(),
                ParamValue::String(String::new()),
            ),
            (
                REQUEST_TOKEN_KEY.to_string(),
                ParamValue::Int(DEFAULT_REQUEST_TOKEN),
            ),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::int_enum("mode", "Mode", vec![(0, "Text"), (1, "Image")])
            .with_help("Generate splats from text or an image prompt."),
        ParamSpec::string("text_prompt", "Text Prompt")
            .with_help("Text prompt for text mode (or optional guidance in image mode)."),
        ParamSpec::path("image_path", "Image", ParamPathKind::ReadTexture)
            .with_help("Image file path or URL for image mode.")
            .visible_when_int("mode", 1),
        ParamSpec::bool("auto_enhance", "Auto Enhance")
            .with_help("Allow recaptioning to improve prompts."),
        ParamSpec::bool("is_pano", "Is Panorama")
            .with_help("Treat the input image as a panorama.")
            .visible_when_int("mode", 1),
        ParamSpec::int_enum(
            "model",
            "Model",
            vec![(0, "Marble 0.1-mini"), (1, "Marble 0.1-plus")],
        )
        .with_help("WorldLabs model selection."),
        ParamSpec::int_slider("seed", "Seed", -1, 1_000_000)
            .with_help("Seed for deterministic outputs (-1 for random)."),
        ParamSpec::string("tags", "Tags")
            .with_help("Optional tags (comma or space separated)."),
        ParamSpec::string("display_name", "Display Name")
            .with_help("Optional display name for the generated world."),
        ParamSpec::string(REQUEST_SNAPSHOT_KEY, "Request Snapshot").hidden(),
        ParamSpec::int(REQUEST_TOKEN_KEY, "Request Token").hidden(),
    ]
}

pub fn compute(params: &NodeParams) -> Result<SplatGeo, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = params;
        return Err("WorldLabs Generate is not supported in web builds.".to_string());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let request_snapshot = params.get_string(REQUEST_SNAPSHOT_KEY, "");
        let request_token = params.get_int(REQUEST_TOKEN_KEY, DEFAULT_REQUEST_TOKEN);
        if request_snapshot.trim().is_empty() || request_token <= 0 {
            return Err("Press Generate to run WorldLabs.".to_string());
        }
        let snapshot: WorldLabsRequestSnapshot = serde_json::from_str(request_snapshot)
            .map_err(|err| format!("WorldLabs request snapshot invalid: {err}"))?;
        let mode = snapshot.mode.clamp(0, 1);
        let text_prompt = snapshot.text_prompt;
        let image_path = snapshot.image_path;
        let auto_enhance = snapshot.auto_enhance;
        let is_pano = snapshot.is_pano;
        let seed = snapshot.seed;
        let tags = snapshot.tags;
        let display_name = snapshot.display_name;

        let world_prompt = if mode == 0 {
            if text_prompt.trim().is_empty() {
                return Err("WorldLabs Generate (Text) requires a text prompt.".to_string());
            }
            json!({
                "type": "text",
                "text_prompt": text_prompt,
                "disable_recaption": !auto_enhance,
            })
        } else {
            if image_path.trim().is_empty() {
                return Err("WorldLabs Generate (Image) requires an image path or URL.".to_string());
            }
            let image_prompt = if assets::is_url(&image_path) {
                json!({
                    "source": "uri",
                    "uri": image_path,
                })
            } else {
                let bytes = std::fs::read(image_path)
                    .map_err(|err| format!("WorldLabs image read failed: {err}"))?;
                let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
                json!({
                    "source": "data_base64",
                    "data_base64": encoded,
                })
            };
            let text_value = if text_prompt.trim().is_empty() {
                Value::Null
            } else {
                Value::String(text_prompt.to_string())
            };
            json!({
                "type": "image",
                "text_prompt": text_value,
                "disable_recaption": !auto_enhance,
                "image_prompt": image_prompt,
                "is_pano": is_pano,
            })
        };

        let model = match snapshot.model {
            0 => "Marble 0.1-mini",
            _ => "Marble 0.1-plus",
        };

        let tags_vec = parse_tags(&tags);
        let request_body = build_request(world_prompt, model, seed, &tags_vec, &display_name);
        let cache_inputs = CacheKeyInputs {
            request_token,
            request_snapshot,
        };
        let cache_key = cache_key(&cache_inputs);

        if let Some(entry) = worldlabs_cache()
            .lock()
            .expect("worldlabs cache lock")
            .get(&cache_key)
            .cloned()
        {
            return match entry.result {
                WorldLabsCachedResult::Ok(asset) => load_worldlabs_asset(&asset),
                WorldLabsCachedResult::Err(err) => Err(err),
            };
        }

        let result = (|| -> Result<WorldLabsAsset, String> {
            let api_key = worldlabs_api_key()?;
            let op_response = api_post_json("worlds:generate", &request_body, &api_key)?;
            let op: GenerateWorldResponse = serde_json::from_value(op_response)
                .map_err(|err| format!("WorldLabs response parse failed: {err}"))?;
            if let Some(err) = op.error {
                return Err(format_operation_error("WorldLabs generation failed", &err));
            }

            let world_id = poll_operation(&api_key, &op.operation_id)?;
            let world = api_get_json(&format!("worlds/{world_id}"), &api_key)?;
            extract_worldlabs_asset(&world)
        })();

        match result {
            Ok(asset) => {
                worldlabs_cache()
                    .lock()
                    .expect("worldlabs cache lock")
                    .insert(
                        cache_key,
                        WorldLabsCacheEntry {
                            result: WorldLabsCachedResult::Ok(asset.clone()),
                        },
                    );
                load_worldlabs_asset(&asset)
            }
            Err(err) => {
                warn!("WorldLabs Generate error: {err}");
                worldlabs_cache()
                    .lock()
                    .expect("worldlabs cache lock")
                    .insert(
                        cache_key,
                        WorldLabsCacheEntry {
                            result: WorldLabsCachedResult::Err(err.clone()),
                        },
                    );
                Err(err)
            }
        }
    }
}

fn worldlabs_cache() -> &'static Mutex<HashMap<String, WorldLabsCacheEntry>> {
    WORLDLABS_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(not(target_arch = "wasm32"))]
fn worldlabs_api_key() -> Result<String, String> {
    if let Ok(key) = std::env::var("WLT_API_KEY") {
        if !key.trim().is_empty() {
            return Ok(key);
        }
    }
    if let Ok(key) = std::env::var("WORLDLABS_API_KEY") {
        if !key.trim().is_empty() {
            return Ok(key);
        }
    }
    Err("Set environment variable WLT_API_KEY with your WorldLabs API key.".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn api_get_json(path: &str, api_key: &str) -> Result<Value, String> {
    api_request_json("GET", path, None, api_key)
}

#[cfg(not(target_arch = "wasm32"))]
fn api_post_json(path: &str, body: &Value, api_key: &str) -> Result<Value, String> {
    api_request_json("POST", path, Some(body), api_key)
}

#[cfg(not(target_arch = "wasm32"))]
fn api_request_json(
    method: &str,
    path: &str,
    body: Option<&Value>,
    api_key: &str,
) -> Result<Value, String> {
    let url = format!("{API_BASE_URL}/{path}");
    let mut request = match method {
        "POST" => ureq::post(&url),
        "GET" => ureq::get(&url),
        _ => return Err(format!("Unsupported WorldLabs method: {method}")),
    };
    request = request.set("WLT-Api-Key", api_key);
    if body.is_some() {
        request = request.set("Content-Type", "application/json");
    }
    let response = match body {
        Some(body) => {
            let payload = serde_json::to_string(body)
                .map_err(|err| format!("WorldLabs request encode failed: {err}"))?;
            request.send_string(&payload)
        }
        None => request.call(),
    };
    match response {
        Ok(resp) => {
            let text = resp
                .into_string()
                .map_err(|err| format!("WorldLabs response read failed: {err}"))?;
            serde_json::from_str(&text)
                .map_err(|err| format!("WorldLabs response parse failed: {err}"))
        }
        Err(ureq::Error::Status(code, resp)) => {
            let text = resp.into_string().unwrap_or_default();
            Err(format!("WorldLabs {path} failed ({code}): {text}"))
        }
        Err(err) => Err(format!("WorldLabs {path} failed: {err}")),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn poll_operation(api_key: &str, operation_id: &str) -> Result<String, String> {
    let start = Instant::now();
    loop {
        let response = api_get_json(&format!("operations/{operation_id}"), api_key)?;
        let op: GetOperationResponse = serde_json::from_value(response)
            .map_err(|err| format!("WorldLabs operation parse failed: {err}"))?;
        if op.done {
            if let Some(err) = op.error {
                return Err(format_operation_error("WorldLabs operation failed", &err));
            }
            let response = op
                .response
                .ok_or_else(|| "WorldLabs operation returned no response.".to_string())?;
            let world_id = response
                .get("world_id")
                .and_then(Value::as_str)
                .ok_or_else(|| "WorldLabs response missing world_id.".to_string())?;
            return Ok(world_id.to_string());
        }
        if start.elapsed() > Duration::from_secs(MAX_WAIT_SECS) {
            return Err(format!(
                "WorldLabs generation timed out after {}s.",
                MAX_WAIT_SECS
            ));
        }
        std::thread::sleep(Duration::from_secs(POLL_INTERVAL_SECS));
    }
}

fn format_operation_error(prefix: &str, err: &OperationError) -> String {
    let mut message = err.message.clone().unwrap_or_else(|| "Unknown error".to_string());
    if let Some(code) = err.code {
        message = format!("{message} (code {code})");
    }
    format!("{prefix}: {message}")
}

fn parse_tags(raw: &str) -> Vec<String> {
    raw.split(|c: char| c == ',' || c.is_whitespace())
        .filter(|tag| !tag.trim().is_empty())
        .map(|tag| tag.trim().to_string())
        .collect()
}

fn build_request(
    world_prompt: Value,
    model: &str,
    seed: i32,
    tags: &[String],
    display_name: &str,
) -> Value {
    let mut map = serde_json::Map::new();
    map.insert("world_prompt".to_string(), world_prompt);
    map.insert("model".to_string(), Value::String(model.to_string()));
    if seed >= 0 {
        map.insert(
            "seed".to_string(),
            Value::Number(serde_json::Number::from(seed)),
        );
    }
    if !tags.is_empty() {
        map.insert("tags".to_string(), json!(tags));
    }
    if !display_name.trim().is_empty() {
        map.insert(
            "display_name".to_string(),
            Value::String(display_name.trim().to_string()),
        );
    }
    map.insert(
        "permission".to_string(),
        json!({
            "public": false,
            "allowed_readers": [],
            "allowed_writers": [],
        }),
    );
    Value::Object(map)
}

#[derive(Hash)]
struct CacheKeyInputs<'a> {
    request_token: i32,
    request_snapshot: &'a str,
}

fn cache_key(inputs: &CacheKeyInputs<'_>) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    inputs.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn extract_worldlabs_asset(world: &Value) -> Result<WorldLabsAsset, String> {
    let assets = world
        .get("assets")
        .ok_or_else(|| "WorldLabs response missing assets.".to_string())?;
    let splats = assets
        .get("splats")
        .ok_or_else(|| "WorldLabs response missing splat assets.".to_string())?;
    if let Some(url) = find_url_with_extension(splats, ".ply") {
        return Ok(WorldLabsAsset::Ply(url));
    }
    if let Some(url) = find_url_with_extension(splats, ".spz") {
        return Ok(WorldLabsAsset::Spz(url));
    }
    Err("WorldLabs splat assets did not include a PLY URL.".to_string())
}

fn load_worldlabs_asset(asset: &WorldLabsAsset) -> Result<SplatGeo, String> {
    match asset {
        WorldLabsAsset::Ply(url) => load_splat_ply_with_mode(url, SplatLoadMode::Full),
        WorldLabsAsset::Spz(url) => load_splat_spz_with_mode(url, SplatLoadMode::Full),
    }
}

fn find_url_with_extension(value: &Value, ext: &str) -> Option<String> {
    match value {
        Value::String(s) => {
            if s.contains(ext) {
                Some(s.to_string())
            } else {
                None
            }
        }
        Value::Array(items) => items
            .iter()
            .find_map(|item| find_url_with_extension(item, ext)),
        Value::Object(map) => map
            .values()
            .find_map(|item| find_url_with_extension(item, ext)),
        _ => None,
    }
}
