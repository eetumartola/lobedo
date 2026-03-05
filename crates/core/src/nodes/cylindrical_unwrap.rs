use std::collections::BTreeMap;

use glam::{Mat3, Quat, Vec3};

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{geometry_in, geometry_out, require_mesh_input};
use crate::param_spec::ParamSpec;
use crate::splat::SplatGeo;

pub const NAME: &str = "Cylindrical Unwrap";
const INVERSE_KEY: &str = "inverse";
const FULL_PROCESSING_KEY: &str = "full_processing";
const SEAM_ANGLE_DEG_KEY: &str = "seam_angle_deg";
const AXIS_MULT_KEY: &str = "axis_mult";
const COVERAGE_BOOST_MAX_KEY: &str = "coverage_boost_max";
const COVERAGE_BOOST_MUL_KEY: &str = "coverage_boost_mul";
const DEFAULT_AXIS_MULT: [f32; 3] = [30.0, 1.0, 1.0];
const DEFAULT_COVERAGE_BOOST_MAX: f32 = 3.0;
const DEFAULT_COVERAGE_BOOST_MUL: f32 = 1.0;
const SPLAT_LOG_SCALE_MIN: f32 = -10.0;
const SPLAT_LOG_SCALE_MAX: f32 = 10.0;

#[derive(Clone, Copy)]
struct CoverageSettings {
    inverse: bool,
    seam_angle: f32,
    axis_mult: Vec3,
    max_boost: f32,
    boost_mul: f32,
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![geometry_in("in")],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            (INVERSE_KEY.to_string(), ParamValue::Bool(true)),
            (FULL_PROCESSING_KEY.to_string(), ParamValue::Bool(true)),
            (SEAM_ANGLE_DEG_KEY.to_string(), ParamValue::Float(0.0)),
            (
                AXIS_MULT_KEY.to_string(),
                ParamValue::Vec3(DEFAULT_AXIS_MULT),
            ),
            (
                COVERAGE_BOOST_MAX_KEY.to_string(),
                ParamValue::Float(DEFAULT_COVERAGE_BOOST_MAX),
            ),
            (
                COVERAGE_BOOST_MUL_KEY.to_string(),
                ParamValue::Float(DEFAULT_COVERAGE_BOOST_MUL),
            ),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::bool(INVERSE_KEY, "Inverse").with_help(
            "Convert Cartesian (x, y, z) to cylindrical (theta, y, radius) instead of unwrapping.",
        ),
        ParamSpec::bool(FULL_PROCESSING_KEY, "Full Processing")
            .with_help("Transform splat scale/rotation and point normals using the Jacobian."),
        ParamSpec::float_slider(SEAM_ANGLE_DEG_KEY, "Seam Angle (deg)", -180.0, 180.0)
            .with_help("Angular seam offset around world Y axis."),
        ParamSpec::vec3(AXIS_MULT_KEY, "Axis Mult")
            .with_help("Per-axis multiplier in cylindrical space (x=theta, y=height, z=radius)."),
        ParamSpec::float_slider(COVERAGE_BOOST_MAX_KEY, "Coverage Boost Max", 1.0, 10.0)
            .with_help(
                "Upper clamp for nonlinear full-processing scale expansion (1 disables extra boost).",
            ),
        ParamSpec::float_slider(COVERAGE_BOOST_MUL_KEY, "Coverage Boost Mul", 0.0, 4.0)
            .with_help(
                "User multiplier for nonlinear expansion (0 disables, 1 keeps auto, >1 exaggerates) before max clamp.",
            ),
    ]
}

pub fn compute(_params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    require_mesh_input(inputs, 0, "Cylindrical Unwrap requires a mesh input")
}

