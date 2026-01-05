use std::collections::BTreeMap;

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    geometry_in, geometry_out, group_utils::{mesh_group_mask, splat_group_mask}, require_mesh_input,
};
use crate::splat::SplatGeo;

pub const NAME: &str = "Color";

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
            ("color".to_string(), ParamValue::Vec3([1.0, 1.0, 1.0])),
            ("domain".to_string(), ParamValue::Int(0)),
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut input = require_mesh_input(inputs, 0, "Color requires a mesh input")?;
    let color = params.get_vec3("color", [1.0, 1.0, 1.0]);
    let domain = match params.get_int("domain", 0).clamp(0, 3) {
        0 => AttributeDomain::Point,
        1 => AttributeDomain::Vertex,
        2 => AttributeDomain::Primitive,
        _ => AttributeDomain::Detail,
    };
    let count = input.attribute_domain_len(domain);
    let mask = mesh_group_mask(&input, params, domain);
    if let Some(mask) = &mask {
        if !mask.iter().any(|value| *value) {
            return Ok(input);
        }
    }
    let mut values = if let Some(existing) = input
        .attribute(domain, "Cd")
        .and_then(|attr| match attr {
            crate::attributes::AttributeRef::Vec3(values) if values.len() == count => {
                Some(values.to_vec())
            }
            _ => None,
        })
    {
        existing
    } else {
        vec![[0.0, 0.0, 0.0]; count]
    };

    if let Some(mask) = mask {
        for (idx, value) in values.iter_mut().enumerate() {
            if mask.get(idx).copied().unwrap_or(false) {
                *value = color;
            }
        }
    } else {
        values.iter_mut().for_each(|value| *value = color);
    }
    input
        .set_attribute(domain, "Cd", AttributeStorage::Vec3(values))
        .map_err(|err| format!("Color attribute error: {:?}", err))?;
    Ok(input)
}

pub(crate) fn apply_to_splats(params: &NodeParams, splats: &mut SplatGeo) -> Result<(), String> {
    let color = params.get_vec3("color", [1.0, 1.0, 1.0]);
    let domain = match params.get_int("domain", 0).clamp(0, 3) {
        0 => AttributeDomain::Point,
        1 => AttributeDomain::Vertex,
        2 => AttributeDomain::Primitive,
        _ => AttributeDomain::Detail,
    };
    let count = splats.attribute_domain_len(domain);
    if count == 0 {
        return Ok(());
    }

    let mask = splat_group_mask(splats, params, domain);
    if let Some(mask) = &mask {
        if !mask.iter().any(|value| *value) {
            return Ok(());
        }
    }

    let mut values = if let Some(existing) = splats
        .attribute(domain, "Cd")
        .and_then(|attr| match attr {
            crate::attributes::AttributeRef::Vec3(values) if values.len() == count => {
                Some(values.to_vec())
            }
            _ => None,
        }) {
        existing
    } else {
        vec![[0.0, 0.0, 0.0]; count]
    };

    if let Some(mask) = mask {
        for (idx, value) in values.iter_mut().enumerate() {
            if mask.get(idx).copied().unwrap_or(false) {
                *value = color;
            }
        }
    } else {
        values.iter_mut().for_each(|value| *value = color);
    }

    splats
        .set_attribute(domain, "Cd", AttributeStorage::Vec3(values))
        .map_err(|err| format!("Color attribute error: {:?}", err))?;
    Ok(())
}

