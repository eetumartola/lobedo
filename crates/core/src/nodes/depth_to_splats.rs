use std::collections::BTreeMap;

use glam::{Mat3, Quat, Vec3};

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::image_data::ImageData;
use crate::nodes::{geometry_out, image_in};
use crate::param_spec::ParamSpec;
use crate::splat::SplatGeo;

pub const NAME: &str = "Depth to Splats";

const DEFAULT_FOV_DEG: f32 = 60.0;
const DEFAULT_DEPTH_SCALE: f32 = 1.0;
const DEFAULT_SCALE_K: f32 = 1.0;
const DEFAULT_SCALE_TAU: f32 = 0.2;
const DEFAULT_ALPHA: f32 = 0.1;
const LOG_SCALE_MIN: f32 = -10.0;
const LOG_SCALE_MAX: f32 = 10.0;
const DEPTH_EDGE_REL_THRESHOLD: f32 = 0.5;
const MAX_TANGENT_ANISOTROPY: f32 = 4.0;
const MIN_VIEW_COS: f32 = 0.35;

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "ML".to_string(),
        inputs: vec![
            image_in("color"),
            image_in("depth"),
            image_in("segmentation"),
        ],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            ("fov_deg".to_string(), ParamValue::Float(DEFAULT_FOV_DEG)),
            (
                "depth_scale".to_string(),
                ParamValue::Float(DEFAULT_DEPTH_SCALE),
            ),
            ("scale_k".to_string(), ParamValue::Float(DEFAULT_SCALE_K)),
            (
                "scale_tau".to_string(),
                ParamValue::Float(DEFAULT_SCALE_TAU),
            ),
            ("opacity".to_string(), ParamValue::Float(DEFAULT_ALPHA)),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::float_slider("fov_deg", "FOV (deg)", 10.0, 160.0)
            .with_help("Horizontal field of view used for unprojection."),
        ParamSpec::float_slider("depth_scale", "Depth Scale", 0.01, 200.0)
            .with_help("Scale factor applied to depth values before unprojection."),
        ParamSpec::float_slider("scale_k", "XY Scale", 0.1, 4.0)
            .with_help("XY footprint multiplier. 1.0 matches local pixel spacing."),
        ParamSpec::float_slider("scale_tau", "Thickness Ratio", 0.01, 0.5)
            .with_help("Z thickness relative to the XY footprint."),
        ParamSpec::float_slider("opacity", "Opacity", 0.01, 1.0)
            .with_help("Initial alpha (stored as logit)."),
    ]
}

