use std::collections::BTreeMap;

use glam::{Mat3, Vec3};

use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{geometry_in, geometry_out};

pub const NAME: &str = "Sweep";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![geometry_in("profile"), geometry_in("path")],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            ("profile_closed".to_string(), ParamValue::Bool(true)),
            ("path_closed".to_string(), ParamValue::Bool(false)),
            ("up".to_string(), ParamValue::Vec3([0.0, 1.0, 0.0])),
        ]),
    }
}

pub fn apply_to_geometry(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let profile_geo = inputs
        .first()
        .ok_or_else(|| "Sweep requires a profile input".to_string())?;
    let path_geo = inputs
        .get(1)
        .ok_or_else(|| "Sweep requires a path input".to_string())?;

    let (profile_points, profile_closed) = resolve_profile(profile_geo, params)?;
    let (path_points, path_closed) = resolve_path(path_geo, params)?;

    if profile_points.len() < 2 {
        return Err("Sweep profile needs at least two points".to_string());
    }
    if path_points.len() < 2 {
        return Err("Sweep path needs at least two points".to_string());
    }

    let up = Vec3::from(params.get_vec3("up", [0.0, 1.0, 0.0]));
    let mesh = sweep_points(&profile_points, profile_closed, &path_points, path_closed, up);
    Ok(Geometry::with_mesh(mesh))
}

fn resolve_profile(
    geometry: &Geometry,
    params: &NodeParams,
) -> Result<(Vec<Vec3>, bool), String> {
    if let Some(curve) = geometry.curves.first() {
        let mesh = geometry
            .merged_mesh()
            .ok_or_else(|| "Profile curve has no point pool".to_string())?;
        let points = curve
            .resolved_points(&mesh.positions)
            .into_iter()
            .map(Vec3::from)
            .collect::<Vec<_>>();
        return Ok((points, curve.closed));
    }

    if let Some(mesh) = geometry.merged_mesh() {
        let points = mesh
            .positions
            .iter()
            .copied()
            .map(Vec3::from)
            .collect::<Vec<_>>();
        let closed = params.get_bool("profile_closed", true);
        return Ok((points, closed));
    }

    Err("Sweep profile must contain a curve or mesh points".to_string())
}

fn resolve_path(
    geometry: &Geometry,
    params: &NodeParams,
) -> Result<(Vec<Vec3>, bool), String> {
    if let Some(curve) = geometry.curves.first() {
        let mesh = geometry
            .merged_mesh()
            .ok_or_else(|| "Path curve has no point pool".to_string())?;
        let points = curve
            .resolved_points(&mesh.positions)
            .into_iter()
            .map(Vec3::from)
            .collect::<Vec<_>>();
        return Ok((points, curve.closed));
    }

    if let Some(mesh) = geometry.merged_mesh() {
        let points = mesh
            .positions
            .iter()
            .copied()
            .map(Vec3::from)
            .collect::<Vec<_>>();
        let closed = params.get_bool("path_closed", false);
        return Ok((points, closed));
    }

    Err("Sweep path must contain a curve or mesh points".to_string())
}

fn sweep_points(
    profile: &[Vec3],
    profile_closed: bool,
    path: &[Vec3],
    path_closed: bool,
    up: Vec3,
) -> Mesh {
    let (profile_origin, profile_frame) = profile_frame(profile);
    let profile_coords = profile
        .iter()
        .map(|p| profile_frame.transpose() * (*p - profile_origin))
        .collect::<Vec<_>>();

    let ring_len = profile_coords.len();
    let path_len = path.len();
    let mut positions = Vec::with_capacity(ring_len * path_len);

    for i in 0..path_len {
        let tangent = path_tangent(path, i, path_closed);
        let (normal, binormal) = frame_from_tangent(tangent, up);
        for coord in &profile_coords {
            let world = path[i] + normal * coord.x + binormal * coord.y;
            positions.push(world.to_array());
        }
    }

    let mut indices = Vec::new();
    let profile_segments = if profile_closed { ring_len } else { ring_len.saturating_sub(1) };
    let path_segments = if path_closed { path_len } else { path_len.saturating_sub(1) };

    for path_idx in 0..path_segments {
        let next_path = if path_idx + 1 < path_len { path_idx + 1 } else { 0 };
        for seg in 0..profile_segments {
            let next_seg = if seg + 1 < ring_len { seg + 1 } else { 0 };
            let a = (path_idx * ring_len + seg) as u32;
            let b = (next_path * ring_len + seg) as u32;
            let c = (next_path * ring_len + next_seg) as u32;
            let d = (path_idx * ring_len + next_seg) as u32;
            indices.extend_from_slice(&[a, b, c, a, c, d]);
        }
    }

    let mut mesh = Mesh::with_positions_indices(positions, indices);
    let _ = mesh.compute_normals();
    mesh
}

fn profile_frame(points: &[Vec3]) -> (Vec3, Mat3) {
    let mut centroid = Vec3::ZERO;
    for p in points {
        centroid += *p;
    }
    centroid /= points.len().max(1) as f32;

    let normal = profile_normal(points).unwrap_or(Vec3::Y);
    let axis_x = profile_axis(points, normal);
    let axis_y = normal.cross(axis_x).normalize_or_zero();
    let frame = Mat3::from_cols(axis_x, axis_y, normal);
    (centroid, frame)
}

fn profile_normal(points: &[Vec3]) -> Option<Vec3> {
    if points.len() < 3 {
        return None;
    }
    for i in 0..points.len() - 2 {
        let v0 = points[i + 1] - points[i];
        let v1 = points[i + 2] - points[i];
        let n = v0.cross(v1);
        if n.length_squared() > 1.0e-8 {
            return Some(n.normalize());
        }
    }
    None
}

fn profile_axis(points: &[Vec3], normal: Vec3) -> Vec3 {
    for i in 0..points.len().saturating_sub(1) {
        let edge = points[i + 1] - points[i];
        if edge.length_squared() > 1.0e-8 {
            let projected = edge - normal * edge.dot(normal);
            if projected.length_squared() > 1.0e-8 {
                return projected.normalize();
            }
        }
    }
    let fallback = if normal.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    normal.cross(fallback).normalize_or_zero()
}

fn path_tangent(path: &[Vec3], index: usize, closed: bool) -> Vec3 {
    let count = path.len();
    let prev = if index == 0 {
        if closed { count - 1 } else { 0 }
    } else {
        index - 1
    };
    let next = if index + 1 >= count {
        if closed { 0 } else { count - 1 }
    } else {
        index + 1
    };
    let dir = path[next] - path[prev];
    if dir.length_squared() > 1.0e-8 {
        dir.normalize()
    } else {
        Vec3::X
    }
}

fn frame_from_tangent(tangent: Vec3, up: Vec3) -> (Vec3, Vec3) {
    let mut up = if up.length_squared() > 1.0e-8 {
        up.normalize()
    } else {
        Vec3::Y
    };
    if tangent.cross(up).length_squared() < 1.0e-6 {
        up = if tangent.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    }
    let binormal = tangent.cross(up).normalize_or_zero();
    let normal = binormal.cross(tangent).normalize_or_zero();
    (normal, binormal)
}
