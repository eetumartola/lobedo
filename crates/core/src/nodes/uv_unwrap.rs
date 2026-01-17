use std::collections::BTreeMap;

use glam::{Vec2, Vec3};

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{geometry_in, geometry_out, require_mesh_input};

pub const NAME: &str = "UV Unwrap";

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
            ("padding".to_string(), ParamValue::Float(0.02)),
            ("normal_threshold".to_string(), ParamValue::Float(45.0)),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut mesh = require_mesh_input(inputs, 0, "UV Unwrap requires a mesh input")?;
    apply_uv_unwrap(params, &mut mesh);
    Ok(mesh)
}

fn apply_uv_unwrap(params: &NodeParams, mesh: &mut Mesh) {
    if mesh.positions.is_empty() {
        return;
    }

    let padding = params.get_float("padding", 0.02).max(0.0);
    let threshold_deg = params.get_float("normal_threshold", 45.0).clamp(0.0, 180.0);
    let cos_threshold = threshold_deg.to_radians().cos();
    let triangulation = mesh.triangulate();
    if triangulation.indices.len() < 3 {
        return;
    }
    let tri_indices = triangulation.indices;
    let tri_corners = triangulation.corner_indices;
    let tri_count = tri_indices.len() / 3;

    let mut tri_normals = Vec::with_capacity(tri_count);
    let mut tri_areas = Vec::with_capacity(tri_count);
    for tri in tri_indices.chunks_exact(3) {
        let p0 = mesh.positions.get(tri[0] as usize).copied().unwrap_or([0.0, 0.0, 0.0]);
        let p1 = mesh.positions.get(tri[1] as usize).copied().unwrap_or([0.0, 0.0, 0.0]);
        let p2 = mesh.positions.get(tri[2] as usize).copied().unwrap_or([0.0, 0.0, 0.0]);
        let n = (Vec3::from(p1) - Vec3::from(p0)).cross(Vec3::from(p2) - Vec3::from(p0));
        let area = n.length() * 0.5;
        let n = if n.length_squared() > 1.0e-6 { n.normalize() } else { Vec3::Y };
        tri_normals.push(n);
        tri_areas.push(area);
    }

    let islands = build_islands(&tri_indices, &tri_normals, cos_threshold);
    let mut island_uvs = Vec::with_capacity(islands.len());
    let mut total_area = 0.0f32;
    for island in islands.iter() {
        let mut normal_sum = Vec3::ZERO;
        let mut area_sum = 0.0f32;
        for &tri_idx in &island.tris {
            let area = tri_areas.get(tri_idx).copied().unwrap_or(0.0);
            area_sum += area;
            normal_sum += tri_normals.get(tri_idx).copied().unwrap_or(Vec3::Y) * area;
        }
        let island_normal = if normal_sum.length_squared() > 1.0e-6 {
            normal_sum.normalize()
        } else {
            Vec3::Y
        };
        let (tangent, bitangent) = island_basis(island_normal);

        let mut min = Vec2::splat(f32::INFINITY);
        let mut max = Vec2::splat(f32::NEG_INFINITY);
        let mut area = 0.0f32;
        let mut uv_list = Vec::with_capacity(island.tris.len());
        for &tri_idx in &island.tris {
            let base = tri_idx * 3;
            let tri = &tri_indices[base..base + 3];
            let p0 = mesh.positions.get(tri[0] as usize).copied().unwrap_or([0.0, 0.0, 0.0]);
            let p1 = mesh.positions.get(tri[1] as usize).copied().unwrap_or([0.0, 0.0, 0.0]);
            let p2 = mesh.positions.get(tri[2] as usize).copied().unwrap_or([0.0, 0.0, 0.0]);
            let uvs = project_triangle_uvs(
                Vec3::from(p0),
                Vec3::from(p1),
                Vec3::from(p2),
                tangent,
                bitangent,
            );
            area += triangle_area_uv(&uvs);
            let (tri_min, tri_max) = uv_bounds(&uvs);
            min = min.min(tri_min);
            max = max.max(tri_max);
            uv_list.push(uvs);
        }
        total_area += area.max(area_sum * 0.5);
        island_uvs.push(IslandUv { min, max, tris: island.tris.clone(), uvs: uv_list });
    }

    let mut row_width = total_area.sqrt().max(1.0e-3);
    let max_width = island_uvs
        .iter()
        .map(|island| island.max.x - island.min.x)
        .fold(0.0f32, f32::max);
    if row_width < max_width {
        row_width = max_width;
    }

    island_uvs.sort_by(|a, b| {
        let ha = a.max.y - a.min.y;
        let hb = b.max.y - b.min.y;
        hb.partial_cmp(&ha).unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut cursor = Vec2::ZERO;
    let mut row_height = 0.0f32;
    let mut max_extent = Vec2::ZERO;
    for island in &mut island_uvs {
        let width = island.max.x - island.min.x;
        let height = island.max.y - island.min.y;
        if cursor.x + width > row_width && cursor.x > 0.0 {
            cursor.x = 0.0;
            cursor.y += row_height + padding;
            row_height = 0.0;
        }
        let offset = Vec2::new(cursor.x - island.min.x, cursor.y - island.min.y);
        for tri_uvs in &mut island.uvs {
            for uv in tri_uvs.iter_mut() {
                let uv_vec = Vec2::from(*uv) + offset;
                *uv = uv_vec.to_array();
            }
        }
        cursor.x += width + padding;
        row_height = row_height.max(height);
        max_extent.x = max_extent.x.max(cursor.x);
        max_extent.y = max_extent.y.max(cursor.y + row_height);
    }

    let mut corner_uvs = vec![[0.0, 0.0]; mesh.indices.len()];
    for island in &island_uvs {
        for (tri_idx, tri_uvs) in island.tris.iter().zip(island.uvs.iter()) {
            let base = tri_idx * 3;
            let c0 = *tri_corners.get(base).unwrap_or(&base);
            let c1 = *tri_corners.get(base + 1).unwrap_or(&(base + 1));
            let c2 = *tri_corners.get(base + 2).unwrap_or(&(base + 2));
            if let Some(slot) = corner_uvs.get_mut(c0) {
                *slot = tri_uvs[0];
            }
            if let Some(slot) = corner_uvs.get_mut(c1) {
                *slot = tri_uvs[1];
            }
            if let Some(slot) = corner_uvs.get_mut(c2) {
                *slot = tri_uvs[2];
            }
        }
    }

    normalize_uvs(&mut corner_uvs, max_extent);
    let _ = mesh.set_attribute(
        AttributeDomain::Vertex,
        "uv",
        AttributeStorage::Vec2(corner_uvs),
    );
}

#[derive(Clone)]
struct Island {
    tris: Vec<usize>,
}

struct IslandUv {
    min: Vec2,
    max: Vec2,
    tris: Vec<usize>,
    uvs: Vec<[[f32; 2]; 3]>,
}

fn island_basis(normal: Vec3) -> (Vec3, Vec3) {
    let up = if normal.abs().dot(Vec3::Y) < 0.9 {
        Vec3::Y
    } else {
        Vec3::X
    };
    let tangent = normal.cross(up).normalize_or_zero();
    let bitangent = normal.cross(tangent).normalize_or_zero();
    (tangent, bitangent)
}

fn project_triangle_uvs(
    p0: Vec3,
    p1: Vec3,
    p2: Vec3,
    tangent: Vec3,
    bitangent: Vec3,
) -> [[f32; 2]; 3] {
    let uv0 = [p0.dot(tangent), p0.dot(bitangent)];
    let uv1 = [p1.dot(tangent), p1.dot(bitangent)];
    let uv2 = [p2.dot(tangent), p2.dot(bitangent)];
    [uv0, uv1, uv2]
}

fn triangle_area_uv(uvs: &[[f32; 2]; 3]) -> f32 {
    let a = Vec2::from(uvs[0]);
    let b = Vec2::from(uvs[1]);
    let c = Vec2::from(uvs[2]);
    let ab = b - a;
    let ac = c - a;
    (ab.x * ac.y - ab.y * ac.x).abs() * 0.5
}

fn build_islands(tri_indices: &[u32], normals: &[Vec3], cos_threshold: f32) -> Vec<Island> {
    let tri_count = tri_indices.len() / 3;
    let mut parent: Vec<usize> = (0..tri_count).collect();
    let mut rank = vec![0u8; tri_count];
    let mut edge_map: std::collections::HashMap<(u32, u32), Vec<usize>> =
        std::collections::HashMap::new();

    for (tri_idx, tri) in tri_indices.chunks_exact(3).enumerate() {
        let edges = [
            (tri[0], tri[1]),
            (tri[1], tri[2]),
            (tri[2], tri[0]),
        ];
        for (a, b) in edges {
            let key = if a < b { (a, b) } else { (b, a) };
            edge_map.entry(key).or_default().push(tri_idx);
        }
    }

    for tris in edge_map.values() {
        if tris.len() < 2 {
            continue;
        }
        let a = tris[0];
        let b = tris[1];
        let na = normals[a];
        let nb = normals[b];
        if na.dot(nb) >= cos_threshold {
            union_sets(&mut parent, &mut rank, a, b);
        }
    }

    let mut island_map: std::collections::HashMap<usize, Vec<usize>> =
        std::collections::HashMap::new();
    for tri in 0..tri_count {
        let root = find_root(&mut parent, tri);
        island_map.entry(root).or_default().push(tri);
    }

    island_map
        .into_values()
        .map(|tris| Island { tris })
        .collect()
}

fn find_root(parent: &mut [usize], idx: usize) -> usize {
    if parent[idx] != idx {
        parent[idx] = find_root(parent, parent[idx]);
    }
    parent[idx]
}

fn union_sets(parent: &mut [usize], rank: &mut [u8], a: usize, b: usize) {
    let root_a = find_root(parent, a);
    let root_b = find_root(parent, b);
    if root_a == root_b {
        return;
    }
    let rank_a = rank[root_a];
    let rank_b = rank[root_b];
    if rank_a < rank_b {
        parent[root_a] = root_b;
    } else if rank_a > rank_b {
        parent[root_b] = root_a;
    } else {
        parent[root_b] = root_a;
        rank[root_a] = rank_a.saturating_add(1);
    }
}

fn uv_bounds(uvs: &[[f32; 2]; 3]) -> (Vec2, Vec2) {
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    for uv in uvs {
        min.x = min.x.min(uv[0]);
        min.y = min.y.min(uv[1]);
        max.x = max.x.max(uv[0]);
        max.y = max.y.max(uv[1]);
    }
    (min, max)
}

fn normalize_uvs(uvs: &mut [[f32; 2]], max_extent: Vec2) {
    let max_u = max_extent.x.max(1.0e-6);
    let max_v = max_extent.y.max(1.0e-6);
    for uv in uvs {
        uv[0] /= max_u;
        uv[1] /= max_v;
    }
}
