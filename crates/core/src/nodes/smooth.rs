use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap, HashMap, HashSet};

use glam::Vec3;

use crate::attributes::{AttributeDomain, AttributeRef, AttributeStorage};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    attribute_utils::{
        domain_from_params, mesh_positions_for_domain, parse_attribute_list,
        splat_positions_for_domain,
    },
    geometry_in,
    geometry_out,
    group_utils::{mask_has_any, mesh_group_mask, splat_group_mask},
    require_mesh_input,
};
use crate::splat::SplatGeo;

pub const NAME: &str = "Smooth";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SmoothSpace {
    World,
    Surface,
}

impl SmoothSpace {
    fn from_params(params: &NodeParams) -> Self {
        match params.get_int("smooth_space", 0) {
            1 => SmoothSpace::Surface,
            _ => SmoothSpace::World,
        }
    }
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
            ("attr".to_string(), ParamValue::String("P".to_string())),
            ("domain".to_string(), ParamValue::Int(0)),
            ("smooth_space".to_string(), ParamValue::Int(0)),
            ("radius".to_string(), ParamValue::Float(0.0)),
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
    let smooth_space = SmoothSpace::from_params(params);
    let radius = params.get_float("radius", 0.0).max(0.0);
    let iterations = params.get_int("iterations", 1).max(0) as usize;
    if iterations == 0 {
        return Ok(());
    }
    let strength = params.get_float("strength", 0.5).clamp(0.0, 1.0);
    let mask = splat_group_mask(splats, params, domain);
    if !mask_has_any(mask.as_deref()) {
        return Ok(());
    }

    let neighbors = splat_neighbors(splats, domain, smooth_space, radius);
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
    let smooth_space = SmoothSpace::from_params(params);
    let radius = params.get_float("radius", 0.0).max(0.0);
    let iterations = params.get_int("iterations", 1).max(0) as usize;
    if iterations == 0 {
        return Ok(());
    }
    let strength = params.get_float("strength", 0.5).clamp(0.0, 1.0);
    let mask = mesh_group_mask(mesh, params, domain);
    if !mask_has_any(mask.as_deref()) {
        return Ok(());
    }

