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
        0, 2, 1, 0, 3, 2, // -Z
        4, 5, 6, 4, 6, 7, // +Z
        0, 1, 5, 0, 5, 4, // -Y
        2, 3, 7, 2, 7, 6, // +Y
        1, 2, 6, 1, 6, 5, // +X
        3, 0, 4, 3, 4, 7, // -X
    ];

    Mesh::with_positions_indices(positions, indices)
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
    let stride = div_x + 1;
    for z in 0..div_z {
        for x in 0..div_x {
            let i0 = z * stride + x;
            let i1 = i0 + 1;
            let i2 = i0 + stride;
            let i3 = i2 + 1;

            indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
        }
    }

    Mesh::with_positions_indices(positions, indices)
}

pub fn make_uv_sphere(radius: f32, rows: u32, cols: u32) -> Mesh {
    let rows = rows.max(3);
    let cols = cols.max(3);
    let mut positions = Vec::new();
    let mut indices = Vec::new();

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
            indices.extend_from_slice(&[i0, i1, i2, i1, i3, i2]);
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

    let mut mesh = Mesh::with_positions_indices(positions, indices);
    mesh.normals = Some(normals);
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_has_expected_counts() {
        let mesh = make_box([2.0, 2.0, 2.0]);
        assert_eq!(mesh.positions.len(), 8);
        assert_eq!(mesh.indices.len(), 36);
    }

    #[test]
    fn grid_has_expected_counts() {
        let mesh = make_grid([2.0, 2.0], [2, 3]);
        assert_eq!(mesh.positions.len(), (2 + 1) * (3 + 1));
        assert_eq!(mesh.indices.len(), 2 * 3 * 6);
    }

    #[test]
    fn sphere_has_expected_counts() {
        let mesh = make_uv_sphere(1.0, 4, 8);
        assert_eq!(mesh.positions.len(), (4 + 1) * (8 + 1));
        assert_eq!(mesh.indices.len(), 4 * 8 * 6);
    }
}
