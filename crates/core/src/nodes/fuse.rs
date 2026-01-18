use std::collections::BTreeMap;

use glam::Vec3;

use crate::attributes::{AttributeDomain, AttributeStorage, MeshAttributes, StringTableAttribute};
use crate::curve::Curve;
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::{Mesh, MeshGroups};
use crate::nodes::{geometry_in, geometry_out, recompute_mesh_normals, require_mesh_input};

pub const NAME: &str = "Fuse";
const DEFAULT_RADIUS: f32 = 0.001;

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
            ("radius".to_string(), ParamValue::Float(DEFAULT_RADIUS)),
            ("unfuse".to_string(), ParamValue::Bool(false)),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mesh = require_mesh_input(inputs, 0, "Fuse requires a mesh input")?;
    if params.get_bool("unfuse", false) {
        let sources = if mesh.indices.is_empty() {
            (0..mesh.positions.len()).collect::<Vec<_>>()
        } else {
            mesh.indices.iter().map(|idx| *idx as usize).collect::<Vec<_>>()
        };
        Ok(unfuse_mesh(&mesh, &sources).0)
    } else {
        Ok(fuse_mesh(&mesh, params.get_float("radius", DEFAULT_RADIUS)).0)
    }
}

pub fn apply_to_geometry(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };
    let mut meshes = Vec::new();
    let mut curves = Vec::new();

    if let Some(mesh) = input.merged_mesh() {
        let unfuse = params.get_bool("unfuse", false);
        if unfuse {
            let mut point_sources: Vec<usize> = if mesh.indices.is_empty() {
                (0..mesh.positions.len()).collect()
            } else {
                mesh.indices.iter().map(|idx| *idx as usize).collect()
            };
            if !input.curves.is_empty() {
                for curve in &input.curves {
                    let base = point_sources.len() as u32;
                    let mut indices = Vec::with_capacity(curve.indices.len());
                    for (i, &idx) in curve.indices.iter().enumerate() {
                        point_sources.push(idx as usize);
                        indices.push(base + i as u32);
                    }
                    if !indices.is_empty() {
                        curves.push(Curve::new(indices, curve.closed));
                    }
                }
            }
            let (fused, _) = unfuse_mesh(&mesh, &point_sources);
            meshes.push(fused);
            if curves.is_empty() {
                curves = input.curves.clone();
            }
        } else {
            let (fused, mapping) =
                fuse_mesh(&mesh, params.get_float("radius", DEFAULT_RADIUS));
            if !input.curves.is_empty() {
                for curve in &input.curves {
                    let mut indices = Vec::with_capacity(curve.indices.len());
                    for &idx in &curve.indices {
                        let mapped = mapping.get(idx as usize).copied().unwrap_or(0);
                        indices.push(mapped);
                    }
                    curves.push(Curve::new(indices, curve.closed));
                }
            }
            meshes.push(fused);
            if curves.is_empty() {
                curves = input.curves.clone();
            }
        }
    }

    Ok(Geometry {
        meshes,
        splats: input.splats.clone(),
        curves,
        volumes: input.volumes.clone(),
        materials: input.materials.clone(),
    })
}

#[derive(Clone)]
struct Cluster {
    sum: Vec3,
    count: u32,
    rep: usize,
}

