use std::collections::BTreeMap;

use glam::Vec3;

use crate::attributes::AttributeDomain;
use crate::curve::Curve;
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{geometry_in, geometry_out, group_utils::mesh_group_mask, require_mesh_input};
use crate::volume::{Volume, VolumeKind};
use crate::volume_sampling::VolumeSampler;

pub const NAME: &str = "Scatter";

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
            ("count".to_string(), ParamValue::Int(100)),
            ("seed".to_string(), ParamValue::Int(1)),
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let input = require_mesh_input(inputs, 0, "Scatter requires a mesh input")?;
    let count = params.get_int("count", 200).max(0) as usize;
    let seed = params.get_int("seed", 1).max(0) as u32;
    let mask = mesh_group_mask(&input, params, AttributeDomain::Primitive);
    scatter_points(&input, count, seed, mask.as_deref())
}

pub fn apply_to_geometry(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };
    let count = params.get_int("count", 200).max(0) as usize;
    if count == 0 {
        return Ok(Geometry::default());
    }
    let seed = params.get_int("seed", 1).max(0) as u32;

    let merged_mesh = input.merged_mesh();
    if let Some(mesh) = merged_mesh.as_ref() {
        if !mesh.positions.is_empty()
            && mesh.indices.len() >= 3
            && mesh.indices.len().is_multiple_of(3)
        {
            let mask = mesh_group_mask(mesh, params, AttributeDomain::Primitive);
            let mesh = scatter_points(mesh, count, seed, mask.as_deref())?;
            return Ok(Geometry::with_mesh(mesh));
        }
    }

    if !input.curves.is_empty() {
        let positions = merged_mesh
            .as_ref()
            .map(|mesh| mesh.positions.as_slice())
            .unwrap_or(&[]);
        let mesh = scatter_curves(positions, &input.curves, count, seed)?;
        return Ok(Geometry::with_mesh(mesh));
    }

    if let Some(volume) = input.volumes.first() {
        let mesh = scatter_volume(volume, count, seed)?;
        return Ok(Geometry::with_mesh(mesh));
    }

    Ok(Geometry::default())
}

fn scatter_points(
    input: &Mesh,
    count: usize,
    seed: u32,
    mask: Option<&[bool]>,
) -> Result<Mesh, String> {
    if count == 0 {
        return Ok(Mesh::default());
    }
    if !input.indices.len().is_multiple_of(3) || input.positions.is_empty() {
        return Err("Scatter requires a triangle mesh input".to_string());
    }

    let mut areas = Vec::new();
    let mut total = 0.0f32;
    for (prim_index, tri) in input.indices.chunks_exact(3).enumerate() {
        if mask
            .as_ref()
            .is_some_and(|mask| !mask.get(prim_index).copied().unwrap_or(false))
        {
            areas.push(total);
            continue;
        }
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;
        if i0 >= input.positions.len() || i1 >= input.positions.len() || i2 >= input.positions.len()
        {
            areas.push(total);
            continue;
        }
        let p0 = Vec3::from(input.positions[i0]);
        let p1 = Vec3::from(input.positions[i1]);
        let p2 = Vec3::from(input.positions[i2]);
        let area = 0.5 * (p1 - p0).cross(p2 - p0).length();
        total += area.max(0.0);
        areas.push(total);
    }

    if total <= 0.0 {
        return if mask.is_some() {
            Ok(Mesh::default())
        } else {
            Err("Scatter requires non-degenerate triangles".to_string())
        };
    }

    let mut rng = XorShift32::new(seed);
    let mut positions = Vec::with_capacity(count);
    let mut normals = Vec::with_capacity(count);

    for _ in 0..count {
        let sample = rng.next_f32() * total;
        let tri_index = find_area_index(&areas, sample);
        let tri = input
            .indices
            .get(tri_index * 3..tri_index * 3 + 3)
            .ok_or_else(|| "scatter triangle index out of range".to_string())?;
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;
        let p0 = Vec3::from(input.positions[i0]);
        let p1 = Vec3::from(input.positions[i1]);
        let p2 = Vec3::from(input.positions[i2]);

        let r1 = rng.next_f32().clamp(0.0, 1.0);
        let r2 = rng.next_f32().clamp(0.0, 1.0);
        let sqrt_r1 = r1.sqrt();
        let u = 1.0 - sqrt_r1;
        let v = r2 * sqrt_r1;
        let w = 1.0 - u - v;
        let point = p0 * u + p1 * v + p2 * w;

        let normal = (p1 - p0).cross(p2 - p0);
        let normal = if normal.length_squared() > 0.0 {
            normal.normalize().to_array()
        } else {
            [0.0, 1.0, 0.0]
        };

        positions.push(point.to_array());
        normals.push(normal);
    }

    Ok(Mesh {
        positions,
        indices: Vec::new(),
        normals: Some(normals),
        corner_normals: None,
        uvs: None,
        attributes: Default::default(),
        groups: Default::default(),
    })
}