pub fn apply_to_splats(params: &NodeParams, splats: &SplatGeo) -> Result<SplatGeo, String> {
    if splats.positions.is_empty() {
        return Ok(splats.clone());
    }

    let inverse = params.get_bool(INVERSE_KEY, true);
    let full_processing = params.get_bool(FULL_PROCESSING_KEY, true);
    let seam_angle = params.get_float(SEAM_ANGLE_DEG_KEY, 0.0).to_radians();
    let axis_mult = sanitize_axis_mult(params.get_vec3(AXIS_MULT_KEY, DEFAULT_AXIS_MULT));
    let coverage_boost_max = sanitize_coverage_boost_max(
        params.get_float(COVERAGE_BOOST_MAX_KEY, DEFAULT_COVERAGE_BOOST_MAX),
    );
    let coverage_boost_mul = sanitize_coverage_boost_mul(
        params.get_float(COVERAGE_BOOST_MUL_KEY, DEFAULT_COVERAGE_BOOST_MUL),
    );
    let coverage_settings = CoverageSettings {
        inverse,
        seam_angle,
        axis_mult,
        max_boost: coverage_boost_max,
        boost_mul: coverage_boost_mul,
    };

    let mut output = splats.clone();
    let mut normals_storage = if full_processing {
        output.attributes.remove(AttributeDomain::Point, "N")
    } else {
        None
    };
    for idx in 0..output.positions.len() {
        let source = Vec3::from(output.positions[idx]);
        if !source.is_finite() {
            continue;
        }
        let (target, linear) = if inverse {
            cartesian_to_cylindrical(source, seam_angle, axis_mult)
        } else {
            cylindrical_to_cartesian(source, seam_angle, axis_mult)
        };
        if !target.is_finite() {
            continue;
        }
        output.positions[idx] = target.to_array();
        if full_processing {
            if let Some(linear) = linear {
                let coverage_boost = nonlinear_coverage_boost(
                    source,
                    output
                        .rotations
                        .get(idx)
                        .copied()
                        .unwrap_or([1.0, 0.0, 0.0, 0.0]),
                    output.scales.get(idx).copied().unwrap_or([0.0, 0.0, 0.0]),
                    linear,
                    coverage_settings,
                );
                output.apply_linear_deform(idx, linear);
                apply_uniform_scale_boost(&mut output, idx, coverage_boost);
                if let Some(AttributeStorage::Vec3(normals)) = normals_storage.as_mut() {
                    if let Some(slot) = normals.get_mut(idx) {
                        *slot = transform_normal(*slot, linear);
                    }
                }
            }
        }
    }
    if let Some(storage) = normals_storage {
        output
            .attributes
            .map_mut(AttributeDomain::Point)
            .insert("N".to_string(), storage);
    }

    Ok(output)
}

fn cylindrical_to_cartesian(
    source: Vec3,
    seam_angle: f32,
    axis_mult: Vec3,
) -> (Vec3, Option<Mat3>) {
    let axis_inv = Vec3::new(1.0 / axis_mult.x, 1.0 / axis_mult.y, 1.0 / axis_mult.z);
    let source_cyl = source * axis_inv;
    let theta = source_cyl.x + seam_angle;
    let y = source_cyl.y;
    let radius = source_cyl.z;
    let (sin_t, cos_t) = theta.sin_cos();
    let target = Vec3::new(radius * cos_t, y, radius * sin_t);
    let jacobian = Mat3::from_cols(
        Vec3::new(-radius * sin_t, 0.0, radius * cos_t),
        Vec3::Y,
        Vec3::new(cos_t, 0.0, sin_t),
    );
    let linear = jacobian * Mat3::from_diagonal(axis_inv);
    let valid_linear = matrix_is_finite(linear).then_some(linear);
    (target, valid_linear)
}