fn fuse_mesh(mesh: &Mesh, radius: f32) -> (Mesh, Vec<u32>) {
    if mesh.positions.is_empty() {
        return (Mesh::default(), Vec::new());
    }
    if radius <= 0.0 {
        return (mesh.clone(), (0..mesh.positions.len() as u32).collect());
    }

    let cell_size = radius.max(1.0e-6);
    let inv_cell = 1.0 / cell_size;
    let mut clusters: Vec<Cluster> = Vec::new();
    let mut cell_map: std::collections::HashMap<(i32, i32, i32), Vec<usize>> =
        std::collections::HashMap::new();
    let mut mapping = vec![0u32; mesh.positions.len()];

    for (idx, pos) in mesh.positions.iter().enumerate() {
        let p = Vec3::from(*pos);
        let cell = (
            (p.x * inv_cell).floor() as i32,
            (p.y * inv_cell).floor() as i32,
            (p.z * inv_cell).floor() as i32,
        );
        let mut best_cluster = None;
        let mut best_dist = radius * radius;
        for dz in -1..=1 {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let key = (cell.0 + dx, cell.1 + dy, cell.2 + dz);
                    if let Some(list) = cell_map.get(&key) {
                        for &cluster_idx in list {
                            let cluster = &clusters[cluster_idx];
                            let center = cluster.sum / cluster.count as f32;
                            let dist = (p - center).length_squared();
                            if dist <= best_dist {
                                best_dist = dist;
                                best_cluster = Some(cluster_idx);
                            }
                        }
                    }
                }
            }
        }

        let cluster_idx = if let Some(cluster_idx) = best_cluster {
            let cluster = &mut clusters[cluster_idx];
            cluster.sum += p;
            cluster.count += 1;
            cluster_idx
        } else {
            let cluster_idx = clusters.len();
            clusters.push(Cluster {
                sum: p,
                count: 1,
                rep: idx,
            });
            cell_map.entry(cell).or_default().push(cluster_idx);
            cluster_idx
        };
        mapping[idx] = cluster_idx as u32;
    }

    let mut positions = Vec::with_capacity(clusters.len());
    for cluster in &clusters {
        positions.push((cluster.sum / cluster.count as f32).to_array());
    }

    let mut indices = Vec::with_capacity(mesh.indices.len());
    for &idx in &mesh.indices {
        let mapped = mapping.get(idx as usize).copied().unwrap_or(0);
        indices.push(mapped);
    }

    let mut out = Mesh::with_positions_faces(positions, indices, mesh.face_counts.clone());
    out.attributes = remap_attributes_fused(mesh, &mapping, &clusters);
    out.groups = remap_groups_fused(mesh, &mapping, clusters.len());
    if let Some(uvs) = remap_uvs_fused(mesh, &mapping, clusters.len()) {
        out.uvs = Some(uvs);
    }
    if let Some(normals) = remap_normals_fused(mesh, &mapping, clusters.len()) {
        out.normals = Some(normals);
    }

    if !out.indices.is_empty() {
        recompute_mesh_normals(&mut out);
    }
    (out, mapping)
}

fn unfuse_mesh(mesh: &Mesh, point_sources: &[usize]) -> (Mesh, Vec<usize>) {
    let mut face_counts = mesh.face_counts.clone();
    if face_counts.is_empty() && !mesh.indices.is_empty() {
        if mesh.indices.len().is_multiple_of(3) {
            face_counts = vec![3u32; mesh.indices.len() / 3];
        } else {
            face_counts = vec![mesh.indices.len() as u32];
        }
    }

    let corner_count = mesh.indices.len();
    let positions = point_sources
        .iter()
        .map(|&src| *mesh.positions.get(src).unwrap_or(&[0.0, 0.0, 0.0]))
        .collect::<Vec<_>>();
    let indices = (0..corner_count as u32).collect::<Vec<_>>();

    let mut out = Mesh::with_positions_faces(positions, indices, face_counts);
    out.attributes = remap_attributes_unfused(mesh, point_sources);
    out.groups = remap_groups_unfused(mesh, point_sources);
    if let Some(normals) = remap_normals_unfused(mesh, point_sources) {
        out.normals = Some(normals);
    }
    if let Some(uvs) = remap_uvs_unfused(mesh, point_sources) {
        out.uvs = Some(uvs);
    }
    out.corner_normals = mesh.corner_normals.clone();

    (out, point_sources.to_vec())
}