fn scatter_curves(
    positions: &[[f32; 3]],
    curves: &[Curve],
    count: usize,
    seed: u32,
) -> Result<Mesh, String> {
    if count == 0 || positions.is_empty() {
        return Ok(Mesh::default());
    }

    let mut segments = Vec::new();
    let mut cumulative = Vec::new();
    let mut total = 0.0f32;
    for curve in curves {
        let indices = &curve.indices;
        if indices.len() < 2 {
            continue;
        }
        let seg_count = if curve.closed { indices.len() } else { indices.len() - 1 };
        for i in 0..seg_count {
            let a = indices.get(i).copied().unwrap_or(0) as usize;
            let b = indices
                .get((i + 1) % indices.len())
                .copied()
                .unwrap_or(0) as usize;
            let Some(p0) = positions.get(a) else { continue };
            let Some(p1) = positions.get(b) else { continue };
            let p0 = Vec3::from(*p0);
            let p1 = Vec3::from(*p1);
            let len = (p1 - p0).length();
            if len <= 0.0 {
                continue;
            }
            total += len;
            segments.push((p0, p1));
            cumulative.push(total);
        }
    }

    if total <= 0.0 || segments.is_empty() {
        return Ok(Mesh::default());
    }

    let mut rng = XorShift32::new(seed);
    let mut out_positions = Vec::with_capacity(count);
    let mut normals = Vec::with_capacity(count);
    for _ in 0..count {
        let sample = rng.next_f32() * total;
        let seg_index = find_area_index(&cumulative, sample).min(segments.len() - 1);
        let (p0, p1) = segments[seg_index];
        let t = rng.next_f32().clamp(0.0, 1.0);
        let point = p0.lerp(p1, t);
        out_positions.push(point.to_array());
        normals.push([0.0, 1.0, 0.0]);
    }

    Ok(Mesh {
        positions: out_positions,
        indices: Vec::new(),
        normals: Some(normals),
        corner_normals: None,
        uvs: None,
        attributes: Default::default(),
        groups: Default::default(),
    })
}

fn scatter_volume(volume: &Volume, count: usize, seed: u32) -> Result<Mesh, String> {
    if count == 0 || volume.values.is_empty() {
        return Ok(Mesh::default());
    }

    let (world_min, world_max) = volume.world_bounds();
    let sampler = VolumeSampler::new(volume);
    let max_density = if matches!(volume.kind, VolumeKind::Density) {
        volume
            .values
            .iter()
            .copied()
            .fold(0.0_f32, f32::max)
            .max(1.0e-6)
    } else {
        1.0
    };

    let mut rng = XorShift32::new(seed);
    let mut out_positions = Vec::with_capacity(count);
    let mut normals = Vec::with_capacity(count);
    let mut attempts = 0usize;
    let max_attempts = (count.max(1) * 50).max(100);
    while out_positions.len() < count && attempts < max_attempts {
        attempts += 1;
        let x = world_min.x + rng.next_f32() * (world_max.x - world_min.x);
        let y = world_min.y + rng.next_f32() * (world_max.y - world_min.y);
        let z = world_min.z + rng.next_f32() * (world_max.z - world_min.z);
        let world_pos = Vec3::new(x, y, z);
        let mut density = sampler.sample_world(world_pos);
        if matches!(volume.kind, VolumeKind::Sdf) {
            density = if density <= 0.0 { 1.0 } else { 0.0 };
        }
        if density <= 0.0 {
            continue;
        }
        let accept = (density / max_density).clamp(0.0, 1.0);
        if rng.next_f32() <= accept {
            out_positions.push(world_pos.to_array());
            normals.push([0.0, 1.0, 0.0]);
        }
    }

    Ok(Mesh {
        positions: out_positions,
        indices: Vec::new(),
        normals: Some(normals),
        corner_normals: None,
        uvs: None,
        attributes: Default::default(),
        groups: Default::default(),
    })
}

fn find_area_index(cumulative: &[f32], sample: f32) -> usize {
    let mut lo = 0usize;
    let mut hi = cumulative.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        if sample < cumulative[mid] {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    lo.min(cumulative.len().saturating_sub(1))
}

struct XorShift32 {
    state: u32,
}

impl XorShift32 {
    fn new(seed: u32) -> Self {
        let seed = if seed == 0 { 0x12345678 } else { seed };
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    fn next_f32(&mut self) -> f32 {
        let value = self.next_u32();
        value as f32 / u32::MAX as f32
    }
}

