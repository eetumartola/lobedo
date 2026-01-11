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
    let mut count = 0u32;
    for idx in 0..splats.len() {
        if !selected(mask, idx) {
            continue;
        }
        sum[0][0] += splats.sh0[idx][0];
        sum[0][1] += splats.sh0[idx][1];
        sum[0][2] += splats.sh0[idx][2];
        let base = idx * sh_coeffs;
        for coeff in 0..sh_coeffs {
            if let Some(slot) = splats.sh_rest.get(base + coeff) {
                sum[coeff + 1][0] += slot[0];
                sum[coeff + 1][1] += slot[1];
                sum[coeff + 1][2] += slot[2];
            }
        }
        count += 1;
    }
    if count == 0 {
        return sum;
    }
    let inv = 1.0 / count as f32;
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
