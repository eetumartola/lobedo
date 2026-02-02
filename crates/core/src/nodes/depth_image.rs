use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::image_data::ImageData;
use crate::nodes::{image_in, image_out};
use crate::param_spec::ParamSpec;

pub const NAME: &str = "Depth Image";

const MODEL_DIR: &str = "models/depthpro";
const MODEL_INPUT_SIZE: u32 = 1536;
const DEPTH_EPS: f32 = 1.0e-6;
const INPUT_RANGE_EPS: f32 = 1.0e-6;
const INV_DEPTH_RANGE_EPS: f32 = 1.0e-5;
const ORT_RUNTIME_DIR: &str = "models/onnxruntime";
const ORT_DIRECTML_RUNTIME_DIR: &str = "models/onnxruntime-directml";

#[cfg(target_os = "windows")]
const ORT_DYLIB_NAME: &str = "onnxruntime.dll";
#[cfg(target_os = "linux")]
const ORT_DYLIB_NAME: &str = "libonnxruntime.so";
#[cfg(target_os = "macos")]
const ORT_DYLIB_NAME: &str = "libonnxruntime.dylib";
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
const ORT_DYLIB_NAME: &str = "onnxruntime.dll";

#[cfg(not(target_arch = "wasm32"))]
use ort::session::Session;
#[cfg(not(target_arch = "wasm32"))]
use ort::ep::ExecutionProvider;

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "ML".to_string(),
        inputs: vec![image_in("image")],
        outputs: vec![
            image_out("color"),
            image_out("depth"),
            image_out("segmentation"),
        ],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            ("debug".to_string(), ParamValue::Bool(false)),
            ("directml_device_id".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::bool("debug", "Debug")
            .with_help("Print extra DepthPro diagnostics to the console."),
        ParamSpec::int("directml_device_id", "DirectML Device")
            .with_help("DirectML device id to use on Windows (0 = default adapter)."),
    ]
}

