use std::collections::BTreeMap;

use glam::{Mat3, Mat4, Quat, Vec3};

#[derive(Debug, Clone, Default)]
pub struct SplatGeo {
    pub positions: Vec<[f32; 3]>,
    pub rotations: Vec<[f32; 4]>,
    pub scales: Vec<[f32; 3]>,
    pub opacity: Vec<f32>,
    pub sh0: Vec<[f32; 3]>,
    pub sh_coeffs: usize,
    pub sh_rest: Vec<[f32; 3]>,
    pub groups: BTreeMap<String, Vec<bool>>,
}

impl SplatGeo {
    pub fn with_len(count: usize) -> Self {
        Self {
            positions: vec![[0.0, 0.0, 0.0]; count],
            rotations: vec![[0.0, 0.0, 0.0, 1.0]; count],
            scales: vec![[1.0, 1.0, 1.0]; count],
            opacity: vec![1.0; count],
            sh0: vec![[1.0, 1.0, 1.0]; count],
            sh_coeffs: 0,
            sh_rest: Vec::new(),
            groups: BTreeMap::new(),
        }
    }

    pub fn with_len_and_sh(count: usize, sh_coeffs: usize) -> Self {
        let mut splats = Self::with_len(count);
        if sh_coeffs > 0 {
            splats.sh_coeffs = sh_coeffs;
            splats.sh_rest = vec![[0.0, 0.0, 0.0]; count * sh_coeffs];
        }
        splats
    }

    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    pub fn validate(&self) -> Result<(), String> {
        let count = self.positions.len();
        if self.rotations.len() != count
            || self.scales.len() != count
            || self.opacity.len() != count
            || self.sh0.len() != count
        {
            return Err("SplatGeo arrays have inconsistent lengths".to_string());
        }
        if self.sh_coeffs == 0 {
            if !self.sh_rest.is_empty() {
                return Err("SplatGeo SH coefficients are inconsistent".to_string());
            }
        } else if self.sh_rest.len() != count * self.sh_coeffs {
            return Err("SplatGeo SH coefficients are inconsistent".to_string());
        }
        for (name, values) in &self.groups {
            if values.len() != count {
                return Err(format!("SplatGeo group '{}' has invalid length", name));
            }
        }
        Ok(())
    }

    pub fn transform(&mut self, matrix: Mat4) {
        if self.positions.is_empty() {
            return;
        }

        let sh_mats = if self.sh_coeffs >= 3 {
            Some(build_sh_rotation_matrices(
                rotation_from_matrix(matrix),
                self.sh_coeffs,
            ))
        } else {
            None
        };
        let use_log_scale = self.scales.iter().any(|value| {
            value[0] < 0.0 || value[1] < 0.0 || value[2] < 0.0
        });
        let linear = Mat3::from_mat4(matrix);
        let min_scale = 1.0e-6;

        for idx in 0..self.positions.len() {
            let position = matrix.transform_point3(Vec3::from(self.positions[idx]));
            self.positions[idx] = position.to_array();

            let mut scale = Vec3::from(self.scales[idx]);
            if use_log_scale {
                scale = Vec3::new(scale.x.exp(), scale.y.exp(), scale.z.exp());
            }
            scale = Vec3::new(
                scale.x.max(min_scale),
                scale.y.max(min_scale),
                scale.z.max(min_scale),
            );

            let rotation = self.rotations[idx];
            let mut quat = Quat::from_xyzw(rotation[1], rotation[2], rotation[3], rotation[0]);
            if quat.length_squared() > 0.0 {
                quat = quat.normalize();
            } else {
                quat = Quat::IDENTITY;
            }

            let rot_mat = Mat3::from_quat(quat);
            let cov_local = Mat3::from_diagonal(scale * scale);
            let cov_world = linear * (rot_mat * cov_local * rot_mat.transpose()) * linear.transpose();

            let (eigenvalues, mut eigenvectors) = eigen_decomposition_symmetric(cov_world);
            if eigenvectors.determinant() < 0.0 {
                eigenvectors = Mat3::from_cols(
                    eigenvectors.x_axis,
                    eigenvectors.y_axis,
                    -eigenvectors.z_axis,
                );
            }

            let mut sigma = Vec3::new(
                eigenvalues.x.max(0.0).sqrt(),
                eigenvalues.y.max(0.0).sqrt(),
                eigenvalues.z.max(0.0).sqrt(),
            );
            sigma = Vec3::new(
                sigma.x.max(min_scale),
                sigma.y.max(min_scale),
                sigma.z.max(min_scale),
            );

            let quat = Quat::from_mat3(&eigenvectors).normalize();
            self.rotations[idx] = [quat.w, quat.x, quat.y, quat.z];

            if use_log_scale {
                self.scales[idx] = [sigma.x.ln(), sigma.y.ln(), sigma.z.ln()];
            } else {
                self.scales[idx] = sigma.to_array();
            }

            if let Some(mats) = &sh_mats {
                rotate_sh_bands(self, idx, mats);
            }
        }
    }

