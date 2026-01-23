use std::collections::BTreeMap;

use glam::{EulerRot, Mat4, Quat, Vec3};

use crate::attributes::AttributeDomain;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{geometry_in, geometry_out, group_utils::mesh_group_mask, require_mesh_input};
use crate::param_spec::ParamSpec;
use crate::param_templates;

pub const NAME: &str = "Transform";

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
            ("translate".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0])),
            ("rotate_deg".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0])),
            ("scale".to_string(), ParamValue::Vec3([1.0, 1.0, 1.0])),
            ("pivot".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0])),
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    let mut specs = param_templates::transform_params(true);
    specs.push(
        ParamSpec::string("group", "Group")
            .with_help("Optional group to restrict transform."),
    );
    specs.push(ParamSpec::int_enum(
        "group_type",
        "Group Type",
        vec![
            (0, "Auto"),
            (1, "Vertex"),
            (2, "Point"),
            (3, "Primitive"),
        ],
    )
    .with_help("Group domain to use."));
    specs
}

pub fn transform_matrix(params: &NodeParams) -> Mat4 {
    let translate = params.get_vec3("translate", [0.0, 0.0, 0.0]);
    let rotate_deg = params.get_vec3("rotate_deg", [0.0, 0.0, 0.0]);
    let scale = params.get_vec3("scale", [1.0, 1.0, 1.0]);
    let pivot = params.get_vec3("pivot", [0.0, 0.0, 0.0]);

    let rot = Vec3::from(rotate_deg) * std::f32::consts::PI / 180.0;
    let quat = Quat::from_euler(EulerRot::XYZ, rot.x, rot.y, rot.z);
    Mat4::from_translation(Vec3::from(translate))
        * Mat4::from_translation(Vec3::from(pivot))
        * Mat4::from_quat(quat)
        * Mat4::from_scale(Vec3::from(scale))
        * Mat4::from_translation(-Vec3::from(pivot))
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let input = require_mesh_input(inputs, 0, "Transform requires a mesh input")?;
    let mut mesh = input;
    let matrix = transform_matrix(params);
    apply_to_mesh(params, &mut mesh, matrix);
    Ok(mesh)
}

pub fn apply_to_mesh(params: &NodeParams, mesh: &mut Mesh, matrix: Mat4) {
    let mask = mesh_group_mask(mesh, params, AttributeDomain::Point);
    if let Some(mask) = mask {
        apply_transform_mask(mesh, matrix, &mask);
    } else {
        mesh.transform(matrix);
    }
}

fn apply_transform_mask(mesh: &mut Mesh, matrix: Mat4, mask: &[bool]) {
    if mask.len() != mesh.positions.len() {
        mesh.transform(matrix);
        return;
    }
    for (idx, pos) in mesh.positions.iter_mut().enumerate() {
        if !mask[idx] {
            continue;
        }
        let v = matrix.transform_point3(Vec3::from(*pos));
        *pos = v.to_array();
    }

    let normal_matrix = matrix.inverse().transpose();
    if let Some(normals) = &mut mesh.normals {
        for (idx, n) in normals.iter_mut().enumerate() {
            if !mask.get(idx).copied().unwrap_or(false) {
                continue;
            }
            let v = normal_matrix.transform_vector3(Vec3::from(*n));
            let len = v.length();
            *n = if len > 0.0 {
                (v / len).to_array()
            } else {
                [0.0, 1.0, 0.0]
            };
        }
    }

    if let Some(corner_normals) = &mut mesh.corner_normals {
        for (idx, n) in corner_normals.iter_mut().enumerate() {
            let point = mesh.indices.get(idx).copied().unwrap_or(0) as usize;
            if !mask.get(point).copied().unwrap_or(false) {
                continue;
            }
            let v = normal_matrix.transform_vector3(Vec3::from(*n));
            let len = v.length();
            *n = if len > 0.0 {
                (v / len).to_array()
            } else {
                [0.0, 1.0, 0.0]
            };
        }
    }
}

