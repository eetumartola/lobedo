use std::collections::BTreeMap;

use glam::Vec3;

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::curve::Curve;
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    geometry_in,
    geometry_out,
    group_utils::{mask_has_any, mesh_group_mask},
    require_mesh_input,
};

pub const NAME: &str = "PolyFrame";

const DEFAULT_NORMAL_NAME: &str = "N";
const DEFAULT_TANGENT_NAME: &str = "tangent";
const DEFAULT_BITANGENT_NAME: &str = "bitangent";

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
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
            (
                "normal".to_string(),
                ParamValue::String(DEFAULT_NORMAL_NAME.to_string()),
            ),
            (
                "tangent".to_string(),
                ParamValue::String(DEFAULT_TANGENT_NAME.to_string()),
            ),
            (
                "bitangent".to_string(),
                ParamValue::String(DEFAULT_BITANGENT_NAME.to_string()),
            ),
            ("coherent".to_string(), ParamValue::Bool(false)),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {  
    let mut input = require_mesh_input(inputs, 0, "PolyFrame requires a mesh input")?;
    apply_polyframe(params, &mut input, &[])?;
    Ok(input)
}

pub fn apply_to_geometry(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };

    let mut meshes = Vec::new();
    if let Some(mut mesh) = input.merged_mesh() {
        apply_polyframe(params, &mut mesh, &input.curves)?;
        meshes.push(mesh);
    }

    let curves = if meshes.is_empty() {
        input.curves.clone()
    } else {
        input.curves.clone()
    };

    Ok(Geometry {
        meshes,
        splats: input.splats.clone(),
        curves,
        volumes: input.volumes.clone(),
        materials: input.materials.clone(),
    })
}

