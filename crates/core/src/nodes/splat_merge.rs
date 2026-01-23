use std::collections::BTreeMap;

use glam::{Quat, Vec3};

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::{Mesh, MeshGroups};
use crate::nodes::splat_utils::SpatialHash;
use crate::nodes::{geometry_in, geometry_out, require_mesh_input};
use crate::param_spec::ParamSpec;
use crate::splat::SplatGeo;

pub const NAME: &str = "Splat Merge";

const DEFAULT_METHOD: i32 = 0;
const DEFAULT_BLEND_RADIUS: f32 = 0.2;
const DEFAULT_FADE_ORIGINALS: bool = true;
const DEFAULT_SKIRT_MAX_DIST: f32 = 0.5;
const DEFAULT_SKIRT_STEP: f32 = 0.2;
const DEFAULT_SKIRT_MAX_NEW: i32 = 4;
const DEFAULT_SEAM_ALPHA: f32 = 0.5;
const DEFAULT_SEAM_SCALE: f32 = 1.0;
const DEFAULT_SEAM_DC_ONLY: bool = true;
const DEFAULT_PREVIEW_SKIRT: bool = false;

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![geometry_in("a"), geometry_in("b")],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            ("method".to_string(), ParamValue::Int(DEFAULT_METHOD)),
            ("blend_radius".to_string(), ParamValue::Float(DEFAULT_BLEND_RADIUS)),
            ("fade_originals".to_string(), ParamValue::Bool(DEFAULT_FADE_ORIGINALS)),
            ("skirt_max_dist".to_string(), ParamValue::Float(DEFAULT_SKIRT_MAX_DIST)),
            ("skirt_step".to_string(), ParamValue::Float(DEFAULT_SKIRT_STEP)),
            ("skirt_max_new".to_string(), ParamValue::Int(DEFAULT_SKIRT_MAX_NEW)),
            ("seam_alpha".to_string(), ParamValue::Float(DEFAULT_SEAM_ALPHA)),
            ("seam_scale".to_string(), ParamValue::Float(DEFAULT_SEAM_SCALE)),
            ("seam_dc_only".to_string(), ParamValue::Bool(DEFAULT_SEAM_DC_ONLY)),
            ("preview_skirt".to_string(), ParamValue::Bool(DEFAULT_PREVIEW_SKIRT)),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::int_enum("method", "Method", vec![(0, "Feather"), (1, "Skirt")])
            .with_help("Join method."),
        ParamSpec::float_slider("blend_radius", "Blend Radius", 0.0, 10.0)
            .with_help("Blend radius for feathering/fade."),
        ParamSpec::bool("fade_originals", "Fade Originals")
            .with_help("Fade original splats near the seam."),
        ParamSpec::float_slider("skirt_max_dist", "Skirt Max Dist", 0.0, 10.0)
            .with_help("Maximum distance to bridge with skirt splats."),
        ParamSpec::float_slider("skirt_step", "Skirt Step", 0.0, 10.0)
            .with_help("Spacing between skirt splats."),
        ParamSpec::int_slider("skirt_max_new", "Skirt Max New", 0, 1000)
            .with_help("Maximum skirt splats per pair."),
        ParamSpec::float_slider("seam_alpha", "Seam Alpha", 0.0, 1.0)
            .with_help("Opacity for seam splats."),
        ParamSpec::float_slider("seam_scale", "Seam Scale", 0.01, 10.0)
            .with_help("Scale multiplier for seam splats."),
        ParamSpec::bool("seam_dc_only", "Seam DC Only")
            .with_help("Use DC-only SH for seam splats."),
        ParamSpec::bool("preview_skirt", "Preview Skirt")
            .with_help("Preview skirt geometry as a wireframe when selected."),
    ]
}

pub fn compute(_params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let input = require_mesh_input(inputs, 0, "Splat Merge requires a mesh input")?;
    Ok(input)
}

pub fn apply_to_geometry(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let Some(source) = inputs.first() else {
        return Ok(Geometry::default());
    };
    let Some(target) = inputs.get(1) else {
        return Err("Splat Merge requires two inputs".to_string());
    };

    let Some(splats_a) = source.merged_splats() else {
        return Err("Splat Merge requires splats on input 0".to_string());
    };
    let Some(splats_b) = target.merged_splats() else {
        return Err("Splat Merge requires splats on input 1".to_string());
    };

    let method = params.get_int("method", DEFAULT_METHOD).clamp(0, 1);
    let merged = match method {
        1 => merge_skirt(params, &splats_a, &splats_b),
        _ => merge_feather(params, &splats_a, &splats_b),
    };

    let mut meshes = Vec::new();
    if let Some(mesh) = source.merged_mesh() {
        meshes.push(mesh);
    }
    let curves = if meshes.is_empty() {
        Vec::new()
    } else {
        source.curves.clone()
    };

    Ok(Geometry {
        meshes,
        splats: vec![merged],
        curves,
        volumes: source.volumes.clone(),
        materials: source.materials.clone(),
    })
}

