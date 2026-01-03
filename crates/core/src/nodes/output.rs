use crate::graph::{NodeDefinition, NodeParams};
use crate::mesh::Mesh;
use crate::nodes::{mesh_in, require_mesh_input};

pub const NAME: &str = "Output";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Outputs".to_string(),
        inputs: vec![mesh_in("in")],
        outputs: Vec::new(),
    }
}

pub fn default_params() -> NodeParams {
    NodeParams::default()
}

pub fn compute(_params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let input = require_mesh_input(inputs, 0, "Output requires a mesh input")?;
    Ok(input)
}
