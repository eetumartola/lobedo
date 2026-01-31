use std::collections::BTreeMap;

use glam::{Quat, Vec3};

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::image_data::ImageData;
use crate::nodes::{geometry_out, image_in};
use crate::param_spec::ParamSpec;
use crate::splat::SplatGeo;

pub const NAME: &str = "Depth to Splats";

const DEFAULT_FOV_DEG: f32 = 60.0;
const DEFAULT_SCALE_K: f32 = 1.6;
const DEFAULT_SCALE_TAU: f32 = 0.1;
const DEFAULT_ALPHA: f32 = 0.1;
const LOG_SCALE_MIN: f32 = -10.0;
const LOG_SCALE_MAX: f32 = 10.0;

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
            ("scale_k".to_string(), ParamValue::Float(DEFAULT_SCALE_K)),
            ("scale_tau".to_string(), ParamValue::Float(DEFAULT_SCALE_TAU)),
            ("opacity".to_string(), ParamValue::Float(DEFAULT_ALPHA)),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::float_slider("fov_deg", "FOV (deg)", 10.0, 160.0)
            .with_help("Horizontal field of view used for unprojection."),
        ParamSpec::float_slider("scale_k", "Scale K", 0.1, 4.0)
            .with_help("Pixel footprint scale multiplier for splat size."),
        ParamSpec::float_slider("scale_tau", "Scale Tau", 0.01, 0.5)
            .with_help("Depth-axis scale ratio relative to XY size."),
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

    let fov = params.get_float("fov_deg", DEFAULT_FOV_DEG).clamp(1.0, 179.0);
    let fov_rad = fov.to_radians();
    let fx = 0.5 * width as f32 / (0.5 * fov_rad).tan();
    let fy = fx.max(1.0e-6);
    let cx = (width as f32 - 1.0) * 0.5;
    let cy = (height as f32 - 1.0) * 0.5;

    let scale_k = params.get_float("scale_k", DEFAULT_SCALE_K).max(0.0);
    let scale_tau = params.get_float("scale_tau", DEFAULT_SCALE_TAU).max(0.0);
    let alpha = params.get_float("opacity", DEFAULT_ALPHA).clamp(1.0e-4, 1.0 - 1.0e-4);
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
            let z = depth_data[idx];
            if !z.is_finite() || z <= 0.0 {
                continue;
            }

            let position = unproject(x, y, z, fx, fy, cx, cy);
            let normal = normal_from_depth(x, y, width, height, depth_data, fx, fy, cx, cy);
            let quat = Quat::from_rotation_arc(Vec3::Y, normal);

            let sx = (scale_k * z / fx).max(1.0e-6);
            let sy = (scale_k * z / fy).max(1.0e-6);
            let sz = (scale_tau * sx.min(sy)).max(1.0e-6);
            let log_scale = [
                sx.ln().clamp(LOG_SCALE_MIN, LOG_SCALE_MAX),
                sy.ln().clamp(LOG_SCALE_MIN, LOG_SCALE_MAX),
                sz.ln().clamp(LOG_SCALE_MIN, LOG_SCALE_MAX),
            ];

            let base = idx * 3;
            let color = [
                color_data[base],
                color_data[base + 1],
                color_data[base + 2],
            ];

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

fn unproject(
    x: u32,
    y: u32,
    depth: f32,
    fx: f32,
    fy: f32,
    cx: f32,
    cy: f32,
) -> Vec3 {
    let u = x as f32;
    let v = y as f32;
    let px = (u - cx) * depth / fx;
    let py = (cy - v) * depth / fy;
    Vec3::new(px, py, depth)
}

#[allow(clippy::too_many_arguments)]
fn normal_from_depth(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    depth: &[f32],
    fx: f32,
    fy: f32,
    cx: f32,
    cy: f32,
) -> Vec3 {
    let x0 = x.saturating_sub(1);
    let x1 = (x + 1).min(width - 1);
    let y0 = y.saturating_sub(1);
    let y1 = (y + 1).min(height - 1);

    let idx = |xx: u32, yy: u32| -> usize { (yy * width + xx) as usize };
    let z = depth[idx(x, y)];
    if !z.is_finite() || z <= 0.0 {
        return Vec3::Y;
    }
    let zx0 = depth[idx(x0, y)];
    let zx1 = depth[idx(x1, y)];
    let zy0 = depth[idx(x, y0)];
    let zy1 = depth[idx(x, y1)];

    let p = unproject(x, y, z, fx, fy, cx, cy);
    let px0 = if zx0.is_finite() && zx0 > 0.0 {
        unproject(x0, y, zx0, fx, fy, cx, cy)
    } else {
        p
    };
    let px1 = if zx1.is_finite() && zx1 > 0.0 {
        unproject(x1, y, zx1, fx, fy, cx, cy)
    } else {
        p
    };
    let py0 = if zy0.is_finite() && zy0 > 0.0 {
        unproject(x, y0, zy0, fx, fy, cx, cy)
    } else {
        p
    };
    let py1 = if zy1.is_finite() && zy1 > 0.0 {
        unproject(x, y1, zy1, fx, fy, cx, cy)
    } else {
        p
    };

    let dpx = px1 - px0;
    let dpy = py1 - py0;
    let mut normal = dpx.cross(dpy);
    if normal.length_squared() <= 1.0e-6 {
        normal = Vec3::Y;
    } else {
        normal = normal.normalize();
    }
    normal
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
}
