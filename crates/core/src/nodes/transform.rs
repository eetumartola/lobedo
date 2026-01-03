use std::collections::BTreeMap;

use glam::{EulerRot, Mat4, Quat, Vec3};

use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{mesh_in, mesh_out, require_mesh_input};

pub const NAME: &str = "Transform";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![mesh_in("in")],
        outputs: vec![mesh_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            ("translate".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0])),
            ("rotate_deg".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0])),
            ("scale".to_string(), ParamValue::Vec3([1.0, 1.0, 1.0])),
            ("pivot".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0])),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let input = require_mesh_input(inputs, 0, "Transform requires a mesh input")?;
    let translate = params.get_vec3("translate", [0.0, 0.0, 0.0]);
    let rotate_deg = params.get_vec3("rotate_deg", [0.0, 0.0, 0.0]);
    let scale = params.get_vec3("scale", [1.0, 1.0, 1.0]);
    let pivot = params.get_vec3("pivot", [0.0, 0.0, 0.0]);

    let rot = Vec3::from(rotate_deg) * std::f32::consts::PI / 180.0;
    let quat = Quat::from_euler(EulerRot::XYZ, rot.x, rot.y, rot.z);
    let matrix = Mat4::from_translation(Vec3::from(translate))
        * Mat4::from_translation(Vec3::from(pivot))
        * Mat4::from_quat(quat)
        * Mat4::from_scale(Vec3::from(scale))
        * Mat4::from_translation(-Vec3::from(pivot));
    let mut mesh = input;
    mesh.transform(matrix);
    Ok(mesh)
}
