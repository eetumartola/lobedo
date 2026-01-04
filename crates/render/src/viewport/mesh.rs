use crate::scene::{RenderMesh, SelectionShape};
use egui_wgpu::wgpu;
use glam::{Mat3, Mat4, Vec2, Vec3};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Vertex {
    pub(crate) position: [f32; 3],
    pub(crate) normal: [f32; 3],
    pub(crate) color: [f32; 3],
}

pub(crate) const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3];

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct LineVertex {
    pub(crate) position: [f32; 3],
    pub(crate) color: [f32; 3],
}

pub(crate) const LINE_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct SplatVertex {
    pub(crate) center: [f32; 3],
    pub(crate) offset: [f32; 2],
    pub(crate) uv: [f32; 2],
    pub(crate) color: [f32; 4],
}

pub(crate) const SPLAT_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x2, 3 => Float32x4];

pub(crate) struct SplatBillboardInputs<'a> {
    pub(crate) positions: &'a [[f32; 3]],
    pub(crate) colors: &'a [[f32; 3]],
    pub(crate) opacities: &'a [f32],
    pub(crate) scales: &'a [[f32; 3]],
    pub(crate) rotations: &'a [[f32; 4]],
    pub(crate) view: Mat4,
    pub(crate) viewport: [f32; 2],
    pub(crate) fov_y: f32,
    pub(crate) world_transform: Mat3,
}

pub(crate) struct CubeMesh {
    pub(crate) vertices: Vec<Vertex>,
    pub(crate) indices: Vec<u32>,
    pub(crate) bounds_min: [f32; 3],
    pub(crate) bounds_max: [f32; 3],
}

pub(crate) fn cube_mesh() -> CubeMesh {
    let positions = [
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, 0.5, -0.5],
        [-0.5, 0.5, -0.5],
        [-0.5, -0.5, 0.5],
        [0.5, -0.5, 0.5],
        [0.5, 0.5, 0.5],
        [-0.5, 0.5, 0.5],
    ];

    let faces = [
        ([0, 1, 2, 3], [0.0, 0.0, -1.0]),
        ([4, 5, 6, 7], [0.0, 0.0, 1.0]),
        ([0, 1, 5, 4], [0.0, -1.0, 0.0]),
        ([2, 3, 7, 6], [0.0, 1.0, 0.0]),
        ([1, 2, 6, 5], [1.0, 0.0, 0.0]),
        ([3, 0, 4, 7], [-1.0, 0.0, 0.0]),
    ];

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for (face, normal) in faces {
        let base_index = vertices.len() as u32;
        for &idx in &face {
            vertices.push(Vertex {
                position: positions[idx],
                normal,
                color: [1.0, 1.0, 1.0],
            });
        }
        indices.extend_from_slice(&[
            base_index,
            base_index + 1,
            base_index + 2,
            base_index,
            base_index + 2,
            base_index + 3,
        ]);
    }

    let (bounds_min, bounds_max) = mesh_bounds(&vertices);

    CubeMesh {
        vertices,
        indices,
        bounds_min,
        bounds_max,
    }
}

fn mesh_bounds(vertices: &[Vertex]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for vertex in vertices {
        for i in 0..3 {
            min[i] = min[i].min(vertex.position[i]);
            max[i] = max[i].max(vertex.position[i]);
        }
    }
    (min, max)
}

pub(crate) fn bounds_from_positions(positions: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    if positions.is_empty() {
        return ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
    }
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for position in positions {
        for i in 0..3 {
            min[i] = min[i].min(position[i]);
            max[i] = max[i].max(position[i]);
        }
    }
    (min, max)
}

