use std::collections::BTreeMap;

use glam::{Vec2, Vec3};

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    attribute_utils::{existing_float_attr_mesh, existing_float_attr_splats},
    geometry_in,
    geometry_out,
    group_utils::{mask_has_any, mesh_group_mask, splat_group_mask},
    recompute_mesh_normals,
    require_mesh_input,
};
use crate::splat::SplatGeo;

pub const NAME: &str = "Erosion Noise";

#[derive(Clone, Copy)]
struct ErosionSettings {
    freq: f32,
    octaves: i32,
    roughness: f32,
    lacunarity: f32,
    slope_strength: f32,
    branch_strength: f32,
    strength: f32,
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![geometry_in("in")],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            ("erosion_strength".to_string(), ParamValue::Float(0.1)),
            ("erosion_freq".to_string(), ParamValue::Float(15.0)),
            ("erosion_octaves".to_string(), ParamValue::Int(4)),
            ("erosion_roughness".to_string(), ParamValue::Float(0.3)),
            ("erosion_lacunarity".to_string(), ParamValue::Float(2.17)),
            ("erosion_slope_strength".to_string(), ParamValue::Float(2.0)),
            ("erosion_branch_strength".to_string(), ParamValue::Float(2.5)),
            ("do_mask".to_string(), ParamValue::Bool(false)),
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut input = require_mesh_input(inputs, 0, "Erosion Noise requires a mesh input")?;
    apply_to_mesh(params, &mut input)?;
    Ok(input)
}

pub(crate) fn apply_to_splats(params: &NodeParams, splats: &mut SplatGeo) -> Result<(), String> {
    let count = splats.positions.len();
    if count == 0 {
        return Ok(());
    }
    let settings = erosion_settings(params);
    let mask = splat_group_mask(splats, params, AttributeDomain::Point);
    if !mask_has_any(mask.as_deref()) {
        return Ok(());
    }
    let do_mask = params.get_bool("do_mask", false);
    let mut mask_values = if do_mask {
        Some(existing_float_attr_splats(
            splats,
            AttributeDomain::Point,
            "mask",
            count,
        ))
    } else {
        None
    };

    let bounds = splat_bounds(splats);
    let hmin = bounds.0.y;
    let hmax = bounds.1.y;
    let hrange = (hmax - hmin).max(1.0e-6);
    let x_range = (bounds.1.x - bounds.0.x).max(1.0e-6);
    let z_range = (bounds.1.z - bounds.0.z).max(1.0e-6);

    let mut normals = splats
        .attribute(AttributeDomain::Point, "N")
        .and_then(|attr| match attr {
            crate::attributes::AttributeRef::Vec3(values)
                if values.len() == splats.positions.len() =>
            {
                Some(values.to_vec())
            }
            _ => None,
        })
        .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; splats.positions.len()]);
    if normals.len() != splats.positions.len() {
        normals = vec![[0.0, 1.0, 0.0]; splats.positions.len()];
    }

    for (idx, (pos, normal)) in splats
        .positions
        .iter_mut()
        .zip(normals.iter())
        .enumerate()
    {
        if mask
            .as_ref()
            .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
        {
            continue;
        }
        let position = Vec3::from(*pos);
        let uv = uv_from_bounds(position, bounds.0, x_range, z_range);
        let h_raw = position.y;
        let normalized_height = ((h_raw - hmin) / hrange).clamp(0.0, 1.0);
        let grad = gradient_from_normal(Vec3::from(*normal), normalized_height);
        let eroded = apply_erosion(uv, grad, settings);
        let new_height = hmin + eroded.x * hrange;
        let mut next = position;
        next.y = new_height;
        *pos = next.to_array();
        if let Some(mask_values) = mask_values.as_mut() {
            if let Some(slot) = mask_values.get_mut(idx) {
                *slot = 0.5 + 0.5 * eroded.y;
            }
        }
    }

    if let Some(mask_values) = mask_values {
        splats
            .set_attribute(
                AttributeDomain::Point,
                "mask",
                AttributeStorage::Float(mask_values),
            )
            .map_err(|err| format!("Erosion Noise error: {:?}", err))?;
    }

    Ok(())
}

