use std::collections::BTreeMap;

use crate::curve::{parse_curve_points, sample_catmull_rom, Curve};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::nodes::geometry_out;

pub const NAME: &str = "Curve";

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
            ("points".to_string(), ParamValue::String(String::new())),
            ("subdivs".to_string(), ParamValue::Int(8)),
            ("closed".to_string(), ParamValue::Bool(false)),
        ]),
    }
}

pub fn compute(params: &NodeParams) -> Result<Curve, String> {
    let points = parse_curve_points(params.get_string("points", ""));
    let closed = params.get_bool("closed", false);
    let subdivs = params.get_int("subdivs", 8).max(1) as usize;
    let sampled = if points.len() > 1 && subdivs > 1 {
        sample_catmull_rom(&points, subdivs, closed)
    } else {
        points
    };
    Ok(Curve::new(sampled, closed))
}