fn cartesian_to_cylindrical(
    source: Vec3,
    seam_angle: f32,
    axis_mult: Vec3,
) -> (Vec3, Option<Mat3>) {
    let x = source.x;
    let y = source.y;
    let z = source.z;
    let radius_sq = x * x + z * z;
    let radius = radius_sq.sqrt();
    let theta = z.atan2(x) - seam_angle;
    let target = Vec3::new(theta, y, radius) * axis_mult;
    if !radius_sq.is_finite() || radius_sq <= 1.0e-8 || !radius.is_finite() {
        return (target, None);
    }
    let inv_radius_sq = 1.0 / radius_sq;
    let inv_radius = 1.0 / radius;
    let jacobian = Mat3::from_cols(
        Vec3::new(-z * inv_radius_sq, 0.0, x * inv_radius),
        Vec3::Y,
        Vec3::new(x * inv_radius_sq, 0.0, z * inv_radius),
    );
    let linear = Mat3::from_diagonal(axis_mult) * jacobian;
    let valid_linear = matrix_is_finite(linear).then_some(linear);
    (target, valid_linear)
}

fn sanitize_axis_mult(mult: [f32; 3]) -> Vec3 {
    let mut v = Vec3::from(mult);
    if !v.x.is_finite() || v.x.abs() < 1.0e-6 {
        v.x = 1.0;
    }
    if !v.y.is_finite() || v.y.abs() < 1.0e-6 {
        v.y = 1.0;
    }
    if !v.z.is_finite() || v.z.abs() < 1.0e-6 {
        v.z = 1.0;
    }
    v
}

fn sanitize_coverage_boost_max(value: f32) -> f32 {
    if !value.is_finite() {
        return DEFAULT_COVERAGE_BOOST_MAX;
    }
    value.max(1.0)
}

fn sanitize_coverage_boost_mul(value: f32) -> f32 {
    if !value.is_finite() {
        return DEFAULT_COVERAGE_BOOST_MUL;
    }
    value.max(0.0)
}

fn nonlinear_coverage_boost(
    source: Vec3,
    rotation: [f32; 4],
    log_scale: [f32; 3],
    linear: Mat3,
    settings: CoverageSettings,
) -> f32 {
    if !matrix_is_finite(linear) || !source.is_finite() {
        return 1.0;
    }
    let mut log = Vec3::from(log_scale);
    log = Vec3::new(
        log.x.clamp(SPLAT_LOG_SCALE_MIN, SPLAT_LOG_SCALE_MAX),
        log.y.clamp(SPLAT_LOG_SCALE_MIN, SPLAT_LOG_SCALE_MAX),
        log.z.clamp(SPLAT_LOG_SCALE_MIN, SPLAT_LOG_SCALE_MAX),
    );
    let min_sigma = SPLAT_LOG_SCALE_MIN.exp();
    let sigma = Vec3::new(log.x.exp(), log.y.exp(), log.z.exp()).max(Vec3::splat(min_sigma));

    let mut quat = Quat::from_xyzw(rotation[1], rotation[2], rotation[3], rotation[0]);
    quat = if quat.length_squared() > 0.0 {
        quat.normalize()
    } else {
        Quat::IDENTITY
    };
    let rot = Mat3::from_quat(quat);
    let axes = [
        rot.x_axis * sigma.x,
        rot.y_axis * sigma.y,
        rot.z_axis * sigma.z,
    ];
    let center = map_position(
        source,
        settings.inverse,
        settings.seam_angle,
        settings.axis_mult,
    );
    if !center.is_finite() {
        return 1.0;
    }

    let cov_local = Mat3::from_diagonal(sigma * sigma);
    let cov_source = rot * cov_local * rot.transpose();
    let cov_linear = linear * cov_source * linear.transpose();
    let trace_linear = mat_trace(cov_linear);
    if !trace_linear.is_finite() || trace_linear <= 1.0e-12 {
        return 1.0;
    }

    let mut cov_nonlinear = Mat3::ZERO;
    let mut axis_ratio_max = 1.0f32;
    let mut axis_count = 0.0f32;
    for axis in axes {
        if !axis.is_finite() {
            continue;
        }
        let predicted = (linear * axis).length();
        let mut pair_cov = Mat3::ZERO;
        let mut pair_count = 0.0f32;
        let mut pair_ratio_max = 1.0f32;
        for sign in [-1.0f32, 1.0f32] {
            let sample = map_position(
                source + axis * sign,
                settings.inverse,
                settings.seam_angle,
                settings.axis_mult,
            );
            if !sample.is_finite() {
                continue;
            }
            let delta = sample - center;
            pair_cov += outer(delta, delta);
            if predicted > 1.0e-8 {
                let ratio = delta.length() / predicted;
                if ratio.is_finite() {
                    pair_ratio_max = pair_ratio_max.max(ratio);
                }
            }
            pair_count += 1.0;
        }
        if pair_count <= 0.0 {
            continue;
        }
        cov_nonlinear += pair_cov / pair_count;
        axis_ratio_max = axis_ratio_max.max(pair_ratio_max);
        axis_count += 1.0;
    }
    if axis_count <= 0.0 {
        return 1.0;
    }
    let trace_nonlinear = mat_trace(cov_nonlinear);
    let raw_cov_boost = if trace_nonlinear.is_finite() && trace_nonlinear > 1.0e-12 {
        (trace_nonlinear / trace_linear).sqrt().max(1.0)
    } else {
        1.0
    };
    let raw_boost = raw_cov_boost.max(axis_ratio_max).max(1.0);
    let boosted = 1.0 + (raw_boost - 1.0) * settings.boost_mul.max(0.0);
    boosted.clamp(1.0, settings.max_boost.max(1.0))
}

