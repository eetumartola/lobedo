use std::collections::BTreeMap;

use glam::{Mat3, Quat, Vec3, Vec4};

use crate::attributes::{AttributeDomain, AttributeStorage, MeshAttributes, StringTableAttribute};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::{Mesh, MeshGroups};
use crate::nodes::{
    geometry_in,
    geometry_out,
    require_mesh_input,
    splat_utils::{splat_bounds_indices, splat_cell_key, split_splats_by_group},
};
use crate::splat::SplatGeo;

pub const NAME: &str = "Splat LOD";

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
            ("voxel_size".to_string(), ParamValue::Float(0.1)),
            ("target_count".to_string(), ParamValue::Int(0)),
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn compute(_params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let input = require_mesh_input(inputs, 0, "Splat LOD requires a mesh input")?;
    Ok(input)
}

pub fn apply_to_splats(params: &NodeParams, splats: &SplatGeo) -> SplatGeo {
    if splats.is_empty() {
        return splats.clone();
    }

    let Some((selected, unselected)) =
        split_splats_by_group(splats, params, AttributeDomain::Point)
    else {
        return splats.clone();
    };
    if selected.len() <= 1 {
        return splats.clone();
    }

    let target_count = params.get_int("target_count", 0).max(0) as usize;

    let (min, max) = splat_bounds_indices(splats, &selected);
    let mut voxel_size = params.get_float("voxel_size", 0.1);
    if !voxel_size.is_finite() || voxel_size <= 1.0e-6 {
        if target_count == 0 {
            return splats.clone();
        }
        let extent = max - min;
        let volume = extent.x * extent.y * extent.z;
        if volume.is_finite() && volume > 0.0 {
            voxel_size = (volume / target_count as f32).cbrt();
        }
    }
    if !voxel_size.is_finite() || voxel_size <= 1.0e-6 {
        return splats.clone();
    }
    let inv_cell = 1.0 / voxel_size;

    let clusters = build_clusters(&splats.positions, &selected, min, inv_cell);
    if clusters.len() >= selected.len() {
        return splats.clone();
    }

    let mut cluster_sets: Vec<Vec<usize>> = clusters.values().cloned().collect();
    if target_count > 0 && cluster_sets.len() > target_count {
        cluster_sets.sort_by(|a, b| b.len().cmp(&a.len()));
        cluster_sets.truncate(target_count);
    }
    let output_len = unselected.len() + cluster_sets.len();

    let mut positions = Vec::with_capacity(output_len);
    let mut rotations = Vec::with_capacity(output_len);
    let mut scales = Vec::with_capacity(output_len);
    let mut opacity = Vec::with_capacity(output_len);
    let mut sh0 = Vec::with_capacity(output_len);
    let mut sh_rest = Vec::new();
    let sh_coeffs = splats.sh_coeffs;
    if sh_coeffs > 0 {
        sh_rest.reserve(output_len * sh_coeffs);
    }

    for &idx in &unselected {
        positions.push(splats.positions[idx]);
        rotations.push(splats.rotations[idx]);
        scales.push(splats.scales[idx]);
        opacity.push(splats.opacity[idx]);
        sh0.push(splats.sh0[idx]);
        if sh_coeffs > 0 {
            let base = idx * sh_coeffs;
            for coeff in 0..sh_coeffs {
                sh_rest.push(splats.sh_rest[base + coeff]);
            }
        }
    }

    for cluster in &cluster_sets {
        let count = cluster.len() as f32;
        let mut weights = Vec::with_capacity(cluster.len());
        let mut weight_sum = 0.0f32;
        let mut opacity_prod = 1.0f32;

        for &idx in cluster {
            let mut w = sigmoid(splats.opacity[idx]);
            if !w.is_finite() {
                w = 0.0;
            }
            w = w.clamp(0.0, 1.0);
            weight_sum += w;
            opacity_prod *= 1.0 - w;
            weights.push(w);
        }

        let mut pos_sum = Vec3::ZERO;
        let mut sh0_sum = Vec3::ZERO;
        let mut sh_rest_sum = vec![Vec3::ZERO; sh_coeffs];
        let mut quat_ref: Option<Quat> = None;
        let mut quat_sum = Vec4::ZERO;

        if weight_sum <= 1.0e-6 {
            weights.fill(1.0);
            weight_sum = count.max(1.0);
        }

        for (idx, &weight) in cluster.iter().zip(weights.iter()) {
            pos_sum += Vec3::from(splats.positions[*idx]) * weight;
            sh0_sum += Vec3::from(splats.sh0[*idx]) * weight;

            let mut quat = quat_from_rotation(splats.rotations[*idx]);
            if let Some(reference) = quat_ref {
                if reference.dot(quat) < 0.0 {
                    quat = Quat::from_xyzw(-quat.x, -quat.y, -quat.z, -quat.w);
                }
            } else {
                quat_ref = Some(quat);
            }
            quat_sum += Vec4::new(quat.x, quat.y, quat.z, quat.w) * weight;

            if sh_coeffs > 0 {
                let base = *idx * sh_coeffs;
                for (coeff, sum) in sh_rest_sum.iter_mut().enumerate() {
                    *sum += Vec3::from(splats.sh_rest[base + coeff]) * weight;
                }
            }
        }

        let inv_weight = 1.0 / weight_sum.max(1.0e-6);
        let mean = pos_sum * inv_weight;
        positions.push(mean.to_array());
        sh0.push((sh0_sum * inv_weight).to_array());

        let mut linear_opacity = 1.0 - opacity_prod;
        if !linear_opacity.is_finite() {
            linear_opacity = 0.0;
        }
        let output_opacity = logit(linear_opacity);
        opacity.push(output_opacity);

        let mut quat = Quat::from_xyzw(quat_sum.x, quat_sum.y, quat_sum.z, quat_sum.w);
        if quat.length_squared() > 0.0 {
            quat = quat.normalize();
        } else {
            quat = quat_ref.unwrap_or(Quat::IDENTITY);
        }
        rotations.push([quat.w, quat.x, quat.y, quat.z]);

        let cluster_rot = Mat3::from_quat(quat);
        let cluster_rot_t = cluster_rot.transpose();
        let mut cov_sum = Mat3::ZERO;

        for (idx, &weight) in cluster.iter().zip(weights.iter()) {
            if weight <= 0.0 {
                continue;
            }
            let pos = Vec3::from(splats.positions[*idx]);
            let delta = pos - mean;
            let delta_outer = Mat3::from_cols(delta * delta.x, delta * delta.y, delta * delta.z);

            let mut log_scale = Vec3::from(splats.scales[*idx]);
            log_scale = Vec3::new(
                log_scale.x.clamp(-10.0, 10.0),
                log_scale.y.clamp(-10.0, 10.0),
                log_scale.z.clamp(-10.0, 10.0),
            );
            let scale = Vec3::new(
                log_scale.x.exp(),
                log_scale.y.exp(),
                log_scale.z.exp(),
            );
            let rot = Mat3::from_quat(quat_from_rotation(splats.rotations[*idx]));
            let cov_local = Mat3::from_diagonal(scale * scale);
            let cov_world = rot * cov_local * rot.transpose();
            let cov_world = cov_world + delta_outer;
            let cov_cluster = cluster_rot_t * cov_world * cluster_rot;
            cov_sum += cov_cluster * weight;
        }

        let cov = cov_sum * inv_weight;
        let mut sigma = Vec3::new(cov.x_axis.x, cov.y_axis.y, cov.z_axis.z);
        sigma = Vec3::new(
            sigma.x.max(0.0).sqrt(),
            sigma.y.max(0.0).sqrt(),
            sigma.z.max(0.0).sqrt(),
        );
        if !sigma.x.is_finite() || !sigma.y.is_finite() || !sigma.z.is_finite() {
            sigma = Vec3::splat(1.0e-6);
        }
        sigma = Vec3::new(
            sigma.x.max(1.0e-6),
            sigma.y.max(1.0e-6),
            sigma.z.max(1.0e-6),
        );
        scales.push([sigma.x.ln(), sigma.y.ln(), sigma.z.ln()]);

        if sh_coeffs > 0 {
            for sum in &sh_rest_sum {
                sh_rest.push((*sum * inv_weight).to_array());
            }
        }
    }

    let attributes = aggregate_attributes(splats, &unselected, &cluster_sets);
    let groups = aggregate_groups(splats, &unselected, &cluster_sets);

    SplatGeo {
        positions,
        rotations,
        scales,
        opacity,
        sh0,
        sh_coeffs,
        sh_rest,
        attributes,
        groups,
    }
}

