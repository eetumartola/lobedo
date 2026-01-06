use std::collections::BTreeMap;

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    attribute_utils::{domain_from_params, existing_vec3_attr_mesh, existing_vec3_attr_splats},
    geometry_in,
    geometry_out,
    group_utils::{mask_has_any, mesh_group_mask, splat_group_mask},
    require_mesh_input,
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
    let domain = domain_from_params(params);
    let count = input.attribute_domain_len(domain);
    if count == 0 && domain != AttributeDomain::Detail {
        return Ok(input);
    }
    let mask = mesh_group_mask(&input, params, domain);
    if !mask_has_any(mask.as_deref()) {
        return Ok(input);
    }
    let mut values = existing_vec3_attr_mesh(&input, domain, "Cd", count);
    apply_color_to_values(&mut values, color, mask.as_deref());
    input
        .set_attribute(domain, "Cd", AttributeStorage::Vec3(values))
        .map_err(|err| format!("Color attribute error: {:?}", err))?;
    Ok(input)
}

pub(crate) fn apply_to_splats(params: &NodeParams, splats: &mut SplatGeo) -> Result<(), String> {
    let color = params.get_vec3("color", [1.0, 1.0, 1.0]);
    let domain = domain_from_params(params);
    let count = splats.attribute_domain_len(domain);
    if count == 0 {
        return Ok(());
    }

    let mask = splat_group_mask(splats, params, domain);
    if !mask_has_any(mask.as_deref()) {
        return Ok(());
    }

    let mut values = existing_vec3_attr_splats(splats, domain, "Cd", count);
    apply_color_to_values(&mut values, color, mask.as_deref());

    splats
        .set_attribute(domain, "Cd", AttributeStorage::Vec3(values))
        .map_err(|err| format!("Color attribute error: {:?}", err))?;
    Ok(())
}

fn apply_color_to_values(
    values: &mut [[f32; 3]],
    color: [f32; 3],
    mask: Option<&[bool]>,
) {
    if let Some(mask) = mask {
        for (idx, value) in values.iter_mut().enumerate() {
            if mask.get(idx).copied().unwrap_or(false) {
                *value = color;
            }
        }
    } else {
        values.iter_mut().for_each(|value| *value = color);
    }
}

