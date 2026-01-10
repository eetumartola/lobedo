use std::collections::BTreeMap;

use crate::attributes::AttributeDomain;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::parallel;
use crate::nodes::{geometry_in, geometry_out, group_utils::splat_group_mask, require_mesh_input};
use crate::splat::SplatGeo;

pub const NAME: &str = "Splat Prune";
pub const LEGACY_NAME: &str = "Prune";

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
            ("min_opacity".to_string(), ParamValue::Float(-9.21034)),
            ("max_opacity".to_string(), ParamValue::Float(9.21034)),
            ("min_scale".to_string(), ParamValue::Float(-10.0)),
            ("max_scale".to_string(), ParamValue::Float(10.0)),
            ("remove_invalid".to_string(), ParamValue::Bool(true)),
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn compute(_params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let input = require_mesh_input(inputs, 0, "Splat Prune requires a mesh input")?;
    Ok(input)
}

pub fn apply_to_splats(params: &NodeParams, splats: &SplatGeo) -> SplatGeo {
    let group_mask = splat_group_mask(splats, params, AttributeDomain::Point);
    let mut min_opacity = params.get_float("min_opacity", -9.21034);
    if !min_opacity.is_finite() {
        min_opacity = -9.21034;
    }
    let mut max_opacity = params.get_float("max_opacity", 9.21034);
    if !max_opacity.is_finite() {
        max_opacity = 9.21034;
    }
    max_opacity = max_opacity.max(min_opacity);
    let mut min_scale = params.get_float("min_scale", -10.0);
    if !min_scale.is_finite() {
        min_scale = -10.0;
    }
    let mut max_scale = params.get_float("max_scale", 10.0);
    if !max_scale.is_finite() {
        max_scale = 10.0;
    }
    max_scale = max_scale.max(min_scale);
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

        let opacity = splats.opacity[idx];
        if opacity < min_opacity || opacity > max_opacity {
            return;
        }

        let scale = splats.scales[idx];
        let min_component = scale[0].min(scale[1]).min(scale[2]);
        let max_component = scale[0].max(scale[1]).max(scale[2]);
        if min_component < min_scale || max_component > max_scale {
            return;
        }

        *keep = true;
    });

    let kept: Vec<usize> = keep_flags
        .iter()
        .enumerate()
        .filter_map(|(idx, keep)| (*keep).then_some(idx))
        .collect();

    splats.filter_by_indices(&kept)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::graph::{NodeParams, ParamValue};
    use crate::splat::SplatGeo;

    use super::apply_to_splats;

    #[test]
    fn prune_respects_log_scale_thresholds() {
        let mut splats = SplatGeo::with_len(3);
        splats.rotations.fill([1.0, 0.0, 0.0, 0.0]);
        splats.opacity.fill(0.5);
        splats.scales[0] = [0.05_f32.ln(); 3];
        splats.scales[1] = [0.4_f32.ln(); 3];
        splats.scales[2] = [2.0_f32.ln(); 3];

        let params = NodeParams {
            values: BTreeMap::from([
                ("min_scale".to_string(), ParamValue::Float(0.2_f32.ln())),
                ("max_scale".to_string(), ParamValue::Float(0.6_f32.ln())),
                ("min_opacity".to_string(), ParamValue::Float(-9.21034)),
                ("max_opacity".to_string(), ParamValue::Float(9.21034)),
                ("remove_invalid".to_string(), ParamValue::Bool(true)),
            ]),
        };

        let pruned = apply_to_splats(&params, &splats);
        assert_eq!(pruned.len(), 1);
        assert!((pruned.scales[0][0] - 0.4_f32.ln()).abs() < 1.0e-5);
    }

    #[test]
    fn prune_filters_logit_opacity() {
        let mut splats = SplatGeo::with_len(3);
        splats.rotations.fill([1.0, 0.0, 0.0, 0.0]);
        splats.scales.fill([0.1_f32.ln(), 0.1_f32.ln(), 0.1_f32.ln()]);
        splats.opacity[0] = -2.0;
        splats.opacity[1] = 0.0;
        splats.opacity[2] = 2.0;

        let params = NodeParams {
            values: BTreeMap::from([
                ("min_opacity".to_string(), ParamValue::Float(1.0)),
                ("max_opacity".to_string(), ParamValue::Float(9.21034)),
                ("remove_invalid".to_string(), ParamValue::Bool(true)),
            ]),
        };

        let pruned = apply_to_splats(&params, &splats);
        assert_eq!(pruned.len(), 1);
        assert_eq!(pruned.opacity[0], 2.0);
    }
}
