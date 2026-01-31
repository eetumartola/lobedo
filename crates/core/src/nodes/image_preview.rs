use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use crate::assets;
use crate::attributes::{AttributeDomain, AttributeStorage, StringTableAttribute};
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams};
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
        values: BTreeMap::new(),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    Vec::new()
}

pub fn compute(params: &NodeParams, input: &ImageData) -> Result<Geometry, String> {
    let _ = params;
    let (rgb, width, height) = input
        .rgb_data()
        .ok_or_else(|| "Image Preview requires an RGB image input".to_string())?;
    if width == 0 || height == 0 {
        return Err("Image Preview requires a non-empty image".to_string());
    }

    let hash = image_hash(rgb, width, height);
    let texture_path = encode_preview_texture(rgb, width, height, hash)?;

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
    material.base_color_texture = Some(texture_path);

    let mut geometry = Geometry::with_mesh(mesh);
    geometry.materials.insert(material);

    let material_attr = AttributeStorage::StringTable(StringTableAttribute::new(
        vec![material_name],
        vec![0, 0],
    ));
    if let Some(mesh) = geometry.meshes.first_mut() {
        let _ = mesh.set_attribute(AttributeDomain::Primitive, "material", material_attr);
    }

    Ok(geometry)
}

fn image_hash(rgb: &[f32], width: u32, height: u32) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    width.hash(&mut hasher);
    height.hash(&mut hasher);
    let stride = (rgb.len() / 1024).max(1);
    for idx in (0..rgb.len()).step_by(stride) {
        rgb[idx].to_bits().hash(&mut hasher);
    }
    hasher.finish()
}

fn encode_preview_texture(
    rgb: &[f32],
    width: u32,
    height: u32,
    hash: u64,
) -> Result<String, String> {
    use image::codecs::png::PngEncoder;
    use image::ExtendedColorType;
    use image::ImageEncoder;

    let mut bytes = Vec::with_capacity((width * height * 3) as usize);
    for &value in rgb {
        let clamped = value.clamp(0.0, 1.0);
        bytes.push((clamped * 255.0 + 0.5) as u8);
    }

    let mut png_data = Vec::new();
    let encoder = PngEncoder::new(&mut png_data);
    encoder
        .write_image(&bytes, width, height, ExtendedColorType::Rgb8)
        .map_err(|err| err.to_string())?;

    let key = format!("image_preview/{hash}.png");
    Ok(assets::store_bytes_with_key(key, png_data))
}