fn build_clusters(
    positions: &[[f32; 3]],
    indices: &[usize],
    min: Vec3,
    inv_cell: f32,
) -> BTreeMap<(i32, i32, i32), Vec<usize>> {
    let mut clusters = BTreeMap::new();
    for &idx in indices {
        let Some(position) = positions.get(idx) else {
            continue;
        };
        let key = splat_cell_key(Vec3::from(*position), min, inv_cell);
        clusters.entry(key).or_insert_with(Vec::new).push(idx);
    }
    clusters
}

fn quat_from_rotation(rotation: [f32; 4]) -> Quat {
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

fn aggregate_groups(
    splats: &SplatGeo,
    unselected: &[usize],
    clusters: &[Vec<usize>],
) -> MeshGroups {
    let mut output = MeshGroups::default();
    for domain in [AttributeDomain::Point, AttributeDomain::Primitive] {
        for (name, values) in splats.groups.map(domain) {
            let mut out = Vec::with_capacity(unselected.len() + clusters.len());
            for &idx in unselected {
                out.push(values.get(idx).copied().unwrap_or(false));
            }
            for cluster in clusters {
                out.push(any_group(values, cluster));
            }
            output.map_mut(domain).insert(name.clone(), out);
        }
    }
    output
}

fn any_group(values: &[bool], indices: &[usize]) -> bool {
    indices
        .iter()
        .any(|idx| values.get(*idx).copied().unwrap_or(false))
}

fn aggregate_attributes(
    splats: &SplatGeo,
    unselected: &[usize],
    clusters: &[Vec<usize>],
) -> MeshAttributes {
    let mut output = MeshAttributes::default();
    for domain in [AttributeDomain::Point, AttributeDomain::Primitive] {
        for (name, storage) in splats.attributes.map(domain) {
            let aggregated = aggregate_storage(storage, unselected, clusters);
            output.map_mut(domain).insert(name.clone(), aggregated);
        }
    }
    for (name, storage) in splats.attributes.map(AttributeDomain::Detail) {
        output
            .map_mut(AttributeDomain::Detail)
            .insert(name.clone(), storage.clone());
    }
    output
}

fn aggregate_storage(
    storage: &AttributeStorage,
    unselected: &[usize],
    clusters: &[Vec<usize>],
) -> AttributeStorage {
    match storage {
        AttributeStorage::Float(values) => {
            let mut out = Vec::with_capacity(unselected.len() + clusters.len());
            for &idx in unselected {
                out.push(values.get(idx).copied().unwrap_or(0.0));
            }
            for cluster in clusters {
                out.push(avg_f32(values, cluster));
            }
            AttributeStorage::Float(out)
        }
        AttributeStorage::Int(values) => {
            let mut out = Vec::with_capacity(unselected.len() + clusters.len());
            for &idx in unselected {
                out.push(values.get(idx).copied().unwrap_or(0));
            }
            for cluster in clusters {
                out.push(avg_i32(values, cluster));
            }
            AttributeStorage::Int(out)
        }
        AttributeStorage::Vec2(values) => {
            let mut out = Vec::with_capacity(unselected.len() + clusters.len());
            for &idx in unselected {
                out.push(values.get(idx).copied().unwrap_or([0.0, 0.0]));
            }
            for cluster in clusters {
                out.push(avg_vec2(values, cluster));
            }
            AttributeStorage::Vec2(out)
        }
        AttributeStorage::Vec3(values) => {
            let mut out = Vec::with_capacity(unselected.len() + clusters.len());
            for &idx in unselected {
                out.push(values.get(idx).copied().unwrap_or([0.0, 0.0, 0.0]));
            }
            for cluster in clusters {
                out.push(avg_vec3(values, cluster));
            }
            AttributeStorage::Vec3(out)
        }
        AttributeStorage::Vec4(values) => {
            let mut out = Vec::with_capacity(unselected.len() + clusters.len());
            for &idx in unselected {
                out.push(values.get(idx).copied().unwrap_or([0.0, 0.0, 0.0, 0.0]));
            }
            for cluster in clusters {
                out.push(avg_vec4(values, cluster));
            }
            AttributeStorage::Vec4(out)
        }
        AttributeStorage::StringTable(values) => {
            let mut out = Vec::with_capacity(unselected.len() + clusters.len());
            for &idx in unselected {
                out.push(values.indices.get(idx).copied().unwrap_or(0));
            }
            for cluster in clusters {
                let mut selected = 0u32;
                for &idx in cluster {
                    if let Some(value) = values.indices.get(idx).copied() {
                        selected = value;
                        break;
                    }
                }
                out.push(selected);
            }
            let mut table = values.values.clone();
            if table.is_empty() && !out.is_empty() {
                table.push(String::new());
            }
            AttributeStorage::StringTable(StringTableAttribute::new(table, out))
        }
    }
}

fn avg_f32(values: &[f32], indices: &[usize]) -> f32 {
    let mut sum = 0.0;
    let mut count = 0usize;
    for &idx in indices {
        if let Some(value) = values.get(idx) {
            sum += *value;
            count += 1;
        }
    }
    if count > 0 {
        sum / count as f32
    } else {
        0.0
    }
}

fn avg_i32(values: &[i32], indices: &[usize]) -> i32 {
    let mut sum = 0.0;
    let mut count = 0usize;
    for &idx in indices {
        if let Some(value) = values.get(idx) {
            sum += *value as f32;
            count += 1;
        }
    }
    if count > 0 {
        (sum / count as f32).round() as i32
    } else {
        0
    }
}

fn avg_vec2(values: &[[f32; 2]], indices: &[usize]) -> [f32; 2] {
    let mut sum = [0.0f32; 2];
    let mut count = 0usize;
    for &idx in indices {
        if let Some(value) = values.get(idx) {
            sum[0] += value[0];
            sum[1] += value[1];
            count += 1;
        }
    }
    if count > 0 {
        [sum[0] / count as f32, sum[1] / count as f32]
    } else {
        [0.0, 0.0]
    }
}

fn avg_vec3(values: &[[f32; 3]], indices: &[usize]) -> [f32; 3] {
    let mut sum = [0.0f32; 3];
    let mut count = 0usize;
    for &idx in indices {
        if let Some(value) = values.get(idx) {
            sum[0] += value[0];
            sum[1] += value[1];
            sum[2] += value[2];
            count += 1;
        }
    }
    if count > 0 {
        [
            sum[0] / count as f32,
            sum[1] / count as f32,
            sum[2] / count as f32,
        ]
    } else {
        [0.0, 0.0, 0.0]
    }
}

fn avg_vec4(values: &[[f32; 4]], indices: &[usize]) -> [f32; 4] {
    let mut sum = [0.0f32; 4];
    let mut count = 0usize;
    for &idx in indices {
        if let Some(value) = values.get(idx) {
            sum[0] += value[0];
            sum[1] += value[1];
            sum[2] += value[2];
            sum[3] += value[3];
            count += 1;
        }
    }
    if count > 0 {
        [
            sum[0] / count as f32,
            sum[1] / count as f32,
            sum[2] / count as f32,
            sum[3] / count as f32,
        ]
    } else {
        [0.0, 0.0, 0.0, 0.0]
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::graph::{NodeParams, ParamValue};
    use crate::splat::SplatGeo;

    use super::apply_to_splats;

    #[test]
    fn lod_clusters_by_voxel() {
        let mut splats = SplatGeo::with_len(4);
        splats.positions[0] = [0.0, 0.0, 0.0];
        splats.positions[1] = [0.05, 0.02, 0.0];
        splats.positions[2] = [1.0, 0.0, 0.0];
        splats.positions[3] = [1.1, 0.0, 0.0];

        let params = NodeParams {
            values: BTreeMap::from([
                ("voxel_size".to_string(), ParamValue::Float(0.2)),
                ("target_count".to_string(), ParamValue::Int(0)),
            ]),
        };

        let decimated = apply_to_splats(&params, &splats);
        assert_eq!(decimated.len(), 2);
    }
}
