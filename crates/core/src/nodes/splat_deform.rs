use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

use glam::{Mat3, Vec3};

use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{geometry_in, geometry_out, require_mesh_input};
use crate::nodes::splat_utils::splat_cell_key;
use crate::splat::SplatGeo;

pub const NAME: &str = "Splat Deform";
const MIN_NEIGHBORS: usize = 3;
const MAX_NEIGHBORS: usize = 16;
const STRETCH_THRESHOLD: f32 = 1.5;
const MAX_NEW_PER_EDGE: usize = 2;
const RATIO_CLAMP_MULT: f32 = 2.0;

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![geometry_in("source"), geometry_in("deform")],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([(
            "allow_new".to_string(),
            ParamValue::Bool(false),
        ), (
            "derive_rot_scale".to_string(),
            ParamValue::Bool(true),
        )]),
    }
}

pub fn compute(_params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let input = require_mesh_input(inputs, 0, "Splat Deform requires a mesh input")?;
    Ok(input)
}

pub fn apply_to_geometry(
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    let Some(source) = inputs.first() else {
        return Ok(Geometry::default());
    };
    let Some(target_geo) = inputs.get(1) else {
        return Err("Splat Deform requires a deformed point cloud input".to_string());
    };
    if source.splats.is_empty() {
        return Err("Splat Deform requires splat geometry on input 0".to_string());
    }

    let allow_new = params.get_bool("allow_new", false);
    let derive_rot_scale = params.get_bool("derive_rot_scale", true);
    let mut splats = Vec::with_capacity(source.splats.len());

    if !target_geo.splats.is_empty() && target_geo.splats.len() == source.splats.len() {
        for (source_splat, target_splat) in source.splats.iter().zip(&target_geo.splats) {
            splats.push(deform_pair(
                source_splat,
                &target_splat.positions,
                allow_new,
                derive_rot_scale,
            ));
        }
    } else {
        let Some(target_positions) = extract_target_positions(target_geo) else {
            return Err("Splat Deform requires splat or mesh points on input 1".to_string());
        };
        let mut cursor = 0usize;
        for source_splat in &source.splats {
            let len = source_splat.len();
            let slice = if cursor < target_positions.len() {
                let end = (cursor + len).min(target_positions.len());
                &target_positions[cursor..end]
            } else {
                &[]
            };
            splats.push(deform_pair(
                source_splat,
                slice,
                allow_new,
                derive_rot_scale,
            ));
            cursor = cursor.saturating_add(len);
        }
        if allow_new && cursor < target_positions.len() {
            if let Some(template) = source.splats.last() {
                let extra = deform_pair(
                    template,
                    &target_positions[cursor..],
                    true,
                    derive_rot_scale,
                );
                splats.push(extra);
            }
        }
    }

    let merged_mesh = source.merged_mesh();
    let meshes = merged_mesh.map(|mesh| vec![mesh]).unwrap_or_default();
    let curves = if meshes.is_empty() {
        Vec::new()
    } else {
        source.curves.clone()
    };
    Ok(Geometry {
        meshes,
        splats,
        curves,
        volumes: source.volumes.clone(),
        materials: source.materials.clone(),
    })
}

fn extract_target_positions(geo: &Geometry) -> Option<Vec<[f32; 3]>> {
    if let Some(splats) = geo.merged_splats() {
        return Some(splats.positions);
    }
    geo.merged_mesh().map(|mesh| mesh.positions)
}

fn deform_pair(
    source: &SplatGeo,
    target_positions: &[[f32; 3]],
    allow_new: bool,
    derive_rot_scale: bool,
) -> SplatGeo {
    let neighbors = build_neighbors(&source.positions);
    let densified_targets = if allow_new {
        densify_targets(&source.positions, target_positions, &neighbors)
    } else {
        target_positions.to_vec()
    };
    let (mut out, mapping) = deform_splats_with_mapping(source, &densified_targets, allow_new);
    if derive_rot_scale {
        let linears = derive_linear_map(&source.positions, target_positions, &neighbors);
        apply_local_deform_with_mapping(source, &linears, &mapping, &mut out);
    }
    out
}