pub fn compute(
    params: &NodeParams,
    input: &ImageData,
) -> Result<(ImageData, ImageData, ImageData), String> {
    let debug = params.get_bool("debug", false);
    let directml_device_id = params.get_int("directml_device_id", 0);
    let (rgb, width, height) = input
        .rgb_data()
        .ok_or_else(|| "Depth Image requires an RGB image input".to_string())?;
    let color = ImageData::from_rgb(width, height, rgb.to_vec())?;
    let (input_min, input_max) = finite_min_max(rgb)
        .ok_or_else(|| "Depth Image input contained no finite values".to_string())?;
    if debug {
        eprintln!(
            "Depth Image debug: input size={}x{}, range=[{:.6}, {:.6}]",
            width, height, input_min, input_max
        );
        if params.values.contains_key("depth_scale") {
            eprintln!("Depth Image debug: depth_scale is ignored (moved to Depth to Splats).");
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        return Err("Depth Image is not supported in web builds".to_string());
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let signature = input_signature(rgb, width, height, debug);
        if let Some(cache) = DEPTH_ERROR_CACHE.get() {
            if let Ok(guard) = cache.lock() {
                if let Some(last) = guard.as_ref() {
                    if last.signature == signature {
                        return Err(last.message.clone());
                    }
                }
            }
        }
        let mut inv_depth = run_depthpro(rgb, width, height, debug, directml_device_id)?;
        let (mut min_inv, mut max_inv) = finite_min_max(&inv_depth)
            .ok_or_else(|| "DepthPro output contained no finite values".to_string())?;
        if max_inv <= 0.0 || min_inv < 0.0 {
            let shift = -min_inv + DEPTH_EPS;
            for value in &mut inv_depth {
                if value.is_finite() {
                    *value += shift;
                } else {
                    *value = DEPTH_EPS;
                }
            }
            if let Some((new_min, new_max)) = finite_min_max(&inv_depth) {
                min_inv = new_min;
                max_inv = new_max;
            }
        }
        if (max_inv - min_inv).abs() < INV_DEPTH_RANGE_EPS {
            let message = format!(
                "DepthPro output is nearly constant (min={min_inv:.6}, max={max_inv:.6}, input_min={input_min:.6}, input_max={input_max:.6})."
            );
            let cache = DEPTH_ERROR_CACHE.get_or_init(|| Mutex::new(None));
            if let Ok(mut guard) = cache.lock() {
                *guard = Some(DepthErrorCache { signature, message: message.clone() });
            }
            return Err(message);
        }
        let mut depth = Vec::with_capacity(inv_depth.len());
        for value in inv_depth {
            let inv = if value.is_finite() { value.max(DEPTH_EPS) } else { DEPTH_EPS };
            let mut z = if inv > 0.0 { 1.0 / inv } else { 0.0 };
            if !z.is_finite() {
                z = 0.0;
            }
            depth.push(z);
        }
        if debug {
            if let Some((min_depth, max_depth)) = finite_min_max(&depth) {
                eprintln!(
                    "Depth Image debug: linear depth range=[{:.6}, {:.6}]",
                    min_depth, max_depth
                );
            }
        }
        let depth = ImageData::from_depth(width, height, depth)?;
        let seg = ImageData::from_seg(width, height, vec![0u32; (width * height) as usize])?;
        if let Some(cache) = DEPTH_ERROR_CACHE.get() {
            if let Ok(mut guard) = cache.lock() {
                if let Some(last) = guard.as_ref() {
                    if last.signature == signature {
                        *guard = None;
                    }
                }
            }
        }
        Ok((color, depth, seg))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run_depthpro(
    rgb: &[f32],
    width: u32,
    height: u32,
    debug: bool,
    directml_device_id: i32,
) -> Result<Vec<f32>, String> {
    use half::f16;
    use image::{ImageBuffer, Luma, Rgb};
    use image::imageops::{crop_imm, overlay, resize, FilterType};
    use ort::value::Tensor;

    let model_path = find_model_path()?;
    ensure_ort_initialized(debug)?;
    let (_, input_max) = finite_min_max(rgb)
        .ok_or_else(|| "Depth Image input contained no finite values".to_string())?;
    if input_max <= INPUT_RANGE_EPS {
        return Err("Depth Image input appears to be black or invalid".to_string());
    }
    let input_scale = if input_max > 1.0 + INPUT_RANGE_EPS {
        1.0 / input_max
    } else {
        1.0
    };

    let input =
        ImageBuffer::<Rgb<f32>, Vec<f32>>::from_vec(width, height, rgb.to_vec())
            .ok_or_else(|| "Invalid RGB image buffer".to_string())?;

    let scale = (MODEL_INPUT_SIZE as f32 / width as f32)
        .min(MODEL_INPUT_SIZE as f32 / height as f32);
    let new_w = (width as f32 * scale).round().max(1.0) as u32;
    let new_h = (height as f32 * scale).round().max(1.0) as u32;

    let resized = resize(&input, new_w, new_h, FilterType::Lanczos3);
    let pad_x = (MODEL_INPUT_SIZE - new_w) / 2;
    let pad_y = (MODEL_INPUT_SIZE - new_h) / 2;
    if debug {
        eprintln!(
            "Depth Image debug: model={}, input_scale={:.6}, resized={}x{}, pad=({}, {})",
            model_path.display(),
            input_scale,
            new_w,
            new_h,
            pad_x,
            pad_y
        );
    }

    let mut padded = ImageBuffer::from_pixel(
        MODEL_INPUT_SIZE,
        MODEL_INPUT_SIZE,
        Rgb([0.0, 0.0, 0.0]),
    );
    overlay(&mut padded, &resized, pad_x as i64, pad_y as i64);

    let mut input_data =
        vec![f16::from_f32(0.0); (MODEL_INPUT_SIZE * MODEL_INPUT_SIZE * 3) as usize];
    for y in 0..MODEL_INPUT_SIZE {
        for x in 0..MODEL_INPUT_SIZE {
            let pixel = padded.get_pixel(x, y);
            let base = (y * MODEL_INPUT_SIZE + x) as usize;
            let r = (pixel[0] * input_scale).clamp(0.0, 1.0) * 2.0 - 1.0;
            let g = (pixel[1] * input_scale).clamp(0.0, 1.0) * 2.0 - 1.0;
            let b = (pixel[2] * input_scale).clamp(0.0, 1.0) * 2.0 - 1.0;
            let plane = (MODEL_INPUT_SIZE * MODEL_INPUT_SIZE) as usize;
            input_data[base] = f16::from_f32(r);
            input_data[base + plane] = f16::from_f32(g);
            input_data[base + plane * 2] = f16::from_f32(b);
        }
    }

    let input_tensor = Tensor::from_array((
        [1usize, 3, MODEL_INPUT_SIZE as usize, MODEL_INPUT_SIZE as usize],
        input_data.into_boxed_slice(),
    ))
    .map_err(|err| err.to_string())?;

    let output_data = run_model_tensor(&model_path, input_tensor, debug, directml_device_id)?;
    let output_f32: Vec<f32> = output_data.iter().map(|v| v.to_f32()).collect();
    if debug {
        if let Some((min_out, max_out)) = finite_min_max(&output_f32) {
            let sample = output_f32
                .iter()
                .take(8)
                .map(|v| format!("{v:.5}"))
                .collect::<Vec<_>>()
                .join(", ");
            eprintln!(
                "Depth Image debug: output len={}, range=[{:.6}, {:.6}], sample=[{}]",
                output_f32.len(),
                min_out,
                max_out,
                sample
            );
        } else {
            eprintln!("Depth Image debug: output had no finite values");
        }
    }

    let output_image = ImageBuffer::<Luma<f32>, Vec<f32>>::from_vec(
        MODEL_INPUT_SIZE,
        MODEL_INPUT_SIZE,
        output_f32,
    )
    .ok_or_else(|| "Depth output buffer size mismatch".to_string())?;
    let cropped = crop_imm(&output_image, pad_x, pad_y, new_w, new_h).to_image();
    if debug {
        let raw = cropped.as_raw();
        if let Some((min_crop, max_crop)) = finite_min_max(raw) {
            eprintln!(
                "Depth Image debug: cropped range=[{:.6}, {:.6}]",
                min_crop, max_crop
            );
        }
    }
    let raw = resize_depth(cropped.as_raw(), new_w, new_h, width, height);
    if debug {
        if let Some((min_resize, max_resize)) = finite_min_max(&raw) {
            eprintln!(
                "Depth Image debug: resized range=[{:.6}, {:.6}]",
                min_resize, max_resize
            );
        }
    }
    Ok(raw)
}

#[cfg(not(target_arch = "wasm32"))]
fn find_model_path() -> Result<PathBuf, String> {
    let dir = Path::new(MODEL_DIR);
    if !dir.exists() {
        return Err(format!("DepthPro model directory not found: {}", dir.display()));
    }
    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if path.is_file() {
            let ext = path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if ext == "onnx" {
                candidates.push(path);
            }
        }
    }
    candidates.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    candidates
        .into_iter()
        .next()
        .ok_or_else(|| "No DepthPro ONNX model found in models/depthpro".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
struct DepthModelCache {
    path: PathBuf,
    directml_device_id: i32,
    session: Session,
}

#[cfg(not(target_arch = "wasm32"))]
static DEPTH_MODEL: OnceLock<Mutex<Option<DepthModelCache>>> = OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
enum OrtInitState {
    Uninitialized,
    Ready,
    Failed,
}

#[cfg(not(target_arch = "wasm32"))]
static ORT_INIT: OnceLock<Mutex<OrtInitState>> = OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
struct DepthErrorCache {
    signature: u64,
    message: String,
}

#[cfg(not(target_arch = "wasm32"))]
static DEPTH_ERROR_CACHE: OnceLock<Mutex<Option<DepthErrorCache>>> = OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
fn resolve_ort_dylib_path() -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("ORT_DYLIB_PATH") {
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return Ok(candidate);
        }
        return Err(format!(
            "ORT_DYLIB_PATH points to missing file: {}",
            candidate.display()
        ));
    }

    let mut candidates = Vec::new();
    candidates.push(PathBuf::from(ORT_DIRECTML_RUNTIME_DIR).join(ORT_DYLIB_NAME));
    candidates.push(PathBuf::from(ORT_RUNTIME_DIR).join(ORT_DYLIB_NAME));
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(ORT_DYLIB_NAME));
            candidates.push(dir.join(ORT_RUNTIME_DIR).join(ORT_DYLIB_NAME));
        }
    }
    candidates.push(PathBuf::from(ORT_DYLIB_NAME));

    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(format!(
        "ONNX Runtime >= 1.23.x not found. Place {ORT_DYLIB_NAME} into {ORT_DIRECTML_RUNTIME_DIR} or {ORT_RUNTIME_DIR}, or set ORT_DYLIB_PATH."
    ))
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_ort_initialized(debug: bool) -> Result<(), String> {
    let state = ORT_INIT.get_or_init(|| Mutex::new(OrtInitState::Uninitialized));
    let mut guard = state
        .lock()
        .map_err(|_| "ONNX Runtime init lock poisoned".to_string())?;
    if matches!(*guard, OrtInitState::Ready) {
        return Ok(());
    }

    let path = resolve_ort_dylib_path()?;
    if debug {
        eprintln!("Depth Image debug: using ONNX Runtime {}", path.display());
    }
    let init_result = ort::init_from(&path)
        .map_err(|err| format!("Failed to load ONNX Runtime from {}: {err}", path.display()))
        .map(|builder| {
            builder.commit();
        });

    match init_result {
        Ok(()) => {
            *guard = OrtInitState::Ready;
            Ok(())
        }
        Err(err) => {
            *guard = OrtInitState::Failed;
            Err(err)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run_model_tensor(
    path: &Path,
    input: ort::value::Tensor<half::f16>,
    debug: bool,
    directml_device_id: i32,
) -> Result<Vec<half::f16>, String> {
    let store = DEPTH_MODEL.get_or_init(|| Mutex::new(None));
    let mut guard = store.lock().map_err(|_| "Depth model lock poisoned".to_string())?;
    let needs_reload = guard
        .as_ref()
        .map(|cache| cache.path != path || cache.directml_device_id != directml_device_id)
        .unwrap_or(true);
    if needs_reload {
        let mut builder = Session::builder().map_err(|err| err.to_string())?;
        #[cfg(target_os = "windows")]
        {
            let directml = ort::ep::DirectML::default().with_device_id(directml_device_id);
            let available = directml.is_available().unwrap_or(false);
            if debug {
                eprintln!(
                    "Depth Image debug: DirectML available={available}, device_id={directml_device_id}"
                );
            }
            if available {
                builder = builder
                    .with_execution_providers([directml.build()])
                    .map_err(|err| format!("Failed to enable DirectML: {err}"))?;
                if debug {
                    eprintln!("Depth Image debug: DirectML requested");
                }
            }
        }
        let session = builder
            .commit_from_file(path)
            .map_err(|err| format!("Failed to load DepthPro model: {err}"))?;
        if debug {
            let input_names = session
                .inputs()
                .iter()
                .map(|input| input.name().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            let output_names = session
                .outputs()
                .iter()
                .map(|output| output.name().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            eprintln!(
                "Depth Image debug: session inputs=[{input_names}], outputs=[{output_names}]"
            );
        }
        *guard = Some(DepthModelCache {
            path: path.to_path_buf(),
            directml_device_id,
            session,
        });
    }

    let cache = guard
        .as_mut()
        .ok_or_else(|| "DepthPro model cache unavailable".to_string())?;
    let outputs = cache
        .session
        .run(ort::inputs![input])
        .map_err(|err| format!("DepthPro inference failed: {err}"))?;
    let output = &outputs[0];
    let (shape, data) = output
        .try_extract_tensor::<half::f16>()
        .map_err(|err| err.to_string())?;
    if debug {
        eprintln!("Depth Image debug: output shape {:?}", shape);
    }
    Ok(data.to_vec())
}

fn finite_min_max(values: &[f32]) -> Option<(f32, f32)> {
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for &value in values {
        if value.is_finite() {
            if value < min {
                min = value;
            }
            if value > max {
                max = value;
            }
        }
    }
    if min.is_finite() && max.is_finite() {
        Some((min, max))
    } else {
        None
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn input_signature(rgb: &[f32], width: u32, height: u32, debug: bool) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    width.hash(&mut hasher);
    height.hash(&mut hasher);
    debug.hash(&mut hasher);
    let stride = (rgb.len() / 1024).max(1);
    for idx in (0..rgb.len()).step_by(stride) {
        rgb[idx].to_bits().hash(&mut hasher);
    }
    hasher.finish()
}

#[cfg(not(target_arch = "wasm32"))]
fn resize_depth(
    input: &[f32],
    in_w: u32,
    in_h: u32,
    out_w: u32,
    out_h: u32,
) -> Vec<f32> {
    if in_w == 0 || in_h == 0 || out_w == 0 || out_h == 0 {
        return Vec::new();
    }
    if in_w == out_w && in_h == out_h {
        return input.to_vec();
    }

    let scale_x = in_w as f32 / out_w as f32;
    let scale_y = in_h as f32 / out_h as f32;
    let mut out = vec![0.0f32; (out_w * out_h) as usize];

    let sample = |x: i32, y: i32| -> f32 {
        let xx = x.clamp(0, (in_w - 1) as i32) as u32;
        let yy = y.clamp(0, (in_h - 1) as i32) as u32;
        let idx = (yy * in_w + xx) as usize;
        let value = input.get(idx).copied().unwrap_or(0.0);
        if value.is_finite() { value } else { 0.0 }
    };

    for y in 0..out_h {
        let src_y = (y as f32 + 0.5) * scale_y - 0.5;
        let y0 = src_y.floor() as i32;
        let y1 = y0 + 1;
        let wy = src_y - y0 as f32;
        let wy0 = 1.0 - wy;
        for x in 0..out_w {
            let src_x = (x as f32 + 0.5) * scale_x - 0.5;
            let x0 = src_x.floor() as i32;
            let x1 = x0 + 1;
            let wx = src_x - x0 as f32;
            let wx0 = 1.0 - wx;

            let v00 = sample(x0, y0);
            let v10 = sample(x1, y0);
            let v01 = sample(x0, y1);
            let v11 = sample(x1, y1);
            let v0 = v00 * wx0 + v10 * wx;
            let v1 = v01 * wx0 + v11 * wx;
            let value = v0 * wy0 + v1 * wy;
            out[(y * out_w + x) as usize] = value;
        }
    }

    out
}
