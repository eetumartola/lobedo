use std::collections::BTreeMap;

use glam::Vec3;

use crate::attributes::AttributeDomain;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    geometry_in,
    geometry_out,
    group_utils::{mask_has_any, mesh_group_mask, splat_group_mask},
    recompute_mesh_normals,
    require_mesh_input,
};
use crate::noise::{fbm_noise, NoiseType};
use crate::splat::SplatGeo;

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
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut input = require_mesh_input(inputs, 0, "Noise/Mountain requires a mesh input")?;
    let amplitude = params.get_float("amplitude", 0.2);
    let frequency = params.get_float("frequency", 1.0).max(0.0);
    let seed = params.get_int("seed", 1) as u32;
    let offset = Vec3::from(params.get_vec3("offset", [0.0, 0.0, 0.0]));
    let mask = mesh_group_mask(&input, params, AttributeDomain::Point);
    if !mask_has_any(mask.as_deref()) {
        return Ok(input);
    }

    if input.normals.is_none() {
        let _ = input.compute_normals();
    }
    let normals = input
        .normals
        .clone()
        .ok_or_else(|| "Noise/Mountain requires point normals".to_string())?;

    for (idx, (pos, normal)) in input.positions.iter_mut().zip(normals.iter()).enumerate() {
        if mask
            .as_ref()
            .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
        {
            continue;
        }
        let p = Vec3::from(*pos) * frequency + offset;
        let n = fbm_noise(p, seed, NoiseType::Fast, 3, 2.0, 0.5);
        let displacement = Vec3::from(*normal) * (n * amplitude);
        let next = Vec3::from(*pos) + displacement;
        *pos = next.to_array();
    }

    recompute_mesh_normals(&mut input);
    Ok(input)
}

pub(crate) fn apply_to_splats(params: &NodeParams, splats: &mut SplatGeo) -> Result<(), String> {
    if splats.positions.is_empty() {
        return Ok(());
    }
    let amplitude = params.get_float("amplitude", 0.2);
    let frequency = params.get_float("frequency", 1.0).max(0.0);
    let seed = params.get_int("seed", 1) as u32;
    let offset = Vec3::from(params.get_vec3("offset", [0.0, 0.0, 0.0]));
    let mask = splat_group_mask(splats, params, AttributeDomain::Point);
    if !mask_has_any(mask.as_deref()) {
        return Ok(());
    }

    let normals = splats
        .attribute(AttributeDomain::Point, "N")
        .and_then(|attr| match attr {
            crate::attributes::AttributeRef::Vec3(values)
                if values.len() == splats.positions.len() =>
            {
                Some(values.to_vec())
            }
            _ => None,
        })
        .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; splats.positions.len()]);

    for (idx, (pos, normal)) in splats
        .positions
        .iter_mut()
        .zip(normals.iter())
        .enumerate()
    {
        if mask
            .as_ref()
            .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
        {
            continue;
        }
        let p = Vec3::from(*pos) * frequency + offset;
        let n = fbm_noise(p, seed, NoiseType::Fast, 3, 2.0, 0.5);
        let displacement = Vec3::from(*normal) * (n * amplitude);
        let next = Vec3::from(*pos) + displacement;
        *pos = next.to_array();
    }

    Ok(())
}
