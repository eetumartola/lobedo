use std::collections::BTreeMap;

use crate::attributes::AttributeDomain;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    geometry_in,
    geometry_out,
    group_utils::{mesh_group_mask, splat_group_mask},
    require_mesh_input,
};
use crate::splat::SplatGeo;
use crate::wrangle::{apply_wrangle, apply_wrangle_splats};

pub const NAME: &str = "Wrangle";

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
            ("mode".to_string(), ParamValue::Int(0)),
            (
                "code".to_string(),
                ParamValue::String("@Cd = vec3(1.0, 1.0, 1.0);".to_string()),
            ),
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut input = require_mesh_input(inputs, 0, "Wrangle requires a mesh input")?;
    let code = params.get_string("code", "");
    let domain = match params.get_int("mode", 0).clamp(0, 3) {
        0 => AttributeDomain::Point,
        1 => AttributeDomain::Vertex,
        2 => AttributeDomain::Primitive,
        _ => AttributeDomain::Detail,
    };
    if !code.trim().is_empty() {
        let mask = mesh_group_mask(&input, params, domain);
        apply_wrangle(&mut input, domain, code, mask.as_deref())?;
    }
    Ok(input)
}

pub(crate) fn apply_to_splats(
    params: &NodeParams,
    splats: &mut SplatGeo,
) -> Result<(), String> {
    let code = params.get_string("code", "");
    let domain = match params.get_int("mode", 0).clamp(0, 3) {
        0 => AttributeDomain::Point,
        1 => AttributeDomain::Vertex,
        2 => AttributeDomain::Primitive,
        _ => AttributeDomain::Detail,
    };
    if !code.trim().is_empty() {
        let mask = splat_group_mask(splats, params, domain);
        apply_wrangle_splats(splats, domain, code, mask.as_deref())?;
    }
    Ok(())
}

