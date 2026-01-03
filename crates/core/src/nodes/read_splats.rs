use std::collections::BTreeMap;

use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::nodes::geometry_out;
use crate::splat::{load_splat_ply, SplatGeo};

pub const NAME: &str = "Read Splats";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Sources".to_string(),
        inputs: Vec::new(),
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([(
            "path".to_string(),
            ParamValue::String("C:\\code\\lobedo\\geo\\CL.ply".to_string()),
        )]),
    }
}

pub fn compute(params: &NodeParams) -> Result<SplatGeo, String> {
    let path = params.get_string("path", "");
    if path.trim().is_empty() {
        return Err("Read Splats requires a path".to_string());
    }
    load_splat_ply(path)
}

