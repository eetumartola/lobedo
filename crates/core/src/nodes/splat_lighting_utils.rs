use glam::{Quat, Vec3};

use crate::attributes::{AttributeDomain, AttributeRef};
use crate::splat::SplatGeo;

pub fn selected(mask: Option<&[bool]>, idx: usize) -> bool {
    mask.map(|mask| mask.get(idx).copied().unwrap_or(false))
        .unwrap_or(true)
}

pub fn average_env_coeffs(splats: &SplatGeo, mask: Option<&[bool]>) -> Vec<[f32; 3]> {
    let sh_coeffs = splats.sh_coeffs;
    let mut sum = vec![[0.0, 0.0, 0.0]; 1 + sh_coeffs];
    let mut weight_sum = 0.0f32;
    for idx in 0..splats.len() {
        if !selected(mask, idx) {
            continue;
        }
        let mut weight = splats.opacity.get(idx).copied().unwrap_or(0.0);
        weight = 1.0 / (1.0 + (-weight).exp());
        if !weight.is_finite() {
            weight = 0.0;
        }
        weight = weight.clamp(0.0, 1.0);
        sum[0][0] += splats.sh0[idx][0] * weight;
        sum[0][1] += splats.sh0[idx][1] * weight;
        sum[0][2] += splats.sh0[idx][2] * weight;
        let base = idx * sh_coeffs;
        for coeff in 0..sh_coeffs {
            if let Some(slot) = splats.sh_rest.get(base + coeff) {
                sum[coeff + 1][0] += slot[0] * weight;
                sum[coeff + 1][1] += slot[1] * weight;
                sum[coeff + 1][2] += slot[2] * weight;
            }
        }
        weight_sum += weight;
    }
    if weight_sum <= 1.0e-6 {
        return sum;
    }
    let inv = 1.0 / weight_sum;
    for coeff in &mut sum {
        coeff[0] *= inv;
        coeff[1] *= inv;
        coeff[2] *= inv;
    }
    sum
}

pub fn estimate_splat_normals(splats: &SplatGeo) -> Vec<Vec3> {
    if let Some(AttributeRef::Vec3(values)) = splats.attribute(AttributeDomain::Point, "N") {
        if values.len() == splats.len() {
            return values.iter().copied().map(Vec3::from).collect();
        }
    }

    splats
        .rotations
        .iter()
        .zip(splats.scales.iter())
        .map(|(rotation, scale)| {
            let axis = if scale[0] <= scale[1] && scale[0] <= scale[2] {
                Vec3::X
            } else if scale[1] <= scale[2] {
                Vec3::Y
            } else {
                Vec3::Z
            };
            let mut quat =
                Quat::from_xyzw(rotation[1], rotation[2], rotation[3], rotation[0]);
            if quat.length_squared() > 0.0 {
                quat = quat.normalize();
            } else {
                quat = Quat::IDENTITY;
            }
            let mut normal = quat * axis;
            if normal.length_squared() == 0.0 {
                normal = Vec3::Y;
            } else {
                normal = normal.normalize();
            }
            normal
        })
        .collect()
}
