use std::collections::BTreeMap;

use glam::Vec3;

use crate::attributes::AttributeDomain;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{geometry_in, geometry_out, require_mesh_input, splat_utils::split_splats_by_group};
use crate::param_spec::ParamSpec;
use crate::splat::SplatGeo;
use crate::volume::{Volume, VolumeKind};
use crate::volume_sampling::VolumeSampler;

pub const NAME: &str = "Mesh Outliers SDF";

const DEFAULT_THRESHOLD: f32 = 0.1;
const DEFAULT_ISO: f32 = 0.0;
const DEFAULT_ABS: bool = false;

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![geometry_in("splats"), geometry_in("sdf")],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
            (
                "threshold".to_string(),
                ParamValue::Float(DEFAULT_THRESHOLD),
            ),
            ("iso".to_string(), ParamValue::Float(DEFAULT_ISO)),
            ("abs_distance".to_string(), ParamValue::Bool(DEFAULT_ABS)),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::string("group", "Group")
            .with_help("Optional group to restrict culling."),
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
        ParamSpec::float_slider("threshold", "Threshold", 0.0, 10.0)
            .with_help("Cull splats farther than this distance from the SDF isosurface."),
        ParamSpec::float_slider("iso", "Iso", -10.0, 10.0)
            .with_help("SDF iso value to measure distances from."),
        ParamSpec::bool("abs_distance", "Abs Distance")
            .with_help("Use absolute distance from the iso value."),
    ]
}

pub fn compute(_params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let input = require_mesh_input(inputs, 0, "Mesh Outliers SDF requires a mesh input")?;
    Ok(input)
}

pub fn apply_to_geometry(params: &NodeParams, inputs: &[crate::geometry::Geometry]) -> Result<crate::geometry::Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(crate::geometry::Geometry::default());
    };
    let Some(sdf_geo) = inputs.get(1) else {
        return Err("Mesh Outliers SDF requires an SDF input".to_string());
    };
    let Some(volume) = sdf_geo.volumes.first() else {
        return Err("Mesh Outliers SDF requires an SDF volume input".to_string());
    };
    if volume.kind != VolumeKind::Sdf {
        return Err("Mesh Outliers SDF requires an SDF volume input".to_string());
    }

    let mut meshes = Vec::new();
    if let Some(mesh) = input.merged_mesh() {
        meshes.push(mesh);
    }

    let mut splats: Vec<SplatGeo> = (0..input.splats.len())
        .map(|_| SplatGeo::default())
        .collect();
    let input_splats = input.splats.as_slice();
    crate::parallel::try_for_each_indexed_mut(&mut splats, |idx, slot| {
        *slot = apply_to_splats(params, &input_splats[idx], volume)?;
        Ok::<(), String>(())
    })?;

    let curves = if meshes.is_empty() {
        Vec::new()
    } else {
        input.curves.clone()
    };

    Ok(crate::geometry::Geometry {
        meshes,
        splats,
        curves,
        volumes: input.volumes.clone(),
        materials: input.materials.clone(),
    })
}

pub fn apply_to_splats(
    params: &NodeParams,
    splats: &SplatGeo,
    volume: &Volume,
) -> Result<SplatGeo, String> {
    if splats.is_empty() {
        return Ok(splats.clone());
    }
    let Some((selected, _unselected)) =
        split_splats_by_group(splats, params, AttributeDomain::Point)
    else {
        return Ok(splats.clone());
    };
    let mut threshold = params.get_float("threshold", DEFAULT_THRESHOLD);
    if !threshold.is_finite() {
        return Ok(splats.clone());
    }
    if threshold < 0.0 {
        threshold = 0.0;
    }
    let iso = params.get_float("iso", DEFAULT_ISO);
    let use_abs = params.get_bool("abs_distance", DEFAULT_ABS);

    let sampler = VolumeSampler::new(volume);
    let mut keep_flags = vec![true; splats.len()];
    for &idx in &selected {
        let pos = splats.positions.get(idx).copied().unwrap_or([0.0, 0.0, 0.0]);
        let sdf_val = sampler.sample_world(Vec3::from(pos));
        let dist = sdf_val - iso;
        let measure = if use_abs { dist.abs() } else { dist };
        let keep = measure.is_finite() && measure <= threshold;
        if let Some(slot) = keep_flags.get_mut(idx) {
            *slot = keep;
        }
    }

    let kept: Vec<usize> = keep_flags
        .iter()
        .enumerate()
        .filter_map(|(idx, keep)| (*keep).then_some(idx))
        .collect();
    Ok(splats.filter_by_indices(&kept))
}