fn apply_polyframe(params: &NodeParams, input: &mut Mesh, curves: &[Curve]) -> Result<(), String> {
    let point_count = input.positions.len();
    if point_count == 0 {
        return Ok(());
    }

    let mask = mesh_group_mask(input, params, AttributeDomain::Point);
    if !mask_has_any(mask.as_deref()) {
        return Ok(());
    }

    let normal_name = params.get_string("normal", DEFAULT_NORMAL_NAME).trim();
    let tangent_name = params
        .get_string("tangent", DEFAULT_TANGENT_NAME)
        .trim();
    let bitangent_name = params
        .get_string("bitangent", DEFAULT_BITANGENT_NAME)
        .trim();
    let coherent = params.get_bool("coherent", false);
    let epsilon = 1.0e-6;

    if normal_name.is_empty() && tangent_name.is_empty() && bitangent_name.is_empty() {
        return Ok(());
    }

    let positions: Vec<Vec3> = input.positions.iter().copied().map(Vec3::from).collect();
    let face_counts = if input.face_counts.is_empty() {
        if input.indices.len().is_multiple_of(3) {
            vec![3u32; input.indices.len() / 3]
        } else if input.indices.is_empty() {
            Vec::new()
        } else {
            vec![input.indices.len() as u32]
        }
    } else {
        input.face_counts.clone()
    };

    let mut normal_sum = vec![Vec3::ZERO; point_count];
    let mut poly_tangent_sum = vec![Vec3::ZERO; point_count];
    let mut poly_connected = vec![false; point_count];
    let mut curve_tangent_sum = vec![Vec3::ZERO; point_count];
    let mut curve_bitangent_sum = vec![Vec3::ZERO; point_count];
    let mut curve_connected = vec![false; point_count];

    let mut cursor = 0usize;
    for &count in &face_counts {
        let count = count as usize;
        if count < 2 || cursor + count > input.indices.len() {
            cursor = cursor.saturating_add(count);
            continue;
        }
        let mut face_positions = Vec::with_capacity(count);
        for i in 0..count {
            let idx = input.indices[cursor + i] as usize;
            face_positions.push(*positions.get(idx).unwrap_or(&Vec3::ZERO));
        }
        let face_normal = if count >= 3 {
            newell_normal(&face_positions)
        } else {
            Vec3::ZERO
        };

        for i in 0..count {
            let idx = input.indices[cursor + i] as usize;
            if idx >= point_count {
                continue;
            }
            let next = input.indices[cursor + (i + 1) % count] as usize;
            let p0 = positions.get(idx).copied().unwrap_or(Vec3::ZERO);
            let p1 = positions.get(next).copied().unwrap_or(Vec3::ZERO);
            let edge = p1 - p0;
            if edge.length_squared() > 0.0 {
                poly_tangent_sum[idx] += edge;
            }
            if face_normal.length_squared() > 0.0 {
                normal_sum[idx] += face_normal;
            }
            poly_connected[idx] = true;
        }

        cursor += count;
    }

    for curve in curves {
        if curve.indices.len() < 2 {
            continue;
        }
        let count = curve.indices.len();
        let mut tangents = vec![Vec3::ZERO; count];
        let mut curvatures = vec![Vec3::ZERO; count];
        for i in 0..count {
            let idx = curve.indices[i] as usize;
            if idx >= point_count {
                continue;
            }
            let prev_index = if i == 0 {
                if curve.closed {
                    curve.indices[count - 1] as usize
                } else {
                    idx
                }
            } else {
                curve.indices[i - 1] as usize
            };
            let next_index = if i + 1 < count {
                curve.indices[i + 1] as usize
            } else if curve.closed {
                curve.indices[0] as usize
            } else {
                idx
            };
            let current = positions.get(idx).copied().unwrap_or(Vec3::ZERO);
            let prev = positions.get(prev_index).copied().unwrap_or(current);
            let next = positions.get(next_index).copied().unwrap_or(current);
            let in_dir = (current - prev).normalize_or_zero();
            let out_dir = (next - current).normalize_or_zero();
            let tangent = if !curve.closed && i == 0 {
                out_dir
            } else if !curve.closed && i + 1 == count {
                in_dir
            } else {
                (in_dir + out_dir).normalize_or_zero()
            };
            tangents[i] = tangent;
            if in_dir.length_squared() > 0.0 && out_dir.length_squared() > 0.0 {
                curvatures[i] = out_dir - in_dir;
            }
        }
        let bitangents = build_curve_bitangents(&curvatures, curve.closed, epsilon, coherent);
        for i in 0..count {
            let idx = curve.indices[i] as usize;
            if idx >= point_count {
                continue;
            }
            let tangent = tangents[i];
            if tangent.length_squared() > 0.0 {
                curve_tangent_sum[idx] += tangent;
                curve_connected[idx] = true;
            }
            let bitangent = bitangents[i];
            if bitangent.length_squared() > 0.0 {
                curve_bitangent_sum[idx] += bitangent;
            }
        }
    }

    let mut normal_values = if normal_name.is_empty() {
        Vec::new()
    } else {
        existing_vec3_attr_mesh(input, normal_name, point_count)
    };
    let mut tangent_values = if tangent_name.is_empty() {
        Vec::new()
    } else {
        existing_vec3_attr_mesh(input, tangent_name, point_count)
    };
    let mut bitangent_values = if bitangent_name.is_empty() {
        Vec::new()
    } else {
        existing_vec3_attr_mesh(input, bitangent_name, point_count)
    };

    let axis_normal = Vec3::Y;
    let axis_tangent = Vec3::X;
    let axis_bitangent = Vec3::Z;
    for i in 0..point_count {
        let apply = mask
            .as_ref()
            .map(|mask| mask.get(i).copied().unwrap_or(false))
            .unwrap_or(true);
        if !apply {
            continue;
        }
        let (normal, tangent, bitangent) = if curve_connected.get(i).copied().unwrap_or(false) {
            let mut tangent = curve_tangent_sum[i].normalize_or_zero();
            if tangent.length_squared() < epsilon {
                tangent = axis_tangent;
            }
            let mut bitangent = curve_bitangent_sum[i].normalize_or_zero();
            if bitangent.length_squared() < epsilon {
                let fallback_normal = if normal_sum[i].length_squared() > epsilon {
                    normal_sum[i].normalize()
                } else {
                    axis_normal
                };
                bitangent = fallback_normal.cross(tangent).normalize_or_zero();
                if bitangent.length_squared() < epsilon {
                    bitangent = axis_bitangent;
                }
            }
            let mut normal = tangent.cross(bitangent).normalize_or_zero();
            if normal.length_squared() < epsilon {
                normal = axis_normal;
            }
            (normal, tangent, bitangent)
        } else if poly_connected.get(i).copied().unwrap_or(false) {
            build_frame(
                normal_sum[i],
                poly_tangent_sum[i],
                axis_normal,
                axis_tangent,
                axis_bitangent,
            )
        } else {
            (axis_normal, axis_tangent, axis_bitangent)
        };
        if !normal_values.is_empty() {
            if let Some(slot) = normal_values.get_mut(i) {
                *slot = normal.to_array();
            }
        }
        if !tangent_values.is_empty() {
            if let Some(slot) = tangent_values.get_mut(i) {
                *slot = tangent.to_array();
            }
        }
        if !bitangent_values.is_empty() {
            if let Some(slot) = bitangent_values.get_mut(i) {
                *slot = bitangent.to_array();
            }
        }
    }

    if !normal_values.is_empty() {
        input
            .set_attribute(
                AttributeDomain::Point,
                normal_name,
                AttributeStorage::Vec3(normal_values),
            )
            .map_err(|err| format!("PolyFrame normal attribute error: {:?}", err))?;
    }
    if !tangent_values.is_empty() {
        input
            .set_attribute(
                AttributeDomain::Point,
                tangent_name,
                AttributeStorage::Vec3(tangent_values),
            )
            .map_err(|err| format!("PolyFrame tangent attribute error: {:?}", err))?;
    }
    if !bitangent_values.is_empty() {
        input
            .set_attribute(
                AttributeDomain::Point,
                bitangent_name,
                AttributeStorage::Vec3(bitangent_values),
            )
            .map_err(|err| format!("PolyFrame bitangent attribute error: {:?}", err))?;
    }

    Ok(())
}

