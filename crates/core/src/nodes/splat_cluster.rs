use std::collections::{BTreeMap, HashMap};

use glam::Vec3;

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    geometry_in,
    geometry_out,
    require_mesh_input,
    splat_utils::{split_splats_by_group, splat_cell_key, SpatialHash},
};
use crate::param_spec::ParamSpec;
use crate::splat::SplatGeo;

pub const NAME: &str = "Splat Cluster";

const DEFAULT_METHOD: i32 = 0;
const DEFAULT_ATTR: &str = "cluster";
const DEFAULT_CELL_SIZE: f32 = 0.0;
const DEFAULT_EPS: f32 = 0.0;
const DEFAULT_MIN_PTS: i32 = 12;

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
            ("method".to_string(), ParamValue::Int(DEFAULT_METHOD)),
            (
                "attr".to_string(),
                ParamValue::String(DEFAULT_ATTR.to_string()),
            ),
            ("cell_size".to_string(), ParamValue::Float(DEFAULT_CELL_SIZE)),
            ("eps".to_string(), ParamValue::Float(DEFAULT_EPS)),
            ("min_pts".to_string(), ParamValue::Int(DEFAULT_MIN_PTS)),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::string("group", "Group")
            .with_help("Optional group to restrict clustering."),
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
        ParamSpec::int_enum("method", "Method", vec![(0, "Grid"), (1, "DBSCAN")])
            .with_help("Clustering method (Grid or DBSCAN)."),
        ParamSpec::string("attr", "Attribute")
            .with_help("Attribute name to store cluster ids."),
        ParamSpec::float_slider("cell_size", "Cell Size", 0.0, 10.0)
            .with_help("Grid cell size (<=0 = auto)."),
        ParamSpec::float_slider("eps", "Radius", 0.0, 10.0)
            .with_help("DBSCAN radius (<=0 = auto)."),
        ParamSpec::int_slider("min_pts", "Min Points", 1, 128)
            .with_help("Minimum points per DBSCAN core."),
    ]
}

pub fn compute(_params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let input = require_mesh_input(inputs, 0, "Splat Cluster requires a mesh input")?;
    Ok(input)
}

pub fn apply_to_splats(params: &NodeParams, splats: &SplatGeo) -> Result<SplatGeo, String> {
    if splats.is_empty() {
        return Ok(splats.clone());
    }

    let Some((selected, _unselected)) =
        split_splats_by_group(splats, params, AttributeDomain::Point)
    else {
        return Ok(splats.clone());
    };

    let mut positions = Vec::with_capacity(selected.len());
    for &idx in &selected {
        positions.push(splats.positions[idx]);
    }

    let spacing = estimate_spacing(&positions);
    let method = params.get_int("method", DEFAULT_METHOD).clamp(0, 1);
    let labels = match method {
        1 => {
            let mut eps = params.get_float("eps", DEFAULT_EPS);
            if eps <= 0.0 {
                eps = spacing * 1.5;
            }
            if !eps.is_finite() || eps <= 1.0e-6 {
                return Ok(splats.clone());
            }
            let min_pts = params.get_int("min_pts", DEFAULT_MIN_PTS).max(1) as usize;
            dbscan_labels(&positions, eps, min_pts)
        }
        _ => {
            let mut cell_size = params.get_float("cell_size", DEFAULT_CELL_SIZE);
            if cell_size <= 0.0 {
                cell_size = spacing * 2.0;
            }
            if !cell_size.is_finite() || cell_size <= 1.0e-6 {
                return Ok(splats.clone());
            }
            grid_labels(&positions, cell_size)
        }
    };

    let attr_name = params.get_string("attr", DEFAULT_ATTR);
    let attr_name = attr_name.trim();
    let attr_name = if attr_name.is_empty() {
        DEFAULT_ATTR
    } else {
        attr_name
    };

    let mut values = vec![-1; splats.len()];
    for (local_idx, &global_idx) in selected.iter().enumerate() {
        if let Some(label) = labels.get(local_idx) {
            if let Some(slot) = values.get_mut(global_idx) {
                *slot = *label;
            }
        }
    }

    let mut output = splats.clone();
    output
        .set_attribute(
            AttributeDomain::Point,
            attr_name.to_string(),
            AttributeStorage::Int(values),
        )
        .map_err(|err| format!("Splat Cluster error: {:?}", err))?;
    Ok(output)
}