pub fn compute(
    params: &NodeParams,
    color: &ImageData,
    depth: &ImageData,
    segmentation: Option<&ImageData>,
) -> Result<Geometry, String> {
    let (color_data, width, height) = color
        .rgb_data()
        .ok_or_else(|| "Depth to Splats requires color image input".to_string())?;
    let (depth_data, depth_w, depth_h) = depth
        .depth_data()
        .ok_or_else(|| "Depth to Splats requires depth image input".to_string())?;
    if width != depth_w || height != depth_h {
        return Err("Color and depth image sizes do not match".to_string());
    }

    let seg_values = if let Some(seg) = segmentation {
        let (seg_data, seg_w, seg_h) = seg
            .seg_data()
            .ok_or_else(|| "Segmentation input must be R32U".to_string())?;
        if seg_w != width || seg_h != height {
            return Err("Segmentation image size does not match color/depth".to_string());
        }
        Some(seg_data)
    } else {
        None
    };

    let fov = params
        .get_float("fov_deg", DEFAULT_FOV_DEG)
        .clamp(1.0, 179.0);
    let depth_scale = params
        .get_float("depth_scale", DEFAULT_DEPTH_SCALE)
        .max(0.0);
    let fov_rad = fov.to_radians();
    let fx = 0.5 * width as f32 / (0.5 * fov_rad).tan();
    let fy = fx.max(1.0e-6);
    let cx = (width as f32 - 1.0) * 0.5;
    let cy = (height as f32 - 1.0) * 0.5;

    let scale_k = params.get_float("scale_k", DEFAULT_SCALE_K).max(0.0);
    let scale_tau = params.get_float("scale_tau", DEFAULT_SCALE_TAU).max(0.0);
    let alpha = params
        .get_float("opacity", DEFAULT_ALPHA)
        .clamp(1.0e-4, 1.0 - 1.0e-4);
    let opacity_logit = (alpha / (1.0 - alpha)).ln();

    let mut positions = Vec::new();
    let mut rotations = Vec::new();
    let mut scales = Vec::new();
    let mut opacity = Vec::new();
    let mut sh0 = Vec::new();
    let mut segment_ids: Vec<i32> = Vec::new();

    let pixel_count = (width * height) as usize;
    positions.reserve(pixel_count);
    rotations.reserve(pixel_count);
    scales.reserve(pixel_count);
    opacity.reserve(pixel_count);
    sh0.reserve(pixel_count);
    segment_ids.reserve(pixel_count);

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let z = depth_data[idx] * depth_scale;
            if !z.is_finite() || z <= 0.0 {
                continue;
            }

            let position = unproject(x, y, z, fx, fy, cx, cy);
            let (quat, sx, sy) = splat_frame_from_depth(
                x,
                y,
                width,
                height,
                depth_data,
                depth_scale,
                fx,
                fy,
                cx,
                cy,
                scale_k,
            );
            let sz = (scale_tau * sx.min(sy)).max(1.0e-6);
            let log_scale = [
                sx.ln().clamp(LOG_SCALE_MIN, LOG_SCALE_MAX),
                sy.ln().clamp(LOG_SCALE_MIN, LOG_SCALE_MAX),
                sz.ln().clamp(LOG_SCALE_MIN, LOG_SCALE_MAX),
            ];

            let base = idx * 3;
            let color = [color_data[base], color_data[base + 1], color_data[base + 2]];

            positions.push(position.to_array());
            rotations.push([quat.w, quat.x, quat.y, quat.z]);
            scales.push(log_scale);
            opacity.push(opacity_logit);
            sh0.push(color);
            let seg_id = seg_values
                .and_then(|values| values.get(idx).copied())
                .unwrap_or(0);
            segment_ids.push(seg_id as i32);
        }
    }

    let mut splats = SplatGeo {
        positions,
        rotations,
        scales,
        opacity,
        sh0,
        sh_coeffs: 0,
        sh_rest: Vec::new(),
        attributes: Default::default(),
        groups: Default::default(),
    };

    if !segment_ids.is_empty() {
        let _ = splats.set_attribute(
            AttributeDomain::Point,
            "segment_id",
            AttributeStorage::Int(segment_ids),
        );
    }

    Ok(Geometry::with_splats(splats))
}

fn unproject(x: u32, y: u32, depth: f32, fx: f32, fy: f32, cx: f32, cy: f32) -> Vec3 {
    let u = x as f32;
    let v = y as f32;
    let px = (u - cx) * depth / fx;
    let py = (cy - v) * depth / fy;
    Vec3::new(px, py, depth)
}