    let neighbors = mesh_neighbors(mesh, domain, smooth_space, radius);
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

fn mesh_neighbors(
    mesh: &Mesh,
    domain: AttributeDomain,
    space: SmoothSpace,
    radius: f32,
) -> Vec<Vec<usize>> {
    match space {
        SmoothSpace::World => world_neighbors_for_mesh(mesh, domain, radius),
        SmoothSpace::Surface => surface_neighbors(mesh, domain, radius),
    }
}

fn world_neighbors_for_mesh(
    mesh: &Mesh,
    domain: AttributeDomain,
    radius: f32,
) -> Vec<Vec<usize>> {
    let positions = mesh_positions_for_domain(mesh, domain);
    world_neighbors_from_positions(&positions, radius)
}

fn surface_neighbors(mesh: &Mesh, domain: AttributeDomain, radius: f32) -> Vec<Vec<usize>> {
    if radius <= 0.0 {
        return match domain {
            AttributeDomain::Point => point_neighbors(mesh),
            AttributeDomain::Vertex => vertex_neighbors(mesh),
            AttributeDomain::Primitive => primitive_neighbors(mesh),
            AttributeDomain::Detail => Vec::new(),
        };
    }

    let positions = mesh_positions_for_domain(mesh, domain);
    if positions.is_empty() {
        return Vec::new();
    }

    let adjacency = match domain {
        AttributeDomain::Point => point_adjacency(mesh, &positions),
        AttributeDomain::Vertex => vertex_adjacency(mesh, &positions),
        AttributeDomain::Primitive => primitive_adjacency(mesh, &positions),
        AttributeDomain::Detail => Vec::new(),
    };
    if adjacency.is_empty() {
        return Vec::new();
    }

    let mut neighbors = vec![Vec::new(); adjacency.len()];
    for idx in 0..adjacency.len() {
        neighbors[idx] = dijkstra_neighbors(idx, &adjacency, radius);
    }
    neighbors
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

fn point_adjacency(mesh: &Mesh, positions: &[Vec3]) -> Vec<Vec<(usize, f32)>> {
    let mut adjacency = vec![Vec::new(); positions.len()];
    for tri in mesh.indices.chunks_exact(3) {
        let a = tri[0] as usize;
        let b = tri[1] as usize;
        let c = tri[2] as usize;
        push_edge(&mut adjacency, positions, a, b);
        push_edge(&mut adjacency, positions, b, a);
        push_edge(&mut adjacency, positions, b, c);
        push_edge(&mut adjacency, positions, c, b);
        push_edge(&mut adjacency, positions, c, a);
        push_edge(&mut adjacency, positions, a, c);
    }
    dedup_weighted_adjacency(&mut adjacency);
    adjacency
}

fn vertex_adjacency(mesh: &Mesh, positions: &[Vec3]) -> Vec<Vec<(usize, f32)>> {
    let mut adjacency = vec![Vec::new(); positions.len()];
    let tri_count = mesh.indices.len() / 3;
    for tri_index in 0..tri_count {
        let base = tri_index * 3;
        let a = base;
        let b = base + 1;
        let c = base + 2;
        push_edge(&mut adjacency, positions, a, b);
        push_edge(&mut adjacency, positions, b, a);
        push_edge(&mut adjacency, positions, b, c);
        push_edge(&mut adjacency, positions, c, b);
        push_edge(&mut adjacency, positions, c, a);
        push_edge(&mut adjacency, positions, a, c);
    }
    dedup_weighted_adjacency(&mut adjacency);
    adjacency
}

fn primitive_adjacency(mesh: &Mesh, positions: &[Vec3]) -> Vec<Vec<(usize, f32)>> {
    let neighbors = primitive_neighbors(mesh);
    let mut adjacency = vec![Vec::new(); neighbors.len()];
    for (idx, list) in neighbors.iter().enumerate() {
        for &other in list {
            if other < positions.len() && idx < positions.len() {
                let dist = positions[idx].distance(positions[other]);
                if dist.is_finite() {
                    adjacency[idx].push((other, dist));
                }
            }
        }
    }
    adjacency
}

fn push_edge(
    adjacency: &mut [Vec<(usize, f32)>],
    positions: &[Vec3],
    from: usize,
    to: usize,
) {
    if from >= adjacency.len() || to >= positions.len() {
        return;
    }
    let dist = positions[from].distance(positions[to]);
    if dist.is_finite() {
        adjacency[from].push((to, dist));
    }
}

fn dedup_weighted_adjacency(adjacency: &mut [Vec<(usize, f32)>]) {
    for list in adjacency.iter_mut() {
        list.sort_by(|a, b| a.0.cmp(&b.0));
        list.dedup_by(|a, b| {
            if a.0 == b.0 {
                if b.1 < a.1 {
                    a.1 = b.1;
                }
                true
            } else {
                false
            }
        });
    }
}

fn world_neighbors_from_positions(positions: &[Vec3], radius: f32) -> Vec<Vec<usize>> {
    let count = positions.len();
    if count == 0 {
        return Vec::new();
    }
    let (min, max) = positions_bounds(positions);
    let auto_radius = auto_radius_from_bounds(min, max, count);
    let search_radius = if radius > 0.0 { radius } else { auto_radius };
    let mut cell_size = if radius > 0.0 { radius } else { auto_radius };
    if !cell_size.is_finite() || cell_size <= 1.0e-6 {
        cell_size = 1.0;
    }
    let inv_cell = 1.0 / cell_size;
    let cell_range = (search_radius / cell_size).ceil().max(1.0) as i32;
    let radius_sq = search_radius * search_radius;

    let mut grid: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();
    let mut valid = vec![false; count];
    for (idx, position) in positions.iter().enumerate() {
        if !position.is_finite() {
            continue;
        }
        valid[idx] = true;
        let key = cell_key(*position, min, inv_cell);
        grid.entry(key).or_default().push(idx);
    }

    let mut neighbors = vec![Vec::new(); count];
    for (idx, position) in positions.iter().enumerate() {
        if !valid[idx] {
            continue;
        }
        let base = cell_key(*position, min, inv_cell);
        for dz in -cell_range..=cell_range {
            for dy in -cell_range..=cell_range {
                for dx in -cell_range..=cell_range {
                    let key = (base.0 + dx, base.1 + dy, base.2 + dz);
                    if let Some(list) = grid.get(&key) {
                        for &other in list {
                            if other == idx {
                                continue;
                            }
                            let diff = positions[other] - *position;
                            if diff.length_squared() <= radius_sq {
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

fn positions_bounds(positions: &[Vec3]) -> (Vec3, Vec3) {
    let mut iter = positions.iter().copied().filter(|p| p.is_finite());
    let Some(first) = iter.next() else {
        return (Vec3::ZERO, Vec3::ZERO);
    };
    let mut min = first;
    let mut max = first;
    for p in iter {
        min = min.min(p);
        max = max.max(p);
    }
    (min, max)
}

fn auto_radius_from_bounds(min: Vec3, max: Vec3, count: usize) -> f32 {
    if count == 0 {
        return 1.0;
    }
    let extent = max - min;
    let volume = extent.x * extent.y * extent.z;
    if volume <= 0.0 {
        return 1.0;
    }
    let radius = (volume / count as f32).cbrt();
    if radius.is_finite() && radius > 1.0e-6 {
        radius
    } else {
        1.0
    }
}

fn cell_key(pos: Vec3, origin: Vec3, inv_cell: f32) -> (i32, i32, i32) {
    let rel = (pos - origin) * inv_cell;
    (rel.x.floor() as i32, rel.y.floor() as i32, rel.z.floor() as i32)
}

#[derive(Clone, Copy, Debug)]
struct HeapItem {
    cost: f32,
    index: usize,
}

impl Eq for HeapItem {}

impl PartialEq for HeapItem {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost && self.index == other.index
    }
}

impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .partial_cmp(&self.cost)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for HeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn dijkstra_neighbors(
    start: usize,
    adjacency: &[Vec<(usize, f32)>],
    max_distance: f32,
) -> Vec<usize> {
    let mut dist = vec![f32::INFINITY; adjacency.len()];
    let mut heap = BinaryHeap::new();
    dist[start] = 0.0;
    heap.push(HeapItem {
        cost: 0.0,
        index: start,
    });

    let mut out = Vec::new();
    while let Some(HeapItem { cost, index }) = heap.pop() {
        if cost > max_distance {
            continue;
        }
        if cost > dist[index] {
            continue;
        }
        if index != start {
            out.push(index);
        }
        if let Some(list) = adjacency.get(index) {
            for &(neighbor, weight) in list {
                if !weight.is_finite() {
                    continue;
                }
                let next = cost + weight;
                if next <= max_distance && next < dist[neighbor] {
                    dist[neighbor] = next;
                    heap.push(HeapItem {
                        cost: next,
                        index: neighbor,
                    });
                }
            }
        }
    }
    out
}

fn splat_neighbors(
    splats: &SplatGeo,
    domain: AttributeDomain,
    _space: SmoothSpace,
    radius: f32,
) -> Vec<Vec<usize>> {
    let positions = splat_positions_for_domain(splats, domain);
    world_neighbors_from_positions(&positions, radius)
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
