use glam::Vec3;

use crate::mesh::Mesh;

pub fn make_box(size: [f32; 3]) -> Mesh {
    let hx = size[0] * 0.5;
    let hy = size[1] * 0.5;
    let hz = size[2] * 0.5;

    let positions = vec![
        [-hx, -hy, -hz],
        [hx, -hy, -hz],
        [hx, hy, -hz],
        [-hx, hy, -hz],
        [-hx, -hy, hz],
        [hx, -hy, hz],
        [hx, hy, hz],
        [-hx, hy, hz],
    ];

    let indices = vec![
        0, 3, 2, 1, // -Z
        4, 5, 6, 7, // +Z
        0, 1, 5, 4, // -Y
        3, 7, 6, 2, // +Y
        1, 2, 6, 5, // +X
        0, 4, 7, 3, // -X
    ];
    let face_counts = vec![4; 6];

    Mesh::with_positions_faces(positions, indices, face_counts)
}

pub fn make_grid(size: [f32; 2], divisions: [u32; 2]) -> Mesh {
    let width = size[0].max(0.0);
    let depth = size[1].max(0.0);
    let div_x = divisions[0].max(1);
    let div_z = divisions[1].max(1);

    let step_x = width / div_x as f32;
    let step_z = depth / div_z as f32;
    let origin_x = -width * 0.5;
    let origin_z = -depth * 0.5;

    let mut positions = Vec::new();
    for z in 0..=div_z {
        for x in 0..=div_x {
            positions.push([
                origin_x + x as f32 * step_x,
                0.0,
                origin_z + z as f32 * step_z,
            ]);
        }
    }

    let mut indices = Vec::new();
    let mut face_counts = Vec::new();
    let stride = div_x + 1;
    for z in 0..div_z {
        for x in 0..div_x {
            let i0 = z * stride + x;
            let i1 = i0 + 1;
            let i2 = i0 + stride;
            let i3 = i2 + 1;
            indices.extend_from_slice(&[i0, i2, i3, i1]);
            face_counts.push(4);
        }
    }

    Mesh::with_positions_faces(positions, indices, face_counts)
}

pub fn make_uv_sphere(radius: f32, rows: u32, cols: u32) -> Mesh {
    let rows = rows.max(3);
    let cols = cols.max(3);
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    let mut face_counts = Vec::new();

    for r in 0..=rows {
        let v = r as f32 / rows as f32;
        let theta = v * std::f32::consts::PI;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for c in 0..=cols {
            let u = c as f32 / cols as f32;
            let phi = u * std::f32::consts::TAU;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            let x = sin_theta * cos_phi;
            let y = cos_theta;
            let z = sin_theta * sin_phi;
            positions.push([x * radius, y * radius, z * radius]);
        }
    }

    let stride = cols + 1;
    for r in 0..rows {
        for c in 0..cols {
            let i0 = r * stride + c;
            let i1 = i0 + 1;
            let i2 = i0 + stride;
            let i3 = i2 + 1;
            indices.extend_from_slice(&[i0, i1, i3, i2]);
            face_counts.push(4);
        }
    }

    let normals = positions
        .iter()
        .map(|p| {
            let v = Vec3::from(*p);
            let len = v.length();
            if len > 0.0 {
                (v / len).to_array()
            } else {
                [0.0, 1.0, 0.0]
            }
        })
        .collect();

    let mut mesh = Mesh::with_positions_faces(positions, indices, face_counts);
    mesh.normals = Some(normals);
    mesh
}

pub fn make_tube(radius: f32, height: f32, rows: u32, cols: u32, capped: bool) -> Mesh {
    let rows = rows.max(1);
    let cols = cols.max(3);
    let height = height.max(0.0);
    let radius = radius.max(0.0);

    let mut positions = Vec::new();
    let mut indices = Vec::new();
    let mut face_counts = Vec::new();

    let half = height * 0.5;
    let stride = cols + 1;
    for r in 0..=rows {
        let t = r as f32 / rows as f32;
        let y = -half + t * height;
        for c in 0..=cols {
            let u = c as f32 / cols as f32;
            let angle = u * std::f32::consts::TAU;
            let x = angle.cos() * radius;
            let z = angle.sin() * radius;
            positions.push([x, y, z]);
        }
    }

    for r in 0..rows {
        for c in 0..cols {
            let i0 = r * stride + c;
            let i1 = i0 + 1;
            let i2 = i0 + stride;
            let i3 = i2 + 1;
            indices.extend_from_slice(&[i0, i2, i3, i1]);
            face_counts.push(4);
        }
    }

    if capped && radius > 0.0 {
        let top_ring_start = rows * stride;
        let mut top_indices = Vec::with_capacity(cols as usize);
        for c in 0..cols {
            top_indices.push(top_ring_start + c);
        }
        let bottom_ring_start = 0;
        let mut bottom_indices = Vec::with_capacity(cols as usize);
        for c in (0..cols).rev() {
            bottom_indices.push(bottom_ring_start + c);
        }

        if !top_indices.is_empty() {
            indices.extend_from_slice(&top_indices);
            face_counts.push(cols);
        }
        if !bottom_indices.is_empty() {
            indices.extend_from_slice(&bottom_indices);
            face_counts.push(cols);
        }
    }

    Mesh::with_positions_faces(positions, indices, face_counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_has_expected_counts() {
        let mesh = make_box([2.0, 2.0, 2.0]);
        assert_eq!(mesh.positions.len(), 8);
        assert_eq!(mesh.indices.len(), 24);
        assert_eq!(mesh.face_counts.len(), 6);
    }

    #[test]
    fn grid_has_expected_counts() {
        let mesh = make_grid([2.0, 2.0], [2, 3]);
        assert_eq!(mesh.positions.len(), (2 + 1) * (3 + 1));
        assert_eq!(mesh.indices.len(), 2 * 3 * 4);
        assert_eq!(mesh.face_counts.len(), 2 * 3);
    }

    #[test]
    fn sphere_has_expected_counts() {
        let mesh = make_uv_sphere(1.0, 4, 8);
        assert_eq!(mesh.positions.len(), (4 + 1) * (8 + 1));
        assert_eq!(mesh.indices.len(), 4 * 8 * 4);
        assert_eq!(mesh.face_counts.len(), 4 * 8);
    }

    #[test]
    fn tube_has_expected_counts() {
        let mesh = make_tube(1.0, 2.0, 2, 8, true);
        assert_eq!(mesh.positions.len(), (2 + 1) * (8 + 1));
        assert_eq!(mesh.indices.len(), 2 * 8 * 4 + 2 * 8);
        assert_eq!(mesh.face_counts.len(), 2 * 8 + 2);
    }
}
