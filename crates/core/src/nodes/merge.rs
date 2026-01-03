use crate::graph::{NodeDefinition, NodeParams};
use crate::mesh::Mesh;
use crate::nodes::{mesh_in, mesh_out};

pub const NAME: &str = "Merge";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![mesh_in("a"), mesh_in("b")],
        outputs: vec![mesh_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams::default()
}

pub fn compute(_params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    if inputs.is_empty() {
        return Err("Merge requires at least one mesh input".to_string());
    }
    Ok(Mesh::merge(inputs))
}