fn apply_to_mesh(params: &NodeParams, mesh: &mut Mesh) -> Result<(), String> {
    if mesh.positions.is_empty() {
        return Ok(());
    }
    let settings = erosion_settings(params);
    let mask = mesh_group_mask(mesh, params, AttributeDomain::Point);
    if !mask_has_any(mask.as_deref()) {
        return Ok(());
    }
    let do_mask = params.get_bool("do_mask", false);
    let mut mask_values = if do_mask {
        Some(existing_float_attr_mesh(
            mesh,
            AttributeDomain::Point,
            "mask",
            mesh.positions.len(),
        ))
    } else {
        None
    };

    let bounds = mesh
        .bounds()
        .ok_or_else(|| "Erosion Noise requires a valid mesh bounds".to_string())?;
    let min = Vec3::from(bounds.min);
    let max = Vec3::from(bounds.max);
    let hmin = min.y;
    let hmax = max.y;
    let hrange = (hmax - hmin).max(1.0e-6);
    let x_range = (max.x - min.x).max(1.0e-6);
    let z_range = (max.z - min.z).max(1.0e-6);

    if mesh.normals.is_none() {
        let _ = mesh.compute_normals();
    }
    let mut normals = mesh
        .normals
        .clone()
        .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; mesh.positions.len()]);
    if normals.len() != mesh.positions.len() {
        normals = vec![[0.0, 1.0, 0.0]; mesh.positions.len()];
    }

    for (idx, (pos, normal)) in mesh
        .positions
        .iter_mut()
        .zip(normals.iter())
        .enumerate()
    {
        if mask
            .as_ref()
            .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
        {
            continue;
        }
        let position = Vec3::from(*pos);
        let uv = uv_from_bounds(position, min, x_range, z_range);
        let h_raw = position.y;
        let normalized_height = ((h_raw - hmin) / hrange).clamp(0.0, 1.0);
        let grad = gradient_from_normal(Vec3::from(*normal), normalized_height);
        let eroded = apply_erosion(uv, grad, settings);
        let new_height = hmin + eroded.x * hrange;
        let mut next = position;
        next.y = new_height;
        *pos = next.to_array();
        if let Some(mask_values) = mask_values.as_mut() {
            if let Some(slot) = mask_values.get_mut(idx) {
                *slot = 0.5 + 0.5 * eroded.y;
            }
        }
    }

    if let Some(mask_values) = mask_values {
        mesh.set_attribute(
            AttributeDomain::Point,
            "mask",
            AttributeStorage::Float(mask_values),
        )
        .map_err(|err| format!("Erosion Noise error: {:?}", err))?;
    }

    recompute_mesh_normals(mesh);
    Ok(())
}

fn erosion_settings(params: &NodeParams) -> ErosionSettings {
    ErosionSettings {
        freq: params.get_float("erosion_freq", 15.0).max(0.0),
        octaves: params.get_int("erosion_octaves", 4).clamp(1, 12),
        roughness: params.get_float("erosion_roughness", 0.3).max(0.0),
        lacunarity: params.get_float("erosion_lacunarity", 2.17).max(0.0),
        slope_strength: params.get_float("erosion_slope_strength", 2.0),
        branch_strength: params.get_float("erosion_branch_strength", 2.5),
        strength: params.get_float("erosion_strength", 0.1),
    }
}

fn uv_from_bounds(position: Vec3, min: Vec3, x_range: f32, z_range: f32) -> Vec2 {
    let u = ((position.x - min.x) / x_range).clamp(0.0, 1.0);
    let v = ((position.z - min.z) / z_range).clamp(0.0, 1.0);
    Vec2::new(u, v)
}

fn gradient_from_normal(normal: Vec3, normalized_height: f32) -> Vec3 {
    let ny = normal.y;
    let (dx, dz) = if ny.abs() > 1.0e-6 {
        (-normal.x / ny, -normal.z / ny)
    } else {
        (0.0, 0.0)
    };
    Vec3::new(normalized_height, dx, dz)
}

fn apply_erosion(uv: Vec2, grad: Vec3, settings: ErosionSettings) -> Vec2 {
    let normalized_height = grad.x;
    let dir = Vec2::new(grad.z, -grad.y) * settings.slope_strength;

    let mut h = Vec3::ZERO;
    let mut amplitude = 0.5;
    let mut frequency = 1.0;

    for _ in 0..settings.octaves {
        let branch = Vec2::new(h.z, -h.y) * settings.branch_strength;
        let e = erosion(uv * settings.freq * frequency, dir + branch);
        h += e * amplitude * Vec3::new(1.0, frequency, frequency);
        amplitude *= settings.roughness;
        frequency *= settings.lacunarity;
    }

    let erosion_offset = (h.x - 0.5) * settings.strength;
    let new_height = normalized_height + erosion_offset;
    Vec2::new(new_height, h.x)
}

fn erosion(p: Vec2, dir: Vec2) -> Vec3 {
    let ip = Vec2::new(p.x.floor(), p.y.floor());
    let fp = vec2_fract(p);
    let f = std::f32::consts::TAU;

    let mut value = Vec3::ZERO;
    let mut weight_sum = 0.0;

    for i in -2..=1 {
        for j in -2..=1 {
            let o = Vec2::new(i as f32, j as f32);
            let h = hash(ip - o) * 0.5;
            let pp = fp + o - h;
            let d = pp.dot(pp);
            let w = (-d * 2.0).exp();
            weight_sum += w;
            let mag = pp.dot(dir);
            let sample = Vec3::new((mag * f).cos(), -(mag * f).sin() * dir.x, -(mag * f).sin() * dir.y);
            value += sample * w;
        }
    }

    if weight_sum > 1.0e-6 {
        value / weight_sum
    } else {
        Vec3::ZERO
    }
}

fn hash(x_in: Vec2) -> Vec2 {
    let k = Vec2::new(0.3183099, 0.3678794);
    let x = x_in * k + Vec2::new(k.y, k.x);
    let prod = x.x * x.y * (x.x + x.y);
    let inner = vec2_fract(Vec2::splat(prod));
    let scaled = vec2_fract(k * 16.0 * inner);
    Vec2::new(-1.0, -1.0) + 2.0 * scaled
}

fn vec2_fract(v: Vec2) -> Vec2 {
    Vec2::new(v.x.fract(), v.y.fract())
}

fn splat_bounds(splats: &SplatGeo) -> (Vec3, Vec3) {
    let mut iter = splats.positions.iter();
    let Some(first) = iter.next().copied() else {
        return (Vec3::ZERO, Vec3::ZERO);
    };
    let mut min = Vec3::from(first);
    let mut max = Vec3::from(first);
    for p in iter {
        let v = Vec3::from(*p);
        min = min.min(v);
        max = max.max(v);
    }
    (min, max)
}
