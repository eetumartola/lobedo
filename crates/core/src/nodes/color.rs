use std::collections::BTreeMap;

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{mesh_in, mesh_out, require_mesh_input};

pub const NAME: &str = "Color";

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
            ("color".to_string(), ParamValue::Vec3([1.0, 1.0, 1.0])),
            ("domain".to_string(), ParamValue::Int(0)),
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
    let values = vec![color; count];
    input
        .set_attribute(domain, "Cd", AttributeStorage::Vec3(values))
        .map_err(|err| format!("Color attribute error: {:?}", err))?;
    Ok(input)
}
