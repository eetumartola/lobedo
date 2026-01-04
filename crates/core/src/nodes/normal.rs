use std::collections::BTreeMap;

use crate::attributes::AttributeDomain;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{geometry_in, geometry_out, group_utils::mesh_group_mask, require_mesh_input};

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
        values: BTreeMap::from([
            ("threshold_deg".to_string(), ParamValue::Float(60.0)),
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut input = require_mesh_input(inputs, 0, "Normal requires a mesh input")?;
    let mask = mesh_group_mask(&input, params, AttributeDomain::Point);
    let threshold = params.get_float("threshold_deg", 60.0).clamp(0.0, 180.0);
    if mask.is_none() {
        if !input.compute_normals_with_threshold(threshold) {
            return Err("Normal node requires triangle mesh input".to_string());
        }
        return Ok(input);
    }

    let mut computed = input.clone();
    if !computed.compute_normals_with_threshold(threshold) {
        return Err("Normal node requires triangle mesh input".to_string());
    }
    let mask = mask.unwrap_or_default();

    let point_len = input.positions.len();
    let mut point_normals = input
        .normals
        .take()
        .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; point_len]);
    if let Some(next) = computed.normals.take() {
        for (idx, normal) in next.iter().enumerate() {
            if mask.get(idx).copied().unwrap_or(false) {
                if let Some(slot) = point_normals.get_mut(idx) {
                    *slot = *normal;
                }
            }
        }
    }
    input.normals = Some(point_normals);

    if let Some(next_corner) = computed.corner_normals.take() {
        let mut corner_normals = input
            .corner_normals
            .take()
            .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; next_corner.len()]);
        for (idx, normal) in next_corner.iter().enumerate() {
            let point = input.indices.get(idx).copied().unwrap_or(0) as usize;
            if mask.get(point).copied().unwrap_or(false) {
                if let Some(slot) = corner_normals.get_mut(idx) {
                    *slot = *normal;
                }
            }
        }
        input.corner_normals = Some(corner_normals);
    }
    Ok(input)
}