fn remap_attributes_fused(
    mesh: &Mesh,
    mapping: &[u32],
    clusters: &[Cluster],
) -> MeshAttributes {
    let mut out = MeshAttributes::default();
    for (name, storage) in mesh.attributes.map(AttributeDomain::Vertex) {
        out.map_mut(AttributeDomain::Vertex)
            .insert(name.clone(), storage.clone());
    }
    for (name, storage) in mesh.attributes.map(AttributeDomain::Primitive) {
        out.map_mut(AttributeDomain::Primitive)
            .insert(name.clone(), storage.clone());
    }
    for (name, storage) in mesh.attributes.map(AttributeDomain::Detail) {
        out.map_mut(AttributeDomain::Detail)
            .insert(name.clone(), storage.clone());
    }

    let count = clusters.len();
    for (name, storage) in mesh.attributes.map(AttributeDomain::Point) {
        match storage {
            AttributeStorage::Float(values) => {
                let mut accum = vec![0.0f32; count];
                let mut counts = vec![0u32; count];
                for (idx, value) in values.iter().enumerate() {
                    let mapped = mapping.get(idx).copied().unwrap_or(0) as usize;
                    accum[mapped] += *value;
                    counts[mapped] += 1;
                }
                for (value, count) in accum.iter_mut().zip(counts) {
                    if count > 0 {
                        *value /= count as f32;
                    }
                }
                out.map_mut(AttributeDomain::Point)
                    .insert(name.clone(), AttributeStorage::Float(accum));
            }
            AttributeStorage::Int(values) => {
                let mut out_values = vec![0i32; count];
                for (cluster_idx, cluster) in clusters.iter().enumerate() {
                    out_values[cluster_idx] = *values.get(cluster.rep).unwrap_or(&0);
                }
                out.map_mut(AttributeDomain::Point)
                    .insert(name.clone(), AttributeStorage::Int(out_values));
            }
            AttributeStorage::Vec2(values) => {
                let mut accum = vec![[0.0f32, 0.0]; count];
                let mut counts = vec![0u32; count];
                for (idx, value) in values.iter().enumerate() {
                    let mapped = mapping.get(idx).copied().unwrap_or(0) as usize;
                    accum[mapped][0] += value[0];
                    accum[mapped][1] += value[1];
                    counts[mapped] += 1;
                }
                for (value, count) in accum.iter_mut().zip(counts) {
                    if count > 0 {
                        value[0] /= count as f32;
                        value[1] /= count as f32;
                    }
                }
                out.map_mut(AttributeDomain::Point)
                    .insert(name.clone(), AttributeStorage::Vec2(accum));
            }
            AttributeStorage::Vec3(values) => {
                let mut accum = vec![[0.0f32, 0.0, 0.0]; count];
                let mut counts = vec![0u32; count];
                for (idx, value) in values.iter().enumerate() {
                    let mapped = mapping.get(idx).copied().unwrap_or(0) as usize;
                    accum[mapped][0] += value[0];
                    accum[mapped][1] += value[1];
                    accum[mapped][2] += value[2];
                    counts[mapped] += 1;
                }
                for (value, count) in accum.iter_mut().zip(counts) {
                    if count > 0 {
                        value[0] /= count as f32;
                        value[1] /= count as f32;
                        value[2] /= count as f32;
                    }
                }
                out.map_mut(AttributeDomain::Point)
                    .insert(name.clone(), AttributeStorage::Vec3(accum));
            }
            AttributeStorage::Vec4(values) => {
                let mut accum = vec![[0.0f32, 0.0, 0.0, 0.0]; count];
                let mut counts = vec![0u32; count];
                for (idx, value) in values.iter().enumerate() {
                    let mapped = mapping.get(idx).copied().unwrap_or(0) as usize;
                    accum[mapped][0] += value[0];
                    accum[mapped][1] += value[1];
                    accum[mapped][2] += value[2];
                    accum[mapped][3] += value[3];
                    counts[mapped] += 1;
                }
                for (value, count) in accum.iter_mut().zip(counts) {
                    if count > 0 {
                        value[0] /= count as f32;
                        value[1] /= count as f32;
                        value[2] /= count as f32;
                        value[3] /= count as f32;
                    }
                }
                out.map_mut(AttributeDomain::Point)
                    .insert(name.clone(), AttributeStorage::Vec4(accum));
            }
            AttributeStorage::StringTable(values) => {
                let mut indices = Vec::with_capacity(count);
                for cluster in clusters {
                    indices.push(values.indices.get(cluster.rep).copied().unwrap_or(0));
                }
                out.map_mut(AttributeDomain::Point).insert(
                    name.clone(),
                    AttributeStorage::StringTable(StringTableAttribute::new(
                        values.values.clone(),
                        indices,
                    )),
                );
            }
        }
    }
    out
}

