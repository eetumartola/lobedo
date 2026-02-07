use std::collections::BTreeMap;

use glam::{Mat4, Vec3};

use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::nodes::geometry_out;
use crate::param_spec::{ParamPathKind, ParamSpec};
use crate::splat::{load_splat_ply_with_mode, load_splat_spz_with_mode, SplatGeo, SplatLoadMode};

pub const NAME: &str = "Splat Read";
pub const LEGACY_NAME: &str = "Read Splats";
const SPZ_FLIP_Y_KEY: &str = "spz_flip_y";
const DEFAULT_SPZ_FLIP_Y: bool = true;
const FLIP_Z_KEY: &str = "flip_z";
const DEFAULT_FLIP_Z: bool = true;

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
            (
                "path".to_string(),
                ParamValue::String("C:\\code\\lobedo\\geo\\CL.ply".to_string()),
            ),
            ("read_mode".to_string(), ParamValue::Int(0)),
            (SPZ_FLIP_Y_KEY.to_string(), ParamValue::Bool(DEFAULT_SPZ_FLIP_Y)),
            (FLIP_Z_KEY.to_string(), ParamValue::Bool(DEFAULT_FLIP_Z)),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::path("path", "Path", ParamPathKind::ReadSplat)
            .with_help("Path or URL to a splat file (.ply or .spz)."),
        ParamSpec::int_enum(
            "read_mode",
            "Read Mode",
            vec![(0, "Full (SH)"), (1, "Base Color")],
        )
        .with_help("Read full SH data or base color only."),
        ParamSpec::bool(SPZ_FLIP_Y_KEY, "SPZ Flip Y")
            .with_help("Flip Y after loading .spz files (matches WorldLabs import orientation)."),
        ParamSpec::bool(FLIP_Z_KEY, "Flip Z")
            .with_help("Flip Z after loading .ply/.spz to match WorldLabs/Marble handedness."),
    ]
}

pub fn compute(params: &NodeParams) -> Result<SplatGeo, String> {
    let path = params.get_string("path", "");
    if path.trim().is_empty() {
        return Err("Splat Read requires a path".to_string());
    }
    let mode = if params.get_int("read_mode", 0) == 1 {
        SplatLoadMode::ColorOnly
    } else {
        SplatLoadMode::Full
    };
    let spz_flip_y = params.get_bool(SPZ_FLIP_Y_KEY, DEFAULT_SPZ_FLIP_Y);
    let flip_z = params.get_bool(FLIP_Z_KEY, DEFAULT_FLIP_Z);
    let path_no_fragment = path.split('#').next().unwrap_or(path);
    let path_no_query = path_no_fragment.split('?').next().unwrap_or(path_no_fragment);
    let lower = path_no_query.to_ascii_lowercase();
    if lower.ends_with(".spz") {
        let mut splats = load_splat_spz_with_mode(path, mode)?;
        if spz_flip_y {
            splats.flip_y_axis();
        }
        if flip_z {
            splats.transform(Mat4::from_scale(Vec3::new(1.0, 1.0, -1.0)));
        }
        return Ok(splats);
    }
    if lower.ends_with(".ply") {
        let mut splats = load_splat_ply_with_mode(path, mode)?;
        if flip_z {
            splats.transform(Mat4::from_scale(Vec3::new(1.0, 1.0, -1.0)));
        }
        return Ok(splats);
    }

    // If the path has no extension, try both common splat formats.
    load_splat_ply_with_mode(path, mode).or_else(|ply_err| {
        let mut splats = load_splat_spz_with_mode(path, mode).map_err(|spz_err| {
            format!(
                "Failed to load splat '{path}'. Tried PLY ({ply_err}) and SPZ ({spz_err})."
            )
        })?;
        if spz_flip_y {
            splats.flip_y_axis();
        }
        if flip_z {
            splats.transform(Mat4::from_scale(Vec3::new(1.0, 1.0, -1.0)));
        }
        Ok(splats)
    })
}

