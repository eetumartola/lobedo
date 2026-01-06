use glam::{Mat3, Vec3};

use super::SplatGeo;

pub(super) struct ShRotationMatrices {
    l1: Option<[[f32; 3]; 3]>,
    l2: Option<[[f32; 5]; 5]>,
    l3: Option<[[f32; 7]; 7]>,
}

pub(super) fn build_sh_rotation_matrices(rot: Mat3, sh_coeffs: usize) -> ShRotationMatrices {
    let max_band = sh_max_band(sh_coeffs).min(3);
    let l1 = if max_band >= 1 {
        Some(compute_sh_rotation_matrix(rot, sh_basis_l1))
    } else {
        None
    };
    let l2 = if max_band >= 2 {
        Some(compute_sh_rotation_matrix(rot, sh_basis_l2))
    } else {
        None
    };
    let l3 = if max_band >= 3 {
        Some(compute_sh_rotation_matrix(rot, sh_basis_l3))
    } else {
        None
    };
    ShRotationMatrices { l1, l2, l3 }
}

fn sh_max_band(sh_coeffs: usize) -> usize {
    let mut band = 0usize;
    loop {
        let next = (band + 1) * (band + 1) - 1;
        if next <= sh_coeffs {
            band += 1;
        } else {
            break;
        }
    }
    band
}

pub(super) fn rotate_sh_bands(splats: &mut SplatGeo, index: usize, mats: &ShRotationMatrices) {
    if splats.sh_coeffs < 3 {
        return;
    }

    let base = index * splats.sh_coeffs;
    if base + splats.sh_coeffs > splats.sh_rest.len() {
        return;
    }

    if let Some(l1) = &mats.l1 {
        if splats.sh_coeffs >= 3 {
            rotate_sh_band_3(&mut splats.sh_rest[base..base + 3], l1);
        }
    }
    if let Some(l2) = &mats.l2 {
        if splats.sh_coeffs >= 8 {
            rotate_sh_band_5(&mut splats.sh_rest[base + 3..base + 8], l2);
        }
    }
    if let Some(l3) = &mats.l3 {
        if splats.sh_coeffs >= 15 {
            rotate_sh_band_7(&mut splats.sh_rest[base + 8..base + 15], l3);
        }
    }
}

#[allow(clippy::needless_range_loop)]
fn rotate_sh_band_3(coeffs: &mut [[f32; 3]], mat: &[[f32; 3]; 3]) {
    for channel in 0..3 {
        let v0 = coeffs[0][channel];
        let v1 = coeffs[1][channel];
        let v2 = coeffs[2][channel];
        let out0 = mat[0][0] * v0 + mat[0][1] * v1 + mat[0][2] * v2;
        let out1 = mat[1][0] * v0 + mat[1][1] * v1 + mat[1][2] * v2;
        let out2 = mat[2][0] * v0 + mat[2][1] * v1 + mat[2][2] * v2;
        coeffs[0][channel] = out0;
        coeffs[1][channel] = out1;
        coeffs[2][channel] = out2;
    }
}

#[allow(clippy::needless_range_loop)]
fn rotate_sh_band_5(coeffs: &mut [[f32; 3]], mat: &[[f32; 5]; 5]) {
    for channel in 0..3 {
        let v = [
            coeffs[0][channel],
            coeffs[1][channel],
            coeffs[2][channel],
            coeffs[3][channel],
            coeffs[4][channel],
        ];
        let mut out = [0.0f32; 5];
        for r in 0..5 {
            out[r] = mat[r][0] * v[0]
                + mat[r][1] * v[1]
                + mat[r][2] * v[2]
                + mat[r][3] * v[3]
                + mat[r][4] * v[4];
        }
        for r in 0..5 {
            coeffs[r][channel] = out[r];
        }
    }
}

