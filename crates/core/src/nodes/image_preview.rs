use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use crate::assets;
use crate::attributes::{AttributeDomain, AttributeStorage, StringTableAttribute};
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::image_data::ImageData;
use crate::material::Material;
use crate::mesh::Mesh;
use crate::nodes::{geometry_out, image_in};
use crate::param_spec::ParamSpec;

pub const NAME: &str = "Image Preview";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![image_in("image")],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            ("black_point".to_string(), ParamValue::Float(0.0)),
            ("white_point".to_string(), ParamValue::Float(1.0)),
            ("srgb_gamma".to_string(), ParamValue::Bool(false)),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::float("black_point", "Black Point")
            .with_help("Input value that maps to black in the preview."),
        ParamSpec::float("white_point", "White Point")
            .with_help("Input value that maps to white in the preview."),
        ParamSpec::bool("srgb_gamma", "sRGB Gamma")
            .with_help("Apply linear-to-sRGB gamma before writing the preview texture."),
    ]
}

pub fn compute(params: &NodeParams, input: &ImageData) -> Result<Geometry, String> {
    let black_point = params.get_float("black_point", 0.0);
    let white_point = params.get_float("white_point", 1.0);
    let srgb_gamma = params.get_bool("srgb_gamma", false);
    let (rgb, width, height, range) = match input {
        ImageData::RgbF32 {
            width,
            height,
            data,
        } => {
            let range = finite_min_max(data).unwrap_or((0.0, 1.0));
            let mapped = map_rgb(data, black_point, white_point);
            (mapped, *width, *height, range)
        }
        ImageData::R32F {
            width,
            height,
            data,
        } => {
            let range = finite_min_max(data).unwrap_or((0.0, 1.0));
            let mapped = map_scalar_to_rgb(data, black_point, white_point);
            (mapped, *width, *height, range)
        }
        ImageData::R32U {
            width,
            height,
            data,
        } => {
            let range = finite_min_max_u32(data).unwrap_or((0.0, 1.0));
            let mapped = map_scalar_to_rgb_u32(data, black_point, white_point);
            (mapped, *width, *height, range)
        }
    };
    if width == 0 || height == 0 {
        return Err("Image Preview requires a non-empty image".to_string());
    }

    let hash = image_hash(
        &rgb,
        width,
        height,
        black_point,
        white_point,
        range,
        srgb_gamma,
    );
    let texture_path = encode_preview_texture(&rgb, width, height, hash, srgb_gamma)?;

    let aspect = width as f32 / height as f32;
    let (quad_w, quad_h) = if aspect.is_finite() && aspect > 0.0 {
        (aspect, 1.0)
    } else {
        (1.0, 1.0)
    };
    let half_w = quad_w * 0.5;
    let half_h = quad_h * 0.5;

    let positions = vec![
        [-half_w, -half_h, 0.0],
        [half_w, -half_h, 0.0],
        [half_w, half_h, 0.0],
        [-half_w, half_h, 0.0],
    ];
    let indices = vec![0, 1, 2, 0, 2, 3];
    let mut mesh = Mesh::with_positions_indices(positions, indices);
    mesh.uvs = Some(vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]);

    let material_name = format!("image_preview_{hash}");
    let mut material = Material::new(material_name.clone());
    material.unlit = true;
    material.base_color_texture = Some(texture_path);

    let mut geometry = Geometry::with_mesh(mesh);
    geometry.materials.insert(material);

    let material_attr =
        AttributeStorage::StringTable(StringTableAttribute::new(vec![material_name], vec![0, 0]));
    if let Some(mesh) = geometry.meshes.first_mut() {
        let _ = mesh.set_attribute(AttributeDomain::Primitive, "material", material_attr);
    }

    Ok(geometry)
}

fn image_hash(
    rgb: &[f32],
    width: u32,
    height: u32,
    black_point: f32,
    white_point: f32,
    range: (f32, f32),
    srgb_gamma: bool,
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    width.hash(&mut hasher);
    height.hash(&mut hasher);
    black_point.to_bits().hash(&mut hasher);
    white_point.to_bits().hash(&mut hasher);
    srgb_gamma.hash(&mut hasher);
    range.0.to_bits().hash(&mut hasher);
    range.1.to_bits().hash(&mut hasher);
    rgb.len().hash(&mut hasher);
    for &value in rgb {
        value.to_bits().hash(&mut hasher);
    }
    hasher.finish()
}