#[allow(clippy::too_many_arguments)]
fn splat_frame_from_depth(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    depth: &[f32],
    depth_scale: f32,
    fx: f32,
    fy: f32,
    cx: f32,
    cy: f32,
    scale_k: f32,
) -> (Quat, f32, f32) {
    let eps = 1.0e-8;
    let Some(center) = sample_depth_point(x, y, width, height, depth, depth_scale, fx, fy, cx, cy)
    else {
        let sx = (scale_k / fx.max(1.0e-6)).max(1.0e-6);
        let sy = (scale_k / fy.max(1.0e-6)).max(1.0e-6);
        return (Quat::IDENTITY, sx, sy);
    };

    let left = if x > 0 {
        sample_depth_point(x - 1, y, width, height, depth, depth_scale, fx, fy, cx, cy)
    } else {
        None
    };
    let right = if x + 1 < width {
        sample_depth_point(x + 1, y, width, height, depth, depth_scale, fx, fy, cx, cy)
    } else {
        None
    };
    let up = if y > 0 {
        sample_depth_point(x, y - 1, width, height, depth, depth_scale, fx, fy, cx, cy)
    } else {
        None
    };
    let down = if y + 1 < height {
        sample_depth_point(x, y + 1, width, height, depth, depth_scale, fx, fy, cx, cy)
    } else {
        None
    };

    let left_ok = left.filter(|sample| same_surface_depth(center.z, sample.z));
    let right_ok = right.filter(|sample| same_surface_depth(center.z, sample.z));
    let up_ok = up.filter(|sample| same_surface_depth(center.z, sample.z));
    let down_ok = down.filter(|sample| same_surface_depth(center.z, sample.z));
    let has_left = left_ok.is_some();
    let has_right = right_ok.is_some();
    let has_up = up_ok.is_some();
    let has_down = down_ok.is_some();
    let has_x_neighbors = has_left || has_right;
    let has_y_neighbors = has_up || has_down;
    let has_both_x = has_left && has_right;
    let has_both_y = has_up && has_down;

    let fallback_x = Vec3::new(center.z / fx.max(1.0e-6), 0.0, 0.0);
    let fallback_y = Vec3::new(0.0, center.z / fy.max(1.0e-6), 0.0);

    let tangent_x = axis_tangent(center, left_ok, right_ok, fallback_x);
    let tangent_y = axis_tangent(center, up_ok, down_ok, fallback_y);

    let mut normal = tangent_x.cross(tangent_y);
    if normal.length_squared() <= eps {
        normal = Vec3::Z;
    } else {
        normal = normal.normalize();
    }

    // Use the local ray-plane Jacobian for robust pixel footprint tangents.
    // This preserves depth-based stretch without letting raw depth deltas explode scale.
    let ray = pixel_ray(x, y, fx, fy, cx, cy);
    let du = Vec3::new(1.0 / fx.max(1.0e-6), 0.0, 0.0);
    let dv = Vec3::new(0.0, -1.0 / fy.max(1.0e-6), 0.0);
    let denom_raw = normal.dot(ray);
    let denom_sign = if denom_raw >= 0.0 { 1.0 } else { -1.0 };
    let denom = if denom_raw.abs() < MIN_VIEW_COS {
        denom_sign * MIN_VIEW_COS
    } else {
        denom_raw
    };
    let tangent_u = center.z * (du - ray * (normal.dot(du) / denom));
    let tangent_v = center.z * (dv - ray * (normal.dot(dv) / denom));
    let tangent_u = if tangent_u.length_squared() > eps {
        tangent_u
    } else {
        fallback_x
    };
    let tangent_v = if tangent_v.length_squared() > eps {
        tangent_v
    } else {
        fallback_y
    };

    // Primary scale comes from actual neighboring point spacing.
    // Jacobian estimate is retained only as a fallback/guard.
    let measured_x = tangent_plane_length(tangent_x, normal).max(1.0e-6);
    let measured_y = tangent_plane_length(tangent_y, normal).max(1.0e-6);
    let model_x = tangent_u.length().max(1.0e-6);
    let model_y = tangent_v.length().max(1.0e-6);

    let step_x = if has_x_neighbors {
        let guard = if has_both_x { 1.25 } else { 1.1 };
        measured_x.min(model_x * guard)
    } else {
        model_x
    };
    let step_y = if has_y_neighbors {
        let guard = if has_both_y { 1.25 } else { 1.1 };
        measured_y.min(model_y * guard)
    } else {
        model_y
    };

    let base_sx = (scale_k * step_x).max(1.0e-6);
    let base_sy = (scale_k * step_y).max(1.0e-6);
    let min_axis = base_sx.min(base_sy).max(1.0e-6);
    let max_axis = min_axis * MAX_TANGENT_ANISOTROPY;
    let sx = base_sx.min(max_axis);
    let sy = base_sy.min(max_axis);
    let quat = quat_from_frame(normal, tangent_u, tangent_v);
    (quat, sx, sy)
}

