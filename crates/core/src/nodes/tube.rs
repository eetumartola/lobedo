use std::collections::BTreeMap;

use glam::{Mat4, Vec3};

use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::{make_tube, Mesh};
use crate::nodes::geometry_out;
use crate::param_spec::ParamSpec;

pub const NAME: &str = "Tube";

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
            ("radius".to_string(), ParamValue::Float(1.0)),
            ("height".to_string(), ParamValue::Float(1.0)),
            ("rows".to_string(), ParamValue::Int(1)),
            ("cols".to_string(), ParamValue::Int(16)),
            ("capped".to_string(), ParamValue::Bool(true)),
            ("center".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0])),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::float_slider("radius", "Radius", 0.0, 1000.0)
            .with_help("Tube radius."),
        ParamSpec::float_slider("height", "Height", 0.0, 1000.0)
            .with_help("Tube height."),
        ParamSpec::int_slider("rows", "Rows", 1, 64)
            .with_help("Height segments."),
        ParamSpec::int_slider("cols", "Cols", 3, 256)
            .with_help("Side segments."),
        ParamSpec::bool("capped", "Capped")
            .with_help("Add caps at the ends."),
        ParamSpec::vec3("center", "Center")
            .with_help("Tube center in world space."),
    ]
}

pub fn compute(params: &NodeParams, _inputs: &[Mesh]) -> Result<Mesh, String> {
    let radius = params.get_float("radius", 1.0);
    let height = params.get_float("height", 1.0);
    let rows = params.get_int("rows", 1) as u32;
    let cols = params.get_int("cols", 16) as u32;
    let capped = params.get_bool("capped", true);
    let center = params.get_vec3("center", [0.0, 0.0, 0.0]);

    let mut mesh = make_tube(radius, height, rows, cols, capped);
    if center != [0.0, 0.0, 0.0] {
        mesh.transform(Mat4::from_translation(Vec3::from(center)));
    }
    if mesh.normals.is_none() {
        mesh.compute_normals();
    }
    Ok(mesh)
}
