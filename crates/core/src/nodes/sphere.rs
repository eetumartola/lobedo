use std::collections::BTreeMap;

use glam::{Mat4, Vec3};

use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::{make_uv_sphere, Mesh};
use crate::nodes::geometry_out;

pub const NAME: &str = "Sphere";

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
            ("rows".to_string(), ParamValue::Int(16)),
            ("cols".to_string(), ParamValue::Int(32)),
            ("center".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0])),
        ]),
    }
}

pub fn compute(params: &NodeParams, _inputs: &[Mesh]) -> Result<Mesh, String> {
    let radius = params.get_float("radius", 1.0).max(0.0);
    let rows = params.get_int("rows", 16).max(3) as u32;
    let cols = params.get_int("cols", 32).max(3) as u32;
    let center = params.get_vec3("center", [0.0, 0.0, 0.0]);
    let mut mesh = make_uv_sphere(radius, rows, cols);
    if center != [0.0, 0.0, 0.0] {
        mesh.transform(Mat4::from_translation(Vec3::from(center)));
    }
    if mesh.normals.is_none() {
        mesh.compute_normals();
    }
    Ok(mesh)
}