fn existing_vec3_attr_mesh(mesh: &Mesh, name: &str, count: usize) -> Vec<[f32; 3]> {
    if let Some(crate::attributes::AttributeRef::Vec3(values)) =
        mesh.attribute(AttributeDomain::Point, name)
    {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![[0.0, 0.0, 0.0]; count]
}

fn newell_normal(points: &[Vec3]) -> Vec3 {
    if points.len() < 3 {
        return Vec3::ZERO;
    }
    let mut normal = Vec3::ZERO;
    for i in 0..points.len() {
        let current = points[i];
        let next = points[(i + 1) % points.len()];
        normal.x += (current.y - next.y) * (current.z + next.z);
        normal.y += (current.z - next.z) * (current.x + next.x);
        normal.z += (current.x - next.x) * (current.y + next.y);
    }
    if normal.length_squared() > 0.0 {
        normal.normalize()
    } else {
        Vec3::ZERO
    }
}

fn build_frame(
    normal_sum: Vec3,
    tangent_sum: Vec3,
    axis_normal: Vec3,
    axis_tangent: Vec3,
    axis_bitangent: Vec3,
) -> (Vec3, Vec3, Vec3) {
    let mut normal = if normal_sum.length_squared() > 0.0 {
        normal_sum.normalize()
    } else {
        axis_normal
    };
    if normal.length_squared() == 0.0 {
        normal = axis_normal;
    }
    let mut tangent = if tangent_sum.length_squared() > 0.0 {
        tangent_sum.normalize()
    } else {
        axis_tangent
    };
    tangent = (tangent - normal * tangent.dot(normal)).normalize_or_zero();
    if tangent.length_squared() < 1.0e-6 {
        tangent = if normal.y.abs() < 0.9 {
            normal.cross(Vec3::Y).normalize_or_zero()
        } else {
            normal.cross(Vec3::X).normalize_or_zero()
        };
    }
    if tangent.length_squared() < 1.0e-6 {
        tangent = axis_tangent;
    }
    let mut bitangent = normal.cross(tangent).normalize_or_zero();
    if bitangent.length_squared() < 1.0e-6 {
        bitangent = axis_bitangent;
    }
    (normal, tangent, bitangent)
}

fn build_curve_bitangents(
    curvatures: &[Vec3],
    closed: bool,
    epsilon: f32,
    coherent: bool,
) -> Vec<Vec3> {
    let count = curvatures.len();
    let mut out = vec![Vec3::ZERO; count];
    if count == 0 {
        return out;
    }

    let raw: Vec<Vec3> = curvatures
        .iter()
        .map(|c| {
            if c.length_squared() > epsilon {
                c.normalize()
            } else {
                Vec3::ZERO
            }
        })
        .collect();

    if !coherent {
        return raw;
    }

    let anchors: Vec<usize> = raw
        .iter()
        .enumerate()
        .filter_map(|(idx, v)| {
            if v.length_squared() > epsilon {
                Some(idx)
            } else {
                None
            }
        })
        .collect();

    if anchors.is_empty() {
        return out;
    }

    let mut oriented = vec![Vec3::ZERO; count];
    let first = anchors[0];
    let mut prev = raw[first];
    oriented[first] = prev;
    for &idx in anchors.iter().skip(1) {
        let mut current = raw[idx];
        if current.dot(prev) < 0.0 {
            current = -current;
        }
        oriented[idx] = current;
        prev = current;
    }

    if anchors.len() == 1 {
        let value = oriented[first];
        for slot in &mut out {
            *slot = value;
        }
        return out;
    }

    let mut fill_segment = |start_idx: usize, end_idx: usize, start: Vec3, end: Vec3| {
        if start_idx == end_idx {
            out[start_idx] = start;
            return;
        }
        let span = if end_idx > start_idx {
            end_idx - start_idx
        } else {
            (count - start_idx) + end_idx
        };
        if span == 0 {
            return;
        }
        for step in 0..=span {
            let t = step as f32 / span as f32;
            let blended = (start * (1.0 - t) + end * t).normalize_or_zero();
            let value = if blended.length_squared() > epsilon {
                blended
            } else {
                start
            };
            let idx = (start_idx + step) % count;
            out[idx] = value;
        }
    };

    if closed {
        for window in anchors.iter().zip(anchors.iter().cycle().skip(1)).take(anchors.len()) {
            let (start_idx, end_idx) = (*window.0, *window.1);
            let start = oriented[start_idx];
            let end = oriented[end_idx];
            fill_segment(start_idx, end_idx, start, end);
        }
    } else {
        let first = anchors[0];
        let last = *anchors.last().unwrap();
        let first_value = oriented[first];
        for idx in 0..=first {
            out[idx] = first_value;
        }
        for pair in anchors.windows(2) {
            let start_idx = pair[0];
            let end_idx = pair[1];
            let start = oriented[start_idx];
            let end = oriented[end_idx];
            fill_segment(start_idx, end_idx, start, end);
        }
        let last_value = oriented[last];
        for idx in last..count {
            out[idx] = last_value;
        }
    }

    out
}