pub(crate) fn build_vertices(mesh: &RenderMesh) -> (Vec<Vertex>, Vec<u32>) {
    let corner_normals = mesh
        .corner_normals
        .as_ref()
        .filter(|normals| normals.len() == mesh.indices.len());
    let corner_colors = mesh
        .corner_colors
        .as_ref()
        .filter(|colors| colors.len() == mesh.indices.len());
    let use_corner_data = corner_normals.is_some() || corner_colors.is_some();

    let fallback_normal = [0.0, 1.0, 0.0];
    let fallback_color = [1.0, 1.0, 1.0];

    if use_corner_data && !mesh.indices.is_empty() {
        let mut vertices = Vec::with_capacity(mesh.indices.len());
        let mut indices = Vec::with_capacity(mesh.indices.len());
        for (idx, corner) in mesh.indices.iter().enumerate() {
            let position = mesh
                .positions
                .get(*corner as usize)
                .copied()
                .unwrap_or([0.0, 0.0, 0.0]);
            let normal = corner_normals
                .and_then(|normals| normals.get(idx).copied())
                .or_else(|| mesh.normals.get(*corner as usize).copied())
                .unwrap_or(fallback_normal);
            let color = corner_colors
                .and_then(|colors| colors.get(idx).copied())
                .or_else(|| {
                    mesh.colors
                        .as_ref()
                        .and_then(|colors| colors.get(*corner as usize).copied())
                })
                .unwrap_or(fallback_color);
            vertices.push(Vertex {
                position,
                normal,
                color,
            });
            indices.push(idx as u32);
        }
        return (vertices, indices);
    }

    let mut vertices = Vec::with_capacity(mesh.positions.len());
    for (index, position) in mesh.positions.iter().enumerate() {
        let normal = mesh.normals.get(index).copied().unwrap_or(fallback_normal);
        let color = mesh
            .colors
            .as_ref()
            .and_then(|colors| colors.get(index).copied())
            .unwrap_or(fallback_color);
        vertices.push(Vertex {
            position: *position,
            normal,
            color,
        });
    }
    (vertices, mesh.indices.clone())
}

pub(crate) fn normals_vertices(vertices: &[Vertex], length: f32) -> Vec<LineVertex> {
    let mut lines = Vec::with_capacity(vertices.len() * 2);
    let color = [1.0, 0.85, 0.3];
    for vertex in vertices {
        let start = vertex.position;
        let end = [
            vertex.position[0] + vertex.normal[0] * length,
            vertex.position[1] + vertex.normal[1] * length,
            vertex.position[2] + vertex.normal[2] * length,
        ];
        lines.push(LineVertex {
            position: start,
            color,
        });
        lines.push(LineVertex {
            position: end,
            color,
        });
    }
    lines
}

pub(crate) fn point_cross_vertices(positions: &[[f32; 3]], size: f32) -> Vec<LineVertex> {
    if positions.is_empty() || size <= 0.0 {
        return Vec::new();
    }
    point_cross_vertices_with_colors(positions, &[], size)
}

pub(crate) fn point_cross_vertices_with_colors(
    positions: &[[f32; 3]],
    colors: &[[f32; 3]],
    size: f32,
) -> Vec<LineVertex> {
    if positions.is_empty() || size <= 0.0 {
        return Vec::new();
    }
    let mut lines = Vec::with_capacity(positions.len() * 6);
    for (idx, p) in positions.iter().enumerate() {
        let color = colors.get(idx).copied().unwrap_or([0.9, 0.9, 0.9]);
        let [x, y, z] = *p;
        lines.push(LineVertex {
            position: [x - size, y, z],
            color,
        });
        lines.push(LineVertex {
            position: [x + size, y, z],
            color,
        });
        lines.push(LineVertex {
            position: [x, y - size, z],
            color,
        });
        lines.push(LineVertex {
            position: [x, y + size, z],
            color,
        });
        lines.push(LineVertex {
            position: [x, y, z - size],
            color,
        });
        lines.push(LineVertex {
            position: [x, y, z + size],
            color,
        });
    }
    lines
}