fn merge_feather(params: &NodeParams, a: &SplatGeo, b: &SplatGeo) -> SplatGeo {
    if a.is_empty() && b.is_empty() {
        return SplatGeo::default();
    }
    let blend_radius = params
        .get_float("blend_radius", DEFAULT_BLEND_RADIUS)
        .max(0.0);
    let (dist_a, _) = nearest_distances(&a.positions, &b.positions, blend_radius);
    let (dist_b, _) = nearest_distances(&b.positions, &a.positions, blend_radius);

    let mut a_scaled = a.clone();
    let mut b_scaled = b.clone();
    if blend_radius > 0.0 {
        let weights_a: Vec<f32> = dist_a
            .iter()
            .map(|d| weight_from_distance(*d, blend_radius))
            .collect();
        let weights_b: Vec<f32> = dist_b
            .iter()
            .map(|d| weight_from_distance(*d, blend_radius))
            .collect();
        apply_weights(&mut a_scaled, &weights_a);
        apply_weights(&mut b_scaled, &weights_b);
    }

    crate::geometry::merge_splats(&[a_scaled, b_scaled])
}

fn merge_skirt(params: &NodeParams, a: &SplatGeo, b: &SplatGeo) -> SplatGeo {
    if a.is_empty() && b.is_empty() {
        return SplatGeo::default();
    }
    let blend_radius = params
        .get_float("blend_radius", DEFAULT_BLEND_RADIUS)
        .max(0.0);
    let fade_originals = params.get_bool("fade_originals", DEFAULT_FADE_ORIGINALS);
    let max_dist = params
        .get_float("skirt_max_dist", DEFAULT_SKIRT_MAX_DIST)
        .max(0.0);
    let (dist_a, nearest_a) = nearest_distances(&a.positions, &b.positions, max_dist);
    let (dist_b, _) = nearest_distances(&b.positions, &a.positions, blend_radius);

    let mut a_scaled = a.clone();
    let mut b_scaled = b.clone();
    if fade_originals && blend_radius > 0.0 {
        let weights_a: Vec<f32> = dist_a
            .iter()
            .map(|d| weight_from_distance(*d, blend_radius))
            .collect();
        let weights_b: Vec<f32> = dist_b
            .iter()
            .map(|d| weight_from_distance(*d, blend_radius))
            .collect();
        apply_weights(&mut a_scaled, &weights_a);
        apply_weights(&mut b_scaled, &weights_b);
    }

    let seam = build_skirt_splats(params, a, b, &nearest_a);
    let mut merged = crate::geometry::merge_splats(&[a_scaled, b_scaled]);
    append_seam_splats(&mut merged, &seam);
    merged
}

pub fn build_skirt_preview_mesh(
    params: &NodeParams,
    a: &SplatGeo,
    b: &SplatGeo,
) -> Option<Mesh> {
    if a.is_empty() || b.is_empty() {
        return None;
    }
    let max_dist = params
        .get_float("skirt_max_dist", DEFAULT_SKIRT_MAX_DIST)
        .max(0.0);
    let step = params
        .get_float("skirt_step", DEFAULT_SKIRT_STEP)
        .max(1.0e-4);
    let max_new = params
        .get_int("skirt_max_new", DEFAULT_SKIRT_MAX_NEW)
        .max(0) as usize;
    if max_dist <= 0.0 || max_new == 0 {
        return None;
    }

    let (_, nearest) = nearest_distances(&a.positions, &b.positions, max_dist);
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    for (idx, hit) in nearest.iter().enumerate() {
        let Some(hit) = hit else { continue };
        if hit.dist > max_dist {
            continue;
        }
        let segments = ((hit.dist / step).ceil() as i32 - 1).max(0) as usize;
        let count = segments.min(max_new);
        if count == 0 {
            continue;
        }
        let pos_a = Vec3::from(a.positions[idx]);
        let pos_b = Vec3::from(b.positions[hit.index]);
        let mut prev = pos_a;
        for step_idx in 0..count {
            let t = (step_idx + 1) as f32 / (count + 1) as f32;
            let pos = pos_a.lerp(pos_b, t);
            push_preview_segment(&mut positions, &mut indices, prev, pos);
            prev = pos;
        }
        push_preview_segment(&mut positions, &mut indices, prev, pos_b);
    }

    if positions.is_empty() {
        None
    } else {
        Some(Mesh::with_positions_indices(positions, indices))
    }
}

