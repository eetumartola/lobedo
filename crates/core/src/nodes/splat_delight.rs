use std::collections::BTreeMap;

use glam::Vec3;

use crate::attributes::AttributeDomain;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    geometry_in,
    geometry_out,
    group_utils::splat_group_mask,
    require_mesh_input,
    splat_lighting_utils::{average_env_coeffs, estimate_splat_normals, selected},
};
use crate::parallel;
#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;
use crate::splat::SplatGeo;

pub const NAME: &str = "Splat Delight";

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
        inputs: vec![geometry_in("splats")],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
            ("delight_mode".to_string(), ParamValue::Int(1)),
            ("source_env".to_string(), ParamValue::Int(0)),
            ("neutral_env".to_string(), ParamValue::Int(0)),
            (
                "source_color".to_string(),
                ParamValue::Vec3([1.0, 1.0, 1.0]),
            ),
            (
                "neutral_color".to_string(),
                ParamValue::Vec3([1.0, 1.0, 1.0]),
            ),
            ("eps".to_string(), ParamValue::Float(1.0e-3)),
            ("ratio_min".to_string(), ParamValue::Float(0.25)),
            ("ratio_max".to_string(), ParamValue::Float(4.0)),
            ("high_band_gain".to_string(), ParamValue::Float(0.25)),
            ("output_sh_order".to_string(), ParamValue::Int(3)),
            ("albedo_max".to_string(), ParamValue::Float(2.0)),
        ]),
    }
}

pub fn compute(_params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let input = require_mesh_input(inputs, 0, "Splat Delight requires a mesh input")?;
    Ok(input)
}

pub fn apply_to_splats(params: &NodeParams, splats: &SplatGeo) -> SplatGeo {
    if splats.is_empty() {
        return splats.clone();
    }
    let mut output = splats.clone();
    apply_to_splats_in_place(params, &mut output);
    output
}

fn apply_to_splats_in_place(params: &NodeParams, splats: &mut SplatGeo) {
    let Some(mask) = splat_group_mask(splats, params, AttributeDomain::Point) else {
        apply_to_splats_internal(params, splats, None);
        return;
    };
    apply_to_splats_internal(params, splats, Some(&mask));
}

fn apply_to_splats_internal(params: &NodeParams, splats: &mut SplatGeo, mask: Option<&[bool]>) {
    let sh_coeffs = splats.sh_coeffs;
    let count = splats.len();
    if count == 0 {
        return;
    }

    let mode = params.get_int("delight_mode", 1).clamp(0, 2);
    let output_order = params.get_int("output_sh_order", 3).clamp(0, 3);
    let max_coeffs = sh_coeffs_for_order(output_order).min(sh_coeffs);
    let high_band_gain = params.get_float("high_band_gain", 0.25).clamp(0.0, 1.0);
    let eps_scale = params.get_float("eps", 1.0e-3).abs().max(1.0e-8);

    let mut next_sh0 = splats.sh0.clone();
    let mut next_rest = splats.sh_rest.clone();

    match mode {
        0 => {
            for_each_splat_mut(&mut next_sh0, &mut next_rest, sh_coeffs, |idx, _sh0, rest| {
                if !selected(mask, idx) {
                    return;
                }
                zero_sh_rest_slice(rest);
            });
        }
        1 => {
            let source_env = build_env_coeffs(params, splats, mask, EnvSource::Source);
            let neutral_env = build_env_coeffs(params, splats, mask, EnvSource::Neutral);
            let eps = eps_from_env(&source_env, eps_scale);
            let (ratio_min, ratio_max) = ratio_bounds(params);
            let ratios = build_ratio_table(&source_env, &neutral_env, eps, ratio_min, ratio_max);

            for_each_splat_mut(&mut next_sh0, &mut next_rest, sh_coeffs, |idx, sh0, rest| {
                if !selected(mask, idx) {
                    return;
                }
                if rest.is_empty() {
                    let ratio = ratios.first().copied().unwrap_or([1.0, 1.0, 1.0]);
                    sh0[0] *= ratio[0];
                    sh0[1] *= ratio[1];
                    sh0[2] *= ratio[2];
                    return;
                }
                apply_ratio_to_arrays(sh0, rest, &ratios);
                apply_high_band_gain_slice(rest, max_coeffs, high_band_gain);
                clamp_sh_order_slice(rest, max_coeffs);
            });
        }
        _ => {
            let source_env = build_env_coeffs(params, splats, mask, EnvSource::Source);
            let env_l2 = env_l2_from_coeffs(&source_env);
            let eps = eps_from_env(&env_l2, eps_scale);
            let albedo_max = params.get_float("albedo_max", 2.0).max(0.0);
            let normals = estimate_splat_normals(splats);

            for_each_splat_mut(&mut next_sh0, &mut next_rest, sh_coeffs, |idx, sh0, rest| {
                if !selected(mask, idx) {
                    return;
                }
                let n = normals.get(idx).copied().unwrap_or(Vec3::Y);
                let irradiance = irradiance_from_env_l2(n, &env_l2);
                let avg = splat_dc_color_from(sh0, sh_coeffs);
                let albedo = clamp_color(divide_color(avg, irradiance, eps), 0.0, albedo_max);
                set_splat_dc_color_into(sh0, sh_coeffs, albedo);
                if rest.is_empty() {
                    return;
                }
                apply_high_band_gain_slice(rest, max_coeffs, high_band_gain);
                clamp_sh_order_slice(rest, max_coeffs);
            });
        }
    }

    splats.sh0 = next_sh0;
    splats.sh_rest = next_rest;
}

