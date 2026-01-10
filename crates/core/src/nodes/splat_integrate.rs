use std::collections::BTreeMap;

use glam::{Quat, Vec3};

use crate::attributes::{AttributeDomain, AttributeRef};
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{geometry_in, geometry_out, group_utils::splat_group_mask, require_mesh_input};
use crate::splat::SplatGeo;

pub const NAME: &str = "Splat Integrate";

#[allow(clippy::excessive_precision)]
const SH_C0: f32 = 0.28209479177387814;

#[allow(clippy::excessive_precision)]
const IRRADIANCE_C1: f32 = 0.429043;
#[allow(clippy::excessive_precision)]
const IRRADIANCE_C2: f32 = 0.511664;
#[allow(clippy::excessive_precision)]
const IRRADIANCE_C3: f32 = 0.743125;
#[allow(clippy::excessive_precision)]
const IRRADIANCE_C4: f32 = 0.886227;
#[allow(clippy::excessive_precision)]
const IRRADIANCE_C5: f32 = 0.247708;

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![geometry_in("source"), geometry_in("target")],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
            ("relight_mode".to_string(), ParamValue::Int(2)),
            ("source_env".to_string(), ParamValue::Int(0)),
            ("target_env".to_string(), ParamValue::Int(0)),
            (
                "source_color".to_string(),
                ParamValue::Vec3([1.0, 1.0, 1.0]),
            ),
            (
                "target_color".to_string(),
                ParamValue::Vec3([1.0, 1.0, 1.0]),
            ),
            ("eps".to_string(), ParamValue::Float(1.0e-3)),
            ("ratio_min".to_string(), ParamValue::Float(0.25)),
            ("ratio_max".to_string(), ParamValue::Float(4.0)),
            ("high_band_gain".to_string(), ParamValue::Float(0.4)),
            ("high_band_mode".to_string(), ParamValue::Int(0)),
            ("output_sh_order".to_string(), ParamValue::Int(3)),
            ("albedo_max".to_string(), ParamValue::Float(2.0)),
        ]),
    }
}

pub fn compute(_params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let input = require_mesh_input(inputs, 0, "Splat Integrate requires a mesh input")?;
    Ok(input)
}

pub fn apply_to_geometry(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let Some(source) = inputs.first() else {
        return Ok(Geometry::default());
    };
    let target = inputs.get(1);
    let target_splats = target.and_then(|geo| geo.merged_splats());

    let mut meshes = Vec::new();
    if let Some(mesh) = source.merged_mesh() {
        meshes.push(mesh);
    }

    let mut splats = Vec::with_capacity(source.splats.len());
    for splat in &source.splats {
        splats.push(apply_to_splats(params, splat, target_splats.as_ref()));
    }

    let curves = if meshes.is_empty() { Vec::new() } else { source.curves.clone() };
    Ok(Geometry {
        meshes,
        splats,
        curves,
        volumes: source.volumes.clone(),
        materials: source.materials.clone(),
    })
}

fn apply_to_splats(
    params: &NodeParams,
    splats: &SplatGeo,
    target: Option<&SplatGeo>,
) -> SplatGeo {
    if splats.is_empty() {
        return splats.clone();
    }
    let mut output = splats.clone();
    let mask = splat_group_mask(&output, params, AttributeDomain::Point);
    let mask = mask.as_deref();
    apply_to_splats_internal(params, &mut output, mask, target);
    output
}

