use std::collections::BTreeMap;

use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::nodes::{geometry_in, geometry_out};
use crate::splat::SplatGeo;

pub const NAME: &str = "Write Splats";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Outputs".to_string(),
        inputs: vec![geometry_in("in")],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([(
            "path".to_string(),
            ParamValue::String("output.ply".to_string()),
        )]),
    }
}

pub fn compute(params: &NodeParams, splats: &SplatGeo) -> Result<(), String> {
    let path = params.get_string("path", "output.ply");
    if path.trim().is_empty() {
        return Err("Write Splats requires a path".to_string());
    }
    crate::splat::save_splat_ply(path, splats)
}