#[allow(clippy::needless_range_loop)]
fn rotate_sh_band_7(coeffs: &mut [[f32; 3]], mat: &[[f32; 7]; 7]) {
    for channel in 0..3 {
        let v = [
            coeffs[0][channel],
            coeffs[1][channel],
            coeffs[2][channel],
            coeffs[3][channel],
            coeffs[4][channel],
            coeffs[5][channel],
            coeffs[6][channel],
        ];
        let mut out = [0.0f32; 7];
        for r in 0..7 {
            out[r] = mat[r][0] * v[0]
                + mat[r][1] * v[1]
                + mat[r][2] * v[2]
                + mat[r][3] * v[3]
                + mat[r][4] * v[4]
                + mat[r][5] * v[5]
                + mat[r][6] * v[6];
        }
        for r in 0..7 {
            coeffs[r][channel] = out[r];
        }
    }
}

fn compute_sh_rotation_matrix<const N: usize>(
    rot: Mat3,
    basis: fn(Vec3) -> [f32; N],
) -> [[f32; N]; N] {
    let samples = sh_sample_dirs();
    let sample_count = samples.len();
    if sample_count == 0 {
        return identity_matrix();
    }

    let mut b = vec![vec![0.0f32; N]; sample_count];
    for (row, dir) in samples.iter().enumerate() {
        let values = basis(*dir);
        b[row].copy_from_slice(&values);
    }

    let Some(pinv) = pseudo_inverse(&b) else {
        return identity_matrix();
    };

    let rot_inv = rot.transpose();
    let mut b_rot = vec![vec![0.0f32; N]; sample_count];
    for (row, dir) in samples.iter().enumerate() {
        let rotated = rot_inv * *dir;
        let values = basis(rotated);
        b_rot[row].copy_from_slice(&values);
    }

    let mut mat = [[0.0f32; N]; N];
    for r in 0..N {
        for c in 0..N {
            let mut sum = 0.0;
            for k in 0..sample_count {
                sum += pinv[r][k] * b_rot[k][c];
            }
            mat[r][c] = sum;
        }
    }
    if mat.iter().flatten().any(|value| !value.is_finite()) {
        return identity_matrix();
    }
    mat
}

#[allow(clippy::needless_range_loop)]
fn identity_matrix<const N: usize>() -> [[f32; N]; N] {
    let mut mat = [[0.0f32; N]; N];
    for i in 0..N {
        mat[i][i] = 1.0;
    }
    mat
}

#[allow(clippy::needless_range_loop)]
fn pseudo_inverse(matrix: &[Vec<f32>]) -> Option<Vec<Vec<f32>>> {
    if matrix.is_empty() {
        return None;
    }
    let rows = matrix.len();
    let cols = matrix[0].len();
    if cols == 0 {
        return None;
    }

    let mut bt_b = vec![vec![0.0f32; cols]; cols];
    for i in 0..cols {
        for j in 0..cols {
            let mut sum = 0.0;
            for r in 0..rows {
                sum += matrix[r][i] * matrix[r][j];
            }
            bt_b[i][j] = sum;
        }
    }

    let bt_b_inv = invert_square(&bt_b)?;

    let mut bt = vec![vec![0.0f32; rows]; cols];
    for i in 0..cols {
        for r in 0..rows {
            bt[i][r] = matrix[r][i];
        }
    }

    let mut result = vec![vec![0.0f32; rows]; cols];
    for i in 0..cols {
        for j in 0..rows {
            let mut sum = 0.0;
            for k in 0..cols {
                sum += bt_b_inv[i][k] * bt[k][j];
            }
            result[i][j] = sum;
        }
    }

    Some(result)
}

