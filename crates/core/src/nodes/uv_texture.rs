use std::collections::BTreeMap;

use glam::Vec3;

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{geometry_in, geometry_out, require_mesh_input};
use crate::param_spec::ParamSpec;

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

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::int_enum(
            "projection",
            "Projection",
            vec![
                (0, "Planar"),
                (1, "Box"),
                (2, "Cylindrical"),
                (3, "Spherical"),
            ],
        )
        .with_help("Projection type."),
        ParamSpec::int_enum("axis", "Axis", vec![(0, "X"), (1, "Y"), (2, "Z")])
            .with_help("Primary projection axis."),
        ParamSpec::vec2("scale", "Scale").with_help("UV scale."),
        ParamSpec::vec2("offset", "Offset").with_help("UV offset."),
    ]
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

    let triangulation = mesh.triangulate();
    if triangulation.indices.is_empty() {
        return;
    }
    let tri_indices = triangulation.indices;
    let tri_corners = triangulation.corner_indices;
    let tri_faces = triangulation.tri_to_face;
    let tri_count = tri_indices.len() / 3;

    let face_normals = if projection == 1 {
        Some(compute_face_normals(mesh))
    } else {
        None
    };

    let mut corner_uvs = vec![[0.0, 0.0]; mesh.indices.len()];
    for tri_index in 0..tri_count {
        let base = tri_index * 3;
        let face_index = *tri_faces.get(tri_index).unwrap_or(&tri_index);
        let tri_normal = face_normals
            .as_ref()
            .and_then(|normals| normals.get(face_index).copied());
        for corner_offset in 0..3 {
            let idx = *tri_indices.get(base + corner_offset).unwrap_or(&0) as usize;
            let corner_idx = *tri_corners
                .get(base + corner_offset)
                .unwrap_or(&(base + corner_offset));
            let position = mesh
                .positions
                .get(idx)
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
            if let Some(slot) = corner_uvs.get_mut(corner_idx) {
                *slot = apply_uv_scale_offset(uv, scale, offset);
            }
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
    let face_counts = if mesh.face_counts.is_empty() {
        if mesh.indices.len().is_multiple_of(3) {
            vec![3u32; mesh.indices.len() / 3]
        } else if mesh.indices.is_empty() {
            Vec::new()
        } else {
            vec![mesh.indices.len() as u32]
        }
    } else {
        mesh.face_counts.clone()
    };

    let mut normals = Vec::with_capacity(face_counts.len());
    let mut cursor = 0usize;
    for &count in &face_counts {
        let count = count as usize;
        if count < 3 || cursor + count > mesh.indices.len() {
            normals.push(Vec3::Y);
            cursor = cursor.saturating_add(count);
            continue;
        }
        let mut normal = Vec3::ZERO;
        for i in 0..count {
            let a_idx = mesh.indices[cursor + i] as usize;
            let b_idx = mesh.indices[cursor + (i + 1) % count] as usize;
            let a = Vec3::from(*mesh.positions.get(a_idx).unwrap_or(&[0.0, 0.0, 0.0]));
            let b = Vec3::from(*mesh.positions.get(b_idx).unwrap_or(&[0.0, 0.0, 0.0]));
            normal.x += (a.y - b.y) * (a.z + b.z);
            normal.y += (a.z - b.z) * (a.x + b.x);
            normal.z += (a.x - b.x) * (a.y + b.y);
        }
        if normal.length_squared() <= 1.0e-6 && count >= 3 {
            let a = Vec3::from(*mesh.positions.get(mesh.indices[cursor] as usize).unwrap_or(&[0.0, 0.0, 0.0]));
            let b = Vec3::from(*mesh.positions.get(mesh.indices[cursor + 1] as usize).unwrap_or(&[0.0, 0.0, 0.0]));
            let c = Vec3::from(*mesh.positions.get(mesh.indices[cursor + 2] as usize).unwrap_or(&[0.0, 0.0, 0.0]));
            normal = (b - a).cross(c - a);
        }
        normals.push(if normal.length_squared() > 1.0e-6 { normal.normalize() } else { Vec3::Y });
        cursor += count;
    }
    normals
}
