use std::collections::BTreeMap;

use glam::Vec3;

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    attribute_utils::{
        domain_from_params, existing_float_attr_mesh, existing_float_attr_splats,
        existing_vec2_attr_mesh, existing_vec2_attr_splats, existing_vec3_attr_mesh,
        existing_vec3_attr_splats, mesh_sample_position, splat_sample_position,
    },
    geometry_in,
    geometry_out,
    group_utils::{mask_has_any, mesh_group_mask, splat_group_mask},
    recompute_mesh_normals,
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
    let domain = domain_from_params(params);
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
    if !mask_has_any(mask.as_deref()) {
        return Ok(());
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
    let domain = domain_from_params(params);
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
    if !mask_has_any(mask.as_deref()) {
        return Ok(());
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

    if attr == "P" && domain == AttributeDomain::Point && data_type == 2 {
        recompute_mesh_normals(mesh);
    }
    Ok(())
}
