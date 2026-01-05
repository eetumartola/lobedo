use std::collections::BTreeMap;

use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::nodes::geometry_out;
use crate::splat::{load_splat_ply_with_mode, SplatGeo, SplatLoadMode};

pub const NAME: &str = "Splat Read";
pub const LEGACY_NAME: &str = "Read Splats";

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
        values: BTreeMap::from([
            (
                "path".to_string(),
                ParamValue::String("C:\\code\\lobedo\\geo\\CL.ply".to_string()),
            ),
            ("read_mode".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn compute(params: &NodeParams) -> Result<SplatGeo, String> {
    let path = params.get_string("path", "");
    if path.trim().is_empty() {
        return Err("Splat Read requires a path".to_string());
    }
    let mode = if params.get_int("read_mode", 0) == 1 {
        SplatLoadMode::ColorOnly
    } else {
        SplatLoadMode::Full
    };
    load_splat_ply_with_mode(path, mode)
}

