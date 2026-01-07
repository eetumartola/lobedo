use std::collections::BTreeMap;

use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::nodes::{geometry_in, geometry_out};
use crate::splat::SplatGeo;

pub const NAME: &str = "Splat Write";
pub const LEGACY_NAME: &str = "Write Splats";

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
        values: BTreeMap::from([
            (
                "path".to_string(),
                ParamValue::String("output.ply".to_string()),
            ),
            ("format".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn compute(params: &NodeParams, splats: &SplatGeo) -> Result<(), String> {
    let _ = params;
    let _ = splats;
    Ok(())
}
