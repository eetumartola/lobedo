use glam::Vec3;

use crate::attributes::{AttributeDomain, AttributeStorage, MeshAttributes, StringTableAttribute};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::{Mesh, MeshGroups};
use crate::nodes::{
    geometry_in,
    geometry_out,
    group_utils::mesh_group_mask,
    require_mesh_input,
    selection_shape_params,
};

pub const NAME: &str = "Delete";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![geometry_in("in")],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    let mut values = selection_shape_params();
    values.insert("group".to_string(), ParamValue::String(String::new()));
    values.insert("group_type".to_string(), ParamValue::Int(0));
    NodeParams { values }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let input = require_mesh_input(inputs, 0, "Delete requires a mesh input")?;
    Ok(delete_mesh(params, &input))
}

fn delete_mesh(params: &NodeParams, mesh: &Mesh) -> Mesh {
    let shape = params.get_string("shape", "box");
    let invert = params.get_bool("invert", false);

    let mut keep_points = Vec::with_capacity(mesh.positions.len());
    for position in &mesh.positions {
        let inside = is_inside(params, shape, Vec3::from(*position));
        keep_points.push(if invert { inside } else { !inside });
    }

    if let Some(mask) = mesh_group_mask(mesh, params, AttributeDomain::Point) {
        for (idx, keep) in keep_points.iter_mut().enumerate() {
            if !mask.get(idx).copied().unwrap_or(false) {
                *keep = true;
            }
        }
    }

    if mesh.indices.is_empty() {
        return filter_point_cloud(mesh, &keep_points);
    }

    let mut kept_tris = Vec::new();
    let mut kept_indices = Vec::new();
    for (tri_index, tri) in mesh.indices.chunks_exact(3).enumerate() {
        let a = tri[0] as usize;
        let b = tri[1] as usize;
        let c = tri[2] as usize;
        let keep = keep_points.get(a).copied().unwrap_or(false)
            && keep_points.get(b).copied().unwrap_or(false)
            && keep_points.get(c).copied().unwrap_or(false);
        if keep {
            kept_tris.push(tri_index);
            kept_indices.extend_from_slice(tri);
        }
    }

    let mut used = vec![false; mesh.positions.len()];
    for index in &kept_indices {
        if let Some(slot) = used.get_mut(*index as usize) {
            *slot = true;
        }
    }
    let (mapping, kept_points_indices) = build_index_mapping(&used);

    let mut new_positions = Vec::with_capacity(kept_points_indices.len());
    for &old in &kept_points_indices {
        new_positions.push(mesh.positions[old]);
    }

    let mut new_indices = Vec::with_capacity(kept_indices.len());
    for index in kept_indices {
        let mapped = mapping[index as usize];
        if mapped != u32::MAX {
            new_indices.push(mapped);
        }
    }

    let new_normals = mesh.normals.as_ref().map(|normals| {
        kept_points_indices.iter().map(|&i| normals[i]).collect()
    });
    let new_uvs = mesh.uvs.as_ref().and_then(|uvs| {
        if uvs.len() == mesh.positions.len() {
            Some(kept_points_indices.iter().map(|&i| uvs[i]).collect())
        } else {
            None
        }
    });

    let new_corner_normals = mesh.corner_normals.as_ref().and_then(|corner| {
        if corner.len() == mesh.indices.len() {
            let mut filtered = Vec::with_capacity(new_indices.len());
            for tri_index in kept_tris.iter() {
                let base = tri_index * 3;
                filtered.push(corner[base]);
                filtered.push(corner[base + 1]);
                filtered.push(corner[base + 2]);
            }
            Some(filtered)
        } else {
            None
        }
    });

    let new_attributes = filter_mesh_attributes(mesh, &kept_points_indices, &kept_tris, &new_indices);
    let new_groups = filter_mesh_groups(mesh, &kept_points_indices, &kept_tris, &new_indices);

    Mesh {
        positions: new_positions,
        indices: new_indices,
        normals: new_normals,
        corner_normals: new_corner_normals,
        uvs: new_uvs,
        attributes: new_attributes,
        groups: new_groups,
    }
}