fn apply_uniform_scale_boost(splats: &mut SplatGeo, idx: usize, boost: f32) {
    if boost <= 1.0 || !boost.is_finite() {
        return;
    }
    let Some(scale) = splats.scales.get_mut(idx) else {
        return;
    };
    let min_sigma = SPLAT_LOG_SCALE_MIN.exp();
    let max_sigma = SPLAT_LOG_SCALE_MAX.exp();
    for axis in scale.iter_mut() {
        let log = axis.clamp(SPLAT_LOG_SCALE_MIN, SPLAT_LOG_SCALE_MAX);
        let sigma = (log.exp() * boost).clamp(min_sigma, max_sigma);
        *axis = sigma.ln();
    }
}

fn map_position(source: Vec3, inverse: bool, seam_angle: f32, axis_mult: Vec3) -> Vec3 {
    if inverse {
        cartesian_to_cylindrical(source, seam_angle, axis_mult).0
    } else {
        cylindrical_to_cartesian(source, seam_angle, axis_mult).0
    }
}

fn outer(a: Vec3, b: Vec3) -> Mat3 {
    Mat3::from_cols(a * b.x, a * b.y, a * b.z)
}

fn mat_trace(m: Mat3) -> f32 {
    m.x_axis.x + m.y_axis.y + m.z_axis.z
}

fn matrix_is_finite(m: Mat3) -> bool {
    m.to_cols_array().iter().all(|value| value.is_finite())
}

fn transform_normal(normal: [f32; 3], linear: Mat3) -> [f32; 3] {
    let det = linear.determinant();
    if !det.is_finite() || det.abs() < 1.0e-8 {
        return normal;
    }
    let normal_matrix = linear.inverse().transpose();
    let v = normal_matrix * Vec3::from(normal);
    let len = v.length();
    if len > 0.0 {
        (v / len).to_array()
    } else {
        normal
    }
}

#[cfg(test)]
mod tests {
    use glam::{Vec3, Vec4};
    use std::collections::BTreeMap;
    use std::f32::consts::PI;

    use crate::attributes::{AttributeDomain, AttributeStorage};
    use crate::graph::{NodeParams, ParamValue};
    use crate::splat::SplatGeo;

    use super::apply_to_splats;