fn apply_to_splats_internal(
    params: &NodeParams,
    splats: &mut SplatGeo,
    mask: Option<&[bool]>,
    target: Option<&SplatGeo>,
) {
    let sh_coeffs = splats.sh_coeffs;
    let count = splats.len();
    if count == 0 {
        return;
    }

    let mode = params.get_int("relight_mode", 2).clamp(0, 2);
    let output_order = params.get_int("output_sh_order", 3).clamp(0, 3);
    let max_coeffs = sh_coeffs_for_order(output_order).min(sh_coeffs);
    let high_band_gain = params.get_float("high_band_gain", 0.4).clamp(0.0, 1.0);
    let high_band_mode = params.get_int("high_band_mode", 0).clamp(0, 1);
    let eps_scale = params.get_float("eps", 1.0e-3).abs().max(1.0e-8);

    let source_env = build_env_coeffs(params, splats, mask, EnvSource::Source);
    let target_env = build_target_env_coeffs(params, target);
    let eps = eps_from_env(&source_env, eps_scale);
    let (ratio_min, ratio_max) = ratio_bounds(params);
    let ratios = build_ratio_table(&source_env, &target_env, eps, ratio_min, ratio_max);

    match mode {
        0 => {
            for idx in 0..count {
                if !selected(mask, idx) {
                    continue;
                }
                apply_ratio_to_splat(splats, idx, &ratios);
                apply_high_band_gain(splats, idx, max_coeffs, high_band_gain);
                clamp_sh_order(splats, idx, max_coeffs);
            }
        }
        1 => {
            let env_l2 = env_l2_from_coeffs(&target_env);
            let albedo_max = params.get_float("albedo_max", 2.0).max(0.0);
            let normals = estimate_splat_normals(splats);
            for idx in 0..count {
                if !selected(mask, idx) {
                    continue;
                }
                let n = normals.get(idx).copied().unwrap_or(Vec3::Y);
                let irradiance = irradiance_from_env_l2(n, &env_l2);
                let albedo = clamp_color(
                    splat_dc_color(splats, idx),
                    0.0,
                    albedo_max,
                );
                let lit = multiply_color(albedo, irradiance);
                set_splat_dc_color(splats, idx, lit);
                zero_sh_rest(splats, idx);
                clamp_sh_order(splats, idx, max_coeffs);
            }
        }
        _ => {
            let env_l2 = env_l2_from_coeffs(&target_env);
            let albedo_max = params.get_float("albedo_max", 2.0).max(0.0);
            let normals = estimate_splat_normals(splats);
            for idx in 0..count {
                if !selected(mask, idx) {
                    continue;
                }
                let n = normals.get(idx).copied().unwrap_or(Vec3::Y);
                let irradiance = irradiance_from_env_l2(n, &env_l2);
                let albedo = clamp_color(
                    splat_dc_color(splats, idx),
                    0.0,
                    albedo_max,
                );
                let lit = multiply_color(albedo, irradiance);
                set_splat_dc_color(splats, idx, lit);
                if high_band_mode == 1 {
                    apply_ratio_to_sh_rest(splats, idx, &ratios);
                }
                apply_high_band_gain(splats, idx, max_coeffs, high_band_gain);
                clamp_sh_order(splats, idx, max_coeffs);
            }
        }
    }
}

fn selected(mask: Option<&[bool]>, idx: usize) -> bool {
    match mask {
        Some(mask) => mask.get(idx).copied().unwrap_or(false),
        None => true,
    }
}

fn sh_coeffs_for_order(order: i32) -> usize {
    match order.clamp(0, 3) {
        0 => 0,
        1 => 3,
        2 => 8,
        _ => 15,
    }
}

fn zero_sh_rest(splats: &mut SplatGeo, idx: usize) {
    if splats.sh_coeffs == 0 {
        return;
    }
    let base = idx * splats.sh_coeffs;
    for coeff in 0..splats.sh_coeffs {
        if let Some(slot) = splats.sh_rest.get_mut(base + coeff) {
            *slot = [0.0, 0.0, 0.0];
        }
    }
}

fn clamp_sh_order(splats: &mut SplatGeo, idx: usize, max_coeffs: usize) {
    if splats.sh_coeffs == 0 || max_coeffs >= splats.sh_coeffs {
        return;
    }
    let base = idx * splats.sh_coeffs;
    for coeff in max_coeffs..splats.sh_coeffs {
        if let Some(slot) = splats.sh_rest.get_mut(base + coeff) {
            *slot = [0.0, 0.0, 0.0];
        }
    }
}

fn apply_high_band_gain(splats: &mut SplatGeo, idx: usize, max_coeffs: usize, gain: f32) {
    if splats.sh_coeffs == 0 || (gain - 1.0).abs() < 1.0e-6 {
        return;
    }
    let base = idx * splats.sh_coeffs;
    let limit = max_coeffs.min(splats.sh_coeffs);
    for coeff in 0..limit {
        if let Some(slot) = splats.sh_rest.get_mut(base + coeff) {
            slot[0] *= gain;
            slot[1] *= gain;
            slot[2] *= gain;
        }
    }
}