fn filter_point_cloud(mesh: &Mesh, keep_points: &[bool]) -> Mesh {
    let mut kept_points = Vec::new();
    for (idx, keep) in keep_points.iter().copied().enumerate() {
        if keep {
            kept_points.push(idx);
        }
    }

    let mut new_positions = Vec::with_capacity(kept_points.len());
    for &idx in &kept_points {
        new_positions.push(mesh.positions[idx]);
    }

    let new_normals = mesh.normals.as_ref().map(|normals| {
        kept_points.iter().map(|&i| normals[i]).collect()
    });
    let new_uvs = mesh.uvs.as_ref().and_then(|uvs| {
        if uvs.len() == mesh.positions.len() {
            Some(kept_points.iter().map(|&i| uvs[i]).collect())
        } else {
            None
        }
    });

    let new_attributes = filter_mesh_attributes(mesh, &kept_points, &[], &[]);
    let new_groups = filter_mesh_groups(mesh, &kept_points, &[], &[]);

    Mesh {
        positions: new_positions,
        indices: Vec::new(),
        normals: new_normals,
        corner_normals: None,
        uvs: new_uvs,
        attributes: new_attributes,
        groups: new_groups,
    }
}

fn filter_mesh_attributes(
    mesh: &Mesh,
    kept_points: &[usize],
    kept_tris: &[usize],
    kept_indices: &[u32],
) -> MeshAttributes {
    let mut attributes = MeshAttributes::default();

    for (name, storage) in mesh.attributes.map(AttributeDomain::Point) {
        let filtered = filter_attribute_storage(storage, kept_points);
        attributes.map_mut(AttributeDomain::Point).insert(name.clone(), filtered);
    }
    for (name, storage) in mesh.attributes.map(AttributeDomain::Vertex) {
        if !kept_indices.is_empty() {
            let mut kept_corners = Vec::with_capacity(kept_indices.len());
            for tri_index in kept_tris {
                kept_corners.push(tri_index * 3);
                kept_corners.push(tri_index * 3 + 1);
                kept_corners.push(tri_index * 3 + 2);
            }
            let filtered = filter_attribute_storage(storage, &kept_corners);
            attributes.map_mut(AttributeDomain::Vertex).insert(name.clone(), filtered);
        }
    }
    for (name, storage) in mesh.attributes.map(AttributeDomain::Primitive) {
        let filtered = filter_attribute_storage(storage, kept_tris);
        attributes
            .map_mut(AttributeDomain::Primitive)
            .insert(name.clone(), filtered);
    }
    for (name, storage) in mesh.attributes.map(AttributeDomain::Detail) {
        attributes
            .map_mut(AttributeDomain::Detail)
            .insert(name.clone(), storage.clone());
    }

    attributes
}

fn filter_mesh_groups(
    mesh: &Mesh,
    kept_points: &[usize],
    kept_tris: &[usize],
    kept_indices: &[u32],
) -> MeshGroups {
    let mut groups = MeshGroups::default();

    for (name, values) in mesh.groups.map(AttributeDomain::Point) {
        let filtered = filter_group_values(values, kept_points);
        groups.map_mut(AttributeDomain::Point).insert(name.clone(), filtered);
    }
    for (name, values) in mesh.groups.map(AttributeDomain::Vertex) {
        if !kept_indices.is_empty() {
            let mut kept_corners = Vec::with_capacity(kept_indices.len());
            for tri_index in kept_tris {
                kept_corners.push(tri_index * 3);
                kept_corners.push(tri_index * 3 + 1);
                kept_corners.push(tri_index * 3 + 2);
            }
            let filtered = filter_group_values(values, &kept_corners);
            groups
                .map_mut(AttributeDomain::Vertex)
                .insert(name.clone(), filtered);
        }
    }
    for (name, values) in mesh.groups.map(AttributeDomain::Primitive) {
        let filtered = filter_group_values(values, kept_tris);
        groups
            .map_mut(AttributeDomain::Primitive)
            .insert(name.clone(), filtered);
    }

    groups
}

fn filter_group_values(values: &[bool], indices: &[usize]) -> Vec<bool> {
    let mut out = Vec::with_capacity(indices.len());
    for &idx in indices {
        if let Some(value) = values.get(idx) {
            out.push(*value);
        }
    }
    out
}