fn encode_preview_texture(
    rgb: &[f32],
    width: u32,
    height: u32,
    hash: u64,
    srgb_gamma: bool,
) -> Result<String, String> {
    use image::codecs::png::PngEncoder;
    use image::ExtendedColorType;
    use image::ImageEncoder;

    let mut bytes = Vec::with_capacity((width * height * 3) as usize);
    for &value in rgb {
        let clamped = value.clamp(0.0, 1.0);
        let encoded = if srgb_gamma {
            linear_to_srgb(clamped)
        } else {
            clamped
        };
        bytes.push((encoded * 255.0 + 0.5) as u8);
    }

    let mut png_data = Vec::new();
    let encoder = PngEncoder::new(&mut png_data);
    encoder
        .write_image(&bytes, width, height, ExtendedColorType::Rgb8)
        .map_err(|err| err.to_string())?;

    let key = format!("image_preview/{hash}.png");
    Ok(assets::store_bytes_with_key(key, png_data))
}

fn linear_to_srgb(value: f32) -> f32 {
    if value <= 0.003_130_8 {
        12.92 * value
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
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

fn finite_min_max_u32(values: &[u32]) -> Option<(f32, f32)> {
    let mut min = u32::MAX;
    let mut max = u32::MIN;
    for &value in values {
        min = min.min(value);
        max = max.max(value);
    }
    if values.is_empty() {
        None
    } else {
        Some((min as f32, max as f32))
    }
}

fn map_rgb(values: &[f32], black_point: f32, white_point: f32) -> Vec<f32> {
    let (b, w) = normalize_range(black_point, white_point);
    let scale = if w > b { 1.0 / (w - b) } else { 1.0 };
    let mut out = Vec::with_capacity(values.len());
    for chunk in values.chunks_exact(3) {
        let r = ((chunk[0] - b) * scale).clamp(0.0, 1.0);
        let g = ((chunk[1] - b) * scale).clamp(0.0, 1.0);
        let bch = ((chunk[2] - b) * scale).clamp(0.0, 1.0);
        out.push(r);
        out.push(g);
        out.push(bch);
    }
    out
}

fn map_scalar_to_rgb(values: &[f32], black_point: f32, white_point: f32) -> Vec<f32> {
    let (b, w) = normalize_range(black_point, white_point);
    let scale = if w > b { 1.0 / (w - b) } else { 1.0 };
    let mut out = Vec::with_capacity(values.len() * 3);
    for &value in values {
        let v = if value.is_finite() { value } else { b };
        let mapped = ((v - b) * scale).clamp(0.0, 1.0);
        out.extend_from_slice(&[mapped, mapped, mapped]);
    }
    out
}

fn map_scalar_to_rgb_u32(values: &[u32], black_point: f32, white_point: f32) -> Vec<f32> {
    let (b, w) = normalize_range(black_point, white_point);
    let scale = if w > b { 1.0 / (w - b) } else { 1.0 };
    let mut out = Vec::with_capacity(values.len() * 3);
    for &value in values {
        let mapped = ((value as f32 - b) * scale).clamp(0.0, 1.0);
        out.extend_from_slice(&[mapped, mapped, mapped]);
    }
    out
}

fn normalize_range(black_point: f32, white_point: f32) -> (f32, f32) {
    if white_point > black_point {
        (black_point, white_point)
    } else {
        (black_point, black_point + 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_to_srgb_maps_mid_gray() {
        let encoded = linear_to_srgb(0.5);
        assert!((encoded - 0.735_356_9).abs() < 1.0e-6);
    }

    #[test]
    fn image_hash_changes_when_srgb_toggle_changes() {
        let rgb = vec![0.2, 0.4, 0.6, 0.1, 0.3, 0.5];
        let h_linear = image_hash(&rgb, 2, 1, 0.0, 1.0, (0.0, 1.0), false);
        let h_srgb = image_hash(&rgb, 2, 1, 0.0, 1.0, (0.0, 1.0), true);
        assert_ne!(h_linear, h_srgb);
    }

    #[test]
    fn image_hash_changes_when_single_pixel_changes() {
        let mut rgb = vec![0.0; 4096 * 3];
        let h0 = image_hash(&rgb, 64, 64, 0.0, 1.0, (0.0, 1.0), false);
        rgb[3072] = 0.25;
        let h1 = image_hash(&rgb, 64, 64, 0.0, 1.0, (0.0, 1.0), false);
        assert_ne!(h0, h1);
    }
}