pub(crate) fn splat_billboard_vertices(
    inputs: SplatBillboardInputs<'_>,
) -> Vec<SplatVertex> {
    if inputs.positions.is_empty() {
        return Vec::new();
    }
    let width = inputs.viewport[0].max(1.0);
    let height = inputs.viewport[1].max(1.0);
    let tan_half = (inputs.fov_y * 0.5).tan().max(1.0e-6);
    let fy = 0.5 * height / tan_half;
    let fx = fy * (width / height);
    let view_rot = Mat3::from_mat4(inputs.view);
    let flip = Mat3::from_diagonal(Vec3::new(1.0, 1.0, -1.0));
    let radius = 3.0;

    let mut vertices = Vec::with_capacity(inputs.positions.len() * 6);
    for (idx, pos) in inputs.positions.iter().enumerate() {
        let center = inputs.world_transform * Vec3::from(*pos);
        let pos_view = inputs.view.transform_point3(center);
        let pos_cam = Vec3::new(pos_view.x, pos_view.y, -pos_view.z);
        if pos_cam.z <= 1.0e-6 || !pos_cam.z.is_finite() {
            continue;
        }
        let raw_scale = inputs
            .scales
            .get(idx)
            .copied()
            .unwrap_or([1.0, 1.0, 1.0]);
        let scale = if raw_scale[0] < 0.0 || raw_scale[1] < 0.0 || raw_scale[2] < 0.0 {
            [raw_scale[0].exp(), raw_scale[1].exp(), raw_scale[2].exp()]
        } else {
            raw_scale
        };
        if !scale[0].is_finite() || !scale[1].is_finite() || !scale[2].is_finite() {
            continue;
        }
        let rotation = inputs
            .rotations
            .get(idx)
            .copied()
            .unwrap_or([0.0, 0.0, 0.0, 1.0]);
        let quat =
            glam::Quat::from_xyzw(rotation[1], rotation[2], rotation[3], rotation[0]).normalize();
        let rot = Mat3::from_quat(quat);
        let scale = Vec3::from(scale);
        let cov_local = Mat3::from_diagonal(scale * scale);
        let cov_world = inputs.world_transform * (rot * cov_local * rot.transpose())
            * inputs.world_transform.transpose();
        let cov_view = view_rot * cov_world * view_rot.transpose();
        let cov_cam = flip * cov_view * flip;

        let cov = cov_cam.to_cols_array_2d();
        let c00 = cov[0][0];
        let c01 = cov[0][1];
        let c02 = cov[0][2];
        let c10 = cov[1][0];
        let c11 = cov[1][1];
        let c12 = cov[1][2];
        let c20 = cov[2][0];
        let c21 = cov[2][1];
        let c22 = cov[2][2];

        let z = pos_cam.z;
        let inv_z = 1.0 / z;
        let inv_z2 = inv_z * inv_z;
        let j11 = fx * inv_z;
        let j22 = fy * inv_z;
        let j13 = -fx * pos_cam.x * inv_z2;
        let j23 = -fy * pos_cam.y * inv_z2;

        let r0 = Vec3::new(j11, 0.0, j13);
        let r1 = Vec3::new(0.0, j22, j23);

        let cov_r0 = Vec3::new(
            c00 * r0.x + c01 * r0.y + c02 * r0.z,
            c10 * r0.x + c11 * r0.y + c12 * r0.z,
            c20 * r0.x + c21 * r0.y + c22 * r0.z,
        );
        let cov_r1 = Vec3::new(
            c00 * r1.x + c01 * r1.y + c02 * r1.z,
            c10 * r1.x + c11 * r1.y + c12 * r1.z,
            c20 * r1.x + c21 * r1.y + c22 * r1.z,
        );

        let a = r0.dot(cov_r0);
        let b = r0.dot(cov_r1);
        let c = r1.dot(cov_r1);
        if !a.is_finite() || !b.is_finite() || !c.is_finite() {
            continue;
        }

        let trace = a + c;
        let delta = ((a - c) * (a - c) + 4.0 * b * b).sqrt();
        let lambda1 = 0.5 * (trace + delta).max(0.0);
        let lambda2 = 0.5 * (trace - delta).max(0.0);
        let sigma1 = lambda1.sqrt();
        let sigma2 = lambda2.sqrt();
        if sigma1 <= 0.0 || sigma2 <= 0.0 {
            continue;
        }

        let mut v1 = if b.abs() > 1.0e-6 {
            Vec2::new(lambda1 - c, b).normalize_or_zero()
        } else if a >= c {
            Vec2::new(1.0, 0.0)
        } else {
            Vec2::new(0.0, 1.0)
        };
        if v1.length_squared() < 1.0e-6 {
            v1 = Vec2::new(1.0, 0.0);
        }
        let v2 = Vec2::new(-v1.y, v1.x);

        let axis1 = v1 * (sigma1 * radius);
        let axis2 = v2 * (sigma2 * radius);
        let axis1_ndc = Vec2::new(axis1.x * 2.0 / width, axis1.y * 2.0 / height);
        let axis2_ndc = Vec2::new(axis2.x * 2.0 / width, axis2.y * 2.0 / height);

        let color = inputs
            .colors
            .get(idx)
            .copied()
            .unwrap_or([1.0, 1.0, 1.0]);
        let alpha = inputs
            .opacities
            .get(idx)
            .copied()
            .unwrap_or(1.0)
            .clamp(0.0, 1.0);
        let color = [
            color[0].clamp(0.0, 1.0),
            color[1].clamp(0.0, 1.0),
            color[2].clamp(0.0, 1.0),
            alpha,
        ];

        let corners = [
            (-1.0, -1.0),
            (1.0, -1.0),
            (1.0, 1.0),
            (-1.0, 1.0),
        ];

        let mut verts = [SplatVertex {
            center: center.to_array(),
            offset: [0.0, 0.0],
            uv: [0.0, 0.0],
            color,
        }; 4];

        for (i, (sx, sy)) in corners.into_iter().enumerate() {
            let offset = axis1_ndc * sx + axis2_ndc * sy;
            verts[i] = SplatVertex {
                center: center.to_array(),
                offset: offset.to_array(),
                uv: [sx * radius, sy * radius],
                color,
            };
        }

        vertices.push(verts[0]);
        vertices.push(verts[1]);
        vertices.push(verts[2]);
        vertices.push(verts[0]);
        vertices.push(verts[2]);
        vertices.push(verts[3]);
    }
    vertices
}

