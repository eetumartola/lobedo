use std::collections::BTreeMap;

use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{geometry_in, geometry_out, require_mesh_input};

pub const NAME: &str = "Normal";

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
        values: BTreeMap::from([("threshold_deg".to_string(), ParamValue::Float(60.0))]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut input = require_mesh_input(inputs, 0, "Normal requires a mesh input")?;
    let threshold = params.get_float("threshold_deg", 60.0).clamp(0.0, 180.0);
    if !input.compute_normals_with_threshold(threshold) {
        return Err("Normal node requires triangle mesh input".to_string());
    }
    Ok(input)
}

