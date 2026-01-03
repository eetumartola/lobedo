use std::collections::BTreeMap;

use glam::{EulerRot, Mat4, Quat, Vec3};

use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{mesh_in, mesh_out, require_mesh_input};

pub const NAME: &str = "Copy to Points";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![mesh_in("source"), mesh_in("template")],
        outputs: vec![mesh_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            ("align_to_normals".to_string(), ParamValue::Bool(true)),
            ("translate".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0])),
            ("rotate_deg".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0])),
            ("scale".to_string(), ParamValue::Vec3([1.0, 1.0, 1.0])),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let source = require_mesh_input(inputs, 0, "Copy to Points requires a source mesh")?;
    let template = require_mesh_input(inputs, 1, "Copy to Points requires a template mesh")?;

    if template.positions.is_empty() {
        return Err("Copy to Points requires template points".to_string());
    }

    let align_to_normals = params.get_bool("align_to_normals", true);
    let translate = params.get_vec3("translate", [0.0, 0.0, 0.0]);
    let rotate_deg = params.get_vec3("rotate_deg", [0.0, 0.0, 0.0]);
    let scale = params.get_vec3("scale", [1.0, 1.0, 1.0]);

    let mut normals = template.normals.clone().unwrap_or_default();
    if align_to_normals && normals.len() != template.positions.len() {
        let mut temp = template.clone();
        if temp.normals.is_none() {
            temp.compute_normals();
        }
        normals = temp.normals.unwrap_or_default();
    }

    let rot = Vec3::from(rotate_deg) * std::f32::consts::PI / 180.0;
    let user_quat = Quat::from_euler(EulerRot::XYZ, rot.x, rot.y, rot.z);
    let scale = Vec3::from(scale);
    let translate = Vec3::from(translate);

    let mut copies = Vec::with_capacity(template.positions.len());
    for (idx, pos) in template.positions.iter().enumerate() {
        let mut rotation = user_quat;
        if align_to_normals {
            let normal = normals.get(idx).copied().unwrap_or([0.0, 1.0, 0.0]);
            let normal = Vec3::from(normal);
            if normal.length_squared() > 0.0001 {
                let align = Quat::from_rotation_arc(Vec3::Y, normal.normalize());
                rotation = align * user_quat;
            }
        }
        let matrix =
            Mat4::from_scale_rotation_translation(scale, rotation, Vec3::from(*pos) + translate);
        let mut mesh = source.clone();
        mesh.transform(matrix);
        copies.push(mesh);
    }
    Ok(Mesh::merge(&copies))
}
