use std::collections::BTreeMap;

use crate::attributes::AttributeDomain;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    geometry_in,
    geometry_out,
    require_mesh_input,
    splat_utils::split_splats_by_group,
};
use crate::param_spec::ParamSpec;
use crate::splat::SplatGeo;

use super::splat_cluster::{dbscan_labels, estimate_spacing};

pub const NAME: &str = "Splat Outlier";

const DEFAULT_EPS: f32 = 0.0;
const DEFAULT_MIN_PTS: i32 = 12;
const DEFAULT_MIN_CLUSTER: i32 = 0;

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
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
            ("eps".to_string(), ParamValue::Float(DEFAULT_EPS)),
            ("min_pts".to_string(), ParamValue::Int(DEFAULT_MIN_PTS)),
            (
                "min_cluster_size".to_string(),
                ParamValue::Int(DEFAULT_MIN_CLUSTER),
            ),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::string("group", "Group")
            .with_help("Optional group to restrict outlier removal."),
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
        ParamSpec::float_slider("eps", "Radius", 0.0, 10.0)
            .with_help("Neighborhood radius for density clustering (<=0 = auto)."),
        ParamSpec::int_slider("min_pts", "Min Points", 1, 128)
            .with_help("Minimum neighbors for a core point."),
        ParamSpec::int_slider("min_cluster_size", "Min Cluster Size", 0, 100_000)
            .with_help("Remove clusters smaller than this size (0 disables)."),
    ]
}

pub fn compute(_params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let input = require_mesh_input(inputs, 0, "Splat Outlier requires a mesh input")?;
    Ok(input)
}

pub fn apply_to_splats(params: &NodeParams, splats: &SplatGeo) -> SplatGeo {
    if splats.is_empty() {
        return splats.clone();
    }
    let Some((selected, _unselected)) =
        split_splats_by_group(splats, params, AttributeDomain::Point)
    else {
        return splats.clone();
    };
    if selected.len() <= 1 {
        return splats.clone();
    }

    let mut positions = Vec::with_capacity(selected.len());
    for &idx in &selected {
        positions.push(splats.positions[idx]);
    }

    let spacing = estimate_spacing(&positions);
    let mut eps = params.get_float("eps", DEFAULT_EPS);
    if eps <= 0.0 {
        eps = spacing * 1.5;
    }
    if !eps.is_finite() || eps <= 1.0e-6 {
        return splats.clone();
    }
    let min_pts = params.get_int("min_pts", DEFAULT_MIN_PTS).max(1) as usize;
    let labels = dbscan_labels(&positions, eps, min_pts);

    let mut cluster_sizes = Vec::new();
    for label in &labels {
        if *label >= 0 {
            let idx = *label as usize;
            if cluster_sizes.len() <= idx {
                cluster_sizes.resize(idx + 1, 0usize);
            }
            cluster_sizes[idx] += 1;
        }
    }
    let min_cluster_size = params
        .get_int("min_cluster_size", DEFAULT_MIN_CLUSTER)
        .max(0) as usize;

    let mut keep_flags = vec![true; splats.len()];
    for (local_idx, &global_idx) in selected.iter().enumerate() {
        let label = labels.get(local_idx).copied().unwrap_or(-1);
        let mut keep = label >= 0;
        if keep && min_cluster_size > 0 {
            if let Some(size) = cluster_sizes.get(label as usize) {
                if *size < min_cluster_size {
                    keep = false;
                }
            }
        }
        if let Some(slot) = keep_flags.get_mut(global_idx) {
            *slot = keep;
        }
    }

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
    fn outlier_removes_isolated_points() {
        let mut splats = SplatGeo::with_len(3);
        splats.positions[0] = [0.0, 0.0, 0.0];
        splats.positions[1] = [0.1, 0.0, 0.0];
        splats.positions[2] = [4.0, 0.0, 0.0];
        splats.rotations.fill([1.0, 0.0, 0.0, 0.0]);
        splats.scales.fill([0.0, 0.0, 0.0]);
        splats.opacity.fill(0.0);
        splats.sh0.fill([0.0, 0.0, 0.0]);

        let params = NodeParams {
            values: BTreeMap::from([
                ("eps".to_string(), ParamValue::Float(0.3)),
                ("min_pts".to_string(), ParamValue::Int(2)),
            ]),
        };

        let filtered = apply_to_splats(&params, &splats);
        assert_eq!(filtered.len(), 2);
    }
}
