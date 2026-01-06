use std::collections::BTreeMap;

use glam::Vec3;

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{geometry_in, geometry_out, require_mesh_input};

pub const NAME: &str = "UV Texture";

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
            ("projection".to_string(), ParamValue::Int(0)),
            ("axis".to_string(), ParamValue::Int(1)),
            ("scale".to_string(), ParamValue::Vec2([1.0, 1.0])),
            ("offset".to_string(), ParamValue::Vec2([0.0, 0.0])),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut mesh = require_mesh_input(inputs, 0, "UV Texture requires a mesh input")?;
    apply_uv_texture(params, &mut mesh);
    Ok(mesh)
}

fn apply_uv_texture(params: &NodeParams, mesh: &mut Mesh) {
    if mesh.positions.is_empty() {
        return;
    }

    let projection = params.get_int("projection", 0).clamp(0, 3);
    let axis = params.get_int("axis", 1).clamp(0, 2);
    let scale = params.get_vec2("scale", [1.0, 1.0]);
    let offset = params.get_vec2("offset", [0.0, 0.0]);

    let bounds = mesh.bounds();
    let (min, max) = if let Some(bounds) = bounds {
        (Vec3::from(bounds.min), Vec3::from(bounds.max))
    } else {
        (Vec3::ZERO, Vec3::ZERO)
    };
    let size = (max - min).max(Vec3::splat(1.0e-6));

    if mesh.indices.is_empty() {
        let mut uvs = Vec::with_capacity(mesh.positions.len());
        for position in &mesh.positions {
            let uv = project_uv(
                Vec3::from(*position),
                None,
                projection,
                axis,
                min,
                size,
            );
            uvs.push(apply_uv_scale_offset(uv, scale, offset));
        }
        let _ = mesh.set_attribute(
            AttributeDomain::Point,
            "uv",
            AttributeStorage::Vec2(uvs.clone()),
        );
        mesh.uvs = Some(uvs);
        return;
    }

    let face_normals = if projection == 1 {
        Some(compute_face_normals(mesh))
    } else {
        None
    };

    let mut corner_uvs = Vec::with_capacity(mesh.indices.len());
    for (tri_idx, tri) in mesh.indices.chunks_exact(3).enumerate() {
        let tri_normal = face_normals
            .as_ref()
            .and_then(|normals| normals.get(tri_idx).copied());
        for &idx in tri {
            let position = mesh
                .positions
                .get(idx as usize)
                .copied()
                .unwrap_or([0.0, 0.0, 0.0]);
            let uv = project_uv(
                Vec3::from(position),
                tri_normal,
                projection,
                axis,
                min,
                size,
            );
            corner_uvs.push(apply_uv_scale_offset(uv, scale, offset));
        }
    }

    let _ = mesh.set_attribute(
        AttributeDomain::Vertex,
        "uv",
        AttributeStorage::Vec2(corner_uvs),
    );
}

fn apply_uv_scale_offset(uv: [f32; 2], scale: [f32; 2], offset: [f32; 2]) -> [f32; 2] {
    [uv[0] * scale[0] + offset[0], uv[1] * scale[1] + offset[1]]
}

fn project_uv(
    position: Vec3,
    face_normal: Option<Vec3>,
    projection: i32,
    axis: i32,
    min: Vec3,
    size: Vec3,
) -> [f32; 2] {
    match projection {
        1 => box_uv(position, face_normal, min, size),
        2 => cylindrical_uv(position, axis, min, size),
        3 => spherical_uv(position, axis),
        _ => planar_uv(position, axis, min, size),
    }
}

fn planar_uv(position: Vec3, axis: i32, min: Vec3, size: Vec3) -> [f32; 2] {
    match axis {
        0 => {
            let u = (position.z - min.z) / size.z;
            let v = (position.y - min.y) / size.y;
            [u, v]
        }
        2 => {
            let u = (position.x - min.x) / size.x;
            let v = (position.y - min.y) / size.y;
            [u, v]
        }
        _ => {
            let u = (position.x - min.x) / size.x;
            let v = (position.z - min.z) / size.z;
            [u, v]
        }
    }
}

fn box_uv(position: Vec3, face_normal: Option<Vec3>, min: Vec3, size: Vec3) -> [f32; 2] {
    let normal = face_normal.unwrap_or_else(|| {
        let centered = position - (min + size * 0.5);
        let abs = centered.abs();
        if abs.x >= abs.y && abs.x >= abs.z {
            Vec3::X * centered.x.signum()
        } else if abs.y >= abs.z {
            Vec3::Y * centered.y.signum()
        } else {
            Vec3::Z * centered.z.signum()
        }
    });
    if normal.x.abs() >= normal.y.abs() && normal.x.abs() >= normal.z.abs() {
        let u = (position.z - min.z) / size.z;
        let v = (position.y - min.y) / size.y;
        [u, v]
    } else if normal.y.abs() >= normal.z.abs() {
        let u = (position.x - min.x) / size.x;
        let v = (position.z - min.z) / size.z;
        [u, v]
    } else {
        let u = (position.x - min.x) / size.x;
        let v = (position.y - min.y) / size.y;
        [u, v]
    }
}

fn cylindrical_uv(position: Vec3, axis: i32, min: Vec3, size: Vec3) -> [f32; 2] {
    match axis {
        0 => {
            let angle = position.z.atan2(position.y);
            let u = angle / std::f32::consts::TAU + 0.5;
            let v = (position.x - min.x) / size.x;
            [u, v]
        }
        2 => {
            let angle = position.y.atan2(position.x);
            let u = angle / std::f32::consts::TAU + 0.5;
            let v = (position.z - min.z) / size.z;
            [u, v]
        }
        _ => {
            let angle = position.z.atan2(position.x);
            let u = angle / std::f32::consts::TAU + 0.5;
            let v = (position.y - min.y) / size.y;
            [u, v]
        }
    }
}

fn spherical_uv(position: Vec3, axis: i32) -> [f32; 2] {
    let dir = match axis {
        0 => Vec3::new(position.y, position.z, position.x),
        2 => Vec3::new(position.x, position.y, position.z),
        _ => Vec3::new(position.x, position.z, position.y),
    };
    let r = dir.length().max(1.0e-6);
    let theta = dir.z.atan2(dir.x);
    let phi = (dir.y / r).clamp(-1.0, 1.0).acos();
    let u = theta / std::f32::consts::TAU + 0.5;
    let v = phi / std::f32::consts::PI;
    [u, v]
}

fn compute_face_normals(mesh: &Mesh) -> Vec<Vec3> {
    let mut normals = Vec::with_capacity(mesh.indices.len() / 3);
    for tri in mesh.indices.chunks_exact(3) {
        let a = mesh.positions.get(tri[0] as usize).copied().unwrap_or([0.0, 0.0, 0.0]);
        let b = mesh.positions.get(tri[1] as usize).copied().unwrap_or([0.0, 0.0, 0.0]);
        let c = mesh.positions.get(tri[2] as usize).copied().unwrap_or([0.0, 0.0, 0.0]);
        let n = (Vec3::from(b) - Vec3::from(a)).cross(Vec3::from(c) - Vec3::from(a));
        normals.push(if n.length_squared() > 1.0e-6 { n.normalize() } else { Vec3::Y });
    }
    normals
}
