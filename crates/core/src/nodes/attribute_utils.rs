use glam::Vec3;

use crate::attributes::{AttributeDomain, AttributeRef};
use crate::graph::NodeParams;
use crate::mesh::Mesh;
use crate::splat::SplatGeo;

pub fn domain_from_params(params: &NodeParams) -> AttributeDomain {
    match params.get_int("domain", 0).clamp(0, 3) {
        0 => AttributeDomain::Point,
        1 => AttributeDomain::Vertex,
        2 => AttributeDomain::Primitive,
        _ => AttributeDomain::Detail,
    }
}

pub fn parse_attribute_list(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string())
        .collect()
}

pub fn mesh_sample_position(mesh: &Mesh, domain: AttributeDomain, index: usize) -> Vec3 {
    match domain {
        AttributeDomain::Point => mesh
            .positions
            .get(index)
            .copied()
            .map(Vec3::from)
            .unwrap_or(Vec3::ZERO),
        AttributeDomain::Vertex => mesh
            .indices
            .get(index)
            .and_then(|idx| mesh.positions.get(*idx as usize))
            .copied()
            .map(Vec3::from)
            .unwrap_or(Vec3::ZERO),
        AttributeDomain::Primitive => {
            let base = index * 3;
            let tri = mesh.indices.get(base..base + 3);
            if let Some(tri) = tri {
                let p0 = mesh.positions.get(tri[0] as usize).copied().unwrap_or([0.0; 3]);
                let p1 = mesh.positions.get(tri[1] as usize).copied().unwrap_or([0.0; 3]);
                let p2 = mesh.positions.get(tri[2] as usize).copied().unwrap_or([0.0; 3]);
                (Vec3::from(p0) + Vec3::from(p1) + Vec3::from(p2)) / 3.0
            } else {
                Vec3::ZERO
            }
        }
        AttributeDomain::Detail => mesh
            .bounds()
            .map(|bounds| {
                Vec3::new(
                    (bounds.min[0] + bounds.max[0]) * 0.5,
                    (bounds.min[1] + bounds.max[1]) * 0.5,
                    (bounds.min[2] + bounds.max[2]) * 0.5,
                )
            })
            .unwrap_or(Vec3::ZERO),
    }
}

pub fn splat_sample_position(splats: &SplatGeo, domain: AttributeDomain, index: usize) -> Vec3 {
    match domain {
        AttributeDomain::Point | AttributeDomain::Primitive => splats
            .positions
            .get(index)
            .copied()
            .map(Vec3::from)
            .unwrap_or(Vec3::ZERO),
        AttributeDomain::Detail => splat_bounds_center(splats),
        AttributeDomain::Vertex => Vec3::ZERO,
    }
}

pub fn mesh_positions_for_domain(mesh: &Mesh, domain: AttributeDomain) -> Vec<Vec3> {
    match domain {
        AttributeDomain::Point => mesh.positions.iter().copied().map(Vec3::from).collect(),
        AttributeDomain::Vertex => mesh
            .indices
            .iter()
            .filter_map(|idx| mesh.positions.get(*idx as usize))
            .copied()
            .map(Vec3::from)
            .collect(),
        AttributeDomain::Primitive => {
            let mut positions = Vec::with_capacity(mesh.indices.len() / 3);
            for tri in mesh.indices.chunks_exact(3) {
                let p0 = mesh.positions.get(tri[0] as usize).copied().unwrap_or([0.0; 3]);
                let p1 = mesh.positions.get(tri[1] as usize).copied().unwrap_or([0.0; 3]);
                let p2 = mesh.positions.get(tri[2] as usize).copied().unwrap_or([0.0; 3]);
                let center = (Vec3::from(p0) + Vec3::from(p1) + Vec3::from(p2)) / 3.0;
                positions.push(center);
            }
            positions
        }
        AttributeDomain::Detail => mesh
            .bounds()
            .map(|bounds| {
                Vec3::new(
                    (bounds.min[0] + bounds.max[0]) * 0.5,
                    (bounds.min[1] + bounds.max[1]) * 0.5,
                    (bounds.min[2] + bounds.max[2]) * 0.5,
                )
            })
            .into_iter()
            .collect(),
    }
}

pub fn splat_positions_for_domain(splats: &SplatGeo, domain: AttributeDomain) -> Vec<Vec3> {
    match domain {
        AttributeDomain::Point | AttributeDomain::Primitive => {
            splats.positions.iter().copied().map(Vec3::from).collect()
        }
        AttributeDomain::Detail => {
            if splats.positions.is_empty() {
                Vec::new()
            } else {
                vec![splat_bounds_center(splats)]
            }
        }
        AttributeDomain::Vertex => Vec::new(),
    }
}

pub fn existing_float_attr_mesh(
    mesh: &Mesh,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<f32> {
    if let Some(AttributeRef::Float(values)) = mesh.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![0.0; count.max(1)]
}

pub fn existing_int_attr_mesh(
    mesh: &Mesh,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<i32> {
    if let Some(AttributeRef::Int(values)) = mesh.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![0; count.max(1)]
}

pub fn existing_vec2_attr_mesh(
    mesh: &Mesh,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<[f32; 2]> {
    if let Some(AttributeRef::Vec2(values)) = mesh.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![[0.0, 0.0]; count.max(1)]
}

pub fn existing_vec3_attr_mesh(
    mesh: &Mesh,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<[f32; 3]> {
    if let Some(AttributeRef::Vec3(values)) = mesh.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![[0.0, 0.0, 0.0]; count.max(1)]
}

pub fn existing_vec4_attr_mesh(
    mesh: &Mesh,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<[f32; 4]> {
    if let Some(AttributeRef::Vec4(values)) = mesh.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![[0.0, 0.0, 0.0, 0.0]; count.max(1)]
}

pub fn existing_float_attr_splats(
    splats: &SplatGeo,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<f32> {
    if let Some(AttributeRef::Float(values)) = splats.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![0.0; count.max(1)]
}

pub fn existing_int_attr_splats(
    splats: &SplatGeo,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<i32> {
    if let Some(AttributeRef::Int(values)) = splats.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![0; count.max(1)]
}

pub fn existing_vec2_attr_splats(
    splats: &SplatGeo,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<[f32; 2]> {
    if let Some(AttributeRef::Vec2(values)) = splats.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![[0.0, 0.0]; count.max(1)]
}

pub fn existing_vec3_attr_splats(
    splats: &SplatGeo,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<[f32; 3]> {
    if let Some(AttributeRef::Vec3(values)) = splats.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![[0.0, 0.0, 0.0]; count.max(1)]
}

pub fn existing_vec4_attr_splats(
    splats: &SplatGeo,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<[f32; 4]> {
    if let Some(AttributeRef::Vec4(values)) = splats.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![[0.0, 0.0, 0.0, 0.0]; count.max(1)]
}

fn splat_bounds_center(splats: &SplatGeo) -> Vec3 {
    let mut iter = splats.positions.iter();
    let Some(first) = iter.next().copied() else {
        return Vec3::ZERO;
    };
    let mut min = first;
    let mut max = first;
    for p in iter {
        min[0] = min[0].min(p[0]);
        min[1] = min[1].min(p[1]);
        min[2] = min[2].min(p[2]);
        max[0] = max[0].max(p[0]);
        max[1] = max[1].max(p[1]);
        max[2] = max[2].max(p[2]);
    }
    Vec3::new(
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    )
}