fn remap_groups_fused(mesh: &Mesh, mapping: &[u32], count: usize) -> MeshGroups {
    let mut out = MeshGroups::default();
    for (name, values) in mesh.groups.map(AttributeDomain::Vertex) {
        out.map_mut(AttributeDomain::Vertex)
            .insert(name.clone(), values.clone());
    }
    for (name, values) in mesh.groups.map(AttributeDomain::Primitive) {
        out.map_mut(AttributeDomain::Primitive)
            .insert(name.clone(), values.clone());
    }
    for (name, values) in mesh.groups.map(AttributeDomain::Point) {
        let mut out_values = vec![false; count];
        for (idx, value) in values.iter().enumerate() {
            let mapped = mapping.get(idx).copied().unwrap_or(0) as usize;
            out_values[mapped] |= *value;
        }
        out.map_mut(AttributeDomain::Point)
            .insert(name.clone(), out_values);
    }
    out
}

fn remap_uvs_fused(mesh: &Mesh, mapping: &[u32], count: usize) -> Option<Vec<[f32; 2]>> {
    let uvs = mesh.uvs.as_ref()?;
    if uvs.len() != mesh.positions.len() {
        return None;
    }
    let mut accum = vec![[0.0f32, 0.0]; count];
    let mut counts = vec![0u32; count];
    for (idx, uv) in uvs.iter().enumerate() {
        let mapped = mapping.get(idx).copied().unwrap_or(0) as usize;
        accum[mapped][0] += uv[0];
        accum[mapped][1] += uv[1];
        counts[mapped] += 1;
    }
    for (uv, count) in accum.iter_mut().zip(counts) {
        if count > 0 {
            uv[0] /= count as f32;
            uv[1] /= count as f32;
        }
    }
    Some(accum)
}

fn remap_normals_fused(
    mesh: &Mesh,
    mapping: &[u32],
    count: usize,
) -> Option<Vec<[f32; 3]>> {
    let normals = mesh.normals.as_ref()?;
    if normals.len() != mesh.positions.len() {
        return None;
    }
    let mut accum = vec![[0.0f32, 0.0, 0.0]; count];
    let mut counts = vec![0u32; count];
    for (idx, normal) in normals.iter().enumerate() {
        let mapped = mapping.get(idx).copied().unwrap_or(0) as usize;
        accum[mapped][0] += normal[0];
        accum[mapped][1] += normal[1];
        accum[mapped][2] += normal[2];
        counts[mapped] += 1;
    }
    for (normal, count) in accum.iter_mut().zip(counts) {
        if count > 0 {
            let v = Vec3::from(*normal);
            let len = v.length();
            *normal = if len > 0.0 {
                (v / len).to_array()
            } else {
                [0.0, 1.0, 0.0]
            };
        }
    }
    Some(accum)
}

