use glam::Quat;

use crate::attributes::AttributeDomain;

use super::{SplatGeo, SPLAT_ALPHA_MAX, SPLAT_ALPHA_MIN, SPLAT_LOG_SCALE_MAX, SPLAT_LOG_SCALE_MIN};

impl SplatGeo {
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