fn sh_coeffs_for_order(order: i32) -> usize {
    match order.clamp(0, 3) {
        0 => 0,
        1 => 3,
        2 => 8,
        _ => 15,
    }
}

fn zero_sh_rest_slice(rest: &mut [[f32; 3]]) {
    for slot in rest {
        *slot = [0.0, 0.0, 0.0];
    }
}

fn clamp_sh_order_slice(rest: &mut [[f32; 3]], max_coeffs: usize) {
    if max_coeffs >= rest.len() {
        return;
    }
    for slot in &mut rest[max_coeffs..] {
        *slot = [0.0, 0.0, 0.0];
    }
}

fn apply_high_band_gain_slice(rest: &mut [[f32; 3]], max_coeffs: usize, gain: f32) {
    if rest.is_empty() || (gain - 1.0).abs() < 1.0e-6 {
        return;
    }
    let limit = max_coeffs.min(rest.len());
    for slot in &mut rest[..limit] {
        slot[0] *= gain;
        slot[1] *= gain;
        slot[2] *= gain;
    }
}

fn apply_ratio_to_arrays(sh0: &mut [f32; 3], rest: &mut [[f32; 3]], ratios: &[[f32; 3]]) {
    let ratio = ratios.first().copied().unwrap_or([1.0, 1.0, 1.0]);
    sh0[0] *= ratio[0];
    sh0[1] *= ratio[1];
    sh0[2] *= ratio[2];
    for (coeff, slot) in rest.iter_mut().enumerate() {
        let ratio = ratios.get(coeff + 1).copied().unwrap_or([1.0, 1.0, 1.0]);
        slot[0] *= ratio[0];
        slot[1] *= ratio[1];
        slot[2] *= ratio[2];
    }
}

