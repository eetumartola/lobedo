use std::collections::BTreeMap;

use glam::{Mat4, Vec3};

use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::{make_grid, Mesh};
use crate::nodes::geometry_out;
use crate::param_spec::ParamSpec;

pub const NAME: &str = "Grid";

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
            ("size".to_string(), ParamValue::Vec2([2.0, 2.0])),
            ("rows".to_string(), ParamValue::Int(10)),
            ("cols".to_string(), ParamValue::Int(10)),
            ("center".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0])),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::vec2("size", "Size")
            .with_help("Grid size in X/Z."),
        ParamSpec::int_slider("rows", "Rows", 2, 64)
            .with_help("Rows (subdivisions) along Z."),
        ParamSpec::int_slider("cols", "Cols", 2, 64)
            .with_help("Columns (subdivisions) along X."),
        ParamSpec::vec3("center", "Center")
            .with_help("Grid center in world space."),
    ]
}

pub fn compute(params: &NodeParams, _inputs: &[Mesh]) -> Result<Mesh, String> {
    let size = params.get_vec2("size", [2.0, 2.0]);
    let rows = params.get_int("rows", 10).max(1) as u32;
    let cols = params.get_int("cols", 10).max(1) as u32;
    let center = params.get_vec3("center", [0.0, 0.0, 0.0]);
    let divisions = [cols, rows];
    let mut mesh = make_grid(size, divisions);
    if center != [0.0, 0.0, 0.0] {
        mesh.transform(Mat4::from_translation(Vec3::from(center)));
    }
    if mesh.normals.is_none() {
        mesh.compute_normals();
    }
    Ok(mesh)
}