fn remap_attributes_unfused(mesh: &Mesh, point_sources: &[usize]) -> MeshAttributes {
    let mut out = MeshAttributes::default();
    for (name, storage) in mesh.attributes.map(AttributeDomain::Vertex) {
        out.map_mut(AttributeDomain::Vertex)
            .insert(name.clone(), storage.clone());
    }
    for (name, storage) in mesh.attributes.map(AttributeDomain::Primitive) {
        out.map_mut(AttributeDomain::Primitive)
            .insert(name.clone(), storage.clone());
    }
    for (name, storage) in mesh.attributes.map(AttributeDomain::Detail) {
        out.map_mut(AttributeDomain::Detail)
            .insert(name.clone(), storage.clone());
    }
    for (name, storage) in mesh.attributes.map(AttributeDomain::Point) {
        match storage {
            AttributeStorage::Float(values) => {
                out.map_mut(AttributeDomain::Point).insert(
                    name.clone(),
                    AttributeStorage::Float(remap_storage_values(values, point_sources, 0.0)),
                );
            }
            AttributeStorage::Int(values) => {
                out.map_mut(AttributeDomain::Point).insert(
                    name.clone(),
                    AttributeStorage::Int(
                        point_sources
                            .iter()
                            .map(|&src| *values.get(src).unwrap_or(&0))
                            .collect(),
                    ),
                );
            }
            AttributeStorage::Vec2(values) => {
                out.map_mut(AttributeDomain::Point).insert(
                    name.clone(),
                    AttributeStorage::Vec2(remap_storage_values(values, point_sources, [0.0, 0.0])),
                );
            }
            AttributeStorage::Vec3(values) => {
                out.map_mut(AttributeDomain::Point).insert(
                    name.clone(),
                    AttributeStorage::Vec3(remap_storage_values(
                        values,
                        point_sources,
                        [0.0, 0.0, 0.0],
                    )),
                );
            }
            AttributeStorage::Vec4(values) => {
                out.map_mut(AttributeDomain::Point).insert(
                    name.clone(),
                    AttributeStorage::Vec4(remap_storage_values(
                        values,
                        point_sources,
                        [0.0, 0.0, 0.0, 0.0],
                    )),
                );
            }
            AttributeStorage::StringTable(values) => {
                let mut indices = Vec::with_capacity(point_sources.len());
                for &src in point_sources {
                    indices.push(values.indices.get(src).copied().unwrap_or(0));
                }
                out.map_mut(AttributeDomain::Point).insert(
                    name.clone(),
                    AttributeStorage::StringTable(StringTableAttribute::new(
                        values.values.clone(),
                        indices,
                    )),
                );
            }
        }
    }
    out
}

fn remap_groups_unfused(mesh: &Mesh, point_sources: &[usize]) -> MeshGroups {
    let mut out = MeshGroups::default();
    for (name, values) in mesh.groups.map(AttributeDomain::Vertex) {
        out.map_mut(AttributeDomain::Vertex)
            .insert(name.clone(), values.clone());
    }
    for (name, values) in mesh.groups.map(AttributeDomain::Primitive) {
        out.map_mut(AttributeDomain::Primitive)
            .insert(name.clone(), values.clone());
    }
    for (name, values) in mesh.groups.map(AttributeDomain::Point) {
        let mut out_values = Vec::with_capacity(point_sources.len());
        for &src in point_sources {
            out_values.push(values.get(src).copied().unwrap_or(false));
        }
        out.map_mut(AttributeDomain::Point)
            .insert(name.clone(), out_values);
    }
    out
}

fn remap_uvs_unfused(mesh: &Mesh, point_sources: &[usize]) -> Option<Vec<[f32; 2]>> {
    let uvs = mesh.uvs.as_ref()?;
    if uvs.len() != mesh.positions.len() {
        return None;
    }
    Some(remap_storage_values(uvs, point_sources, [0.0, 0.0]))
}

fn remap_normals_unfused(mesh: &Mesh, point_sources: &[usize]) -> Option<Vec<[f32; 3]>> {
    let normals = mesh.normals.as_ref()?;
    if normals.len() != mesh.positions.len() {
        return None;
    }
    Some(remap_storage_values(normals, point_sources, [0.0, 1.0, 0.0]))
}

fn remap_storage_values<T: Copy>(values: &[T], sources: &[usize], default: T) -> Vec<T> {
    let mut out = Vec::with_capacity(sources.len());
    for &src in sources {
        out.push(*values.get(src).unwrap_or(&default));
    }
    out
}