pub(crate) fn wireframe_vertices(positions: &[[f32; 3]], indices: &[u32]) -> Vec<LineVertex> {
    if positions.is_empty() || indices.len() < 3 {
        return Vec::new();
    }
    let mut lines = Vec::with_capacity(indices.len() / 3 * 6);
    let color = [0.3, 0.75, 0.95];
    for tri in indices.chunks_exact(3) {
        let [a, b, c] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
        let (pa, pb, pc) = match (positions.get(a), positions.get(b), positions.get(c)) {
            (Some(pa), Some(pb), Some(pc)) => (*pa, *pb, *pc),
            _ => continue,
        };
        lines.push(LineVertex {
            position: pa,
            color,
        });
        lines.push(LineVertex {
            position: pb,
            color,
        });
        lines.push(LineVertex {
            position: pb,
            color,
        });
        lines.push(LineVertex {
            position: pc,
            color,
        });
        lines.push(LineVertex {
            position: pc,
            color,
        });
        lines.push(LineVertex {
            position: pa,
            color,
        });
    }
    lines
}

pub(crate) fn bounds_vertices(min: [f32; 3], max: [f32; 3]) -> Vec<LineVertex> {
    bounds_vertices_with_color(min, max, [0.85, 0.85, 0.9])
}

pub(crate) fn bounds_vertices_with_color(
    min: [f32; 3],
    max: [f32; 3],
    color: [f32; 3],
) -> Vec<LineVertex> {
    let [min_x, min_y, min_z] = min;
    let [max_x, max_y, max_z] = max;

    let corners = [
        [min_x, min_y, min_z],
        [max_x, min_y, min_z],
        [max_x, max_y, min_z],
        [min_x, max_y, min_z],
        [min_x, min_y, max_z],
        [max_x, min_y, max_z],
        [max_x, max_y, max_z],
        [min_x, max_y, max_z],
    ];

    let edges = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];

    let mut lines = Vec::with_capacity(edges.len() * 2);
    for (a, b) in edges {
        lines.push(LineVertex {
            position: corners[a],
            color,
        });
        lines.push(LineVertex {
            position: corners[b],
            color,
        });
    }
    lines
}