fn push_preview_segment(
    positions: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    a: Vec3,
    b: Vec3,
) {
    let base = positions.len() as u32;
    positions.push(a.to_array());
    positions.push(b.to_array());
    indices.extend_from_slice(&[base, base + 1, base + 1]);
}

fn build_skirt_splats(
    params: &NodeParams,
    a: &SplatGeo,
    b: &SplatGeo,
    nearest: &[Option<NearestHit>],
) -> SplatGeo {
    let max_dist = params
        .get_float("skirt_max_dist", DEFAULT_SKIRT_MAX_DIST)
        .max(0.0);
    let step = params
        .get_float("skirt_step", DEFAULT_SKIRT_STEP)
        .max(1.0e-4);
    let max_new = params
        .get_int("skirt_max_new", DEFAULT_SKIRT_MAX_NEW)
        .max(0) as usize;
    let seam_alpha = params
        .get_float("seam_alpha", DEFAULT_SEAM_ALPHA)
        .clamp(0.0, 1.0);
    let seam_scale = params
        .get_float("seam_scale", DEFAULT_SEAM_SCALE)
        .max(0.01);
    let dc_only = params.get_bool("seam_dc_only", DEFAULT_SEAM_DC_ONLY);

    if a.is_empty() || b.is_empty() || max_dist <= 0.0 || max_new == 0 {
        return SplatGeo::default();
    }

    let max_coeffs = a.sh_coeffs.max(b.sh_coeffs);
    let mut positions = Vec::new();
    let mut rotations = Vec::new();
    let mut scales = Vec::new();
    let mut opacity = Vec::new();
    let mut sh0 = Vec::new();
    let mut sh_rest = Vec::new();
    if max_coeffs > 0 {
        sh_rest.reserve(1024 * max_coeffs);
    }

    for (idx, hit) in nearest.iter().enumerate() {
        let Some(hit) = hit else { continue };
        if hit.dist > max_dist {
            continue;
        }
        let segments = ((hit.dist / step).ceil() as i32 - 1).max(0) as usize;
        let count = segments.min(max_new);
        if count == 0 {
            continue;
        }
        let pos_a = Vec3::from(a.positions[idx]);
        let pos_b = Vec3::from(b.positions[hit.index]);
        let (rot_a, rot_b) = (quat_from_splat(a.rotations[idx]), quat_from_splat(b.rotations[hit.index]));
        let mut rot_b = rot_b;
        if rot_a.dot(rot_b) < 0.0 {
            rot_b = Quat::from_xyzw(-rot_b.x, -rot_b.y, -rot_b.z, -rot_b.w);
        }
        let scale_a = Vec3::from(a.scales[idx]);
        let scale_b = Vec3::from(b.scales[hit.index]);
        for step_idx in 0..count {
            let t = (step_idx + 1) as f32 / (count + 1) as f32;
            let position = pos_a.lerp(pos_b, t);
            let rotation = rot_a.slerp(rot_b, t);
            let log_scale = scale_a.lerp(scale_b, t);
            let sigma = Vec3::new(
                log_scale.x.exp() * seam_scale,
                log_scale.y.exp() * seam_scale,
                log_scale.z.exp() * seam_scale,
            );
            let log_scale = Vec3::new(
                sigma.x.max(1.0e-6).ln(),
                sigma.y.max(1.0e-6).ln(),
                sigma.z.max(1.0e-6).ln(),
            );

            let profile = 1.0 - (2.0 * t - 1.0).abs();
            let alpha = (seam_alpha * profile).clamp(1.0e-4, 1.0 - 1.0e-4);
            let op = logit(alpha);

            let sh0_a = a.sh0.get(idx).copied().unwrap_or([0.0, 0.0, 0.0]);
            let sh0_b = b
                .sh0
                .get(hit.index)
                .copied()
                .unwrap_or([0.0, 0.0, 0.0]);
            let sh0_out = lerp_vec3(sh0_a, sh0_b, t);

            positions.push(position.to_array());
            rotations.push([rotation.w, rotation.x, rotation.y, rotation.z]);
            scales.push([log_scale.x, log_scale.y, log_scale.z]);
            opacity.push(op);
            sh0.push(sh0_out);
            if max_coeffs > 0 {
                if dc_only {
                    sh_rest.extend(std::iter::repeat_n([0.0, 0.0, 0.0], max_coeffs));
                } else {
                    for c in 0..max_coeffs {
                        let a_val = splat_rest_coeff(a, idx, c);
                        let b_val = splat_rest_coeff(b, hit.index, c);
                        sh_rest.push(lerp_vec3(a_val, b_val, t));
                    }
                }
            }
        }
    }

    let mut splats = SplatGeo::with_len(positions.len());
    splats.positions = positions;
    splats.rotations = rotations;
    splats.scales = scales;
    splats.opacity = opacity;
    splats.sh0 = sh0;
    splats.sh_coeffs = max_coeffs;
    splats.sh_rest = sh_rest;
    splats
}

