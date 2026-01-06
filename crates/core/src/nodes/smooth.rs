use std::collections::{BTreeMap, HashMap, HashSet};

use glam::Vec3;

use crate::attributes::{AttributeDomain, AttributeRef, AttributeStorage};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    attribute_utils::{domain_from_params, parse_attribute_list},
    geometry_in,
    geometry_out,
    group_utils::{mask_has_any, mesh_group_mask, splat_group_mask},
    require_mesh_input,
};
use crate::nodes::splat_utils::{splat_bounds, splat_cell_key};
use crate::splat::SplatGeo;

pub const NAME: &str = "Smooth";

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
            ("attr".to_string(), ParamValue::String("P".to_string())),
            ("domain".to_string(), ParamValue::Int(0)),
            ("iterations".to_string(), ParamValue::Int(1)),
            ("strength".to_string(), ParamValue::Float(0.5)),
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut input = require_mesh_input(inputs, 0, "Smooth requires a mesh input")?;
    apply_to_mesh(params, &mut input)?;
    Ok(input)
}

pub(crate) fn apply_to_splats(params: &NodeParams, splats: &mut SplatGeo) -> Result<(), String> {
    let attrs = parse_attribute_list(params.get_string("attr", "P"));
    if attrs.is_empty() {
        return Ok(());
    }
    let domain = domain_from_params(params);
    if domain == AttributeDomain::Detail || domain == AttributeDomain::Vertex {
        return Ok(());
    }
    let iterations = params.get_int("iterations", 1).max(0) as usize;
    if iterations == 0 {
        return Ok(());
    }
    let strength = params.get_float("strength", 0.5).clamp(0.0, 1.0);
    let mask = splat_group_mask(splats, params, domain);
    if !mask_has_any(mask.as_deref()) {
        return Ok(());
    }

    let neighbors = splat_neighbors(splats);
    if neighbors.is_empty() {
        return Ok(());
    }

    for attr in attrs {
        let Some(attr_ref) = splats.attribute(domain, &attr) else {
            continue;
        };
        match attr_ref {
            AttributeRef::Float(values) => {
                let smoothed =
                    smooth_scalar(values, &neighbors, mask.as_deref(), iterations, strength);
                splats
                    .set_attribute(domain, attr, AttributeStorage::Float(smoothed))
                    .map_err(|err| format!("Smooth error: {:?}", err))?;
            }
            AttributeRef::Int(values) => {
                let smoothed =
                    smooth_int(values, &neighbors, mask.as_deref(), iterations, strength);
                splats
                    .set_attribute(domain, attr, AttributeStorage::Int(smoothed))
                    .map_err(|err| format!("Smooth error: {:?}", err))?;
            }
            AttributeRef::Vec2(values) => {
                let smoothed =
                    smooth_vec2(values, &neighbors, mask.as_deref(), iterations, strength);
                splats
                    .set_attribute(domain, attr, AttributeStorage::Vec2(smoothed))
                    .map_err(|err| format!("Smooth error: {:?}", err))?;
            }
            AttributeRef::Vec3(values) => {
                let smoothed =
                    smooth_vec3(values, &neighbors, mask.as_deref(), iterations, strength);
                splats
                    .set_attribute(domain, attr, AttributeStorage::Vec3(smoothed))
                    .map_err(|err| format!("Smooth error: {:?}", err))?;
            }
            AttributeRef::Vec4(values) => {
                let smoothed =
                    smooth_vec4(values, &neighbors, mask.as_deref(), iterations, strength);
                splats
                    .set_attribute(domain, attr, AttributeStorage::Vec4(smoothed))
                    .map_err(|err| format!("Smooth error: {:?}", err))?;
            }
            AttributeRef::StringTable(_) => {}
        }
    }

    Ok(())
}

