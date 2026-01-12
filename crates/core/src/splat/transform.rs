use glam::{Mat3, Mat4, Quat, Vec3};

use crate::attributes::{AttributeDomain, AttributeStorage, StringTableAttribute};

use super::math::{eigen_decomposition_symmetric, mat3_is_finite, rotation_from_linear, rotation_from_matrix};
use super::sh::{build_sh_rotation_matrices, rotate_sh_bands};
use super::{SplatGeo, SPLAT_LOG_SCALE_MAX, SPLAT_LOG_SCALE_MIN};

impl SplatGeo {
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
        let linear = Mat3::from_mat4(matrix);
        let min_scale = SPLAT_LOG_SCALE_MIN.exp();

        for idx in 0..self.positions.len() {
            let position = matrix.transform_point3(Vec3::from(self.positions[idx]));
            self.positions[idx] = position.to_array();

            let mut log_scale = Vec3::from(self.scales[idx]);
            log_scale = Vec3::new(
                log_scale.x.clamp(SPLAT_LOG_SCALE_MIN, SPLAT_LOG_SCALE_MAX),
                log_scale.y.clamp(SPLAT_LOG_SCALE_MIN, SPLAT_LOG_SCALE_MAX),
                log_scale.z.clamp(SPLAT_LOG_SCALE_MIN, SPLAT_LOG_SCALE_MAX),
            );
            let mut scale = Vec3::new(log_scale.x.exp(), log_scale.y.exp(), log_scale.z.exp());
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
            let cov_world =
                linear * (rot_mat * cov_local * rot_mat.transpose()) * linear.transpose();

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

            self.scales[idx] = [sigma.x.ln(), sigma.y.ln(), sigma.z.ln()];

            if let Some(mats) = &sh_mats {
                rotate_sh_bands(self, idx, mats);
            }
        }