fn append_seam_splats(merged: &mut SplatGeo, seam: &SplatGeo) {
    if seam.is_empty() {
        return;
    }
    let seam_len = seam.len();
    let max_coeffs = merged.sh_coeffs.max(seam.sh_coeffs);
    if merged.sh_coeffs != max_coeffs {
        let old_coeffs = merged.sh_coeffs;
        let old_len = merged.len();
        let mut upgraded = Vec::with_capacity(old_len * max_coeffs);
        if old_coeffs == 0 {
            upgraded.extend(std::iter::repeat_n([0.0, 0.0, 0.0], old_len * max_coeffs));
        } else {
            for i in 0..old_len {
                let base = i * old_coeffs;
                for c in 0..max_coeffs {
                    let value = if c < old_coeffs {
                        merged.sh_rest[base + c]
                    } else {
                        [0.0, 0.0, 0.0]
                    };
                    upgraded.push(value);
                }
            }
        }
        merged.sh_rest = upgraded;
        merged.sh_coeffs = max_coeffs;
    }

    merged.positions.extend_from_slice(&seam.positions);
    merged.rotations.extend_from_slice(&seam.rotations);
    merged.scales.extend_from_slice(&seam.scales);
    merged.opacity.extend_from_slice(&seam.opacity);
    merged.sh0.extend_from_slice(&seam.sh0);
    if max_coeffs > 0 {
        if seam.sh_coeffs == 0 {
            merged
                .sh_rest
                .extend(std::iter::repeat_n([0.0, 0.0, 0.0], seam_len * max_coeffs));
        } else {
            let coeffs = seam.sh_coeffs;
            for i in 0..seam_len {
                let base = i * coeffs;
                for c in 0..max_coeffs {
                    let value = if c < coeffs {
                        seam.sh_rest[base + c]
                    } else {
                        [0.0, 0.0, 0.0]
                    };
                    merged.sh_rest.push(value);
                }
            }
        }
    }

    extend_attribute_defaults(&mut merged.attributes, AttributeDomain::Point, seam_len);
    extend_attribute_defaults(&mut merged.attributes, AttributeDomain::Primitive, seam_len);
    extend_group_defaults(&mut merged.groups, AttributeDomain::Point, seam_len);
    extend_group_defaults(&mut merged.groups, AttributeDomain::Primitive, seam_len);
}

fn extend_attribute_defaults(
    attrs: &mut crate::attributes::MeshAttributes,
    domain: AttributeDomain,
    count: usize,
) {
    if count == 0 {
        return;
    }
    for storage in attrs.map_mut(domain).values_mut() {
        match storage {
            AttributeStorage::Float(values) => values.extend(std::iter::repeat_n(0.0, count)),
            AttributeStorage::Int(values) => values.extend(std::iter::repeat_n(0, count)),
            AttributeStorage::Vec2(values) => {
                values.extend(std::iter::repeat_n([0.0, 0.0], count))
            }
            AttributeStorage::Vec3(values) => {
                values.extend(std::iter::repeat_n([0.0, 0.0, 0.0], count))
            }
            AttributeStorage::Vec4(values) => {
                values.extend(std::iter::repeat_n([0.0, 0.0, 0.0, 0.0], count))
            }
            AttributeStorage::StringTable(values) => {
                if values.values.is_empty() {
                    values.values.push(String::new());
                }
                values
                    .indices
                    .extend(std::iter::repeat_n(0u32, count));
            }
        }
    }
}