fn deform_splats_with_mapping(
    source: &SplatGeo,
    target_positions: &[[f32; 3]],
    allow_new: bool,
) -> (SplatGeo, Vec<usize>) {
    if source.is_empty() {
        return (source.clone(), Vec::new());
    }
    if target_positions.is_empty() {
        if allow_new {
            return (source.filter_by_indices(&[]), Vec::new());
        }
        return (source.clone(), (0..source.len()).collect());
    }

    if allow_new {
        let source_len = source.len();
        let target_len = target_positions.len();
        let mut mapping = Vec::with_capacity(target_len);
        let min_len = source_len.min(target_len);
        for idx in 0..min_len {
            mapping.push(idx);
        }
        if target_len > source_len {
            for pos in target_positions.iter().skip(source_len) {
                let nearest = find_nearest_index(*pos, &source.positions);
                mapping.push(nearest);
            }
        }
        let mut out = source.filter_by_indices(&mapping);
        for (idx, pos) in target_positions.iter().enumerate() {
            if let Some(slot) = out.positions.get_mut(idx) {
                *slot = *pos;
            }
        }
        return (out, mapping);
    }

    let mut out = source.clone();
    let min_len = source.len().min(target_positions.len());
    out.positions[..min_len].copy_from_slice(&target_positions[..min_len]);
    let mapping: Vec<usize> = (0..source.len()).collect();
    (out, mapping)
}

fn derive_linear_map(
    source_positions: &[[f32; 3]],
    target_positions: &[[f32; 3]],
    neighbors: &[Vec<usize>],
) -> Vec<Option<LinearEstimate>> {
    if source_positions.is_empty() || target_positions.is_empty() {
        return Vec::new();
    }
    let limit = source_positions.len().min(target_positions.len());
    let mut linears = vec![None; source_positions.len()];
    for (idx, slot) in linears.iter_mut().enumerate().take(limit) {
        *slot = derive_linear(idx, source_positions, target_positions, neighbors);
    }
    linears
}

fn apply_local_deform_with_mapping(
    source: &SplatGeo,
    linears: &[Option<LinearEstimate>],
    mapping: &[usize],
    output: &mut SplatGeo,
) {
    if output.is_empty() || mapping.is_empty() {
        return;
    }
    for (out_idx, &src_idx) in mapping.iter().enumerate() {
        let Some(estimate) = linears.get(src_idx).and_then(|linear| linear.as_ref()) else {
            continue;
        };
        if out_idx >= output.len() {
            break;
        }
        output.apply_linear_deform(out_idx, estimate.linear);
        clamp_output_scales(output, out_idx, source, src_idx, estimate.max_ratio);
    }
}

fn build_neighbors(positions: &[[f32; 3]]) -> Vec<Vec<usize>> {
    let count = positions.len();
    if count == 0 {
        return Vec::new();
    }
    let (min, max) = positions_bounds(positions);
    let extent = max - min;
    let volume = extent.x * extent.y * extent.z;
    let mut cell_size = if volume > 0.0 {
        (volume / count as f32).cbrt()
    } else {
        0.0
    };
    if !cell_size.is_finite() || cell_size <= 1.0e-6 {
        cell_size = 1.0;
    }
    let inv_cell = 1.0 / cell_size;

    let mut grid: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();
    for (idx, position) in positions.iter().enumerate() {
        let pos = Vec3::from(*position);
        let key = splat_cell_key(pos, min, inv_cell);
        grid.entry(key).or_default().push(idx);
    }

    let mut neighbors = vec![Vec::new(); count];
    for (idx, position) in positions.iter().enumerate() {
        let pos = Vec3::from(*position);
        let base = splat_cell_key(pos, min, inv_cell);
        for dz in -1..=1 {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let key = (base.0 + dx, base.1 + dy, base.2 + dz);
                    if let Some(list) = grid.get(&key) {
                        for &other in list {
                            if other != idx {
                                neighbors[idx].push(other);
                            }
                        }
                    }
                }
            }
        }
        let list = &mut neighbors[idx];
        list.sort_unstable();
        list.dedup();
        if list.len() > MAX_NEIGHBORS {
            let origin = Vec3::from(*position);
            let mut sorted: Vec<(usize, f32)> = list
                .iter()
                .map(|&other| {
                    let d = origin.distance_squared(Vec3::from(positions[other]));
                    (other, d)
                })
                .collect();
            sorted.sort_by(|a, b| {
                a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal)
            });
            list.clear();
            list.extend(sorted.into_iter().take(MAX_NEIGHBORS).map(|(idx, _)| idx));
        }
    }

    neighbors
}

fn positions_bounds(positions: &[[f32; 3]]) -> (Vec3, Vec3) {
    let mut iter = positions.iter();
    let first = iter.next().copied().unwrap_or([0.0, 0.0, 0.0]);
    let mut min = Vec3::from(first);
    let mut max = Vec3::from(first);
    for p in iter {
        let v = Vec3::from(*p);
        min = min.min(v);
        max = max.max(v);
    }
    (min, max)
}

#[derive(Clone, Copy)]
struct LinearEstimate {
    linear: Mat3,
    max_ratio: f32,
}

