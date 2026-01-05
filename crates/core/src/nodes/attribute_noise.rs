use std::collections::BTreeMap;

use glam::Vec3;

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    geometry_in,
    geometry_out,
    group_utils::{mesh_group_mask, splat_group_mask},
    require_mesh_input,
};
use crate::noise::{fbm_noise, NoiseType};
use crate::splat::SplatGeo;

pub const NAME: &str = "Attribute Noise";

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
            ("data_type".to_string(), ParamValue::Int(2)),
            ("noise_type".to_string(), ParamValue::Int(0)),
            ("amplitude".to_string(), ParamValue::Float(0.5)),
            ("frequency".to_string(), ParamValue::Float(1.0)),
            ("offset".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0])),
            ("seed".to_string(), ParamValue::Int(1)),
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut input = require_mesh_input(inputs, 0, "Attribute Noise requires a mesh input")?;
    apply_to_mesh(params, &mut input)?;
    Ok(input)
}

pub(crate) fn apply_to_splats(params: &NodeParams, splats: &mut SplatGeo) -> Result<(), String> {
    let attr = params.get_string("attr", "P");
    let domain = match params.get_int("domain", 0).clamp(0, 3) {
        0 => AttributeDomain::Point,
        1 => AttributeDomain::Vertex,
        2 => AttributeDomain::Primitive,
        _ => AttributeDomain::Detail,
    };
    let data_type = params.get_int("data_type", 2).clamp(0, 2);
    let noise_type = NoiseType::from_int(params.get_int("noise_type", 0));
    let amplitude = params.get_float("amplitude", 0.5);
    let frequency = params.get_float("frequency", 1.0);
    let offset = Vec3::from(params.get_vec3("offset", [0.0, 0.0, 0.0]));
    let seed = params.get_int("seed", 1) as u32;

    let count = splats.attribute_domain_len(domain);
    if count == 0 && domain != AttributeDomain::Detail {
        return Ok(());
    }

    let mask = splat_group_mask(splats, params, domain);
    if let Some(mask) = &mask {
        if !mask.iter().any(|value| *value) {
            return Ok(());
        }
    }

    match data_type {
        0 => {
            let mut values = existing_float_attr_splats(splats, domain, attr, count);
            for (idx, value) in values.iter_mut().enumerate() {
                if mask
                    .as_ref()
                    .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
                {
                    continue;
                }
                let p = splat_sample_position(splats, domain, idx) * frequency + offset;
                let n = fbm_noise(p, seed, noise_type, 3, 2.0, 0.5);
                *value += n * amplitude;
            }
            splats
                .set_attribute(domain, attr, AttributeStorage::Float(values))
                .map_err(|err| format!("Attribute Noise error: {:?}", err))?;
        }
        1 => {
            let mut values = existing_vec2_attr_splats(splats, domain, attr, count);
            let offsets = [Vec3::new(12.7, 45.3, 19.1), Vec3::new(31.9, 7.2, 58.4)];
            for (idx, value) in values.iter_mut().enumerate() {
                if mask
                    .as_ref()
                    .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
                {
                    continue;
                }
                let p = splat_sample_position(splats, domain, idx) * frequency + offset;
                let n0 = fbm_noise(p + offsets[0], seed, noise_type, 3, 2.0, 0.5);
                let n1 = fbm_noise(
                    p + offsets[1],
                    seed.wrapping_add(7),
                    noise_type,
                    3,
                    2.0,
                    0.5,
                );
                value[0] += n0 * amplitude;
                value[1] += n1 * amplitude;
            }
            splats
                .set_attribute(domain, attr, AttributeStorage::Vec2(values))
                .map_err(|err| format!("Attribute Noise error: {:?}", err))?;
        }
        _ => {
            let mut values = existing_vec3_attr_splats(splats, domain, attr, count);
            let offsets = [
                Vec3::new(12.7, 45.3, 19.1),
                Vec3::new(31.9, 7.2, 58.4),
                Vec3::new(23.1, 91.7, 3.7),
            ];
            for (idx, value) in values.iter_mut().enumerate() {
                if mask
                    .as_ref()
                    .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
                {
                    continue;
                }
                let p = splat_sample_position(splats, domain, idx) * frequency + offset;
                let n0 = fbm_noise(p + offsets[0], seed, noise_type, 3, 2.0, 0.5);
                let n1 = fbm_noise(
                    p + offsets[1],
                    seed.wrapping_add(7),
                    noise_type,
                    3,
                    2.0,
                    0.5,
                );
                let n2 = fbm_noise(
                    p + offsets[2],
                    seed.wrapping_add(13),
                    noise_type,
                    3,
                    2.0,
                    0.5,
                );
                value[0] += n0 * amplitude;
                value[1] += n1 * amplitude;
                value[2] += n2 * amplitude;
            }
            splats
                .set_attribute(domain, attr, AttributeStorage::Vec3(values))
                .map_err(|err| format!("Attribute Noise error: {:?}", err))?;
        }
    }

    Ok(())
}