fn pixel_ray(x: u32, y: u32, fx: f32, fy: f32, cx: f32, cy: f32) -> Vec3 {
    Vec3::new(
        (x as f32 - cx) / fx.max(1.0e-6),
        (cy - y as f32) / fy.max(1.0e-6),
        1.0,
    )
}

fn quat_from_frame(normal: Vec3, tangent_u: Vec3, tangent_v: Vec3) -> Quat {
    let eps = 1.0e-8;
    let z = if normal.length_squared() > eps {
        normal.normalize()
    } else {
        Vec3::Z
    };

    let mut x = tangent_u - z * z.dot(tangent_u);
    if x.length_squared() <= eps {
        x = tangent_v - z * z.dot(tangent_v);
    }
    if x.length_squared() <= eps {
        x = if z.z.abs() < 0.9 {
            Vec3::Z.cross(z)
        } else {
            Vec3::Y.cross(z)
        };
    }
    if x.length_squared() <= eps {
        x = Vec3::X;
    } else {
        x = x.normalize();
    }

    let mut y = z.cross(x);
    if y.length_squared() <= eps {
        y = Vec3::Y;
    } else {
        y = y.normalize();
    }

    if tangent_v.length_squared() > eps && y.dot(tangent_v) < 0.0 {
        x = -x;
        y = -y;
    }

    let rot = Mat3::from_cols(x, y, z);
    let quat = Quat::from_mat3(&rot);
    if quat.x.is_finite() && quat.y.is_finite() && quat.z.is_finite() && quat.w.is_finite() {
        quat
    } else {
        Quat::IDENTITY
    }
}

#[derive(Clone, Copy)]
struct DepthPointSample {
    pos: Vec3,
    z: f32,
}

#[allow(clippy::too_many_arguments)]
fn sample_depth_point(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    depth: &[f32],
    depth_scale: f32,
    fx: f32,
    fy: f32,
    cx: f32,
    cy: f32,
) -> Option<DepthPointSample> {
    if x >= width || y >= height {
        return None;
    }
    let idx = (y * width + x) as usize;
    let z = depth.get(idx).copied()? * depth_scale;
    if !z.is_finite() || z <= 0.0 {
        return None;
    }
    Some(DepthPointSample {
        pos: unproject(x, y, z, fx, fy, cx, cy),
        z,
    })
}

fn axis_tangent(
    center: DepthPointSample,
    neg: Option<DepthPointSample>,
    pos: Option<DepthPointSample>,
    fallback: Vec3,
) -> Vec3 {
    let eps = 1.0e-8;
    if let (Some(neg), Some(pos)) = (neg, pos) {
        let neg_step = center.pos - neg.pos;
        let pos_step = pos.pos - center.pos;
        let neg_len = neg_step.length();
        let pos_len = pos_step.length();
        let mut central = (pos.pos - neg.pos) * 0.5;
        let central_len = central.length();
        if central_len > eps {
            let safe_limit = neg_len.min(pos_len) * 1.5;
            if safe_limit > eps && central_len > safe_limit {
                central *= safe_limit / central_len;
            }
        }
        if central.length_squared() > eps {
            return central;
        }
        if pos_len >= neg_len && pos_len > eps {
            return pos_step;
        }
        if neg_len > eps {
            return neg_step;
        }
    } else if let Some(pos) = pos {
        let step = pos.pos - center.pos;
        if step.length_squared() > eps {
            return step;
        }
    } else if let Some(neg) = neg {
        let step = center.pos - neg.pos;
        if step.length_squared() > eps {
            return step;
        }
    }

    if fallback.length_squared() > eps {
        fallback
    } else {
        Vec3::new(1.0e-6, 0.0, 0.0)
    }
}