#[allow(clippy::needless_range_loop)]
fn invert_square(matrix: &[Vec<f32>]) -> Option<Vec<Vec<f32>>> {
    let n = matrix.len();
    if n == 0 {
        return None;
    }
    let mut aug = vec![vec![0.0f32; n * 2]; n];
    for i in 0..n {
        if matrix[i].len() != n {
            return None;
        }
        for j in 0..n {
            aug[i][j] = matrix[i][j];
        }
        aug[i][n + i] = 1.0;
    }

    for i in 0..n {
        let mut pivot = i;
        let mut max = aug[i][i].abs();
        for r in (i + 1)..n {
            let value = aug[r][i].abs();
            if value > max {
                max = value;
                pivot = r;
            }
        }
        if max < 1.0e-8 {
            return None;
        }
        if pivot != i {
            aug.swap(i, pivot);
        }

        let inv = 1.0 / aug[i][i];
        for j in 0..(n * 2) {
            aug[i][j] *= inv;
        }
        for r in 0..n {
            if r == i {
                continue;
            }
            let factor = aug[r][i];
            if factor.abs() < 1.0e-8 {
                continue;
            }
            for j in 0..(n * 2) {
                aug[r][j] -= factor * aug[i][j];
            }
        }
    }

    let mut inv = vec![vec![0.0f32; n]; n];
    for i in 0..n {
        for j in 0..n {
            inv[i][j] = aug[i][n + j];
        }
    }
    Some(inv)
}

#[allow(clippy::excessive_precision)]
const SH_C1: f32 = 0.4886025119029199;
#[allow(clippy::excessive_precision)]
const SH_C2: [f32; 5] = [
    1.0925484305920792,
    1.0925484305920792,
    0.31539156525252005,
    1.0925484305920792,
    0.5462742152960396,
];
#[allow(clippy::excessive_precision)]
const SH_C3: [f32; 7] = [
    0.5900435899266435,
    2.890611442640554,
    0.4570457994644658,
    0.3731763325901154,
    0.4570457994644658,
    1.445305721320277,
    0.5900435899266435,
];

fn sh_basis_l1(dir: Vec3) -> [f32; 3] {
    let x = dir.x;
    let y = dir.y;
    let z = dir.z;
    [-SH_C1 * y, SH_C1 * z, -SH_C1 * x]
}

fn sh_basis_l2(dir: Vec3) -> [f32; 5] {
    let x = dir.x;
    let y = dir.y;
    let z = dir.z;
    [
        SH_C2[0] * x * y,
        SH_C2[1] * y * z,
        SH_C2[2] * (3.0 * z * z - 1.0),
        SH_C2[3] * x * z,
        SH_C2[4] * (x * x - y * y),
    ]
}

fn sh_basis_l3(dir: Vec3) -> [f32; 7] {
    let x = dir.x;
    let y = dir.y;
    let z = dir.z;
    [
        SH_C3[0] * y * (3.0 * x * x - y * y),
        SH_C3[1] * x * y * z,
        SH_C3[2] * y * (5.0 * z * z - 1.0),
        SH_C3[3] * z * (5.0 * z * z - 3.0),
        SH_C3[4] * x * (5.0 * z * z - 1.0),
        SH_C3[5] * z * (x * x - y * y),
        SH_C3[6] * x * (x * x - 3.0 * y * y),
    ]
}

fn sh_sample_dirs() -> Vec<Vec3> {
    let phi = (1.0 + 5.0f32.sqrt()) * 0.5;
    let mut dirs = vec![
        Vec3::new(-1.0, phi, 0.0),
        Vec3::new(1.0, phi, 0.0),
        Vec3::new(-1.0, -phi, 0.0),
        Vec3::new(1.0, -phi, 0.0),
        Vec3::new(0.0, -1.0, phi),
        Vec3::new(0.0, 1.0, phi),
        Vec3::new(0.0, -1.0, -phi),
        Vec3::new(0.0, 1.0, -phi),
        Vec3::new(phi, 0.0, -1.0),
        Vec3::new(phi, 0.0, 1.0),
        Vec3::new(-phi, 0.0, -1.0),
        Vec3::new(-phi, 0.0, 1.0),
    ];
    for dir in &mut dirs {
        *dir = dir.normalize();
    }
    dirs
}
