use std::collections::BTreeMap;

use glam::{Quat, Vec3};

use crate::attributes::AttributeDomain;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::parallel;
use crate::nodes::{geometry_in, geometry_out, group_utils::splat_group_mask, require_mesh_input};
use crate::param_spec::ParamSpec;
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

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::float_slider("min_scale", "Min Scale", -10.0, 10.0)
            .with_help("Minimum log-scale to keep."),
        ParamSpec::float_slider("max_scale", "Max Scale", -10.0, 10.0)
            .with_help("Maximum log-scale to keep."),
        ParamSpec::bool("normalize_opacity", "Normalize Opacity")
            .with_help("Renormalize opacity to a stable range."),
        ParamSpec::bool("normalize_rotation", "Normalize Rotation")
            .with_help("Normalize/repair rotations."),
        ParamSpec::bool("remove_invalid", "Remove Invalid")
            .with_help("Drop splats with NaN/Inf."),
        ParamSpec::string("group", "Group")
            .with_help("Optional group to restrict regularize."),
        ParamSpec::int_enum(
            "group_type",
            "Group Type",
            vec![
                (0, "Auto"),
                (1, "Vertex"),
                (2, "Point"),
                (3, "Primitive"),
            ],
        )
        .with_help("Group domain to use."),
    ]
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

    let mask = group_mask.as_deref();
    let mut keep_flags = vec![false; splats.len()];
    parallel::for_each_indexed_mut(&mut keep_flags, |idx, keep| {
        if mask.is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false)) {
            *keep = true;
            return;
        }
        if remove_invalid && !splats.is_finite_at(idx) {
            return;
        }
        *keep = true;
    });

    let kept: Vec<usize> = keep_flags
        .iter()
        .enumerate()
        .filter_map(|(idx, keep)| (*keep).then_some(idx))
        .collect();

    let mut output = splats.filter_by_indices(&kept);
    if output.is_empty() {
        return output;
    }

    #[derive(Clone, Copy)]
    struct RegularizeUpdate {
        opacity: f32,
        rotation: [f32; 4],
        scale: [f32; 3],
    }

    let mut updates: Vec<RegularizeUpdate> = (0..output.len())
        .map(|idx| RegularizeUpdate {
            opacity: output.opacity[idx],
            rotation: output.rotations[idx],
            scale: output.scales[idx],
        })
        .collect();

    parallel::for_each_indexed_mut(&mut updates, |out_idx, update| {
        let src_idx = kept[out_idx];
        let selected = mask
            .map(|mask| mask.get(src_idx).copied().unwrap_or(false))
            .unwrap_or(true);
        if !selected {
            return;
        }

        if normalize_opacity {
            let mut opacity = update.opacity;
            if !opacity.is_finite() {
                opacity = 0.0;
            }
            let linear = sigmoid(opacity).clamp(1.0e-4, 1.0 - 1.0e-4);
            update.opacity = logit(linear);
        }

        if normalize_rotation {
            let rotation = update.rotation;
            let mut quat = Quat::from_xyzw(rotation[1], rotation[2], rotation[3], rotation[0]);
            if quat.length_squared() > 0.0 {
                quat = quat.normalize();
            } else {
                quat = Quat::IDENTITY;
            }
            update.rotation = [quat.w, quat.x, quat.y, quat.z];
        }

        let mut scale = Vec3::from(update.scale);
        if !scale.x.is_finite() || !scale.y.is_finite() || !scale.z.is_finite() {
            scale = Vec3::splat(min_scale);
        }
        scale = Vec3::new(
            scale.x.clamp(min_scale, max_scale),
            scale.y.clamp(min_scale, max_scale),
            scale.z.clamp(min_scale, max_scale),
        );
        update.scale = scale.to_array();
    });

    for (idx, update) in updates.iter().enumerate() {
        output.opacity[idx] = update.opacity;
        output.rotations[idx] = update.rotation;
        output.scales[idx] = update.scale;
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
