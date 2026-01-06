use glam::{Mat3, Mat4, Quat, Vec3};

use crate::attributes::{
    AttributeDomain, AttributeError, AttributeInfo, AttributeRef, AttributeStorage, AttributeType,
    MeshAttributes,
};
use crate::mesh::MeshGroups;
pub use crate::splat_ply::{load_splat_ply_with_mode, save_splat_ply, SplatLoadMode};

const SPLAT_LOG_SCALE_MIN: f32 = -10.0;
const SPLAT_LOG_SCALE_MAX: f32 = 10.0;
const SPLAT_ALPHA_MIN: f32 = 1.0e-4;
const SPLAT_ALPHA_MAX: f32 = 1.0 - 1.0e-4;

#[derive(Debug, Clone, Default)]
pub struct SplatGeo {
    pub positions: Vec<[f32; 3]>,
    pub rotations: Vec<[f32; 4]>,
    pub scales: Vec<[f32; 3]>,
    pub opacity: Vec<f32>,
    pub sh0: Vec<[f32; 3]>,
    pub sh_coeffs: usize,
    pub sh_rest: Vec<[f32; 3]>,
    pub attributes: MeshAttributes,
    pub groups: MeshGroups,
}

impl SplatGeo {
    pub fn with_len(count: usize) -> Self {
        Self {
            positions: vec![[0.0, 0.0, 0.0]; count],
            rotations: vec![[0.0, 0.0, 0.0, 1.0]; count],
            scales: vec![[0.0, 0.0, 0.0]; count],
            opacity: vec![1.0; count],
            sh0: vec![[1.0, 1.0, 1.0]; count],
            sh_coeffs: 0,
            sh_rest: Vec::new(),
            attributes: MeshAttributes::default(),
            groups: MeshGroups::default(),
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

    pub fn normalize_on_load(&mut self) {
        self.normalize_rotations();
        self.normalize_log_scales();
        self.normalize_logit_opacity();
    }

    pub fn normalized_for_save(&self) -> SplatGeo {
        let mut out = self.clone();
        out.normalize_rotations();
        out.normalize_log_scales();
        out.normalize_logit_opacity();
        out
    }

    fn normalize_rotations(&mut self) {
        for rot in &mut self.rotations {
            let mut quat = Quat::from_xyzw(rot[1], rot[2], rot[3], rot[0]);
            if quat.length_squared() > 0.0 {
                quat = quat.normalize();
            } else {
                quat = Quat::IDENTITY;
            }
            *rot = [quat.w, quat.x, quat.y, quat.z];
        }
    }

    fn normalize_log_scales(&mut self) {
        for scale in &mut self.scales {
            for component in scale.iter_mut() {
                if !component.is_finite() {
                    *component = 0.0;
                }
                *component = component.clamp(SPLAT_LOG_SCALE_MIN, SPLAT_LOG_SCALE_MAX);
            }
        }
    }

    fn normalize_logit_opacity(&mut self) {
        let min_logit = logit(SPLAT_ALPHA_MIN);
        let max_logit = logit(SPLAT_ALPHA_MAX);
        for value in &mut self.opacity {
            if !value.is_finite() {
                *value = 0.0;
            }
            *value = value.clamp(min_logit, max_logit);
        }
    }

    pub fn attribute_domain_len(&self, domain: AttributeDomain) -> usize {
        match domain {
            AttributeDomain::Point => self.positions.len(),
            AttributeDomain::Vertex => 0,
            AttributeDomain::Primitive => self.positions.len(),
            AttributeDomain::Detail => {
                if self.positions.is_empty() {
                    0
                } else {
                    1
                }
            }
        }
    }

    pub fn list_attributes(&self) -> Vec<AttributeInfo> {
        let mut list = Vec::new();
        if !self.positions.is_empty() {
            list.push(AttributeInfo {
                name: "P".to_string(),
                domain: AttributeDomain::Point,
                data_type: AttributeType::Vec3,
                len: self.positions.len(),
                implicit: true,
            });
        }
        if !self.rotations.is_empty() {
            list.push(AttributeInfo {
                name: "orient".to_string(),
                domain: AttributeDomain::Point,
                data_type: AttributeType::Vec4,
                len: self.rotations.len(),
                implicit: true,
            });
        }
        if !self.scales.is_empty() {
            list.push(AttributeInfo {
                name: "scale".to_string(),
                domain: AttributeDomain::Point,
                data_type: AttributeType::Vec3,
                len: self.scales.len(),
                implicit: true,
            });
        }
        if !self.opacity.is_empty() {
            list.push(AttributeInfo {
                name: "opacity".to_string(),
                domain: AttributeDomain::Point,
                data_type: AttributeType::Float,
                len: self.opacity.len(),
                implicit: true,
            });
        }
        if !self.sh0.is_empty() {
            list.push(AttributeInfo {
                name: "Cd".to_string(),
                domain: AttributeDomain::Point,
                data_type: AttributeType::Vec3,
                len: self.sh0.len(),
                implicit: true,
            });
        }
        for domain in [AttributeDomain::Point, AttributeDomain::Primitive, AttributeDomain::Detail] {
            for (name, storage) in self.attributes.map(domain) {
                list.push(AttributeInfo {
                    name: name.clone(),
                    domain,
                    data_type: storage.data_type(),
                    len: storage.len(),
                    implicit: false,
                });
            }
        }
        list
    }

    pub fn attribute(&self, domain: AttributeDomain, name: &str) -> Option<AttributeRef<'_>> {
        match (name, domain) {
            ("P", AttributeDomain::Point) => Some(AttributeRef::Vec3(self.positions.as_slice())),
            ("Cd", AttributeDomain::Point)
            | ("Cd", AttributeDomain::Primitive)
            | ("sh0", AttributeDomain::Point)
            | ("sh0", AttributeDomain::Primitive) => {
                Some(AttributeRef::Vec3(self.sh0.as_slice()))
            }
            ("opacity", AttributeDomain::Point)
            | ("opacity", AttributeDomain::Primitive) => {
                Some(AttributeRef::Float(self.opacity.as_slice()))
            }
            ("scale", AttributeDomain::Point) | ("scale", AttributeDomain::Primitive) => {
                Some(AttributeRef::Vec3(self.scales.as_slice()))
            }
            ("orient", AttributeDomain::Point)
            | ("orient", AttributeDomain::Primitive)
            | ("rot", AttributeDomain::Point)
            | ("rot", AttributeDomain::Primitive) => {
                Some(AttributeRef::Vec4(self.rotations.as_slice()))
            }
            _ => self
                .attributes
                .get(domain, name)
                .map(AttributeStorage::as_ref),
        }
    }

    pub fn attribute_with_precedence(
        &self,
        name: &str,
    ) -> Option<(AttributeDomain, AttributeRef<'_>)> {
        if let Some(attr) = self.attribute(AttributeDomain::Point, name) {
            return Some((AttributeDomain::Point, attr));
        }
        if let Some(attr) = self.attribute(AttributeDomain::Primitive, name) {
            return Some((AttributeDomain::Primitive, attr));
        }
        if let Some(attr) = self.attribute(AttributeDomain::Detail, name) {
            return Some((AttributeDomain::Detail, attr));
        }
        None
    }

    pub fn set_attribute(
        &mut self,
        domain: AttributeDomain,
        name: impl Into<String>,
        storage: AttributeStorage,
    ) -> Result<(), AttributeError> {
        let name = name.into();
        let expected_len = self.attribute_domain_len(domain);
        let actual_len = storage.len();
        if expected_len != 0 && actual_len != expected_len {
            return Err(AttributeError::InvalidLength {
                expected: expected_len,
                actual: actual_len,
            });
        }

        match (name.as_str(), domain) {
            ("P", AttributeDomain::Point) => {
                if storage.data_type() != AttributeType::Vec3 {
                    return Err(AttributeError::InvalidType {
                        expected: AttributeType::Vec3,
                        actual: storage.data_type(),
                    });
                }
                if let AttributeStorage::Vec3(values) = storage {
                    self.positions = values;
                    return Ok(());
                }
            }
            ("P", _) => return Err(AttributeError::InvalidDomain),
            ("Cd", AttributeDomain::Point)
            | ("Cd", AttributeDomain::Primitive)
            | ("sh0", AttributeDomain::Point)
            | ("sh0", AttributeDomain::Primitive) => {
                if storage.data_type() != AttributeType::Vec3 {
                    return Err(AttributeError::InvalidType {
                        expected: AttributeType::Vec3,
                        actual: storage.data_type(),
                    });
                }
                if let AttributeStorage::Vec3(values) = storage {
                    self.sh0 = values;
                    return Ok(());
                }
            }
            ("opacity", AttributeDomain::Point) | ("opacity", AttributeDomain::Primitive) => {
                if storage.data_type() != AttributeType::Float {
                    return Err(AttributeError::InvalidType {
                        expected: AttributeType::Float,
                        actual: storage.data_type(),
                    });
                }
                if let AttributeStorage::Float(values) = storage {
                    self.opacity = values;
                    return Ok(());
                }
            }
            ("scale", AttributeDomain::Point) | ("scale", AttributeDomain::Primitive) => {
                if storage.data_type() != AttributeType::Vec3 {
                    return Err(AttributeError::InvalidType {
                        expected: AttributeType::Vec3,
                        actual: storage.data_type(),
                    });
                }
                if let AttributeStorage::Vec3(values) = storage {
                    self.scales = values;
                    return Ok(());
                }
            }
            ("orient", AttributeDomain::Point)
            | ("orient", AttributeDomain::Primitive)
            | ("rot", AttributeDomain::Point)
            | ("rot", AttributeDomain::Primitive) => {
                if storage.data_type() != AttributeType::Vec4 {
                    return Err(AttributeError::InvalidType {
                        expected: AttributeType::Vec4,
                        actual: storage.data_type(),
                    });
                }
                if let AttributeStorage::Vec4(values) = storage {
                    self.rotations = values;
                    return Ok(());
                }
            }
            _ => {}
        }

        self.attributes.map_mut(domain).insert(name, storage);
        Ok(())
    }

    pub fn remove_attribute(
        &mut self,
        domain: AttributeDomain,
        name: &str,
    ) -> Option<AttributeStorage> {
        match (name, domain) {
            ("P", AttributeDomain::Point) => None,
            ("Cd", AttributeDomain::Point)
            | ("Cd", AttributeDomain::Primitive)
            | ("sh0", AttributeDomain::Point)
            | ("sh0", AttributeDomain::Primitive) => {
                self.sh0.clear();
                None
            }
            ("opacity", AttributeDomain::Point) | ("opacity", AttributeDomain::Primitive) => {
                self.opacity.clear();
                None
            }
            ("scale", AttributeDomain::Point) | ("scale", AttributeDomain::Primitive) => {
                self.scales.clear();
                None
            }
            ("orient", AttributeDomain::Point)
            | ("orient", AttributeDomain::Primitive)
            | ("rot", AttributeDomain::Point)
            | ("rot", AttributeDomain::Primitive) => {
                self.rotations.clear();
                None
            }
            _ => self.attributes.remove(domain, name),
        }
    }

    pub fn is_finite_at(&self, idx: usize) -> bool {
        let Some(position) = self.positions.get(idx) else {
            return false;
        };
        if position.iter().any(|value| !value.is_finite()) {
            return false;
        }
        let Some(rotation) = self.rotations.get(idx) else {
            return false;
        };
        if rotation.iter().any(|value| !value.is_finite()) {
            return false;
        }
        if !rotation_is_normalized(*rotation) {
            return false;
        }
        let Some(scale) = self.scales.get(idx) else {
            return false;
        };
        if scale.iter().any(|value| !value.is_finite()) {
            return false;
        }
        if !log_scale_in_range(*scale) {
            return false;
        }
        let Some(opacity) = self.opacity.get(idx) else {
            return false;
        };
        if !opacity.is_finite() {
            return false;
        }
        if !logit_in_range(*opacity) {
            return false;
        }
        let Some(sh0) = self.sh0.get(idx) else {
            return false;
        };
        if sh0.iter().any(|value| !value.is_finite()) {
            return false;
        }
        if self.sh_coeffs > 0 {
            let base = idx * self.sh_coeffs;
            for coeff in 0..self.sh_coeffs {
                let Some(values) = self.sh_rest.get(base + coeff) else {
                    return false;
                };
                if values.iter().any(|value| !value.is_finite()) {
                    return false;
                }
            }
        }
        true
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
        if self
            .positions
            .iter()
            .any(|p| p.iter().any(|value| !value.is_finite()))
        {
            return Err("SplatGeo positions contain non-finite values".to_string());
        }
        if self
            .rotations
            .iter()
            .any(|r| r.iter().any(|value| !value.is_finite()))
        {
            return Err("SplatGeo rotations contain non-finite values".to_string());
        }
        if self
            .rotations
            .iter()
            .any(|r| !rotation_is_normalized(*r))
        {
            return Err("SplatGeo rotations are not normalized".to_string());
        }
        if self
            .scales
            .iter()
            .any(|s| s.iter().any(|value| !value.is_finite()))
        {
            return Err("SplatGeo scales contain non-finite values".to_string());
        }
        if self
            .scales
            .iter()
            .any(|s| !log_scale_in_range(*s))
        {
            return Err("SplatGeo scales out of range".to_string());
        }
        if self.opacity.iter().any(|value| !value.is_finite()) {
            return Err("SplatGeo opacity contains non-finite values".to_string());
        }
        if self.opacity.iter().any(|value| !logit_in_range(*value)) {
            return Err("SplatGeo opacity out of range".to_string());
        }
        if self
            .sh0
            .iter()
            .any(|c| c.iter().any(|value| !value.is_finite()))
        {
            return Err("SplatGeo SH0 contains non-finite values".to_string());
        }
        if self
            .sh_rest
            .iter()
            .any(|c| c.iter().any(|value| !value.is_finite()))
        {
            return Err("SplatGeo SH coefficients contain non-finite values".to_string());
        }
        if self.sh_coeffs == 0 {
            if !self.sh_rest.is_empty() {
                return Err("SplatGeo SH coefficients are inconsistent".to_string());
            }
        } else if self.sh_rest.len() != count * self.sh_coeffs {
            return Err("SplatGeo SH coefficients are inconsistent".to_string());
        }
        for domain in [AttributeDomain::Point, AttributeDomain::Primitive] {
            for (name, values) in self.groups.map(domain) {
                if values.len() != count {
                    return Err(format!(
                        "SplatGeo group '{}' has invalid length",
                        name
                    ));
                }
            }
        }
        for domain in [AttributeDomain::Point, AttributeDomain::Primitive, AttributeDomain::Detail] {
            let expected = self.attribute_domain_len(domain);
            for (name, storage) in self.attributes.map(domain) {
                let actual = storage.len();
                if expected != 0 && actual != expected {
                    return Err(format!(
                        "SplatGeo attribute '{}' has invalid length",
                        name
                    ));
                }
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
}

fn rotation_is_normalized(rotation: [f32; 4]) -> bool {
    let len_sq = rotation[0] * rotation[0]
        + rotation[1] * rotation[1]
        + rotation[2] * rotation[2]
        + rotation[3] * rotation[3];
    if !len_sq.is_finite() || len_sq < 1.0e-6 {
        return false;
    }
    (len_sq - 1.0).abs() <= 1.0e-3
}

fn log_scale_in_range(scale: [f32; 3]) -> bool {
    let min = SPLAT_LOG_SCALE_MIN;
    let max = SPLAT_LOG_SCALE_MAX;
    scale[0] >= min
        && scale[0] <= max
        && scale[1] >= min
        && scale[1] <= max
        && scale[2] >= min
        && scale[2] <= max
}

fn logit_in_range(value: f32) -> bool {
    let min = logit(SPLAT_ALPHA_MIN);
    let max = logit(SPLAT_ALPHA_MAX);
    value >= min && value <= max
}

fn logit(value: f32) -> f32 {
    let clamped = value.clamp(SPLAT_ALPHA_MIN, SPLAT_ALPHA_MAX);
    (clamped / (1.0 - clamped)).ln()
}

fn mat3_is_finite(mat: Mat3) -> bool {
    mat.to_cols_array().iter().all(|value| value.is_finite())
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
    rotation_from_linear(Mat3::from_mat4(matrix))
}

fn rotation_from_linear(linear: Mat3) -> Mat3 {
    if !mat3_is_finite(linear) {
        return Mat3::IDENTITY;
    }
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
    if !mat3_is_finite(rot) || rot.determinant().abs() < 1.0e-6 {
        return Mat3::IDENTITY;
    }
    rot
}

fn rotate_sh_bands(splats: &mut SplatGeo, index: usize, mats: &ShRotationMatrices) {
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
    if mat
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
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

#[cfg(test)]
mod tests {
    use glam::{Mat4, Quat, Vec3};

    use super::SplatGeo;

    #[test]
    fn transform_updates_positions_and_scales() {
        let mut splats = SplatGeo::with_len(1);
        splats.positions[0] = [1.0, 2.0, 3.0];
        splats.scales[0] = [0.0, 0.0, 0.0];
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
        assert!((scale[0] - 2.0_f32.ln()).abs() < 1.0e-4);
        assert!((scale[1] - 3.0_f32.ln()).abs() < 1.0e-4);
        assert!((scale[2] - 4.0_f32.ln()).abs() < 1.0e-4);
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

    #[test]
    fn validate_rejects_nan_positions() {
        let mut splats = SplatGeo::with_len(1);
        splats.positions[0][1] = f32::NAN;
        assert!(splats.validate().is_err());
    }

    #[test]
    fn validate_rejects_nan_sh_coeffs() {
        let mut splats = SplatGeo::with_len_and_sh(1, 3);
        splats.sh_rest[1] = [f32::NAN, 0.0, 0.0];
        assert!(splats.validate().is_err());
    }
}