    #[test]
    fn unwrap_maps_theta_height_radius_to_cartesian() {
        let mut splats = SplatGeo::with_len(1);
        splats.positions[0] = [PI * 0.5, 3.0, 2.0];
        splats.rotations[0] = [1.0, 0.0, 0.0, 0.0];
        splats.scales[0] = [0.0, 0.0, 0.0];
        let params = NodeParams {
            values: BTreeMap::from([
                (super::INVERSE_KEY.to_string(), ParamValue::Bool(false)),
                (
                    super::AXIS_MULT_KEY.to_string(),
                    ParamValue::Vec3([1.0, 1.0, 1.0]),
                ),
            ]),
        };

        let output = apply_to_splats(&params, &splats).expect("unwrap");
        let p = output.positions[0];
        assert!(p[0].abs() < 1.0e-4);
        assert!((p[1] - 3.0).abs() < 1.0e-4);
        assert!((p[2] - 2.0).abs() < 1.0e-4);
    }

    #[test]
    fn inverse_maps_cartesian_to_theta_height_radius() {
        let mut splats = SplatGeo::with_len(1);
        splats.positions[0] = [2.0, 2.5, 0.0];
        splats.rotations[0] = [1.0, 0.0, 0.0, 0.0];
        splats.scales[0] = [0.0, 0.0, 0.0];
        let params = NodeParams {
            values: BTreeMap::from([(super::INVERSE_KEY.to_string(), ParamValue::Bool(true))]),
        };

        let output = apply_to_splats(&params, &splats).expect("inverse");
        let p = output.positions[0];
        assert!(p[0].abs() < 1.0e-4);
        assert!((p[1] - 2.5).abs() < 1.0e-4);
        assert!((p[2] - 2.0).abs() < 1.0e-4);
    }

    #[test]
    fn seam_angle_rotates_forward_unwrap() {
        let mut splats = SplatGeo::with_len(1);
        splats.positions[0] = [0.0, 1.0, 2.0];
        let params = NodeParams {
            values: BTreeMap::from([
                (
                    super::SEAM_ANGLE_DEG_KEY.to_string(),
                    ParamValue::Float(90.0),
                ),
                (super::INVERSE_KEY.to_string(), ParamValue::Bool(false)),
                (
                    super::AXIS_MULT_KEY.to_string(),
                    ParamValue::Vec3([1.0, 1.0, 1.0]),
                ),
            ]),
        };

        let output = apply_to_splats(&params, &splats).expect("seam");
        let p = output.positions[0];
        assert!(p[0].abs() < 1.0e-4);
        assert!((p[1] - 1.0).abs() < 1.0e-4);
        assert!((p[2] - 2.0).abs() < 1.0e-4);
    }

    #[test]
    fn move_only_keeps_rotation_and_scale() {
        let mut splats = SplatGeo::with_len(1);
        splats.positions[0] = [PI * 0.25, 0.0, 3.0];
        splats.rotations[0] = [0.9238795, 0.0, 0.3826834, 0.0];
        splats.scales[0] = [0.5, -0.1, 0.2];
        let params = NodeParams {
            values: BTreeMap::from([
                (super::INVERSE_KEY.to_string(), ParamValue::Bool(false)),
                (
                    super::FULL_PROCESSING_KEY.to_string(),
                    ParamValue::Bool(false),
                ),
                (
                    super::AXIS_MULT_KEY.to_string(),
                    ParamValue::Vec3([1.0, 1.0, 1.0]),
                ),
            ]),
        };

        let output = apply_to_splats(&params, &splats).expect("move only");
        assert_eq!(output.rotations[0], splats.rotations[0]);
        assert_eq!(output.scales[0], splats.scales[0]);
    }