fn apply_to_mesh(params: &NodeParams, mesh: &mut Mesh) -> Result<(), String> {
    let attr = params.get_string("attr", "P");
    let domain = match params.get_int("domain", 0).clamp(0, 3) {
        0 => AttributeDomain::Point,
        1 => AttributeDomain::Vertex,
        2 => AttributeDomain::Primitive,
        _ => AttributeDomain::Detail,
    };
    let data_type = params.get_int("data_type", 2).clamp(0, 2);
    let noise_type = NoiseType::from_int(params.get_int("noise_type", 0));
    let amplitude = params.get_float("amplitude", 0.5);
    let frequency = params.get_float("frequency", 1.0);
    let offset = Vec3::from(params.get_vec3("offset", [0.0, 0.0, 0.0]));
    let seed = params.get_int("seed", 1) as u32;

    let count = mesh.attribute_domain_len(domain);
    if count == 0 && domain != AttributeDomain::Detail {
        return Ok(());
    }

    let mask = mesh_group_mask(mesh, params, domain);
    if let Some(mask) = &mask {
        if !mask.iter().any(|value| *value) {
            return Ok(());
        }
    }

    match data_type {
        0 => {
            let mut values = existing_float_attr_mesh(mesh, domain, attr, count);
            for (idx, value) in values.iter_mut().enumerate() {
                if mask
                    .as_ref()
                    .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
                {
                    continue;
                }
                let p = mesh_sample_position(mesh, domain, idx) * frequency + offset;
                let n = fbm_noise(p, seed, noise_type, 3, 2.0, 0.5);
                *value += n * amplitude;
            }
            mesh.set_attribute(domain, attr, AttributeStorage::Float(values))
                .map_err(|err| format!("Attribute Noise error: {:?}", err))?;
        }
        1 => {
            let mut values = existing_vec2_attr_mesh(mesh, domain, attr, count);
            let offsets = [Vec3::new(12.7, 45.3, 19.1), Vec3::new(31.9, 7.2, 58.4)];
            for (idx, value) in values.iter_mut().enumerate() {
                if mask
                    .as_ref()
                    .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
                {
                    continue;
                }
                let p = mesh_sample_position(mesh, domain, idx) * frequency + offset;
                let n0 = fbm_noise(p + offsets[0], seed, noise_type, 3, 2.0, 0.5);
                let n1 = fbm_noise(
                    p + offsets[1],
                    seed.wrapping_add(7),
                    noise_type,
                    3,
                    2.0,
                    0.5,
                );
                value[0] += n0 * amplitude;
                value[1] += n1 * amplitude;
            }
            mesh.set_attribute(domain, attr, AttributeStorage::Vec2(values))
                .map_err(|err| format!("Attribute Noise error: {:?}", err))?;
        }
        _ => {
            let mut values = existing_vec3_attr_mesh(mesh, domain, attr, count);
            let offsets = [
                Vec3::new(12.7, 45.3, 19.1),
                Vec3::new(31.9, 7.2, 58.4),
                Vec3::new(23.1, 91.7, 3.7),
            ];
            for (idx, value) in values.iter_mut().enumerate() {
                if mask
                    .as_ref()
                    .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
                {
                    continue;
                }
                let p = mesh_sample_position(mesh, domain, idx) * frequency + offset;
                let n0 = fbm_noise(p + offsets[0], seed, noise_type, 3, 2.0, 0.5);
                let n1 = fbm_noise(
                    p + offsets[1],
                    seed.wrapping_add(7),
                    noise_type,
                    3,
                    2.0,
                    0.5,
                );
                let n2 = fbm_noise(
                    p + offsets[2],
                    seed.wrapping_add(13),
                    noise_type,
                    3,
                    2.0,
                    0.5,
                );
                value[0] += n0 * amplitude;
                value[1] += n1 * amplitude;
                value[2] += n2 * amplitude;
            }
            mesh.set_attribute(domain, attr, AttributeStorage::Vec3(values))
                .map_err(|err| format!("Attribute Noise error: {:?}", err))?;
        }
    }

    Ok(())
}

fn existing_float_attr_mesh(
    mesh: &Mesh,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<f32> {
    if let Some(crate::attributes::AttributeRef::Float(values)) = mesh.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![0.0; count.max(1)]
}

fn existing_vec2_attr_mesh(
    mesh: &Mesh,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<[f32; 2]> {
    if let Some(crate::attributes::AttributeRef::Vec2(values)) = mesh.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![[0.0, 0.0]; count.max(1)]
}

fn existing_vec3_attr_mesh(
    mesh: &Mesh,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<[f32; 3]> {
    if let Some(crate::attributes::AttributeRef::Vec3(values)) = mesh.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![[0.0, 0.0, 0.0]; count.max(1)]
}

fn existing_float_attr_splats(
    splats: &SplatGeo,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<f32> {
    if let Some(crate::attributes::AttributeRef::Float(values)) = splats.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![0.0; count.max(1)]
}

fn existing_vec2_attr_splats(
    splats: &SplatGeo,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<[f32; 2]> {
    if let Some(crate::attributes::AttributeRef::Vec2(values)) = splats.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![[0.0, 0.0]; count.max(1)]
}

fn existing_vec3_attr_splats(
    splats: &SplatGeo,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<[f32; 3]> {
    if let Some(crate::attributes::AttributeRef::Vec3(values)) = splats.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![[0.0, 0.0, 0.0]; count.max(1)]
}

fn mesh_sample_position(mesh: &Mesh, domain: AttributeDomain, index: usize) -> Vec3 {
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

fn splat_sample_position(splats: &SplatGeo, domain: AttributeDomain, index: usize) -> Vec3 {
    match domain {
        AttributeDomain::Point | AttributeDomain::Primitive => splats
            .positions
            .get(index)
            .copied()
            .map(Vec3::from)
            .unwrap_or(Vec3::ZERO),
        AttributeDomain::Detail => {
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
        AttributeDomain::Vertex => Vec3::ZERO,
    }
}