fn apply_ratio_to_splat(splats: &mut SplatGeo, idx: usize, ratios: &[[f32; 3]]) {
    if splats.sh_coeffs > 0 {
        let ratio = ratios.first().copied().unwrap_or([1.0, 1.0, 1.0]);
        splats.sh0[idx][0] *= ratio[0];
        splats.sh0[idx][1] *= ratio[1];
        splats.sh0[idx][2] *= ratio[2];
        apply_ratio_to_sh_rest(splats, idx, ratios);
    } else {
        let ratio = ratios.first().copied().unwrap_or([1.0, 1.0, 1.0]);
        splats.sh0[idx][0] *= ratio[0];
        splats.sh0[idx][1] *= ratio[1];
        splats.sh0[idx][2] *= ratio[2];
    }
}

fn apply_ratio_to_sh_rest(splats: &mut SplatGeo, idx: usize, ratios: &[[f32; 3]]) {
    if splats.sh_coeffs == 0 {
        return;
    }
    let base = idx * splats.sh_coeffs;
    for coeff in 0..splats.sh_coeffs {
        let ratio = ratios.get(coeff + 1).copied().unwrap_or([1.0, 1.0, 1.0]);
        if let Some(slot) = splats.sh_rest.get_mut(base + coeff) {
            slot[0] *= ratio[0];
            slot[1] *= ratio[1];
            slot[2] *= ratio[2];
        }
    }
}

fn ratio_bounds(params: &NodeParams) -> (f32, f32) {
    let mut min = params.get_float("ratio_min", 0.25);
    let mut max = params.get_float("ratio_max", 4.0);
    if !min.is_finite() {
        min = 0.25;
    }
    if !max.is_finite() {
        max = 4.0;
    }
    if max < min {
        std::mem::swap(&mut min, &mut max);
    }
    (min, max)
}

fn build_ratio_table(
    source: &[[f32; 3]],
    target: &[[f32; 3]],
    eps: f32,
    min: f32,
    max: f32,
) -> Vec<[f32; 3]> {
    let mut ratios = Vec::with_capacity(source.len());
    for (idx, src) in source.iter().enumerate() {
        let tgt = target.get(idx).copied().unwrap_or([0.0, 0.0, 0.0]);
        let mut ratio = [1.0f32; 3];
        for channel in 0..3 {
            let s = src[channel];
            let t = tgt[channel];
            if !s.is_finite() || !t.is_finite() {
                ratio[channel] = 1.0;
                continue;
            }
            let denom = s + eps;
            if denom.abs() < 1.0e-6 {
                ratio[channel] = 1.0;
                continue;
            }
            ratio[channel] = ((t + eps) / denom).clamp(min, max);
        }
        ratios.push(ratio);
    }
    ratios
}

fn build_env_coeffs(
    params: &NodeParams,
    splats: &SplatGeo,
    mask: Option<&[bool]>,
    source: EnvSource,
) -> Vec<[f32; 3]> {
    let mode = match source {
        EnvSource::Source => params.get_int("source_env", 0),
        EnvSource::Target => params.get_int("target_env", 0),
    };
    match (source, mode) {
        (EnvSource::Source, 0) => average_env_coeffs(splats, mask),
        (EnvSource::Target, 0) => average_env_coeffs(splats, None),
        (EnvSource::Source, 1) => uniform_env_coeffs([1.0, 1.0, 1.0], splats.sh_coeffs),
        (EnvSource::Target, 1) => uniform_env_coeffs([1.0, 1.0, 1.0], splats.sh_coeffs),
        (EnvSource::Source, 2) => {
            let color = params.get_vec3("source_color", [1.0, 1.0, 1.0]);
            uniform_env_coeffs(color, splats.sh_coeffs)
        }
        (EnvSource::Target, 2) => {
            let color = params.get_vec3("target_color", [1.0, 1.0, 1.0]);
            uniform_env_coeffs(color, splats.sh_coeffs)
        }
        _ => uniform_env_coeffs([1.0, 1.0, 1.0], splats.sh_coeffs),
    }
}

fn build_target_env_coeffs(params: &NodeParams, target: Option<&SplatGeo>) -> Vec<[f32; 3]> {
    if let Some(target_splats) = target {
        build_env_coeffs(params, target_splats, None, EnvSource::Target)
    } else {
        uniform_env_coeffs([1.0, 1.0, 1.0], 0)
    }
}

fn uniform_env_coeffs(color: [f32; 3], sh_coeffs: usize) -> Vec<[f32; 3]> {
    let mut coeffs = vec![[0.0, 0.0, 0.0]; 1 + sh_coeffs];
    coeffs[0] = if sh_coeffs > 0 {
        let bias = Vec3::splat(0.5);
        let color = (Vec3::from(color) - bias) / SH_C0;
        color.to_array()
    } else {
        color
    };
    coeffs
}