fn tangent_plane_length(v: Vec3, normal: Vec3) -> f32 {
    let tangent = v - normal * normal.dot(v);
    tangent.length()
}

fn same_surface_depth(center_z: f32, neighbor_z: f32) -> bool {
    if !center_z.is_finite() || !neighbor_z.is_finite() || center_z <= 0.0 || neighbor_z <= 0.0 {
        return false;
    }
    ((neighbor_z - center_z).abs() / center_z.max(1.0e-6)) <= DEPTH_EDGE_REL_THRESHOLD
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_to_splats_writes_segment_id() {
        let color = ImageData::from_rgb(2, 1, vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0]).unwrap();
        let depth = ImageData::from_depth(2, 1, vec![1.0, 1.0]).unwrap();
        let seg = ImageData::from_seg(2, 1, vec![3, 7]).unwrap();
        let params = default_params();
        let geometry = compute(&params, &color, &depth, Some(&seg)).unwrap();
        let splats = geometry.merged_splats().unwrap();
        let attr = splats
            .attributes
            .get(AttributeDomain::Point, "segment_id")
            .unwrap();
        if let AttributeStorage::Int(values) = attr {
            assert_eq!(values.len(), splats.len());
            assert_eq!(values[0], 3);
            assert_eq!(values[1], 7);
        } else {
            panic!("segment_id attribute type mismatch");
        }
    }

    #[test]
    fn depth_to_splats_scales_are_anisotropic_on_slanted_depth() {
        let color = ImageData::from_rgb(
            3,
            3,
            vec![
                1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, //
                1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, //
                1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, //
            ],
        )
        .unwrap();
        let depth = ImageData::from_depth(
            3,
            3,
            vec![
                1.0, 2.0, 3.0, //
                1.0, 2.0, 3.0, //
                1.0, 2.0, 3.0, //
            ],
        )
        .unwrap();
        let params = default_params();
        let geometry = compute(&params, &color, &depth, None).unwrap();
        let splats = geometry.merged_splats().unwrap();

        // Center pixel is index 4 in row-major order.
        let scale = splats.scales[4];
        let sx = scale[0].exp();
        let sy = scale[1].exp();
        assert!(sx > sy * 1.1, "expected anisotropy, got sx={sx}, sy={sy}");
    }

    #[test]
    fn depth_to_splats_discontinuity_guard_limits_overshoot() {
        let color = ImageData::from_rgb(
            3,
            1,
            vec![
                1.0, 1.0, 1.0, //
                1.0, 1.0, 1.0, //
                1.0, 1.0, 1.0, //
            ],
        )
        .unwrap();
        let depth = ImageData::from_depth(3, 1, vec![1.0, 1.0, 10.0]).unwrap();
        let mut params = default_params();
        params
            .values
            .insert("fov_deg".to_string(), ParamValue::Float(90.0));
        params
            .values
            .insert("scale_k".to_string(), ParamValue::Float(1.0));
        params
            .values
            .insert("scale_tau".to_string(), ParamValue::Float(0.1));
        let geometry = compute(&params, &color, &depth, None).unwrap();
        let splats = geometry.merged_splats().unwrap();

        // Middle pixel is index 1; right pixel is a large depth jump that should be ignored.
        let sx = splats.scales[1][0].exp();
        let expected_one_px = 1.0 / (1.5f32).max(1.0e-6); // z/fx with z=1, fx=1.5 for width=3 at 90deg
        assert!(
            sx < expected_one_px * 3.0,
            "discontinuity guard failed, sx={sx}, expected<{:.6}",
            expected_one_px * 3.0
        );
    }
}