fn apply_to_mesh(params: &NodeParams, mesh: &mut Mesh) -> Result<(), String> {
    let attrs = parse_attribute_list(params.get_string("attr", "P"));
    if attrs.is_empty() {
        return Ok(());
    }
    let domain = domain_from_params(params);
    let iterations = params.get_int("iterations", 1).max(0) as usize;
    if iterations == 0 {
        return Ok(());
    }
    let strength = params.get_float("strength", 0.5).clamp(0.0, 1.0);
    let mask = mesh_group_mask(mesh, params, domain);
    if !mask_has_any(mask.as_deref()) {
        return Ok(());
    }

    let neighbors = mesh_neighbors(mesh, domain);
    if neighbors.is_empty() {
        return Ok(());
    }

    for attr in attrs {
        let Some(attr_ref) = mesh.attribute(domain, &attr) else {
            continue;
        };
        match attr_ref {
            AttributeRef::Float(values) => {
                let smoothed =
                    smooth_scalar(values, &neighbors, mask.as_deref(), iterations, strength);
                mesh.set_attribute(domain, attr, AttributeStorage::Float(smoothed))
                    .map_err(|err| format!("Smooth error: {:?}", err))?;
            }
            AttributeRef::Int(values) => {
                let smoothed =
                    smooth_int(values, &neighbors, mask.as_deref(), iterations, strength);
                mesh.set_attribute(domain, attr, AttributeStorage::Int(smoothed))
                    .map_err(|err| format!("Smooth error: {:?}", err))?;
            }
            AttributeRef::Vec2(values) => {
                let smoothed =
                    smooth_vec2(values, &neighbors, mask.as_deref(), iterations, strength);
                mesh.set_attribute(domain, attr, AttributeStorage::Vec2(smoothed))
                    .map_err(|err| format!("Smooth error: {:?}", err))?;
            }
            AttributeRef::Vec3(values) => {
                let smoothed =
                    smooth_vec3(values, &neighbors, mask.as_deref(), iterations, strength);
                mesh.set_attribute(domain, attr, AttributeStorage::Vec3(smoothed))
                    .map_err(|err| format!("Smooth error: {:?}", err))?;
            }
            AttributeRef::Vec4(values) => {
                let smoothed =
                    smooth_vec4(values, &neighbors, mask.as_deref(), iterations, strength);
                mesh.set_attribute(domain, attr, AttributeStorage::Vec4(smoothed))
                    .map_err(|err| format!("Smooth error: {:?}", err))?;
            }
            AttributeRef::StringTable(_) => {}
        }
    }

    Ok(())
}

fn mesh_neighbors(mesh: &Mesh, domain: AttributeDomain) -> Vec<Vec<usize>> {
    match domain {
        AttributeDomain::Point => point_neighbors(mesh),
        AttributeDomain::Vertex => vertex_neighbors(mesh),
        AttributeDomain::Primitive => primitive_neighbors(mesh),
        AttributeDomain::Detail => Vec::new(),
    }
}

fn point_neighbors(mesh: &Mesh) -> Vec<Vec<usize>> {
    let mut neighbors = vec![Vec::new(); mesh.positions.len()];
    for tri in mesh.indices.chunks_exact(3) {
        let a = tri[0] as usize;
        let b = tri[1] as usize;
        let c = tri[2] as usize;
        if a < neighbors.len() && b < neighbors.len() && c < neighbors.len() {
            neighbors[a].extend([b, c]);
            neighbors[b].extend([a, c]);
            neighbors[c].extend([a, b]);
        }
    }
    for list in &mut neighbors {
        list.sort_unstable();
        list.dedup();
    }
    neighbors
}

fn vertex_neighbors(mesh: &Mesh) -> Vec<Vec<usize>> {
    let mut neighbors = vec![Vec::new(); mesh.indices.len()];
    for tri_index in 0..mesh.indices.len() / 3 {
        let base = tri_index * 3;
        let a = base;
        let b = base + 1;
        let c = base + 2;
        if c < neighbors.len() {
            neighbors[a].extend([b, c]);
            neighbors[b].extend([a, c]);
            neighbors[c].extend([a, b]);
        }
    }
    for list in &mut neighbors {
        list.sort_unstable();
        list.dedup();
    }
    neighbors
}

fn primitive_neighbors(mesh: &Mesh) -> Vec<Vec<usize>> {
    let tri_count = mesh.indices.len() / 3;
    if tri_count == 0 {
        return Vec::new();
    }
    let mut point_to_prims = vec![Vec::new(); mesh.positions.len()];
    for (prim_index, tri) in mesh.indices.chunks_exact(3).enumerate() {
        for &idx in tri {
            if let Some(list) = point_to_prims.get_mut(idx as usize) {
                list.push(prim_index);
            }
        }
    }

    let mut neighbors = vec![Vec::new(); tri_count];
    for (prim_index, tri) in mesh.indices.chunks_exact(3).enumerate() {
        let mut set = HashSet::new();
        for &idx in tri {
            if let Some(list) = point_to_prims.get(idx as usize) {
                for &other in list {
                    if other != prim_index {
                        set.insert(other);
                    }
                }
            }
        }
        neighbors[prim_index] = set.into_iter().collect();
        neighbors[prim_index].sort_unstable();
    }
    neighbors
}

fn splat_neighbors(splats: &SplatGeo) -> Vec<Vec<usize>> {
    let count = splats.len();
    if count == 0 {
        return Vec::new();
    }
    let (min, max) = splat_bounds(splats);
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
    for (idx, position) in splats.positions.iter().enumerate() {
        let pos = Vec3::from(*position);
        let key = splat_cell_key(pos, min, inv_cell);
        grid.entry(key).or_default().push(idx);
    }

    let mut neighbors = vec![Vec::new(); count];
    for (idx, position) in splats.positions.iter().enumerate() {
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
        neighbors[idx].sort_unstable();
        neighbors[idx].dedup();
    }

    neighbors
}


