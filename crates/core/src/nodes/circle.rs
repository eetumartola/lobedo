use std::collections::BTreeMap;

use glam::{Mat4, Vec3};

use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::geometry_out;

pub const NAME: &str = "Circle";
const DEFAULT_RADIUS: f32 = 1.0;
const DEFAULT_SEGMENTS: i32 = 32;

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
            ("output".to_string(), ParamValue::String("curve".to_string())),
            ("radius".to_string(), ParamValue::Float(DEFAULT_RADIUS)),
            ("segments".to_string(), ParamValue::Int(DEFAULT_SEGMENTS)),
            ("center".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0])),
        ]),
    }
}

pub fn compute(params: &NodeParams, _inputs: &[Mesh]) -> Result<Mesh, String> {
    let output = params.get_string("output", "curve").to_lowercase();
    if !output.contains("mesh") {
        return Err("Circle outputs curve geometry in Curve mode".to_string());
    }
    Ok(build_circle_mesh(params))
}

pub fn apply_to_geometry(params: &NodeParams) -> Result<Geometry, String> {
    let output = params.get_string("output", "curve").to_lowercase();
    if output.contains("mesh") {
        Ok(Geometry::with_mesh(build_circle_mesh(params)))
    } else {
        let points = build_circle_points(params);
        Ok(Geometry::with_curve(points, true))
    }
}

fn build_circle_points(params: &NodeParams) -> Vec<[f32; 3]> {
    let radius = params.get_float("radius", DEFAULT_RADIUS).max(0.0);
    let segments = params
        .get_int("segments", DEFAULT_SEGMENTS)
        .clamp(3, 10_000) as usize;
    let center = Vec3::from(params.get_vec3("center", [0.0, 0.0, 0.0]));
    let mut points = Vec::with_capacity(segments);
    for i in 0..segments {
        let t = i as f32 / segments as f32;
        let angle = std::f32::consts::TAU * t;
        let x = angle.cos() * radius;
        let z = angle.sin() * radius;
        points.push((center + Vec3::new(x, 0.0, z)).to_array());
    }
    points
}

fn build_circle_mesh(params: &NodeParams) -> Mesh {
    let radius = params.get_float("radius", DEFAULT_RADIUS).max(0.0);
    let segments = params
        .get_int("segments", DEFAULT_SEGMENTS)
        .clamp(3, 10_000) as usize;
    let center = Vec3::from(params.get_vec3("center", [0.0, 0.0, 0.0]));

    let mut positions = Vec::with_capacity(segments);
    let mut indices = Vec::with_capacity(segments);
    for i in 0..segments {
        let t = i as f32 / segments as f32;
        let angle = std::f32::consts::TAU * t;
        let x = angle.cos() * radius;
        let z = angle.sin() * radius;
        positions.push([x, 0.0, z]);
    }

    indices.extend((0..segments).map(|i| i as u32));

    let mut mesh = Mesh::with_positions_faces(positions, indices, vec![segments as u32]);
    mesh.transform(Mat4::from_translation(center));
    if mesh.normals.is_none() {
        mesh.compute_normals();
    }
    mesh
}