fn filter_attribute_storage(storage: &AttributeStorage, indices: &[usize]) -> AttributeStorage {
    match storage {
        AttributeStorage::Float(values) => {
            let mut out = Vec::with_capacity(indices.len());
            for &idx in indices {
                if let Some(value) = values.get(idx) {
                    out.push(*value);
                }
            }
            AttributeStorage::Float(out)
        }
        AttributeStorage::Int(values) => {
            let mut out = Vec::with_capacity(indices.len());
            for &idx in indices {
                if let Some(value) = values.get(idx) {
                    out.push(*value);
                }
            }
            AttributeStorage::Int(out)
        }
        AttributeStorage::Vec2(values) => {
            let mut out = Vec::with_capacity(indices.len());
            for &idx in indices {
                if let Some(value) = values.get(idx) {
                    out.push(*value);
                }
            }
            AttributeStorage::Vec2(out)
        }
        AttributeStorage::Vec3(values) => {
            let mut out = Vec::with_capacity(indices.len());
            for &idx in indices {
                if let Some(value) = values.get(idx) {
                    out.push(*value);
                }
            }
            AttributeStorage::Vec3(out)
        }
        AttributeStorage::Vec4(values) => {
            let mut out = Vec::with_capacity(indices.len());
            for &idx in indices {
                if let Some(value) = values.get(idx) {
                    out.push(*value);
                }
            }
            AttributeStorage::Vec4(out)
        }
        AttributeStorage::StringTable(values) => {
            let mut out = Vec::with_capacity(indices.len());
            for &idx in indices {
                if let Some(value) = values.indices.get(idx) {
                    out.push(*value);
                }
            }
            AttributeStorage::StringTable(StringTableAttribute::new(values.values.clone(), out))
        }
    }
}

fn build_index_mapping(used: &[bool]) -> (Vec<u32>, Vec<usize>) {
    let mut mapping = vec![u32::MAX; used.len()];
    let mut kept = Vec::new();
    let mut next = 0u32;
    for (idx, keep) in used.iter().copied().enumerate() {
        if keep {
            mapping[idx] = next;
            kept.push(idx);
            next += 1;
        }
    }
    (mapping, kept)
}

pub(crate) fn is_inside(params: &NodeParams, shape: &str, position: Vec3) -> bool {
    match shape.to_lowercase().as_str() {
        "sphere" => {
            let center = Vec3::from(params.get_vec3("center", [0.0, 0.0, 0.0]));
            let mut size = Vec3::from(params.get_vec3("size", [1.0, 1.0, 1.0]));
            if size == Vec3::ONE {
                let radius = params.get_float("radius", 1.0);
                if (radius - 1.0).abs() > f32::EPSILON {
                    size = Vec3::splat(radius * 2.0);
                }
            }
            let radii = (size * 0.5).max(Vec3::splat(f32::EPSILON));
            let delta = position - center;
            let normalized = delta / radii;
            normalized.length_squared() <= 1.0
        }
        "plane" => {
            let origin = Vec3::from(params.get_vec3("plane_origin", [0.0, 0.0, 0.0]));
            let normal = Vec3::from(params.get_vec3("plane_normal", [0.0, 1.0, 0.0]));
            let normal = if normal.length_squared() > 0.0 {
                normal.normalize()
            } else {
                Vec3::Y
            };
            (position - origin).dot(normal) >= 0.0
        }
        _ => {
            let center = Vec3::from(params.get_vec3("center", [0.0, 0.0, 0.0]));
            let mut size = Vec3::from(params.get_vec3("size", [1.0, 1.0, 1.0]));
            if size == Vec3::ONE {
                let radius = params.get_float("radius", 1.0);
                if (radius - 1.0).abs() > f32::EPSILON {
                    // If only radius was adjusted, treat it as a uniform box size.
                    size = Vec3::splat(radius * 2.0);
                }
            }
            let half = size * 0.5;
            let min = center - half;
            let max = center + half;
            position.x >= min.x
                && position.x <= max.x
                && position.y >= min.y
                && position.y <= max.y
                && position.z >= min.z
                && position.z <= max.z
        }
    }
}