pub(crate) fn estimate_spacing(positions: &[[f32; 3]]) -> f32 {
    if positions.len() <= 1 {
        return 1.0;
    }
    let mut min = Vec3::from(positions[0]);
    let mut max = min;
    for pos in positions.iter().skip(1) {
        let p = Vec3::from(*pos);
        min = min.min(p);
        max = max.max(p);
    }
    let extent = max - min;
    let mut axes = [extent.x.abs(), extent.y.abs(), extent.z.abs()];
    axes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let count = positions.len() as f32;
    let mut spacing = 0.0;
    let volume = axes[0] * axes[1] * axes[2];
    if volume.is_finite() && volume > 1.0e-6 {
        spacing = (volume / count).cbrt();
    } else if axes[1] > 1.0e-6 {
        spacing = ((axes[1] * axes[2]) / count.max(1.0)).sqrt();
    } else if axes[2] > 1.0e-6 {
        spacing = axes[2] / count.max(1.0);
    }
    if !spacing.is_finite() || spacing <= 1.0e-6 {
        spacing = 1.0;
    }
    spacing
}

pub(crate) fn grid_labels(positions: &[[f32; 3]], cell_size: f32) -> Vec<i32> {
    if positions.is_empty() {
        return Vec::new();
    }
    let cell = cell_size.max(1.0e-6);
    let mut min = Vec3::from(positions[0]);
    for pos in positions.iter().skip(1) {
        min = min.min(Vec3::from(*pos));
    }
    let inv_cell = 1.0 / cell;
    let mut map: HashMap<(i32, i32, i32), i32> = HashMap::new();
    let mut next_id = 0i32;
    let mut labels = Vec::with_capacity(positions.len());
    for pos in positions {
        let key = splat_cell_key(Vec3::from(*pos), min, inv_cell);
        let entry = map.entry(key).or_insert_with(|| {
            let id = next_id;
            next_id += 1;
            id
        });
        labels.push(*entry);
    }
    labels
}

pub(crate) fn dbscan_labels(positions: &[[f32; 3]], eps: f32, min_pts: usize) -> Vec<i32> {
    let count = positions.len();
    if count == 0 {
        return Vec::new();
    }
    let eps = eps.max(1.0e-6);
    let min_pts = min_pts.max(1);
    let Some(hash) = SpatialHash::build(positions, eps) else {
        return vec![-1; count];
    };

    let mut labels = vec![-2; count];
    let mut cluster_id = 0i32;
    let mut neighbors = Vec::new();
    let mut stack = Vec::new();

    for idx in 0..count {
        if labels[idx] != -2 {
            continue;
        }
        hash.neighbors_in_radius(positions, idx, eps, &mut neighbors);
        if neighbors.len() + 1 < min_pts {
            labels[idx] = -1;
            continue;
        }
        labels[idx] = cluster_id;
        stack.clear();
        stack.extend(neighbors.iter().copied());
        while let Some(next) = stack.pop() {
            if labels[next] == -1 {
                labels[next] = cluster_id;
            }
            if labels[next] != -2 {
                continue;
            }
            labels[next] = cluster_id;
            hash.neighbors_in_radius(positions, next, eps, &mut neighbors);
            if neighbors.len() + 1 >= min_pts {
                for &neighbor in &neighbors {
                    if labels[neighbor] == -2 || labels[neighbor] == -1 {
                        stack.push(neighbor);
                    }
                }
            }
        }
        cluster_id += 1;
    }

    labels
}

#[cfg(test)]
mod tests {
    use super::{dbscan_labels, grid_labels};

    #[test]
    fn grid_clusters_cell_assignments() {
        let points = vec![[0.0, 0.0, 0.0], [0.2, 0.0, 0.0], [4.0, 0.0, 0.0]];
        let labels = grid_labels(&points, 1.0);
        assert_eq!(labels.len(), 3);
        assert_eq!(labels[0], labels[1]);
        assert_ne!(labels[0], labels[2]);
    }

    #[test]
    fn dbscan_marks_isolated_noise() {
        let points = vec![[0.0, 0.0, 0.0], [0.1, 0.0, 0.0], [4.0, 0.0, 0.0]];
        let labels = dbscan_labels(&points, 0.3, 2);
        assert_eq!(labels.len(), 3);
        assert_eq!(labels[0], labels[1]);
        assert_eq!(labels[2], -1);
    }
}
