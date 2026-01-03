use std::collections::BTreeMap;

use crate::attributes::AttributeDomain;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{mesh_in, mesh_out, require_mesh_input};
use crate::wrangle::apply_wrangle;

pub const NAME: &str = "Wrangle";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![mesh_in("in")],
        outputs: vec![mesh_out("out")],
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
        apply_wrangle(&mut input, domain, code)?;
    }
    Ok(input)
}