        if let Some(AttributeStorage::Vec3(normals)) =
            self.attributes.map_mut(AttributeDomain::Point).get_mut("N")
        {
            if normals.len() == self.positions.len() {
                let normal_matrix = matrix.inverse().transpose();
                for n in normals.iter_mut() {
                    let v = normal_matrix.transform_vector3(Vec3::from(*n));
                    let len = v.length();
                    *n = if len > 0.0 {
                        (v / len).to_array()
                    } else {
                        [0.0, 1.0, 0.0]
                    };
                }
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
        let linear = Mat3::from_mat4(matrix);
        let min_scale = SPLAT_LOG_SCALE_MIN.exp();

        for (idx, selected) in mask.iter().enumerate() {
            if !*selected {
                continue;
            }

            let position = matrix.transform_point3(Vec3::from(self.positions[idx]));
            self.positions[idx] = position.to_array();

            let mut log_scale = Vec3::from(self.scales[idx]);
            log_scale = Vec3::new(
                log_scale.x.clamp(SPLAT_LOG_SCALE_MIN, SPLAT_LOG_SCALE_MAX),
                log_scale.y.clamp(SPLAT_LOG_SCALE_MIN, SPLAT_LOG_SCALE_MAX),
                log_scale.z.clamp(SPLAT_LOG_SCALE_MIN, SPLAT_LOG_SCALE_MAX),
            );
            let mut scale = Vec3::new(log_scale.x.exp(), log_scale.y.exp(), log_scale.z.exp());
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
            let cov_world =
                linear * (rot_mat * cov_local * rot_mat.transpose()) * linear.transpose();

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

            self.scales[idx] = [sigma.x.ln(), sigma.y.ln(), sigma.z.ln()];

            if let Some(mats) = &sh_mats {
                rotate_sh_bands(self, idx, mats);
            }
        }

        if let Some(AttributeStorage::Vec3(normals)) =
            self.attributes.map_mut(AttributeDomain::Point).get_mut("N")
        {
            if normals.len() == self.positions.len() {
                let normal_matrix = matrix.inverse().transpose();
                for (idx, n) in normals.iter_mut().enumerate() {
                    if !mask.get(idx).copied().unwrap_or(false) {
                        continue;
                    }
                    let v = normal_matrix.transform_vector3(Vec3::from(*n));
                    let len = v.length();
                    *n = if len > 0.0 {
                        (v / len).to_array()
                    } else {
                        [0.0, 1.0, 0.0]
                    };
                }
            }
        }
    }

    pub fn apply_linear_deform(&mut self, idx: usize, linear: Mat3) {
        if idx >= self.positions.len() {
            return;
        }
        if !mat3_is_finite(linear) {
            return;
        }

        let sh_mats = if self.sh_coeffs >= 3 {
            Some(build_sh_rotation_matrices(
                rotation_from_linear(linear),
                self.sh_coeffs,
            ))
        } else {
            None
        };
        let min_scale = SPLAT_LOG_SCALE_MIN.exp();

        let mut log_scale = Vec3::from(self.scales[idx]);
        log_scale = Vec3::new(
            log_scale.x.clamp(SPLAT_LOG_SCALE_MIN, SPLAT_LOG_SCALE_MAX),
            log_scale.y.clamp(SPLAT_LOG_SCALE_MIN, SPLAT_LOG_SCALE_MAX),
            log_scale.z.clamp(SPLAT_LOG_SCALE_MIN, SPLAT_LOG_SCALE_MAX),
        );
        let mut scale = Vec3::new(log_scale.x.exp(), log_scale.y.exp(), log_scale.z.exp());
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
        self.scales[idx] = [sigma.x.ln(), sigma.y.ln(), sigma.z.ln()];

        if let Some(mats) = &sh_mats {
            rotate_sh_bands(self, idx, mats);
        }
    }

    pub fn filter_by_indices(&self, kept: &[usize]) -> SplatGeo {
        let mut output = SplatGeo::with_len_and_sh(kept.len(), self.sh_coeffs);
        for (out_idx, src_idx) in kept.iter().copied().enumerate() {
            output.positions[out_idx] = self.positions[src_idx];
            output.rotations[out_idx] = self.rotations[src_idx];
            output.scales[out_idx] = self.scales[src_idx];
            output.opacity[out_idx] = self.opacity[src_idx];
            output.sh0[out_idx] = self.sh0[src_idx];
            if self.sh_coeffs > 0 {
                let src_base = src_idx * self.sh_coeffs;
                let dst_base = out_idx * self.sh_coeffs;
                output.sh_rest[dst_base..dst_base + self.sh_coeffs]
                    .copy_from_slice(&self.sh_rest[src_base..src_base + self.sh_coeffs]);
            }
        }

        for domain in [AttributeDomain::Point, AttributeDomain::Primitive] {
            for (name, values) in self.groups.map(domain) {
                let filtered = kept
                    .iter()
                    .map(|&idx| values.get(idx).copied().unwrap_or(false))
                    .collect();
                output
                    .groups
                    .map_mut(domain)
                    .insert(name.clone(), filtered);
            }
        }

        for domain in [AttributeDomain::Point, AttributeDomain::Primitive] {
            for (name, storage) in self.attributes.map(domain) {
                let filtered = filter_attribute_storage(storage, kept);
                output
                    .attributes
                    .map_mut(domain)
                    .insert(name.clone(), filtered);
            }
        }
        for (name, storage) in self.attributes.map(AttributeDomain::Detail) {
            output
                .attributes
                .map_mut(AttributeDomain::Detail)
                .insert(name.clone(), storage.clone());
        }

        output
    }

    pub fn flip_y_axis(&mut self) {
        if self.positions.is_empty() {
            return;
        }
        let matrix = Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0));
        self.transform(matrix);
    }
}

fn filter_attribute_storage(storage: &AttributeStorage, indices: &[usize]) -> AttributeStorage {
    match storage {
        AttributeStorage::Float(values) => {
            let mut out = Vec::with_capacity(indices.len());
            for &idx in indices {
                if let Some(value) = values.get(idx) {
                    out.push(*value);
                }
            }
            AttributeStorage::Float(out)
        }
        AttributeStorage::Int(values) => {
            let mut out = Vec::with_capacity(indices.len());
            for &idx in indices {
                if let Some(value) = values.get(idx) {
                    out.push(*value);
                }
            }
            AttributeStorage::Int(out)
        }
        AttributeStorage::Vec2(values) => {
            let mut out = Vec::with_capacity(indices.len());
            for &idx in indices {
                if let Some(value) = values.get(idx) {
                    out.push(*value);
                }
            }
            AttributeStorage::Vec2(out)
        }
        AttributeStorage::Vec3(values) => {
            let mut out = Vec::with_capacity(indices.len());
            for &idx in indices {
                if let Some(value) = values.get(idx) {
                    out.push(*value);
                }
            }
            AttributeStorage::Vec3(out)
        }
        AttributeStorage::Vec4(values) => {
            let mut out = Vec::with_capacity(indices.len());
            for &idx in indices {
                if let Some(value) = values.get(idx) {
                    out.push(*value);
                }
            }
            AttributeStorage::Vec4(out)
        }
        AttributeStorage::StringTable(values) => {
            let mut out = Vec::with_capacity(indices.len());
            for &idx in indices {
                if let Some(value) = values.indices.get(idx) {
                    out.push(*value);
                }
            }
            AttributeStorage::StringTable(StringTableAttribute::new(values.values.clone(), out))
        }
    }
}
