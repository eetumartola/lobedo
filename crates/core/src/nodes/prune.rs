use std::collections::BTreeMap;

use glam::Vec3;

use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
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
            ("min_opacity".to_string(), ParamValue::Float(0.0)),
            ("max_opacity".to_string(), ParamValue::Float(1.0)),
            ("min_scale".to_string(), ParamValue::Float(0.0)),
            ("max_scale".to_string(), ParamValue::Float(1000.0)),
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
    let group_mask = splat_group_mask(splats, params);
    let mut min_opacity = params.get_float("min_opacity", 0.0);
    if !min_opacity.is_finite() {
        min_opacity = 0.0;
    }
    let mut max_opacity = params.get_float("max_opacity", 1.0);
    if !max_opacity.is_finite() {
        max_opacity = 1.0;
    }
    min_opacity = min_opacity.max(0.0);
    max_opacity = max_opacity.max(min_opacity);
    let mut min_scale = params.get_float("min_scale", 0.0);
    if !min_scale.is_finite() {
        min_scale = 0.0;
    }
    let mut max_scale = params.get_float("max_scale", 1000.0);
    if !max_scale.is_finite() {
        max_scale = 1000.0;
    }
    min_scale = min_scale.max(0.0);
    max_scale = max_scale.max(min_scale);
    let remove_invalid = params.get_bool("remove_invalid", true);
    let use_log_opacity = splats
        .opacity
        .iter()
        .any(|value| *value < 0.0 || *value > 1.0);
    let use_log_scale = splats
        .scales
        .iter()
        .any(|value| value[0] < 0.0 || value[1] < 0.0 || value[2] < 0.0);

    let mut kept = Vec::with_capacity(splats.len());
    for idx in 0..splats.len() {
        if let Some(mask) = &group_mask {
            if !mask.get(idx).copied().unwrap_or(false) {
                kept.push(idx);
                continue;
            }
        }

        if remove_invalid && !splat_is_finite(splats, idx) {
            continue;
        }

        let opacity = splat_opacity(splats.opacity[idx], use_log_opacity);
        if opacity < min_opacity || opacity > max_opacity {
            continue;
        }

        let (min_component, max_component) =
            scale_extents(splats.scales[idx], use_log_scale);
        if min_component < min_scale || max_component > max_scale {
            continue;
        }

        kept.push(idx);
    }

    filter_splats(splats, &kept)
}

fn splat_opacity(value: f32, use_log_opacity: bool) -> f32 {
    if !use_log_opacity {
        return value;
    }
    1.0 / (1.0 + (-value).exp())
}

fn scale_extents(scale: [f32; 3], use_log_scale: bool) -> (f32, f32) {
    let mut values = Vec3::from(scale);
    if use_log_scale {
        values = Vec3::new(values.x.exp(), values.y.exp(), values.z.exp());
    }
    values = values.abs();
    let min_component = values.x.min(values.y).min(values.z);
    let max_component = values.x.max(values.y).max(values.z);
    (min_component, max_component)
}

fn splat_is_finite(splats: &SplatGeo, idx: usize) -> bool {
    let Some(position) = splats.positions.get(idx) else {
        return false;
    };
    if position.iter().any(|value| !value.is_finite()) {
        return false;
    }
    let Some(rotation) = splats.rotations.get(idx) else {
        return false;
    };
    if rotation.iter().any(|value| !value.is_finite()) {
        return false;
    }
    let Some(scale) = splats.scales.get(idx) else {
        return false;
    };
    if scale.iter().any(|value| !value.is_finite()) {
        return false;
    }
    let Some(opacity) = splats.opacity.get(idx) else {
        return false;
    };
    if !opacity.is_finite() {
        return false;
    }
    let Some(sh0) = splats.sh0.get(idx) else {
        return false;
    };
    if sh0.iter().any(|value| !value.is_finite()) {
        return false;
    }

    if splats.sh_coeffs > 0 {
        let base = idx * splats.sh_coeffs;
        for coeff in 0..splats.sh_coeffs {
            let Some(values) = splats.sh_rest.get(base + coeff) else {
                return false;
            };
            if values.iter().any(|value| !value.is_finite()) {
                return false;
            }
        }
    }

    true
}

fn filter_splats(splats: &SplatGeo, kept: &[usize]) -> SplatGeo {
    let mut output = SplatGeo::with_len_and_sh(kept.len(), splats.sh_coeffs);
    for (out_idx, src_idx) in kept.iter().copied().enumerate() {
        output.positions[out_idx] = splats.positions[src_idx];
        output.rotations[out_idx] = splats.rotations[src_idx];
        output.scales[out_idx] = splats.scales[src_idx];
        output.opacity[out_idx] = splats.opacity[src_idx];
        output.sh0[out_idx] = splats.sh0[src_idx];
        if splats.sh_coeffs > 0 {
            let src_base = src_idx * splats.sh_coeffs;
            let dst_base = out_idx * splats.sh_coeffs;
            output.sh_rest[dst_base..dst_base + splats.sh_coeffs]
                .copy_from_slice(&splats.sh_rest[src_base..src_base + splats.sh_coeffs]);
        }
    }

    if !splats.groups.is_empty() {
        for (name, values) in &splats.groups {
            let filtered = kept
                .iter()
                .map(|&idx| values.get(idx).copied().unwrap_or(false))
                .collect();
            output.groups.insert(name.clone(), filtered);
        }
    }

    output
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
                ("min_scale".to_string(), ParamValue::Float(0.2)),
                ("max_scale".to_string(), ParamValue::Float(0.6)),
                ("min_opacity".to_string(), ParamValue::Float(0.0)),
                ("max_opacity".to_string(), ParamValue::Float(1.0)),
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
        splats.scales.fill([0.1, 0.1, 0.1]);
        splats.opacity[0] = -2.0;
        splats.opacity[1] = 0.0;
        splats.opacity[2] = 2.0;

        let params = NodeParams {
            values: BTreeMap::from([
                ("min_opacity".to_string(), ParamValue::Float(0.7)),
                ("max_opacity".to_string(), ParamValue::Float(1.0)),
                ("remove_invalid".to_string(), ParamValue::Bool(true)),
            ]),
        };

        let pruned = apply_to_splats(&params, &splats);
        assert_eq!(pruned.len(), 1);
        assert_eq!(pruned.opacity[0], 2.0);
    }
}
