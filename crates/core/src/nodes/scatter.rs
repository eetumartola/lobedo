use std::collections::BTreeMap;

use glam::Vec3;

use crate::attributes::AttributeDomain;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{geometry_in, geometry_out, group_utils::mesh_group_mask, require_mesh_input};

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