    #[test]
    fn unwrap_transforms_normals_and_keeps_values_finite() {
        let mut splats = SplatGeo::with_len(1);
        splats.positions[0] = [0.0, 0.0, 2.0];
        splats.rotations[0] = [1.0, 0.0, 0.0, 0.0];
        splats.scales[0] = [0.0, 0.0, 0.0];
        let _ = splats.set_attribute(
            AttributeDomain::Point,
            "N",
            AttributeStorage::Vec3(vec![[1.0, 0.0, 0.0]]),
        );
        let params = NodeParams {
            values: BTreeMap::from([
                (super::INVERSE_KEY.to_string(), ParamValue::Bool(false)),
                (
                    super::AXIS_MULT_KEY.to_string(),
                    ParamValue::Vec3([1.0, 1.0, 1.0]),
                ),
            ]),
        };

        let output = apply_to_splats(&params, &splats).expect("unwrap");
        assert!(output.scales[0].iter().all(|v| v.is_finite()));
        assert!(output.rotations[0].iter().all(|v| v.is_finite()));
        match output.attributes.get(AttributeDomain::Point, "N") {
            Some(AttributeStorage::Vec3(values)) => {
                let n = Vec3::from(values[0]);
                assert!(n.is_finite());
                assert!((n.length() - 1.0).abs() < 1.0e-4);
            }
            _ => panic!("missing transformed normals"),
        }
    }

    #[test]
    fn default_params_enable_inverse_and_axis_multiplier() {
        let mut splats = SplatGeo::with_len(1);
        splats.positions[0] = [1.0, 2.0, 0.0];
        splats.rotations[0] = [1.0, 0.0, 0.0, 0.0];
        splats.scales[0] = [0.0, 0.0, 0.0];

        let output = apply_to_splats(&NodeParams::default(), &splats).expect("defaults");
        let p = Vec3::from(output.positions[0]);
        assert!((p.x - 0.0).abs() < 1.0e-4);
        assert!((p.y - 2.0).abs() < 1.0e-4);
        assert!((p.z - 1.0).abs() < 1.0e-4);
    }

    #[test]
    fn axis_multiplier_x_applies_in_forward_mode() {
        let mut splats = SplatGeo::with_len(1);
        splats.positions[0] = [30.0 * PI * 0.5, 0.0, 2.0];
        splats.rotations[0] = [1.0, 0.0, 0.0, 0.0];
        splats.scales[0] = [0.0, 0.0, 0.0];
        let params = NodeParams {
            values: BTreeMap::from([(super::INVERSE_KEY.to_string(), ParamValue::Bool(false))]),
        };

        let output = apply_to_splats(&params, &splats).expect("axis mult");
        let p = Vec3::from(output.positions[0]);
        assert!(p.x.abs() < 1.0e-4);
        assert!((p.z - 2.0).abs() < 1.0e-4);
    }

    #[test]
    fn default_inverse_and_full_processing_affect_transform() {
        let mut splats = SplatGeo::with_len(1);
        splats.positions[0] = [2.0, 1.0, 0.0];
        splats.rotations[0] = [0.9238795, 0.0, 0.3826834, 0.0];
        splats.scales[0] = [0.5, -0.1, 0.2];
        let in_rotation = Vec4::from(splats.rotations[0]);
        let in_scale = Vec3::from(splats.scales[0]);

        let output = apply_to_splats(&NodeParams::default(), &splats).expect("defaults");
        let out_rotation = Vec4::from(output.rotations[0]);
        let out_scale = Vec3::from(output.scales[0]);
        assert!(out_rotation.is_finite());
        assert!(out_scale.is_finite());
        assert!(
            (out_rotation - in_rotation).length() > 1.0e-4
                || (out_scale - in_scale).length() > 1.0e-4
        );
    }

    #[test]
    fn nonlinear_coverage_boost_expands_vs_pure_linear_deform() {
        let mut source = SplatGeo::with_len(1);
        source.positions[0] = [0.5, 0.0, 0.0];
        source.rotations[0] = [1.0, 0.0, 0.0, 0.0];
        source.scales[0] = [0.0, 0.0, 0.0];

        let params = NodeParams::default();
        let mapped = apply_to_splats(&params, &source).expect("mapped");

        let mut linear_only = source.clone();
        let seam = 0.0f32;
        let axis_mult = Vec3::from(super::DEFAULT_AXIS_MULT);
        let (target, linear) =
            super::cartesian_to_cylindrical(Vec3::from(source.positions[0]), seam, axis_mult);
        linear_only.positions[0] = target.to_array();
        linear_only.apply_linear_deform(0, linear.expect("linear"));

        let mapped_sigma = Vec3::new(
            mapped.scales[0][0].exp(),
            mapped.scales[0][1].exp(),
            mapped.scales[0][2].exp(),
        );
        let linear_sigma = Vec3::new(
            linear_only.scales[0][0].exp(),
            linear_only.scales[0][1].exp(),
            linear_only.scales[0][2].exp(),
        );
        assert!(mapped_sigma.length_squared() >= linear_sigma.length_squared());
    }