    pub fn transform_masked(&mut self, matrix: Mat4, mask: &[bool]) {
        if self.positions.is_empty() {
            return;
        }
        if mask.len() != self.positions.len() {
            self.transform(matrix);
            return;
        }

        let sh_mats = if self.sh_coeffs >= 3 {
            Some(build_sh_rotation_matrices(
                rotation_from_matrix(matrix),
                self.sh_coeffs,
            ))
        } else {
            None
        };
        let use_log_scale = self.scales.iter().any(|value| {
            value[0] < 0.0 || value[1] < 0.0 || value[2] < 0.0
        });
        let linear = Mat3::from_mat4(matrix);
        let min_scale = 1.0e-6;

        for (idx, selected) in mask.iter().enumerate() {
            if !*selected {
                continue;
            }

            let position = matrix.transform_point3(Vec3::from(self.positions[idx]));
            self.positions[idx] = position.to_array();

            let mut scale = Vec3::from(self.scales[idx]);
            if use_log_scale {
                scale = Vec3::new(scale.x.exp(), scale.y.exp(), scale.z.exp());
            }
            scale = Vec3::new(
                scale.x.max(min_scale),
                scale.y.max(min_scale),
                scale.z.max(min_scale),
            );

            let rotation = self.rotations[idx];
            let mut quat = Quat::from_xyzw(rotation[1], rotation[2], rotation[3], rotation[0]);
            if quat.length_squared() > 0.0 {
                quat = quat.normalize();
            } else {
                quat = Quat::IDENTITY;
            }

            let rot_mat = Mat3::from_quat(quat);
            let cov_local = Mat3::from_diagonal(scale * scale);
            let cov_world = linear * (rot_mat * cov_local * rot_mat.transpose()) * linear.transpose();

            let (eigenvalues, mut eigenvectors) = eigen_decomposition_symmetric(cov_world);
            if eigenvectors.determinant() < 0.0 {
                eigenvectors = Mat3::from_cols(
                    eigenvectors.x_axis,
                    eigenvectors.y_axis,
                    -eigenvectors.z_axis,
                );
            }

            let mut sigma = Vec3::new(
                eigenvalues.x.max(0.0).sqrt(),
                eigenvalues.y.max(0.0).sqrt(),
                eigenvalues.z.max(0.0).sqrt(),
            );
            sigma = Vec3::new(
                sigma.x.max(min_scale),
                sigma.y.max(min_scale),
                sigma.z.max(min_scale),
            );

            let quat = Quat::from_mat3(&eigenvectors).normalize();
            self.rotations[idx] = [quat.w, quat.x, quat.y, quat.z];

            if use_log_scale {
                self.scales[idx] = [sigma.x.ln(), sigma.y.ln(), sigma.z.ln()];
            } else {
                self.scales[idx] = sigma.to_array();
            }

            if let Some(mats) = &sh_mats {
                rotate_sh_bands(self, idx, mats);
            }
        }
    }
}

struct ShRotationMatrices {
    l1: Option<[[f32; 3]; 3]>,
    l2: Option<[[f32; 5]; 5]>,
    l3: Option<[[f32; 7]; 7]>,
}

