use std::collections::BTreeMap;

use glam::{EulerRot, Mat4, Quat, Vec3};

use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{geometry_in, geometry_out, require_mesh_input};
use crate::nodes::transform;
use crate::parallel;
use crate::param_spec::ParamSpec;
use crate::param_templates;

pub const NAME: &str = "Copy/Transform";

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
            ("count".to_string(), ParamValue::Int(5)),
            (
                "translate_step".to_string(),
                ParamValue::Vec3([1.0, 0.0, 0.0]),
            ),
            (
                "rotate_step_deg".to_string(),
                ParamValue::Vec3([0.0, 0.0, 0.0]),
            ),
            ("scale_step".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0])),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    let mut specs = param_templates::transform_params(true);
    specs.push(
        ParamSpec::int_slider("count", "Count", 0, 1000)
            .with_help("Number of copies."),
    );
    specs.push(
        ParamSpec::vec3("translate_step", "Translate Step")
            .with_help("Per-copy translation step."),
    );
    specs.push(
        ParamSpec::vec3("rotate_step_deg", "Rotate Step")
            .with_help("Per-copy rotation step (degrees)."),
    );
    specs.push(
        ParamSpec::vec3("scale_step", "Scale Step")
            .with_help("Per-copy scale step."),
    );
    specs
}

pub fn transform_matrices(params: &NodeParams) -> Vec<Mat4> {
    let count = params.get_int("count", 1).max(0) as usize;
    if count == 0 {
        return Vec::new();
    }
    let translate_step = params.get_vec3("translate_step", [0.0, 0.0, 0.0]);
    let rotate_step = params.get_vec3("rotate_step_deg", [0.0, 0.0, 0.0]);
    let scale_step = params.get_vec3("scale_step", [0.0, 0.0, 0.0]);
    let base = transform::transform_matrix(params);

    let mut matrices = Vec::with_capacity(count);
    for i in 0..count {
        let factor = i as f32;
        let translate = Vec3::from(translate_step) * factor;
        let rot = Vec3::from(rotate_step) * factor * std::f32::consts::PI / 180.0;
        let quat = Quat::from_euler(EulerRot::XYZ, rot.x, rot.y, rot.z);
        let scale = Vec3::new(1.0, 1.0, 1.0) + Vec3::from(scale_step) * factor;
        let step = Mat4::from_scale_rotation_translation(scale, quat, translate);
        matrices.push(base * step);
    }
    matrices
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let input = require_mesh_input(inputs, 0, "Copy/Transform requires a mesh input")?;
    let matrices = transform_matrices(params);
    if matrices.is_empty() {
        return Ok(Mesh::default());
    }

    let mut copies: Vec<Mesh> = (0..matrices.len()).map(|_| Mesh::default()).collect();
    parallel::for_each_indexed_mut(&mut copies, |idx, slot| {
        let mut mesh = input.clone();
        mesh.transform(matrices[idx]);
        *slot = mesh;
    });
    Ok(Mesh::merge(&copies))
}