fn average_env_coeffs(splats: &SplatGeo, mask: Option<&[bool]>) -> Vec<[f32; 3]> {
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

fn eps_from_env(env: &[[f32; 3]], eps_scale: f32) -> f32 {
    let mut max_abs = 0.0f32;
    for coeff in env {
        for channel in coeff {
            if channel.is_finite() {
                max_abs = max_abs.max(channel.abs());
            }
        }
    }
    if max_abs <= 0.0 {
        eps_scale
    } else {
        max_abs * eps_scale
    }
}

fn env_l2_from_coeffs(env: &[[f32; 3]]) -> Vec<[f32; 3]> {
    let mut out = vec![[0.0, 0.0, 0.0]; 9];
    for (idx, slot) in out.iter_mut().enumerate().take(9) {
        if let Some(value) = env.get(idx) {
            *slot = *value;
        }
    }
    out
}

fn irradiance_from_env_l2(n: Vec3, env: &[[f32; 3]]) -> [f32; 3] {
    let n = if n.length_squared() > 0.0 {
        n.normalize()
    } else {
        Vec3::Y
    };
    let x = n.x;
    let y = n.y;
    let z = n.z;

    let mut e = Vec3::ZERO;
    let mut add = |a: f32, v: [f32; 3]| {
        e += Vec3::from(v) * a;
    };

    add(IRRADIANCE_C4, env[0]);
    add(2.0 * IRRADIANCE_C2 * x, env[3]);
    add(2.0 * IRRADIANCE_C2 * y, env[1]);
    add(2.0 * IRRADIANCE_C2 * z, env[2]);
    add(IRRADIANCE_C1 * (x * x - y * y), env[8]);
    add(2.0 * IRRADIANCE_C1 * x * y, env[4]);
    add(2.0 * IRRADIANCE_C1 * y * z, env[5]);
    add(2.0 * IRRADIANCE_C1 * x * z, env[7]);
    add(IRRADIANCE_C3 * z * z - IRRADIANCE_C5, env[6]);

    e.x = e.x.max(0.0);
    e.y = e.y.max(0.0);
    e.z = e.z.max(0.0);
    e.to_array()
}

fn splat_dc_color(splats: &SplatGeo, idx: usize) -> [f32; 3] {
    if splats.sh_coeffs > 0 {
        let coeff = Vec3::from(splats.sh0[idx]);
        let color = coeff * SH_C0 + Vec3::splat(0.5);
        color.to_array()
    } else {
        splats.sh0[idx]
    }
}

fn set_splat_dc_color(splats: &mut SplatGeo, idx: usize, color: [f32; 3]) {
    if splats.sh_coeffs > 0 {
        let coeff = (Vec3::from(color) - Vec3::splat(0.5)) / SH_C0;
        splats.sh0[idx] = coeff.to_array();
    } else {
        splats.sh0[idx] = color;
    }
}

fn multiply_color(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2]]
}

fn clamp_color(color: [f32; 3], min: f32, max: f32) -> [f32; 3] {
    [
        color[0].clamp(min, max),
        color[1].clamp(min, max),
        color[2].clamp(min, max),
    ]
}

fn estimate_splat_normals(splats: &SplatGeo) -> Vec<Vec3> {
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

enum EnvSource {
    Source,
    Target,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::geometry::Geometry;
    use crate::graph::{NodeParams, ParamValue};
    use crate::splat::SplatGeo;

    use super::apply_to_geometry;

    #[test]
    fn integrate_ratio_scales_sh0() {
        let mut source = SplatGeo::with_len_and_sh(1, 3);
        source.sh0[0] = [0.5, 0.5, 0.5];
        let mut target = SplatGeo::with_len_and_sh(1, 3);
        target.sh0[0] = [1.0, 1.0, 1.0];

        let params = NodeParams {
            values: BTreeMap::from([
                ("relight_mode".to_string(), ParamValue::Int(0)),
                ("source_env".to_string(), ParamValue::Int(0)),
                ("target_env".to_string(), ParamValue::Int(0)),
            ]),
        };
        let geo = apply_to_geometry(
            &params,
            &[
                Geometry::with_splats(source),
                Geometry::with_splats(target),
            ],
        )
        .expect("geometry");
        let out = geo.merged_splats().expect("splats");
        assert!(out.sh0[0][0] > 0.5);
    }
}
