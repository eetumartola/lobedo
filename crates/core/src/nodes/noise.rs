use std::collections::BTreeMap;

use glam::Vec3;

use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{geometry_in, geometry_out, require_mesh_input};

pub const NAME: &str = "Noise/Mountain";

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
            ("amplitude".to_string(), ParamValue::Float(0.5)),
            ("frequency".to_string(), ParamValue::Float(1.0)),
            ("seed".to_string(), ParamValue::Int(1)),
            ("offset".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0])),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut input = require_mesh_input(inputs, 0, "Noise/Mountain requires a mesh input")?;
    let amplitude = params.get_float("amplitude", 0.2);
    let frequency = params.get_float("frequency", 1.0).max(0.0);
    let seed = params.get_int("seed", 1) as u32;
    let offset = Vec3::from(params.get_vec3("offset", [0.0, 0.0, 0.0]));

    if input.normals.is_none() {
        let _ = input.compute_normals();
    }
    let normals = input
        .normals
        .clone()
        .ok_or_else(|| "Noise/Mountain requires point normals".to_string())?;

    for (pos, normal) in input.positions.iter_mut().zip(normals.iter()) {
        let p = Vec3::from(*pos) * frequency + offset;
        let n = fractal_noise(p, seed);
        let displacement = Vec3::from(*normal) * (n * amplitude);
        let next = Vec3::from(*pos) + displacement;
        *pos = next.to_array();
    }

    Ok(input)
}

fn fractal_noise(p: Vec3, seed: u32) -> f32 {
    let mut value = 0.0;
    let mut amp = 1.0;
    let mut freq = 1.0;
    for _ in 0..3 {
        value += value_noise(p * freq, seed) * amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    value
}

fn value_noise(p: Vec3, seed: u32) -> f32 {
    let base = p.floor();
    let frac = p - base;
    let f = frac * frac * (Vec3::splat(3.0) - 2.0 * frac);

    let x0 = base.x as i32;
    let y0 = base.y as i32;
    let z0 = base.z as i32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;
    let z1 = z0 + 1;

    let c000 = hash3(x0, y0, z0, seed);
    let c100 = hash3(x1, y0, z0, seed);
    let c010 = hash3(x0, y1, z0, seed);
    let c110 = hash3(x1, y1, z0, seed);
    let c001 = hash3(x0, y0, z1, seed);
    let c101 = hash3(x1, y0, z1, seed);
    let c011 = hash3(x0, y1, z1, seed);
    let c111 = hash3(x1, y1, z1, seed);

    let x00 = lerp(c000, c100, f.x);
    let x10 = lerp(c010, c110, f.x);
    let x01 = lerp(c001, c101, f.x);
    let x11 = lerp(c011, c111, f.x);
    let y0 = lerp(x00, x10, f.y);
    let y1 = lerp(x01, x11, f.y);
    lerp(y0, y1, f.z) * 2.0 - 1.0
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn hash3(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut h = x as u32;
    h ^= (y as u32).wrapping_mul(374761393);
    h = h.rotate_left(13);
    h ^= (z as u32).wrapping_mul(668265263);
    h = h.rotate_left(17);
    h ^= seed.wrapping_mul(2246822519);
    h = h.wrapping_mul(3266489917);
    h = (h ^ (h >> 16)).wrapping_mul(2246822519);
    (h as f32) / (u32::MAX as f32)
}

