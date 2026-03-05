use std::collections::BTreeMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

use crate::assets;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::image_data::ImageData;
use crate::nodes::image_out;
use crate::param_spec::{ParamPathKind, ParamSpec};

pub const NAME: &str = "Image";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Sources".to_string(),
        inputs: Vec::new(),
        outputs: vec![image_out("image")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([("path".to_string(), ParamValue::String(String::new()))]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![ParamSpec::path("path", "Path", ParamPathKind::ReadTexture)
        .with_help("Path or URL to an image (PNG/JPEG).")]
}

pub fn compute(params: &NodeParams) -> Result<ImageData, String> {
    let path = params.get_string("path", "");
    if path.trim().is_empty() {
        return Err("Image node requires a path".to_string());
    }
    load_image(path)
}

fn load_image(path: &str) -> Result<ImageData, String> {
    if let Some(data) = assets::load_bytes(path) {
        return decode_image_bytes(&data);
    }
    #[cfg(target_arch = "wasm32")]
    {
        if assets::is_url(path) {
            return Err("Image URL is downloading; retrying shortly.".to_string());
        }
        return Err("Image node is not supported in web builds without a picked file".to_string());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if assets::is_url(path) {
            return Err(format!("Failed to download URL: {path}"));
        }
        let path = Path::new(path);
        if !path.exists() {
            return Err(format!("Image not found: {}", path.display()));
        }
        let data = std::fs::read(path).map_err(|err| err.to_string())?;
        decode_image_bytes(&data)
    }
}

fn decode_image_bytes(data: &[u8]) -> Result<ImageData, String> {
    let image = ::image::load_from_memory(data).map_err(|err| err.to_string())?;
    let rgb = image.to_rgb32f();
    let (width, height) = rgb.dimensions();
    let data = rgb.into_raw();
    ImageData::from_rgb(width, height, data)
}