fn build_sh_rotation_matrices(rot: Mat3, sh_coeffs: usize) -> ShRotationMatrices {
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

fn rotation_from_matrix(matrix: Mat4) -> Mat3 {
    let linear = Mat3::from_mat4(matrix);
    let mut x = linear.x_axis;
    let mut y = linear.y_axis;

    if x.length_squared() > 0.0 {
        x = x.normalize();
    } else {
        x = Vec3::X;
    }
    y = (y - x * y.dot(x)).normalize_or_zero();
    if y.length_squared() == 0.0 {
        y = Vec3::Y;
    }
    let mut z = x.cross(y);
    if z.length_squared() == 0.0 {
        z = Vec3::Z;
    } else {
        z = z.normalize();
    }

    let mut rot = Mat3::from_cols(x, y, z);
    if rot.determinant() < 0.0 {
        rot = Mat3::from_cols(x, y, -z);
    }
    rot
}

fn rotate_sh_bands(splats: &mut SplatGeo, index: usize, mats: &ShRotationMatrices) {
    if splats.sh_coeffs < 3 {
        return;
    }

    let base = index * splats.sh_coeffs;

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

#[allow(clippy::needless_range_loop)]
fn eigen_decomposition_symmetric(mat: Mat3) -> (Vec3, Mat3) {
    let cols = mat.to_cols_array_2d();
    let mut a = [
        [cols[0][0], cols[1][0], cols[2][0]],
        [cols[0][1], cols[1][1], cols[2][1]],
        [cols[0][2], cols[1][2], cols[2][2]],
    ];
    let mut v = [[1.0f32, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

    const MAX_ITERS: usize = 16;
    const EPS: f32 = 1.0e-6;

    for _ in 0..MAX_ITERS {
        let mut p = 0usize;
        let mut q = 1usize;
        let mut max = a[0][1].abs();

        let a02 = a[0][2].abs();
        if a02 > max {
            max = a02;
            p = 0;
            q = 2;
        }
        let a12 = a[1][2].abs();
        if a12 > max {
            max = a12;
            p = 1;
            q = 2;
        }

        if max < EPS {
            break;
        }

        let app = a[p][p];
        let aqq = a[q][q];
        let apq = a[p][q];

        if apq.abs() < EPS {
            continue;
        }

        let tau = (aqq - app) / (2.0 * apq);
        let t = if tau >= 0.0 {
            1.0 / (tau + (1.0 + tau * tau).sqrt())
        } else {
            -1.0 / (-tau + (1.0 + tau * tau).sqrt())
        };
        let c = 1.0 / (1.0 + t * t).sqrt();
        let s = t * c;

        for i in 0..3 {
            if i == p || i == q {
                continue;
            }
            let aip = a[i][p];
            let aiq = a[i][q];
            a[i][p] = c * aip - s * aiq;
            a[p][i] = a[i][p];
            a[i][q] = s * aip + c * aiq;
            a[q][i] = a[i][q];
        }

        a[p][p] = c * c * app - 2.0 * s * c * apq + s * s * aqq;
        a[q][q] = s * s * app + 2.0 * s * c * apq + c * c * aqq;
        a[p][q] = 0.0;
        a[q][p] = 0.0;

        for i in 0..3 {
            let vip = v[i][p];
            let viq = v[i][q];
            v[i][p] = c * vip - s * viq;
            v[i][q] = s * vip + c * viq;
        }
    }

    let eigenvalues = Vec3::new(a[0][0], a[1][1], a[2][2]);
    let eigenvectors = Mat3::from_cols(
        Vec3::new(v[0][0], v[1][0], v[2][0]),
        Vec3::new(v[0][1], v[1][1], v[2][1]),
        Vec3::new(v[0][2], v[1][2], v[2][2]),
    );
    (eigenvalues, eigenvectors)
}

#[derive(Debug, Clone, Copy)]
enum PlyFormat {
    Ascii,
    BinaryLittle,
    BinaryBig,
}

#[derive(Debug, Clone, Copy)]
enum PlyScalarType {
    Int8,
    Uint8,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Float32,
    Float64,
}

impl PlyScalarType {
    fn size(self) -> usize {
        match self {
            PlyScalarType::Int8 | PlyScalarType::Uint8 => 1,
            PlyScalarType::Int16 | PlyScalarType::Uint16 => 2,
            PlyScalarType::Int32 | PlyScalarType::Uint32 | PlyScalarType::Float32 => 4,
            PlyScalarType::Float64 => 8,
        }
    }
}

#[derive(Debug)]
struct PlyProperty {
    name: String,
    data_type: PlyScalarType,
}

#[derive(Debug)]
struct PlyHeader {
    format: PlyFormat,
    vertex_count: usize,
    vertex_properties: Vec<PlyProperty>,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_splat_ply(path: &str) -> Result<SplatGeo, String> {
    let data = std::fs::read(path).map_err(|err| err.to_string())?;
    parse_splat_ply_bytes(&data)
}

#[cfg(target_arch = "wasm32")]
pub fn load_splat_ply(_path: &str) -> Result<SplatGeo, String> {
    Err("Read Splats is not supported in web builds".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_splat_ply(path: &str, splats: &SplatGeo) -> Result<(), String> {
    use std::io::Write;

    splats.validate()?;
    let mut file = std::fs::File::create(path).map_err(|err| err.to_string())?;

    writeln!(file, "ply").map_err(|err| err.to_string())?;
    writeln!(file, "format ascii 1.0").map_err(|err| err.to_string())?;
    writeln!(file, "element vertex {}", splats.len()).map_err(|err| err.to_string())?;
    writeln!(file, "property float x").map_err(|err| err.to_string())?;
    writeln!(file, "property float y").map_err(|err| err.to_string())?;
    writeln!(file, "property float z").map_err(|err| err.to_string())?;
    writeln!(file, "property float opacity").map_err(|err| err.to_string())?;
    writeln!(file, "property float scale_0").map_err(|err| err.to_string())?;
    writeln!(file, "property float scale_1").map_err(|err| err.to_string())?;
    writeln!(file, "property float scale_2").map_err(|err| err.to_string())?;
    writeln!(file, "property float rot_0").map_err(|err| err.to_string())?;
    writeln!(file, "property float rot_1").map_err(|err| err.to_string())?;
    writeln!(file, "property float rot_2").map_err(|err| err.to_string())?;
    writeln!(file, "property float rot_3").map_err(|err| err.to_string())?;
    writeln!(file, "property float f_dc_0").map_err(|err| err.to_string())?;
    writeln!(file, "property float f_dc_1").map_err(|err| err.to_string())?;
    writeln!(file, "property float f_dc_2").map_err(|err| err.to_string())?;

    if splats.sh_coeffs > 0 {
        for i in 0..(splats.sh_coeffs * 3) {
            writeln!(file, "property float f_rest_{}", i).map_err(|err| err.to_string())?;
        }
    }

    writeln!(file, "end_header").map_err(|err| err.to_string())?;

    for idx in 0..splats.len() {
        let [x, y, z] = splats.positions[idx];
        let [sx, sy, sz] = splats.scales[idx];
        let [r0, r1, r2, r3] = splats.rotations[idx];
        let opacity = splats.opacity[idx];
        let [c0, c1, c2] = splats.sh0[idx];
        write!(
            file,
            "{x} {y} {z} {opacity} {sx} {sy} {sz} {r0} {r1} {r2} {r3} {c0} {c1} {c2}"
        )
        .map_err(|err| err.to_string())?;

        if splats.sh_coeffs > 0 {
            let base = idx * splats.sh_coeffs;
            for coeff in 0..splats.sh_coeffs {
                let value = splats.sh_rest[base + coeff][0];
                write!(file, " {value}").map_err(|err| err.to_string())?;
            }
            for coeff in 0..splats.sh_coeffs {
                let value = splats.sh_rest[base + coeff][1];
                write!(file, " {value}").map_err(|err| err.to_string())?;
            }
            for coeff in 0..splats.sh_coeffs {
                let value = splats.sh_rest[base + coeff][2];
                write!(file, " {value}").map_err(|err| err.to_string())?;
            }
        }

        writeln!(file).map_err(|err| err.to_string())?;
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub fn save_splat_ply(_path: &str, _splats: &SplatGeo) -> Result<(), String> {
    Err("Write Splats is not supported in web builds".to_string())
}

fn parse_splat_ply_bytes(data: &[u8]) -> Result<SplatGeo, String> {
    let (header, data_start) = parse_header_bytes(data)?;
    let indices = SplatPropertyIndices::from_properties(&header.vertex_properties);
    if indices.x.is_none() || indices.y.is_none() || indices.z.is_none() {
        return Err("PLY is missing position properties (x, y, z)".to_string());
    }

    match header.format {
        PlyFormat::Ascii => {
            let text = std::str::from_utf8(&data[data_start..])
                .map_err(|_| "PLY ASCII data is not UTF-8".to_string())?;
            parse_ascii_vertices(text, &header, &indices)
        }
        PlyFormat::BinaryLittle => parse_binary_vertices(&data[data_start..], &header, &indices, true),
        PlyFormat::BinaryBig => parse_binary_vertices(&data[data_start..], &header, &indices, false),
    }
}

fn parse_header<'a, I>(lines: &mut I) -> Result<PlyHeader, String>
where
    I: Iterator<Item = &'a str>,
{
    let first = lines
        .next()
        .ok_or_else(|| "PLY header is missing".to_string())?;
    if first.trim() != "ply" {
        return Err("Not a PLY file".to_string());
    }

    let mut format = None;
    let mut vertex_count = None;
    let mut vertex_properties = Vec::new();
    let mut in_vertex = false;

    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with("comment") {
            continue;
        }
        if line == "end_header" {
            break;
        }

        let mut parts = line.split_whitespace();
        let Some(tag) = parts.next() else {
            continue;
        };
        match tag {
            "format" => {
                let fmt = parts.next().unwrap_or("");
                format = Some(match fmt {
                    "ascii" => PlyFormat::Ascii,
                    "binary_little_endian" => PlyFormat::BinaryLittle,
                    "binary_big_endian" => PlyFormat::BinaryBig,
                    _ => return Err("Unknown PLY format".to_string()),
                });
            }
            "element" => {
                let name = parts.next().unwrap_or("");
                let count = parts
                    .next()
                    .ok_or_else(|| "Malformed PLY element".to_string())?
                    .parse::<usize>()
                    .map_err(|_| "Malformed PLY element count".to_string())?;
                in_vertex = name == "vertex";
                if in_vertex {
                    vertex_count = Some(count);
                }
            }
            "property" if in_vertex => {
                let prop_type = parts.next().unwrap_or("");
                if prop_type == "list" {
                    return Err("PLY vertex list properties are not supported".to_string());
                }
                let data_type = parse_scalar_type(prop_type)?;
                let name = parts.next().unwrap_or("").to_string();
                if name.is_empty() {
                    return Err("PLY property missing name".to_string());
                }
                vertex_properties.push(PlyProperty { name, data_type });
            }
            _ => {}
        }
    }

    let format = format.ok_or_else(|| "PLY format not specified".to_string())?;
    let vertex_count = vertex_count.ok_or_else(|| "PLY has no vertex element".to_string())?;
    Ok(PlyHeader {
        format,
        vertex_count,
        vertex_properties,
    })
}

fn parse_header_bytes(data: &[u8]) -> Result<(PlyHeader, usize), String> {
    let mut line_start = 0usize;
    let mut header_end = None;
    for (idx, byte) in data.iter().enumerate() {
        if *byte != b'\n' {
            continue;
        }
        let line_bytes = &data[line_start..idx];
        let line_str = std::str::from_utf8(line_bytes)
            .map_err(|_| "PLY header is not ASCII".to_string())?;
        let line = line_str.trim_end_matches('\r').trim();
        if line == "end_header" {
            header_end = Some(idx + 1);
            break;
        }
        line_start = idx + 1;
    }

    let header_end = header_end.ok_or_else(|| "PLY header is missing end_header".to_string())?;
    let header_text = std::str::from_utf8(&data[..header_end])
        .map_err(|_| "PLY header is not ASCII".to_string())?;
    let mut lines = header_text.lines();
    let header = parse_header(&mut lines)?;
    Ok((header, header_end))
}

fn parse_scalar_type(value: &str) -> Result<PlyScalarType, String> {
    match value {
        "char" | "int8" => Ok(PlyScalarType::Int8),
        "uchar" | "uint8" => Ok(PlyScalarType::Uint8),
        "short" | "int16" => Ok(PlyScalarType::Int16),
        "ushort" | "uint16" => Ok(PlyScalarType::Uint16),
        "int" | "int32" => Ok(PlyScalarType::Int32),
        "uint" | "uint32" => Ok(PlyScalarType::Uint32),
        "float" | "float32" => Ok(PlyScalarType::Float32),
        "double" | "float64" => Ok(PlyScalarType::Float64),
        _ => Err("Unsupported PLY property type".to_string()),
    }
}

fn parse_ascii_vertices(
    text: &str,
    header: &PlyHeader,
    indices: &SplatPropertyIndices,
) -> Result<SplatGeo, String> {
    let mut splats = SplatGeo::with_len_and_sh(header.vertex_count, indices.sh_coeffs());
    let mut read = 0usize;
    for line in text.lines() {
        if read >= header.vertex_count {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        let values: Vec<f32> = line
            .split_whitespace()
            .map(|token| token.parse::<f32>().map_err(|_| "Invalid PLY value".to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        if values.len() < header.vertex_properties.len() {
            return Err("PLY vertex row has too few values".to_string());
        }

        fill_splat_from_values(&mut splats, read, &values, indices);
        read += 1;
    }

    if read < header.vertex_count {
        return Err("Unexpected end of PLY vertex data".to_string());
    }

    splats.validate()?;
    Ok(splats)
}

fn parse_binary_vertices(
    data: &[u8],
    header: &PlyHeader,
    indices: &SplatPropertyIndices,
    little_endian: bool,
) -> Result<SplatGeo, String> {
    let mut splats = SplatGeo::with_len_and_sh(header.vertex_count, indices.sh_coeffs());
    let mut values = vec![0.0f32; header.vertex_properties.len()];
    let mut cursor = 0usize;

    for read in 0..header.vertex_count {
        for (idx, prop) in header.vertex_properties.iter().enumerate() {
            let size = prop.data_type.size();
            let end = cursor + size;
            if end > data.len() {
                return Err("Unexpected end of binary PLY data".to_string());
            }
            values[idx] = read_scalar(&data[cursor..end], prop.data_type, little_endian)?;
            cursor = end;
        }
        fill_splat_from_values(&mut splats, read, &values, indices);
    }

    splats.validate()?;
    Ok(splats)
}

fn read_scalar(data: &[u8], data_type: PlyScalarType, little_endian: bool) -> Result<f32, String> {
    let value = match data_type {
        PlyScalarType::Int8 => data
            .first()
            .copied()
            .ok_or_else(|| "Invalid PLY data".to_string())? as i8 as f32,
        PlyScalarType::Uint8 => data
            .first()
            .copied()
            .ok_or_else(|| "Invalid PLY data".to_string())? as f32,
        PlyScalarType::Int16 => {
            if data.len() < 2 {
                return Err("Invalid PLY data".to_string());
            }
            let mut bytes = [0u8; 2];
            bytes.copy_from_slice(&data[..2]);
            if little_endian {
                i16::from_le_bytes(bytes) as f32
            } else {
                i16::from_be_bytes(bytes) as f32
            }
        }
        PlyScalarType::Uint16 => {
            if data.len() < 2 {
                return Err("Invalid PLY data".to_string());
            }
            let mut bytes = [0u8; 2];
            bytes.copy_from_slice(&data[..2]);
            if little_endian {
                u16::from_le_bytes(bytes) as f32
            } else {
                u16::from_be_bytes(bytes) as f32
            }
        }
        PlyScalarType::Int32 => {
            if data.len() < 4 {
                return Err("Invalid PLY data".to_string());
            }
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&data[..4]);
            if little_endian {
                i32::from_le_bytes(bytes) as f32
            } else {
                i32::from_be_bytes(bytes) as f32
            }
        }
        PlyScalarType::Uint32 => {
            if data.len() < 4 {
                return Err("Invalid PLY data".to_string());
            }
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&data[..4]);
            if little_endian {
                u32::from_le_bytes(bytes) as f32
            } else {
                u32::from_be_bytes(bytes) as f32
            }
        }
        PlyScalarType::Float32 => {
            if data.len() < 4 {
                return Err("Invalid PLY data".to_string());
            }
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&data[..4]);
            if little_endian {
                f32::from_le_bytes(bytes)
            } else {
                f32::from_be_bytes(bytes)
            }
        }
        PlyScalarType::Float64 => {
            if data.len() < 8 {
                return Err("Invalid PLY data".to_string());
            }
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&data[..8]);
            if little_endian {
                f64::from_le_bytes(bytes) as f32
            } else {
                f64::from_be_bytes(bytes) as f32
            }
        }
    };
    Ok(value)
}

fn fill_splat_from_values(
    splats: &mut SplatGeo,
    read: usize,
    values: &[f32],
    indices: &SplatPropertyIndices,
) {
    let x = values[indices.x.unwrap()];
    let y = values[indices.y.unwrap()];
    let z = values[indices.z.unwrap()];
    splats.positions[read] = [x, y, z];

    if let Some(idx) = indices.opacity {
        splats.opacity[read] = values[idx];
    }

    if let (Some(sx), Some(sy), Some(sz)) = (indices.scale[0], indices.scale[1], indices.scale[2])
    {
        splats.scales[read] = [values[sx], values[sy], values[sz]];
    }

    if let (Some(r0), Some(r1), Some(r2), Some(r3)) =
        (indices.rot[0], indices.rot[1], indices.rot[2], indices.rot[3])
    {
        splats.rotations[read] = [values[r0], values[r1], values[r2], values[r3]];
    }

    if let (Some(c0), Some(c1), Some(c2)) = (indices.sh0[0], indices.sh0[1], indices.sh0[2]) {
        splats.sh0[read] = [values[c0], values[c1], values[c2]];
    } else if let (Some(r), Some(g), Some(b)) = (indices.color[0], indices.color[1], indices.color[2]) {
        let mut color = [values[r], values[g], values[b]];
        if color.iter().any(|v| *v > 1.5) {
            color = [color[0] / 255.0, color[1] / 255.0, color[2] / 255.0];
        }
        splats.sh0[read] = color;
    }

    if splats.sh_coeffs > 0 && indices.sh_rest.len() >= splats.sh_coeffs * 3 {
        let base = read * splats.sh_coeffs;
        for coeff in 0..splats.sh_coeffs {
            let r = indices.sh_rest.get(coeff).and_then(|idx| idx.map(|i| values[i]));
            let g = indices
                .sh_rest
                .get(coeff + splats.sh_coeffs)
                .and_then(|idx| idx.map(|i| values[i]));
            let b = indices
                .sh_rest
                .get(coeff + splats.sh_coeffs * 2)
                .and_then(|idx| idx.map(|i| values[i]));
            splats.sh_rest[base + coeff] = [r.unwrap_or(0.0), g.unwrap_or(0.0), b.unwrap_or(0.0)];
        }
    }
}

#[derive(Default)]
struct SplatPropertyIndices {
    x: Option<usize>,
    y: Option<usize>,
    z: Option<usize>,
    opacity: Option<usize>,
    scale: [Option<usize>; 3],
    rot: [Option<usize>; 4],
    sh0: [Option<usize>; 3],
    color: [Option<usize>; 3],
    sh_rest: Vec<Option<usize>>,
}

impl SplatPropertyIndices {
    fn from_properties(properties: &[PlyProperty]) -> Self {
        let mut indices = SplatPropertyIndices::default();
        let mut rest = Vec::new();
        for (idx, prop) in properties.iter().enumerate() {
            match prop.name.as_str() {
                "x" => indices.x = Some(idx),
                "y" => indices.y = Some(idx),
                "z" => indices.z = Some(idx),
                "opacity" => indices.opacity = Some(idx),
                "scale_0" | "scale_x" => indices.scale[0] = Some(idx),
                "scale_1" | "scale_y" => indices.scale[1] = Some(idx),
                "scale_2" | "scale_z" => indices.scale[2] = Some(idx),
                "rot_0" | "rotation_0" | "q_w" => indices.rot[0] = Some(idx),
                "rot_1" | "rotation_1" | "q_x" => indices.rot[1] = Some(idx),
                "rot_2" | "rotation_2" | "q_y" => indices.rot[2] = Some(idx),
                "rot_3" | "rotation_3" | "q_z" => indices.rot[3] = Some(idx),
                "f_dc_0" | "sh0_0" => indices.sh0[0] = Some(idx),
                "f_dc_1" | "sh0_1" => indices.sh0[1] = Some(idx),
                "f_dc_2" | "sh0_2" => indices.sh0[2] = Some(idx),
                "red" | "r" => indices.color[0] = Some(idx),
                "green" | "g" => indices.color[1] = Some(idx),
                "blue" | "b" => indices.color[2] = Some(idx),
                _ => {
                    if let Some(rest_idx) = parse_sh_rest_index(&prop.name) {
                        rest.push((rest_idx, idx));
                    }
                }
            }
        }
        if !rest.is_empty() {
            let max = rest.iter().map(|(i, _)| *i).max().unwrap_or(0);
            indices.sh_rest = vec![None; max + 1];
            for (rest_idx, prop_idx) in rest {
                if rest_idx < indices.sh_rest.len() {
                    indices.sh_rest[rest_idx] = Some(prop_idx);
                }
            }
        }
        indices
    }

    fn sh_coeffs(&self) -> usize {
        let rest = self.sh_rest.len();
        if rest >= 3 && rest.is_multiple_of(3) {
            rest / 3
        } else {
            0
        }
    }
}

fn parse_sh_rest_index(name: &str) -> Option<usize> {
    let suffix = name.strip_prefix("f_rest_").or_else(|| name.strip_prefix("sh_rest_"))?;
    suffix.parse::<usize>().ok()
}

#[cfg(test)]
mod tests {
    use glam::{Mat4, Quat, Vec3};

    use super::{load_splat_ply, parse_splat_ply_bytes, save_splat_ply};
    use super::SplatGeo;

    #[test]
    fn parse_ascii_ply_positions_and_sh0() {
        let data = "\
ply
format ascii 1.0
element vertex 2
property float x
property float y
property float z
property float opacity
property float scale_0
property float scale_1
property float scale_2
property float f_dc_0
property float f_dc_1
property float f_dc_2
end_header
0 0 0 0.5 1 1 1 0.1 0.2 0.3
1 2 3 1.0 2 2 2 0.4 0.5 0.6
";

        let splats = parse_splat_ply_bytes(data.as_bytes()).expect("parse");
        assert_eq!(splats.len(), 2);
        assert!((splats.opacity[0] - 0.5).abs() < 1.0e-6);
        assert_eq!(splats.sh0[1], [0.4, 0.5, 0.6]);
    }

    #[test]
    fn parse_binary_ply_positions_and_opacity() {
        let header = "\
ply
format binary_little_endian 1.0
element vertex 1
property float x
property float y
property float z
property float opacity
end_header
";
        let mut data = Vec::from(header.as_bytes());
        data.extend_from_slice(&1.0f32.to_le_bytes());
        data.extend_from_slice(&2.0f32.to_le_bytes());
        data.extend_from_slice(&3.0f32.to_le_bytes());
        data.extend_from_slice(&0.25f32.to_le_bytes());

        let splats = parse_splat_ply_bytes(&data).expect("parse");
        assert_eq!(splats.len(), 1);
        assert_eq!(splats.positions[0], [1.0, 2.0, 3.0]);
        assert!((splats.opacity[0] - 0.25).abs() < 1.0e-6);
    }

    #[test]
    fn parse_ascii_ply_sh_rest() {
        let data = "\
ply
format ascii 1.0
element vertex 1
property float x
property float y
property float z
property float f_rest_0
property float f_rest_1
property float f_rest_2
property float f_rest_3
property float f_rest_4
property float f_rest_5
property float f_rest_6
property float f_rest_7
property float f_rest_8
end_header
0 0 0 1 2 3 4 5 6 7 8 9
";

        let splats = parse_splat_ply_bytes(data.as_bytes()).expect("parse");
        assert_eq!(splats.sh_coeffs, 3);
        assert_eq!(splats.sh_rest.len(), 3);
        assert_eq!(splats.sh_rest[0], [1.0, 4.0, 7.0]);
        assert_eq!(splats.sh_rest[1], [2.0, 5.0, 8.0]);
        assert_eq!(splats.sh_rest[2], [3.0, 6.0, 9.0]);
    }

    #[test]
    fn transform_updates_positions_and_scales() {
        let mut splats = SplatGeo::with_len(1);
        splats.positions[0] = [1.0, 2.0, 3.0];
        splats.scales[0] = [1.0, 1.0, 1.0];
        splats.rotations[0] = [1.0, 0.0, 0.0, 0.0];

        let matrix = Mat4::from_scale_rotation_translation(
            Vec3::new(2.0, 3.0, 4.0),
            Quat::IDENTITY,
            Vec3::new(1.0, 0.0, 0.0),
        );
        splats.transform(matrix);

        let pos = splats.positions[0];
        assert!((pos[0] - 3.0).abs() < 1.0e-4);
        assert!((pos[1] - 6.0).abs() < 1.0e-4);
        assert!((pos[2] - 12.0).abs() < 1.0e-4);

        let scale = splats.scales[0];
        assert!((scale[0] - 2.0).abs() < 1.0e-4);
        assert!((scale[1] - 3.0).abs() < 1.0e-4);
        assert!((scale[2] - 4.0).abs() < 1.0e-4);
    }

    #[test]
    fn transform_preserves_log_scale_encoding() {
        let mut splats = SplatGeo::with_len(1);
        let log_half = 0.5f32.ln();
        splats.scales[0] = [log_half, log_half, log_half];
        splats.rotations[0] = [1.0, 0.0, 0.0, 0.0];

        splats.transform(Mat4::from_scale(Vec3::splat(2.0)));

        let scale = splats.scales[0];
        assert!(scale[0].abs() < 1.0e-4);
        assert!(scale[1].abs() < 1.0e-4);
        assert!(scale[2].abs() < 1.0e-4);
    }

    #[test]
    fn transform_rotates_sh_l1() {
        let mut splats = SplatGeo::with_len_and_sh(1, 3);
        splats.sh_rest[0] = [0.0, 0.0, 0.0];
        splats.sh_rest[1] = [0.0, 0.0, 0.0];
        splats.sh_rest[2] = [-1.0, 0.0, 0.0];

        let matrix = Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
        splats.transform(matrix);

        let coeffs = &splats.sh_rest[0..3];
        assert!((coeffs[0][0] + 1.0).abs() < 1.0e-4);
        assert!(coeffs[1][0].abs() < 1.0e-4);
        assert!(coeffs[2][0].abs() < 1.0e-4);
    }

    #[test]
    fn transform_rotates_sh_l2() {
        let mut splats = SplatGeo::with_len_and_sh(1, 8);
        splats.sh_rest[4] = [1.0, 0.0, 0.0];

        let matrix = Mat4::from_rotation_z(std::f32::consts::PI);
        splats.transform(matrix);

        let coeff = splats.sh_rest[4][0];
        assert!((coeff + 1.0).abs() < 2.0e-3);
    }

    #[test]
    fn transform_rotates_sh_l3() {
        let mut splats = SplatGeo::with_len_and_sh(1, 15);
        splats.sh_rest[13] = [1.0, 0.0, 0.0];

        let matrix = Mat4::from_rotation_z(std::f32::consts::PI);
        splats.transform(matrix);

        let coeff = splats.sh_rest[13][0];
        assert!((coeff - 1.0).abs() < 2.0e-3);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn save_and_load_roundtrip() {
        let mut splats = SplatGeo::with_len_and_sh(2, 3);
        splats.positions[0] = [1.0, 2.0, 3.0];
        splats.positions[1] = [4.0, 5.0, 6.0];
        splats.opacity[0] = 0.8;
        splats.sh0[0] = [0.1, 0.2, 0.3];
        splats.sh_rest[0] = [1.0, 2.0, 3.0];
        splats.sh_rest[1] = [4.0, 5.0, 6.0];
        splats.sh_rest[2] = [7.0, 8.0, 9.0];

        let path = std::env::temp_dir().join("lobedo_splats_roundtrip.ply");
        save_splat_ply(path.to_str().unwrap(), &splats).expect("save");
        let loaded = load_splat_ply(path.to_str().unwrap()).expect("load");

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.sh_coeffs, 3);
        assert_eq!(loaded.positions[0], [1.0, 2.0, 3.0]);
        assert!((loaded.opacity[0] - 0.8).abs() < 1.0e-4);
        assert_eq!(loaded.sh0[0], [0.1, 0.2, 0.3]);
        assert_eq!(loaded.sh_rest[0], [1.0, 2.0, 3.0]);
        assert_eq!(loaded.sh_rest[1], [4.0, 5.0, 6.0]);
        assert_eq!(loaded.sh_rest[2], [7.0, 8.0, 9.0]);
        let _ = std::fs::remove_file(path);
    }
}