fn smooth_scalar(
    values: &[f32],
    neighbors: &[Vec<usize>],
    mask: Option<&[bool]>,
    iterations: usize,
    strength: f32,
) -> Vec<f32> {
    let mut current = values.to_vec();
    let mut next = current.clone();
    for _ in 0..iterations {
        for (idx, value) in current.iter().enumerate() {
            if mask
                .as_ref()
                .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
            {
                continue;
            }
            let neigh = neighbors.get(idx).map(|list| list.as_slice()).unwrap_or(&[]);
            if neigh.is_empty() {
                continue;
            }
            let mut sum = 0.0;
            for &n in neigh {
                if let Some(v) = current.get(n) {
                    sum += *v;
                }
            }
            let avg = sum / neigh.len() as f32;
            if let Some(slot) = next.get_mut(idx) {
                *slot = lerp(*value, avg, strength);
            }
        }
        std::mem::swap(&mut current, &mut next);
    }
    current
}

fn smooth_int(
    values: &[i32],
    neighbors: &[Vec<usize>],
    mask: Option<&[bool]>,
    iterations: usize,
    strength: f32,
) -> Vec<i32> {
    let as_f32: Vec<f32> = values.iter().map(|v| *v as f32).collect();
    let smoothed = smooth_scalar(&as_f32, neighbors, mask, iterations, strength);
    smoothed.iter().map(|v| v.round() as i32).collect()
}

fn smooth_vec2(
    values: &[[f32; 2]],
    neighbors: &[Vec<usize>],
    mask: Option<&[bool]>,
    iterations: usize,
    strength: f32,
) -> Vec<[f32; 2]> {
    let mut current = values.to_vec();
    let mut next = current.clone();
    for _ in 0..iterations {
        for (idx, value) in current.iter().enumerate() {
            if mask
                .as_ref()
                .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
            {
                continue;
            }
            let neigh = neighbors.get(idx).map(|list| list.as_slice()).unwrap_or(&[]);
            if neigh.is_empty() {
                continue;
            }
            let mut sum = [0.0f32; 2];
            for &n in neigh {
                if let Some(v) = current.get(n) {
                    sum[0] += v[0];
                    sum[1] += v[1];
                }
            }
            let inv = 1.0 / neigh.len() as f32;
            let avg = [sum[0] * inv, sum[1] * inv];
            if let Some(slot) = next.get_mut(idx) {
                *slot = [
                    lerp(value[0], avg[0], strength),
                    lerp(value[1], avg[1], strength),
                ];
            }
        }
        std::mem::swap(&mut current, &mut next);
    }
    current
}

fn smooth_vec3(
    values: &[[f32; 3]],
    neighbors: &[Vec<usize>],
    mask: Option<&[bool]>,
    iterations: usize,
    strength: f32,
) -> Vec<[f32; 3]> {
    let mut current = values.to_vec();
    let mut next = current.clone();
    for _ in 0..iterations {
        for (idx, value) in current.iter().enumerate() {
            if mask
                .as_ref()
                .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
            {
                continue;
            }
            let neigh = neighbors.get(idx).map(|list| list.as_slice()).unwrap_or(&[]);
            if neigh.is_empty() {
                continue;
            }
            let mut sum = [0.0f32; 3];
            for &n in neigh {
                if let Some(v) = current.get(n) {
                    sum[0] += v[0];
                    sum[1] += v[1];
                    sum[2] += v[2];
                }
            }
            let inv = 1.0 / neigh.len() as f32;
            let avg = [sum[0] * inv, sum[1] * inv, sum[2] * inv];
            if let Some(slot) = next.get_mut(idx) {
                *slot = [
                    lerp(value[0], avg[0], strength),
                    lerp(value[1], avg[1], strength),
                    lerp(value[2], avg[2], strength),
                ];
            }
        }
        std::mem::swap(&mut current, &mut next);
    }
    current
}

fn smooth_vec4(
    values: &[[f32; 4]],
    neighbors: &[Vec<usize>],
    mask: Option<&[bool]>,
    iterations: usize,
    strength: f32,
) -> Vec<[f32; 4]> {
    let mut current = values.to_vec();
    let mut next = current.clone();
    for _ in 0..iterations {
        for (idx, value) in current.iter().enumerate() {
            if mask
                .as_ref()
                .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
            {
                continue;
            }
            let neigh = neighbors.get(idx).map(|list| list.as_slice()).unwrap_or(&[]);
            if neigh.is_empty() {
                continue;
            }
            let mut sum = [0.0f32; 4];
            for &n in neigh {
                if let Some(v) = current.get(n) {
                    sum[0] += v[0];
                    sum[1] += v[1];
                    sum[2] += v[2];
                    sum[3] += v[3];
                }
            }
            let inv = 1.0 / neigh.len() as f32;
            let avg = [
                sum[0] * inv,
                sum[1] * inv,
                sum[2] * inv,
                sum[3] * inv,
            ];
            if let Some(slot) = next.get_mut(idx) {
                *slot = [
                    lerp(value[0], avg[0], strength),
                    lerp(value[1], avg[1], strength),
                    lerp(value[2], avg[2], strength),
                    lerp(value[3], avg[3], strength),
                ];
            }
        }
        std::mem::swap(&mut current, &mut next);
    }
    current
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