fn derive_linear(
    idx: usize,
    source_positions: &[[f32; 3]],
    target_positions: &[[f32; 3]],
    neighbors: &[Vec<usize>],
) -> Option<LinearEstimate> {
    if idx >= source_positions.len() || idx >= target_positions.len() {
        return None;
    }
    let neigh = neighbors.get(idx)?;
    if neigh.len() < MIN_NEIGHBORS {
        return None;
    }

    let src_center = Vec3::from(source_positions[idx]);
    let tgt_center = Vec3::from(target_positions[idx]);
    let mut ms = [[0.0f32; 3]; 3];
    let mut mt = [[0.0f32; 3]; 3];
    let mut used = 0usize;
    let mut ratio_sum = 0.0f32;
    let mut ratio_max = 0.0f32;
    let mut ratio_weight = 0.0f32;

    for &other in neigh {
        if other >= source_positions.len() || other >= target_positions.len() {
            continue;
        }
        let s = Vec3::from(source_positions[other]) - src_center;
        let s_len_sq = s.length_squared();
        if s_len_sq < 1.0e-8 {
            continue;
        }
        let t = Vec3::from(target_positions[other]) - tgt_center;
        used += 1;
        let weight = 1.0 / (s_len_sq + 1.0e-6);
        let ratio = t.length() / s_len_sq.sqrt();
        if ratio.is_finite() {
            ratio_sum += ratio * weight;
            ratio_weight += weight;
            ratio_max = ratio_max.max(ratio);
        }

        ms[0][0] += s.x * s.x * weight;
        ms[0][1] += s.x * s.y * weight;
        ms[0][2] += s.x * s.z * weight;
        ms[1][0] += s.y * s.x * weight;
        ms[1][1] += s.y * s.y * weight;
        ms[1][2] += s.y * s.z * weight;
        ms[2][0] += s.z * s.x * weight;
        ms[2][1] += s.z * s.y * weight;
        ms[2][2] += s.z * s.z * weight;

        mt[0][0] += t.x * s.x * weight;
        mt[0][1] += t.x * s.y * weight;
        mt[0][2] += t.x * s.z * weight;
        mt[1][0] += t.y * s.x * weight;
        mt[1][1] += t.y * s.y * weight;
        mt[1][2] += t.y * s.z * weight;
        mt[2][0] += t.z * s.x * weight;
        mt[2][1] += t.z * s.y * weight;
        mt[2][2] += t.z * s.z * weight;
    }

    if used < MIN_NEIGHBORS {
        return None;
    }

    let ms_mat = Mat3::from_cols(
        Vec3::new(ms[0][0], ms[1][0], ms[2][0]),
        Vec3::new(ms[0][1], ms[1][1], ms[2][1]),
        Vec3::new(ms[0][2], ms[1][2], ms[2][2]),
    );
    let det = ms_mat.determinant();
    if !det.is_finite() || det.abs() < 1.0e-6 {
        return None;
    }
    let inv = ms_mat.inverse();

    let mt_mat = Mat3::from_cols(
        Vec3::new(mt[0][0], mt[1][0], mt[2][0]),
        Vec3::new(mt[0][1], mt[1][1], mt[2][1]),
        Vec3::new(mt[0][2], mt[1][2], mt[2][2]),
    );
    let linear = mt_mat * inv;
    if !mat3_is_finite(linear) {
        return None;
    }
    if ratio_weight > 0.0 {
        let avg = (ratio_sum / ratio_weight).max(0.0);
        if avg.is_finite() && avg > 0.0 {
            let limit = (avg * RATIO_CLAMP_MULT).max(avg);
            ratio_max = ratio_max.min(limit);
        }
    }
    Some(LinearEstimate {
        linear,
        max_ratio: ratio_max.max(1.0e-6),
    })
}

fn mat3_is_finite(mat: Mat3) -> bool {
    mat.to_cols_array().iter().all(|value| value.is_finite())
}

fn find_nearest_index(target: [f32; 3], positions: &[[f32; 3]]) -> usize {
    if positions.is_empty() {
        return 0;
    }
    let mut best = 0usize;
    let mut best_dist = f32::MAX;
    for (idx, pos) in positions.iter().enumerate() {
        let dx = target[0] - pos[0];
        let dy = target[1] - pos[1];
        let dz = target[2] - pos[2];
        let dist = dx * dx + dy * dy + dz * dz;
        if dist < best_dist {
            best_dist = dist;
            best = idx;
        }
    }
    best
}

