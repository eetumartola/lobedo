use std::collections::BTreeMap;

use glam::{Mat4, Vec3};

use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::{make_box, Mesh};
use crate::nodes::geometry_out;
use crate::param_spec::ParamSpec;

pub const NAME: &str = "Box";

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
            ("size".to_string(), ParamValue::Vec3([1.0, 1.0, 1.0])),
            ("center".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0])),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::vec3("size", "Size")
            .with_help("Box dimensions in X/Y/Z."),
        ParamSpec::vec3("center", "Center")
            .with_help("Box center in world space."),
    ]
}

pub fn compute(params: &NodeParams, _inputs: &[Mesh]) -> Result<Mesh, String> {
    let size = params.get_vec3("size", [1.0, 1.0, 1.0]);
    let center = params.get_vec3("center", [0.0, 0.0, 0.0]);
    let mut mesh = make_box(size);
    if center != [0.0, 0.0, 0.0] {
        mesh.transform(Mat4::from_translation(Vec3::from(center)));
    }
    if mesh.normals.is_none() {
        mesh.compute_normals();
    }
    Ok(mesh)
}