fn for_each_splat_mut<F>(
    sh0: &mut [[f32; 3]],
    sh_rest: &mut [[f32; 3]],
    sh_coeffs: usize,
    f: F,
)
where
    F: Fn(usize, &mut [f32; 3], &mut [[f32; 3]]) + Sync + Send,
{
    if sh_coeffs == 0 {
        parallel::for_each_indexed_mut(sh0, |idx, sh0| f(idx, sh0, &mut []));
        return;
    }
    #[cfg(target_arch = "wasm32")]
    {
        for (idx, (sh0, rest)) in sh0
            .iter_mut()
            .zip(sh_rest.chunks_exact_mut(sh_coeffs))
            .enumerate()
        {
            f(idx, sh0, rest);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        sh0.par_iter_mut()
            .zip(sh_rest.par_chunks_exact_mut(sh_coeffs))
            .enumerate()
            .for_each(|(idx, (sh0, rest))| f(idx, sh0, rest));
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
        EnvSource::Neutral => params.get_int("neutral_env", 0),
    };
    match (source, mode) {
        (EnvSource::Source, 0) => average_env_coeffs(splats, mask),
        (EnvSource::Neutral, 0) => uniform_env_coeffs([1.0, 1.0, 1.0], splats.sh_coeffs),
        (EnvSource::Source, 2) => {
            let color = params.get_vec3("source_color", [1.0, 1.0, 1.0]);
            uniform_env_coeffs(color, splats.sh_coeffs)
        }
        (EnvSource::Neutral, 1) => {
            let color = params.get_vec3("neutral_color", [1.0, 1.0, 1.0]);
            uniform_env_coeffs(color, splats.sh_coeffs)
        }
        (EnvSource::Source, _) => uniform_env_coeffs([1.0, 1.0, 1.0], splats.sh_coeffs),
        (EnvSource::Neutral, _) => uniform_env_coeffs([1.0, 1.0, 1.0], splats.sh_coeffs),
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

    // idx mapping: 0:L00 1:L1-1 2:L10 3:L11 4:L2-2 5:L2-1 6:L20 7:L21 8:L22
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

fn splat_dc_color_from(sh0: &[f32; 3], sh_coeffs: usize) -> [f32; 3] {
    if sh_coeffs > 0 {
        let coeff = Vec3::from(*sh0);
        let color = coeff * SH_C0 + Vec3::splat(0.5);
        color.to_array()
    } else {
        *sh0
    }
}

fn set_splat_dc_color_into(sh0: &mut [f32; 3], sh_coeffs: usize, color: [f32; 3]) {
    if sh_coeffs > 0 {
        let coeff = (Vec3::from(color) - Vec3::splat(0.5)) / SH_C0;
        *sh0 = coeff.to_array();
    } else {
        *sh0 = color;
    }
}

fn divide_color(color: [f32; 3], irradiance: [f32; 3], eps: f32) -> [f32; 3] {
    let mut out = [0.0f32; 3];
    for channel in 0..3 {
        let denom = irradiance[channel] + eps;
        if denom.abs() < 1.0e-6 || !denom.is_finite() || !color[channel].is_finite() {
            out[channel] = 0.0;
        } else {
            out[channel] = color[channel] / denom;
        }
    }
    out
}

fn clamp_color(color: [f32; 3], min: f32, max: f32) -> [f32; 3] {
    [
        color[0].clamp(min, max),
        color[1].clamp(min, max),
        color[2].clamp(min, max),
    ]
}

enum EnvSource {
    Source,
    Neutral,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::graph::{NodeParams, ParamValue};
    use crate::splat::SplatGeo;

    use super::{apply_to_splats, SH_C0};

    #[test]
    fn band0_only_clears_sh_rest() {
        let mut splats = SplatGeo::with_len_and_sh(1, 3);
        splats.sh0[0] = [1.0, 1.0, 1.0];
        splats.sh_rest[0] = [0.1, 0.2, 0.3];
        splats.sh_rest[1] = [0.4, 0.5, 0.6];
        splats.sh_rest[2] = [0.7, 0.8, 0.9];
        let params = NodeParams {
            values: BTreeMap::from([("delight_mode".to_string(), ParamValue::Int(0))]),
        };
        let out = apply_to_splats(&params, &splats);
        assert_eq!(out.sh_rest[0], [0.0, 0.0, 0.0]);
        assert_eq!(out.sh_rest[1], [0.0, 0.0, 0.0]);
        assert_eq!(out.sh_rest[2], [0.0, 0.0, 0.0]);
    }

    #[test]
    fn irradiance_divide_updates_dc() {
        let mut splats = SplatGeo::with_len_and_sh(1, 3);
        splats.sh0[0] = [0.5 / SH_C0, 0.5 / SH_C0, 0.5 / SH_C0];
        let params = NodeParams {
            values: BTreeMap::from([
                ("delight_mode".to_string(), ParamValue::Int(2)),
                ("source_env".to_string(), ParamValue::Int(1)),
            ]),
        };
        let out = apply_to_splats(&params, &splats);
        assert!(out.sh0[0][0].is_finite());
    }
}