fn clamp_output_scales(
    output: &mut SplatGeo,
    out_idx: usize,
    source: &SplatGeo,
    src_idx: usize,
    max_ratio: f32,
) {
    if out_idx >= output.scales.len() || src_idx >= source.scales.len() {
        return;
    }
    if !max_ratio.is_finite() || max_ratio <= 0.0 {
        return;
    }
    let source_scale = Vec3::from(source.scales[src_idx]);
    let src_sigma = Vec3::new(
        source_scale.x.exp(),
        source_scale.y.exp(),
        source_scale.z.exp(),
    );
    let max_sigma = Vec3::new(
        src_sigma.x * max_ratio,
        src_sigma.y * max_ratio,
        src_sigma.z * max_ratio,
    );
    let mut log_scale = Vec3::from(output.scales[out_idx]);
    let mut sigma = Vec3::new(
        log_scale.x.exp(),
        log_scale.y.exp(),
        log_scale.z.exp(),
    );
    sigma = Vec3::new(
        sigma.x.min(max_sigma.x),
        sigma.y.min(max_sigma.y),
        sigma.z.min(max_sigma.z),
    );
    log_scale = Vec3::new(sigma.x.ln(), sigma.y.ln(), sigma.z.ln());
    output.scales[out_idx] = [log_scale.x, log_scale.y, log_scale.z];
}

fn densify_targets(
    source_positions: &[[f32; 3]],
    target_positions: &[[f32; 3]],
    neighbors: &[Vec<usize>],
) -> Vec<[f32; 3]> {
    let mut out = target_positions.to_vec();
    let limit = source_positions
        .len()
        .min(target_positions.len())
        .min(neighbors.len());
    if limit < 2 {
        return out;
    }
    for i in 0..limit {
        let src_i = Vec3::from(source_positions[i]);
        let tgt_i = Vec3::from(target_positions[i]);
        for &j in neighbors[i].iter() {
            if j <= i || j >= limit {
                continue;
            }
            let src_j = Vec3::from(source_positions[j]);
            let tgt_j = Vec3::from(target_positions[j]);
            let src_dist = src_i.distance(src_j);
            if src_dist <= 1.0e-6 {
                continue;
            }
            let tgt_dist = tgt_i.distance(tgt_j);
            let max_len = src_dist * STRETCH_THRESHOLD;
            if tgt_dist <= max_len {
                continue;
            }
            let segments = (tgt_dist / max_len).ceil().max(1.0) as usize;
            let new_count = segments.saturating_sub(1).min(MAX_NEW_PER_EDGE);
            if new_count == 0 {
                continue;
            }
            for n in 0..new_count {
                let t = (n + 1) as f32 / (new_count + 1) as f32;
                out.push(tgt_i.lerp(tgt_j, t).to_array());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::splat::SplatGeo;

    #[test]
    fn deform_preserves_count_without_new() {
        let mut source = SplatGeo::with_len(2);
        source.positions[0] = [0.0, 0.0, 0.0];
        source.positions[1] = [1.0, 1.0, 1.0];
        source.opacity[0] = -1.0;
        source.opacity[1] = 2.0;

        let target = vec![[2.0, 0.0, 0.0]];
        let out = deform_splats_with_mapping(&source, &target, false).0;
        assert_eq!(out.len(), 2);
        assert_eq!(out.positions[0], [2.0, 0.0, 0.0]);
        assert_eq!(out.positions[1], [1.0, 1.0, 1.0]);
        assert_eq!(out.opacity, source.opacity);
    }

    #[test]
    fn deform_allows_new_splats() {
        let mut source = SplatGeo::with_len(2);
        source.positions[0] = [0.0, 0.0, 0.0];
        source.positions[1] = [10.0, 0.0, 0.0];
        source.opacity[0] = -3.0;
        source.opacity[1] = 1.5;

        let target = vec![
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [9.0, 0.0, 0.0],
        ];
        let out = deform_splats_with_mapping(&source, &target, true).0;
        assert_eq!(out.len(), 3);
        assert_eq!(out.positions, target);
        assert!((out.opacity[2] - 1.5).abs() < 1.0e-6);
    }

    #[test]
    fn deform_trims_when_target_shorter() {
        let mut source = SplatGeo::with_len(3);
        source.positions[0] = [0.0, 0.0, 0.0];
        source.positions[1] = [1.0, 0.0, 0.0];
        source.positions[2] = [2.0, 0.0, 0.0];

        let target = vec![[5.0, 0.0, 0.0]];
        let out = deform_splats_with_mapping(&source, &target, true).0;
        assert_eq!(out.len(), 1);
        assert_eq!(out.positions[0], [5.0, 0.0, 0.0]);
    }

    #[test]
    fn derive_linear_recovers_axis_scale() {
        let source = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let target = vec![
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [0.0, 3.0, 0.0],
            [0.0, 0.0, 4.0],
        ];
        let neighbors = build_neighbors(&source);
        let estimate =
            derive_linear(0, &source, &target, &neighbors).expect("linear");
        let cols = estimate.linear.to_cols_array();
        assert!((cols[0] - 2.0).abs() < 1.0e-3);
        assert!((cols[4] - 3.0).abs() < 1.0e-3);
        assert!((cols[8] - 4.0).abs() < 1.0e-3);
    }
}