    #[test]
    fn nonlinear_coverage_boost_detects_inverse_curvature() {
        let source = Vec3::new(0.15, 0.0, -0.038);
        let axis_mult = Vec3::from(super::DEFAULT_AXIS_MULT);
        let (_, linear) = super::cartesian_to_cylindrical(source, 0.0, axis_mult);
        let settings = super::CoverageSettings {
            inverse: true,
            seam_angle: 0.0,
            axis_mult,
            max_boost: 10.0,
            boost_mul: 1.0,
        };
        let boost = super::nonlinear_coverage_boost(
            source,
            [1.0, 0.0, 0.0, 0.0],
            [-1.6, -1.6, -1.6],
            linear.expect("linear"),
            settings,
        );
        assert!(boost > 1.0);
    }

    #[test]
    fn coverage_boost_max_can_disable_extra_expansion() {
        let mut source = SplatGeo::with_len(1);
        source.positions[0] = [0.15, 0.0, -0.038];
        source.rotations[0] = [1.0, 0.0, 0.0, 0.0];
        source.scales[0] = [-1.6, -1.6, -1.6];

        let boosted = apply_to_splats(&NodeParams::default(), &source).expect("boosted");
        let limited_params = NodeParams {
            values: BTreeMap::from([
                (
                    super::COVERAGE_BOOST_MAX_KEY.to_string(),
                    ParamValue::Float(1.0),
                ),
                (
                    super::AXIS_MULT_KEY.to_string(),
                    ParamValue::Vec3(super::DEFAULT_AXIS_MULT),
                ),
                (super::INVERSE_KEY.to_string(), ParamValue::Bool(true)),
                (
                    super::FULL_PROCESSING_KEY.to_string(),
                    ParamValue::Bool(true),
                ),
            ]),
        };
        let limited = apply_to_splats(&limited_params, &source).expect("limited");

        let boosted_sigma = Vec3::new(
            boosted.scales[0][0].exp(),
            boosted.scales[0][1].exp(),
            boosted.scales[0][2].exp(),
        );
        let limited_sigma = Vec3::new(
            limited.scales[0][0].exp(),
            limited.scales[0][1].exp(),
            limited.scales[0][2].exp(),
        );
        assert!(boosted_sigma.length_squared() > limited_sigma.length_squared());
    }

    #[test]
    fn coverage_boost_multiplier_changes_expansion_strength() {
        let source = Vec3::new(0.15, 0.0, -0.038);
        let axis_mult = Vec3::from(super::DEFAULT_AXIS_MULT);
        let (_, linear) = super::cartesian_to_cylindrical(source, 0.0, axis_mult);
        let linear = linear.expect("linear");
        let settings_low = super::CoverageSettings {
            inverse: true,
            seam_angle: 0.0,
            axis_mult,
            max_boost: 10.0,
            boost_mul: 0.5,
        };
        let settings_high = super::CoverageSettings {
            inverse: true,
            seam_angle: 0.0,
            axis_mult,
            max_boost: 10.0,
            boost_mul: 2.0,
        };

        let low = super::nonlinear_coverage_boost(
            source,
            [1.0, 0.0, 0.0, 0.0],
            [-1.6, -1.6, -1.6],
            linear,
            settings_low,
        );
        let high = super::nonlinear_coverage_boost(
            source,
            [1.0, 0.0, 0.0, 0.0],
            [-1.6, -1.6, -1.6],
            linear,
            settings_high,
        );
        assert!(high > low);
        assert!(low >= 1.0);
    }
}