pub(crate) fn selection_shape_vertices(shape: &SelectionShape) -> Vec<LineVertex> {
    let color = [1.0, 0.85, 0.2];
    match shape {
        SelectionShape::Box { center, size } => {
            let center = Vec3::from(*center);
            let size = Vec3::from(*size);
            let half = size * 0.5;
            let min = (center - half).to_array();
            let max = (center + half).to_array();
            bounds_vertices_with_color(min, max, color)
        }
        SelectionShape::Sphere { center, size } => {
            let center = Vec3::from(*center);
            let mut radii = Vec3::from(*size) * 0.5;
            radii = Vec3::new(
                radii.x.abs().max(0.001),
                radii.y.abs().max(0.001),
                radii.z.abs().max(0.001),
            );
            circle_vertices(center, Vec3::X, Vec3::Y, radii.x, radii.y, color)
                .into_iter()
                .chain(circle_vertices(center, Vec3::X, Vec3::Z, radii.x, radii.z, color))
                .chain(circle_vertices(center, Vec3::Y, Vec3::Z, radii.y, radii.z, color))
                .collect()
        }
        SelectionShape::Plane {
            origin,
            normal,
            size,
        } => {
            let origin = Vec3::from(*origin);
            let mut n = Vec3::from(*normal);
            if n.length_squared() < 1.0e-6 {
                n = Vec3::Y;
            } else {
                n = n.normalize();
            }
            let up = if n.abs().dot(Vec3::Y) < 0.95 {
                Vec3::Y
            } else {
                Vec3::X
            };
            let mut tangent = n.cross(up);
            if tangent.length_squared() < 1.0e-6 {
                tangent = n.cross(Vec3::Z);
            }
            tangent = tangent.normalize_or_zero();
            let bitangent = n.cross(tangent).normalize_or_zero();
            let size = Vec3::from(*size);
            let mut half_u = size.x.abs() * 0.5;
            let mut half_v = size.y.abs() * 0.5;
            if half_u <= 0.0 {
                half_u = 1.0;
            }
            if half_v <= 0.0 {
                half_v = 1.0;
            }
            let corners = [
                origin - tangent * half_u - bitangent * half_v,
                origin + tangent * half_u - bitangent * half_v,
                origin + tangent * half_u + bitangent * half_v,
                origin - tangent * half_u + bitangent * half_v,
            ];
            let mut lines = Vec::with_capacity(8);
            for i in 0..4 {
                let a = corners[i];
                let b = corners[(i + 1) % 4];
                lines.push(LineVertex {
                    position: a.to_array(),
                    color,
                });
                lines.push(LineVertex {
                    position: b.to_array(),
                    color,
                });
            }
            lines
        }
    }
}

fn circle_vertices(
    center: Vec3,
    axis_u: Vec3,
    axis_v: Vec3,
    radius_u: f32,
    radius_v: f32,
    color: [f32; 3],
) -> Vec<LineVertex> {
    let segments = 64usize;
    let mut lines = Vec::with_capacity(segments * 2);
    for i in 0..segments {
        let t0 = i as f32 / segments as f32 * std::f32::consts::TAU;
        let t1 = (i + 1) as f32 / segments as f32 * std::f32::consts::TAU;
        let p0 = center + axis_u * (t0.cos() * radius_u) + axis_v * (t0.sin() * radius_v);
        let p1 = center + axis_u * (t1.cos() * radius_u) + axis_v * (t1.sin() * radius_v);
        lines.push(LineVertex {
            position: p0.to_array(),
            color,
        });
        lines.push(LineVertex {
            position: p1.to_array(),
            color,
        });
    }
    lines
}

pub(crate) fn grid_and_axes() -> (Vec<LineVertex>, Vec<LineVertex>) {
    let grid_size = 10.0;
    let divisions = 20;
    let step = grid_size / divisions as f32;
    let half = grid_size * 0.5;

    let grid_color = [0.25, 0.25, 0.25];
    let mut grid = Vec::new();

    for i in 0..=divisions {
        let offset = -half + i as f32 * step;
        grid.push(LineVertex {
            position: [offset, 0.0, -half],
            color: grid_color,
        });
        grid.push(LineVertex {
            position: [offset, 0.0, half],
            color: grid_color,
        });

        grid.push(LineVertex {
            position: [-half, 0.0, offset],
            color: grid_color,
        });
        grid.push(LineVertex {
            position: [half, 0.0, offset],
            color: grid_color,
        });
    }

    let axis_len = 2.5;
    let axes = vec![
        LineVertex {
            position: [0.0, 0.0, 0.0],
            color: [1.0, 0.0, 0.0],
        },
        LineVertex {
            position: [axis_len, 0.0, 0.0],
            color: [1.0, 0.0, 0.0],
        },
        LineVertex {
            position: [0.0, 0.0, 0.0],
            color: [0.0, 1.0, 0.0],
        },
        LineVertex {
            position: [0.0, axis_len, 0.0],
            color: [0.0, 1.0, 0.0],
        },
        LineVertex {
            position: [0.0, 0.0, 0.0],
            color: [0.1, 0.3, 1.0],
        },
        LineVertex {
            position: [0.0, 0.0, axis_len],
            color: [0.1, 0.3, 1.0],
        },
    ];

    (grid, axes)
}