fn extend_group_defaults(groups: &mut MeshGroups, domain: AttributeDomain, count: usize) {
    if count == 0 {
        return;
    }
    for values in groups.map_mut(domain).values_mut() {
        values.extend(std::iter::repeat_n(false, count));
    }
}

fn apply_weights(splats: &mut SplatGeo, weights: &[f32]) {
    for (idx, weight) in weights.iter().enumerate() {
        let w = weight.clamp(0.0, 1.0);
        if let Some(sh0) = splats.sh0.get_mut(idx) {
            sh0[0] *= w;
            sh0[1] *= w;
            sh0[2] *= w;
        }
        if splats.sh_coeffs > 0 {
            let base = idx * splats.sh_coeffs;
            for coeff in 0..splats.sh_coeffs {
                if let Some(slot) = splats.sh_rest.get_mut(base + coeff) {
                    slot[0] *= w;
                    slot[1] *= w;
                    slot[2] *= w;
                }
            }
        }
        if let Some(opacity) = splats.opacity.get_mut(idx) {
            let alpha = sigmoid(*opacity) * w;
            *opacity = logit(alpha);
        }
    }
}

#[derive(Clone, Copy)]
struct NearestHit {
    index: usize,
    dist: f32,
}

fn nearest_distances(
    positions: &[[f32; 3]],
    other_positions: &[[f32; 3]],
    max_dist: f32,
) -> (Vec<f32>, Vec<Option<NearestHit>>) {
    let mut dists = vec![f32::INFINITY; positions.len()];
    let mut hits = vec![None; positions.len()];
    if positions.is_empty() || other_positions.is_empty() {
        return (dists, hits);
    }
    let search = max_dist.max(1.0e-3);
    let Some(hash) = SpatialHash::build(other_positions, search) else {
        return (dists, hits);
    };
    for (idx, pos) in positions.iter().enumerate() {
        let pos = Vec3::from(*pos);
        if let Some((hit_idx, dist)) = hash.nearest(other_positions, pos, max_dist) {
            dists[idx] = dist;
            hits[idx] = Some(NearestHit {
                index: hit_idx,
                dist,
            });
        }
    }
    (dists, hits)
}

fn splat_rest_coeff(splats: &SplatGeo, index: usize, coeff: usize) -> [f32; 3] {
    if splats.sh_coeffs == 0 {
        return [0.0, 0.0, 0.0];
    }
    let base = index * splats.sh_coeffs;
    if base + coeff >= splats.sh_rest.len() {
        return [0.0, 0.0, 0.0];
    }
    splats.sh_rest[base + coeff]
}

fn weight_from_distance(dist: f32, radius: f32) -> f32 {
    if !dist.is_finite() || radius <= 0.0 {
        return 1.0;
    }
    smoothstep(0.0, radius, dist)
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    if edge1 <= edge0 {
        return if x < edge0 { 0.0 } else { 1.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp_vec3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn quat_from_splat(rotation: [f32; 4]) -> Quat {
    let mut quat = Quat::from_xyzw(rotation[1], rotation[2], rotation[3], rotation[0]);
    if quat.length_squared() > 0.0 {
        quat = quat.normalize();
    } else {
        quat = Quat::IDENTITY;
    }
    quat
}

fn sigmoid(value: f32) -> f32 {
    1.0 / (1.0 + (-value).exp())
}

fn logit(value: f32) -> f32 {
    let clamped = value.clamp(1.0e-6, 1.0 - 1.0e-6);
    (clamped / (1.0 - clamped)).ln()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_feather_keeps_counts() {
        let a = SplatGeo::with_len(2);
        let b = SplatGeo::with_len(3);
        let params = NodeParams {
            values: BTreeMap::from([("method".to_string(), ParamValue::Int(0))]),
        };
        let merged = merge_feather(&params, &a, &b);
        assert_eq!(merged.len(), 5);
    }

    #[test]
    fn merge_skirt_adds_splats() {
        let mut a = SplatGeo::with_len(1);
        a.positions[0] = [0.0, 0.0, 0.0];
        let mut b = SplatGeo::with_len(1);
        b.positions[0] = [0.4, 0.0, 0.0];
        let params = NodeParams {
            values: BTreeMap::from([
                ("method".to_string(), ParamValue::Int(1)),
                ("skirt_max_dist".to_string(), ParamValue::Float(1.0)),
                ("skirt_step".to_string(), ParamValue::Float(0.1)),
                ("skirt_max_new".to_string(), ParamValue::Int(3)),
            ]),
        };
        let merged = merge_skirt(&params, &a, &b);
        assert!(merged.len() >= 2);
    }
}
