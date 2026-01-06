use std::collections::BTreeMap;

use glam::{Quat, Vec3};

use crate::attributes::AttributeDomain;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{geometry_in, geometry_out, group_utils::splat_group_mask, require_mesh_input};
use crate::splat::SplatGeo;

pub const NAME: &str = "Splat Regularize";
pub const LEGACY_NAME: &str = "Regularize";

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
            ("min_scale".to_string(), ParamValue::Float(-10.0)),
            ("max_scale".to_string(), ParamValue::Float(10.0)),
            ("normalize_opacity".to_string(), ParamValue::Bool(true)),
            ("normalize_rotation".to_string(), ParamValue::Bool(true)),
            ("remove_invalid".to_string(), ParamValue::Bool(true)),
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn compute(_params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let input = require_mesh_input(inputs, 0, "Splat Regularize requires a mesh input")?;
    Ok(input)
}

pub fn apply_to_splats(params: &NodeParams, splats: &SplatGeo) -> SplatGeo {
    let group_mask = splat_group_mask(splats, params, AttributeDomain::Point);
    let mut min_scale = params.get_float("min_scale", -10.0);
    if !min_scale.is_finite() {
        min_scale = -10.0;
    }
    let mut max_scale = params.get_float("max_scale", 10.0);
    if !max_scale.is_finite() {
        max_scale = 10.0;
    }
    max_scale = max_scale.max(min_scale);
    let normalize_opacity = params.get_bool("normalize_opacity", true);
    let normalize_rotation = params.get_bool("normalize_rotation", true);
    let remove_invalid = params.get_bool("remove_invalid", true);

    let mut kept = Vec::with_capacity(splats.len());
    for idx in 0..splats.len() {
        if let Some(mask) = &group_mask {
            if !mask.get(idx).copied().unwrap_or(false) {
                kept.push(idx);
                continue;
            }
        }
        if remove_invalid && !splats.is_finite_at(idx) {
            continue;
        }
        kept.push(idx);
    }

    let mut output = splats.filter_by_indices(&kept);
    for (out_idx, src_idx) in kept.iter().copied().enumerate() {
        let selected = group_mask
            .as_ref()
            .map(|mask| mask.get(src_idx).copied().unwrap_or(false))
            .unwrap_or(true);
        if !selected {
            continue;
        }

        if normalize_opacity {
            let mut opacity = output.opacity[out_idx];
            if !opacity.is_finite() {
                opacity = 0.0;
            }
            let linear = sigmoid(opacity).clamp(1.0e-4, 1.0 - 1.0e-4);
            output.opacity[out_idx] = logit(linear);
        }

        if normalize_rotation {
            let rotation = output.rotations[out_idx];
            let mut quat = Quat::from_xyzw(rotation[1], rotation[2], rotation[3], rotation[0]);
            if quat.length_squared() > 0.0 {
                quat = quat.normalize();
            } else {
                quat = Quat::IDENTITY;
            }
            output.rotations[out_idx] = [quat.w, quat.x, quat.y, quat.z];
        }

        let mut scale = Vec3::from(output.scales[out_idx]);
        if !scale.x.is_finite() || !scale.y.is_finite() || !scale.z.is_finite() {
            scale = Vec3::splat(min_scale);
        }
        scale = Vec3::new(
            scale.x.clamp(min_scale, max_scale),
            scale.y.clamp(min_scale, max_scale),
            scale.z.clamp(min_scale, max_scale),
        );
        output.scales[out_idx] = scale.to_array();
    }

    output
}

fn sigmoid(value: f32) -> f32 {
    1.0 / (1.0 + (-value).exp())
}

fn logit(value: f32) -> f32 {
    let clamped = value.clamp(1.0e-4, 1.0 - 1.0e-4);
    (clamped / (1.0 - clamped)).ln()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::graph::{NodeParams, ParamValue};
    use crate::splat::SplatGeo;

    use super::apply_to_splats;

    #[test]
    fn regularize_clamps_log_scale() {
        let mut splats = SplatGeo::with_len(1);
        splats.rotations[0] = [1.0, 0.0, 0.0, 0.0];
        splats.scales[0] = [-2.0; 3];
        splats.opacity[0] = 0.5;

        let params = NodeParams {
            values: BTreeMap::from([
                ("min_scale".to_string(), ParamValue::Float(0.5)),
                ("max_scale".to_string(), ParamValue::Float(1.0)),
                ("normalize_opacity".to_string(), ParamValue::Bool(false)),
                ("normalize_rotation".to_string(), ParamValue::Bool(false)),
                ("remove_invalid".to_string(), ParamValue::Bool(true)),
            ]),
        };

        let regularized = apply_to_splats(&params, &splats);
        assert!((regularized.scales[0][0] - 0.5).abs() < 1.0e-5);
    }

    #[test]
    fn regularize_normalizes_logit_opacity() {
        let mut splats = SplatGeo::with_len(1);
        splats.rotations[0] = [1.0, 0.0, 0.0, 0.0];
        splats.scales[0] = [1.0, 1.0, 1.0];
        splats.opacity[0] = 2.0;

        let params = NodeParams {
            values: BTreeMap::from([
                ("normalize_opacity".to_string(), ParamValue::Bool(true)),
                ("normalize_rotation".to_string(), ParamValue::Bool(false)),
                ("remove_invalid".to_string(), ParamValue::Bool(true)),
            ]),
        };

        let regularized = apply_to_splats(&params, &splats);
        let expected = super::logit(1.0 / (1.0 + (-2.0f32).exp()));
        assert!((regularized.opacity[0] - expected).abs() < 1.0e-5);
    }
}
