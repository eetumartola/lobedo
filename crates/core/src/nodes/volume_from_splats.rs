use std::collections::{BTreeMap, HashMap};

use glam::Vec3;

use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::nodes::{geometry_in, geometry_out};
use crate::param_spec::ParamSpec;
use crate::parallel;
use crate::volume::{try_alloc_f32, Volume, VolumeKind};

pub const NAME: &str = "Volume from Splats";

const DEFAULT_MAX_DIM: i32 = 64;
const DEFAULT_PADDING: f32 = 0.1;
const DEFAULT_RADIUS_SCALE: f32 = 1.0;
const DEFAULT_MIN_RADIUS: f32 = 0.01;
const DEFAULT_FILL_SHELL_VOXELS: f32 = 1.5;
const DEFAULT_FILL_NORMAL_BIAS: f32 = 0.0;
const DEFAULT_DENSITY_SCALE: f32 = 1.0;
const DEFAULT_SDF_BAND: f32 = 0.2;
const DEFAULT_REFINE_STEPS: i32 = 1;
const DEFAULT_SUPPORT_SIGMA: f32 = 1.0;
const DEFAULT_ELLIPSOID_BLEND: f32 = 1.0;
const DEFAULT_OUTLIER_RADIUS: f32 = 0.0;
const DEFAULT_OUTLIER_MIN_NEIGHBORS: i32 = 2;
const DEFAULT_OUTLIER_MIN_OPACITY: f32 = -9.21034;
const MAX_GRID_POINTS: u64 = 32_000_000;

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
            ("mode".to_string(), ParamValue::String("sdf".to_string())),
            ("fill_mode".to_string(), ParamValue::String("fill".to_string())),
            (
                "shape".to_string(),
                ParamValue::String("ellipsoid".to_string()),
            ),
            ("max_dim".to_string(), ParamValue::Int(DEFAULT_MAX_DIM)),
            ("padding".to_string(), ParamValue::Float(DEFAULT_PADDING)),
            (
                "radius_mode".to_string(),
                ParamValue::String("avg".to_string()),
            ),
            (
                "radius_scale".to_string(),
                ParamValue::Float(DEFAULT_RADIUS_SCALE),
            ),
            ("min_radius".to_string(), ParamValue::Float(DEFAULT_MIN_RADIUS)),
            (
                "fill_shell".to_string(),
                ParamValue::Float(DEFAULT_FILL_SHELL_VOXELS),
            ),
            (
                "fill_normal_bias".to_string(),
                ParamValue::Float(DEFAULT_FILL_NORMAL_BIAS),
            ),
            (
                "density_scale".to_string(),
                ParamValue::Float(DEFAULT_DENSITY_SCALE),
            ),
            ("sdf_band".to_string(), ParamValue::Float(DEFAULT_SDF_BAND)),
            (
                "refine_steps".to_string(),
                ParamValue::Int(DEFAULT_REFINE_STEPS),
            ),
            (
                "support_sigma".to_string(),
                ParamValue::Float(DEFAULT_SUPPORT_SIGMA),
            ),
            (
                "ellipsoid_blend".to_string(),
                ParamValue::Float(DEFAULT_ELLIPSOID_BLEND),
            ),
            (
                "outlier_filter".to_string(),
                ParamValue::Bool(false),
            ),
            (
                "outlier_radius".to_string(),
                ParamValue::Float(DEFAULT_OUTLIER_RADIUS),
            ),
            (
                "outlier_min_neighbors".to_string(),
                ParamValue::Int(DEFAULT_OUTLIER_MIN_NEIGHBORS),
            ),
            (
                "outlier_min_opacity".to_string(),
                ParamValue::Float(DEFAULT_OUTLIER_MIN_OPACITY),
            ),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::string_enum("mode", "Mode", vec![("density", "Density"), ("sdf", "SDF")])
            .with_help("Volume type to generate (density or SDF)."),
        ParamSpec::string_enum("fill_mode", "Fill", vec![("shell", "Shell"), ("fill", "Fill")])
            .with_help("Shell keeps a surface band; Fill tags the interior of closed splat shells."),
        ParamSpec::string_enum("shape", "Shape", vec![("ellipsoid", "Ellipsoid"), ("sphere", "Sphere")])
            .with_help("Distance shape used per splat."),
        ParamSpec::int_slider("max_dim", "Max Dim", 8, 512)
            .with_help("Largest voxel dimension (grid resolution)."),
        ParamSpec::float_slider("padding", "Padding", 0.0, 10.0)
            .with_help("Padding around the bounds."),
        ParamSpec::string_enum(
            "radius_mode",
            "Radius",
            vec![("avg", "Avg"), ("min", "Min"), ("max", "Max")],
        )
        .with_help("How to derive a sphere radius from splat scale axes."),
        ParamSpec::float_slider("radius_scale", "Radius Scale", 0.1, 5.0)
            .with_help("Multiplier applied to the derived splat radius."),
        ParamSpec::float_slider("min_radius", "Min Radius", 0.0, 5.0)
            .with_help("Clamp splat radius to at least this value."),
        ParamSpec::float_slider("fill_shell", "Fill Shell (vox)", 0.0, 10.0)
            .with_help("Shell band thickness in voxels (also used for fill detection)."),
        ParamSpec::float_slider("fill_normal_bias", "Fill Normal Bias", 0.0, 2.0)
            .with_help("Expand the fill shell where distance gradients are weak.")
            .visible_when_string("fill_mode", "fill"),
        ParamSpec::float_slider("density_scale", "Density Scale", 0.0, 10.0)
            .with_help("Density value inside the volume.")
            .visible_when_string("mode", "density"),
        ParamSpec::float_slider("sdf_band", "SDF Band", 0.0, 10.0)
            .with_help("SDF band width for rendering.")
            .visible_when_string("mode", "sdf"),
        ParamSpec::int_slider("refine_steps", "Refine Steps", 0, 4)
            .with_help("Extra normal-based refinement steps for ellipsoid distance.")
            .visible_when_string("shape", "ellipsoid"),
        ParamSpec::float_slider("support_sigma", "Support Sigma", 0.1, 6.0)
            .with_help("Scale splat radii to control support, like Splat to Mesh.")
            .visible_when_string_in("shape", &["ellipsoid", "sphere"]),
        ParamSpec::float_slider("ellipsoid_blend", "Ellipsoid Blend", 0.0, 1.0)
            .with_help("Blend between sphere (0) and ellipsoid (1) distances.")
            .visible_when_string("shape", "ellipsoid"),
        ParamSpec::bool("outlier_filter", "Outlier Filter")
            .with_help("Remove isolated splats before voxelization."),
        ParamSpec::float_slider("outlier_radius", "Outlier Radius", 0.0, 5.0)
            .with_help("Neighborhood radius used for outlier detection.")
            .visible_when_bool("outlier_filter", true),
        ParamSpec::int_slider("outlier_min_neighbors", "Min Neighbors", 1, 32)
            .with_help("Minimum neighbor count to keep a splat.")
            .visible_when_bool("outlier_filter", true),
        ParamSpec::float_slider("outlier_min_opacity", "Min Opacity", -10.0, 2.0)
            .with_help("Minimum log-opacity to keep during outlier filtering.")
            .visible_when_bool("outlier_filter", true),
    ]
    .into_iter()
    .map(|spec| {
        if spec.key == "radius_mode" {
            spec.visible_when_string("shape", "sphere")
        } else {
            spec
        }
    })
    .collect()
}

pub fn apply_to_geometry(
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };

    let mode = params.get_string("mode", "sdf").to_lowercase();
    let kind = if mode.contains("sdf") {
        VolumeKind::Sdf
    } else {
        VolumeKind::Density
    };
    let fill_mode = params.get_string("fill_mode", "shell").to_lowercase();
    let fill_enabled = fill_mode.contains("fill");
    let shape_mode = params.get_string("shape", "ellipsoid").to_lowercase();
    let max_dim = params.get_int("max_dim", DEFAULT_MAX_DIM).max(1) as u32;
    let padding = params.get_float("padding", DEFAULT_PADDING).max(0.0);
    let radius_scale = params
        .get_float("radius_scale", DEFAULT_RADIUS_SCALE)
        .max(0.0);
    let min_radius = params
        .get_float("min_radius", DEFAULT_MIN_RADIUS)
        .max(0.0);
    let density_scale = params
        .get_float("density_scale", DEFAULT_DENSITY_SCALE)
        .max(0.0);
    let radius_mode = params.get_string("radius_mode", "avg");
    let support_sigma = params
        .get_float("support_sigma", DEFAULT_SUPPORT_SIGMA)
        .max(0.01);
    let ellipsoid_blend = params
        .get_float("ellipsoid_blend", DEFAULT_ELLIPSOID_BLEND)
        .clamp(0.0, 1.0);
    let refine_steps = params
        .get_int("refine_steps", DEFAULT_REFINE_STEPS)
        .clamp(0, 8) as u32;
    let fill_normal_bias = params
        .get_float("fill_normal_bias", DEFAULT_FILL_NORMAL_BIAS)
        .clamp(0.0, 4.0);
    let outlier_filter = params.get_bool("outlier_filter", false);
    let outlier_radius = params
        .get_float("outlier_radius", DEFAULT_OUTLIER_RADIUS)
        .max(0.0);
    let outlier_min_neighbors = params
        .get_int("outlier_min_neighbors", DEFAULT_OUTLIER_MIN_NEIGHBORS)
        .max(1) as usize;
    let outlier_min_opacity = params
        .get_float("outlier_min_opacity", DEFAULT_OUTLIER_MIN_OPACITY);

    let gather_settings = SplatGatherSettings {
        radius_mode,
        shape_mode: &shape_mode,
        radius_scale,
        min_radius,
        support_sigma,
        ellipsoid_blend,
        outlier_filter,
        outlier_radius,
        outlier_min_neighbors,
        outlier_min_opacity,
    };
    let gathered = gather_splats(input, &gather_settings)?;
    let bounds_min = gathered.min;
    let bounds_max = gathered.max;
    let splats = gathered.splats;

    let mut min = bounds_min - Vec3::splat(padding);
    let mut max = bounds_max + Vec3::splat(padding);
    if (max - min).length_squared() < 1.0e-8 {
        max += Vec3::splat(1.0e-3);
        min -= Vec3::splat(1.0e-3);
    }

    let size = (max - min).max(Vec3::splat(1.0e-6));
    let max_axis = size.x.max(size.y.max(size.z)).max(1.0e-6);
    let voxel_size = (max_axis / max_dim as f32).max(1.0e-6);
    let dims = dims_from_size(size, voxel_size);
    let sdf_band = params
        .get_float("sdf_band", DEFAULT_SDF_BAND)
        .max(voxel_size * 2.0)
        .max(1.0e-6);
    let shell_band = params
        .get_float("fill_shell", DEFAULT_FILL_SHELL_VOXELS)
        .max(0.0)
        * voxel_size;

    let total = dims[0] as u64 * dims[1] as u64 * dims[2] as u64;
    if total == 0 || total > MAX_GRID_POINTS {
        return Err(format!(
            "Volume grid too large ({} voxels, max {})",
            total, MAX_GRID_POINTS
        ));
    }

    let mut unsigned = try_alloc_f32(total as usize, "Volume from Splats")?;
    let dim_x = dims[0] as usize;
    let dim_y = dims[1] as usize;
    let dim_z = dims[2] as usize;
    let stride_xy = dim_x.saturating_mul(dim_y).max(1);
    parallel::for_each_indexed_mut(&mut unsigned, |idx, slot| {
        let z = idx / stride_xy;
        let rem = idx - z * stride_xy;
        let y = rem / dim_x;
        let x = rem - y * dim_x;
        if z >= dim_z {
            return;
        }
        let zf = min.z + z as f32 * voxel_size;
        let yf = min.y + y as f32 * voxel_size;
        let xf = min.x + x as f32 * voxel_size;
        let pos = Vec3::new(xf, yf, zf);
        let mut dist = f32::INFINITY;
        for splat in &splats {
            let bound_dist = pos.distance(splat.center) - splat.bound_radius;
            if bound_dist > dist {
                continue;
            }
            let d = if splat.is_ellipsoid {
                let p_local = splat.rotation.transpose() * (pos - splat.center);
                let ellipsoid = ellipsoid_signed_distance(
                    p_local,
                    splat.inv_radii,
                    splat.inv_radii_sq,
                    splat.radii,
                    refine_steps,
                )
                .abs();
                if splat.ellipsoid_blend >= 0.999 {
                    ellipsoid
                } else {
                    let sphere = (pos.distance(splat.center) - splat.radius).abs();
                    ellipsoid * splat.ellipsoid_blend + sphere * (1.0 - splat.ellipsoid_blend)
                }
            } else {
                let d = pos.distance(splat.center) - splat.radius;
                d.abs()
            };
            if d < dist {
                dist = d;
            }
        }
        if !dist.is_finite() {
            dist = 0.0;
        }
        *slot = dist;
    });

    let grad = if fill_enabled && fill_normal_bias > 0.0 {
        Some(distance_gradient_magnitude(&unsigned, dims, voxel_size))
    } else {
        None
    };
    let inside = if fill_enabled {
        Some(flood_fill_inside(
            &unsigned,
            dims,
            shell_band,
            fill_normal_bias,
            grad.as_deref(),
        ))
    } else {
        None
    };

    let mut values = try_alloc_f32(total as usize, "Volume from Splats")?;
    let shell_offset = shell_band.max(voxel_size * 0.5);
    parallel::for_each_indexed_mut(&mut values, |idx, slot| {
        let unsigned_dist = unsigned.get(idx).copied().unwrap_or(0.0);
        let mut signed_dist = if let Some(inside) = &inside {
            if inside.get(idx).copied().unwrap_or(false) {
                -unsigned_dist
            } else {
                unsigned_dist
            }
        } else {
            unsigned_dist
        };
        if !fill_enabled {
            signed_dist -= shell_offset;
        }

        *slot = match kind {
            VolumeKind::Density => {
                let half = (voxel_size * 0.5).max(1.0e-6);
                let t = ((half - signed_dist) / (2.0 * half)).clamp(0.0, 1.0);
                let smooth = t * t * (3.0 - 2.0 * t);
                smooth * density_scale
            }
            VolumeKind::Sdf => signed_dist,
        };
    });

    let mut volume = Volume::new(kind, min.to_array(), dims, voxel_size, values);
    volume.density_scale = if matches!(kind, VolumeKind::Density) {
        1.0
    } else {
        density_scale
    };
    volume.sdf_band = sdf_band;

    Ok(Geometry::with_volume(volume))
}

struct GatheredSplats {
    min: Vec3,
    max: Vec3,
    splats: Vec<SplatShape>,
}

struct SplatGatherSettings<'a> {
    radius_mode: &'a str,
    shape_mode: &'a str,
    radius_scale: f32,
    min_radius: f32,
    support_sigma: f32,
    ellipsoid_blend: f32,
    outlier_filter: bool,
    outlier_radius: f32,
    outlier_min_neighbors: usize,
    outlier_min_opacity: f32,
}

fn gather_splats(
    input: &Geometry,
    settings: &SplatGatherSettings<'_>,
) -> Result<GatheredSplats, String> {
    let use_ellipsoid = settings.shape_mode.to_lowercase().contains("ellipsoid");
    let mut candidates = Vec::new();
    for splat in &input.splats {
        for (idx, position) in splat.positions.iter().enumerate() {
            let center = Vec3::from(*position);
            let opacity = splat.opacity.get(idx).copied().unwrap_or(0.0);
            let (radius, radii, inv_radii, inv_radii_sq) = splat_radius(
                splat.scales.get(idx).copied(),
                settings.radius_mode,
                settings.radius_scale,
                settings.min_radius,
                settings.support_sigma,
            );
            let rot = splat_rotation(splat.rotations.get(idx).copied());
            let bound_radius = if use_ellipsoid {
                radii.max_element()
            } else {
                radius
            };
            candidates.push(SplatCandidate {
                center,
                opacity,
                radius,
                radii,
                inv_radii,
                inv_radii_sq,
                rotation: rot,
                bound_radius,
                is_ellipsoid: use_ellipsoid,
                ellipsoid_blend: settings.ellipsoid_blend,
            });
        }
    }

    if candidates.is_empty() {
        return Err("Volume from Splats requires splat geometry input".to_string());
    }

    let mut splats = Vec::new();
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    let filtered = if settings.outlier_filter
        && settings.outlier_radius > 0.0
        && settings.outlier_min_neighbors > 0
    {
        filter_outliers(
            &candidates,
            settings.outlier_radius,
            settings.outlier_min_neighbors,
            settings.outlier_min_opacity,
        )
    } else if settings.outlier_filter {
        candidates
            .iter()
            .enumerate()
            .filter_map(|(idx, candidate)| {
                if candidate.opacity < settings.outlier_min_opacity {
                    None
                } else {
                    Some(idx)
                }
            })
            .collect()
    } else {
        (0..candidates.len()).collect::<Vec<_>>()
    };

    for idx in filtered {
        let candidate = candidates[idx];
        min = min.min(candidate.center - Vec3::splat(candidate.bound_radius));
        max = max.max(candidate.center + Vec3::splat(candidate.bound_radius));
        splats.push(SplatShape {
            center: candidate.center,
            radius: candidate.radius,
            radii: candidate.radii,
            inv_radii: candidate.inv_radii,
            inv_radii_sq: candidate.inv_radii_sq,
            rotation: candidate.rotation,
            is_ellipsoid: candidate.is_ellipsoid,
            bound_radius: candidate.bound_radius,
            ellipsoid_blend: candidate.ellipsoid_blend,
        });
    }

    if splats.is_empty() {
        return Err("Volume from Splats filtered out all splats".to_string());
    }

    Ok(GatheredSplats { min, max, splats })
}

fn dims_from_size(size: Vec3, voxel_size: f32) -> [u32; 3] {
    [
        ((size.x / voxel_size).ceil().max(1.0) as u32).saturating_add(1),
        ((size.y / voxel_size).ceil().max(1.0) as u32).saturating_add(1),
        ((size.z / voxel_size).ceil().max(1.0) as u32).saturating_add(1),
    ]
}

fn splat_radius(
    scale: Option<[f32; 3]>,
    radius_mode: &str,
    radius_scale: f32,
    min_radius: f32,
    support_sigma: f32,
) -> (f32, Vec3, Vec3, Vec3) {
    let scale = scale.unwrap_or([0.0, 0.0, 0.0]);
    let clamped = Vec3::new(
        scale[0].clamp(-10.0, 10.0).exp(),
        scale[1].clamp(-10.0, 10.0).exp(),
        scale[2].clamp(-10.0, 10.0).exp(),
    );
    let mut radius = match radius_mode.to_lowercase().as_str() {
        "min" => clamped.x.min(clamped.y.min(clamped.z)),
        "max" => clamped.x.max(clamped.y.max(clamped.z)),
        _ => (clamped.x + clamped.y + clamped.z) / 3.0,
    };
    radius *= radius_scale;
    radius *= support_sigma;
    if !radius.is_finite() {
        radius = min_radius.max(1.0e-4);
    }
    radius = radius.max(min_radius.max(1.0e-6));
    let mut radii = clamped * radius_scale * support_sigma;
    radii.x = radii.x.max(min_radius.max(1.0e-6));
    radii.y = radii.y.max(min_radius.max(1.0e-6));
    radii.z = radii.z.max(min_radius.max(1.0e-6));
    let inv_radii = Vec3::new(1.0 / radii.x, 1.0 / radii.y, 1.0 / radii.z);
    let inv_radii_sq = Vec3::new(
        inv_radii.x * inv_radii.x,
        inv_radii.y * inv_radii.y,
        inv_radii.z * inv_radii.z,
    );
    (radius, radii, inv_radii, inv_radii_sq)
}

fn flood_fill_inside(
    unsigned: &[f32],
    dims: [u32; 3],
    shell: f32,
    normal_bias: f32,
    grad: Option<&[f32]>,
) -> Vec<bool> {
    let dim_x = dims[0] as usize;
    let dim_y = dims[1] as usize;
    let dim_z = dims[2] as usize;
    let stride_xy = dim_x.saturating_mul(dim_y).max(1);
    let total = dim_x.saturating_mul(dim_y).saturating_mul(dim_z);
    let mut outside = vec![false; total];
    let mut stack = Vec::new();

    let push_if_outside = |idx: usize,
                           unsigned: &[f32],
                           outside: &mut [bool],
                           stack: &mut Vec<usize>| {
        if idx >= outside.len() {
            return;
        }
        if outside[idx] {
            return;
        }
        let dist = unsigned.get(idx).copied().unwrap_or(0.0);
        let mut shell_thresh = shell;
        if normal_bias > 0.0 {
            if let Some(grad) = grad {
                let g = grad.get(idx).copied().unwrap_or(1.0).clamp(0.0, 2.0);
                let expand = (1.0 - g).clamp(0.0, 1.0);
                shell_thresh *= 1.0 + normal_bias * expand;
            }
        }
        if dist > shell_thresh {
            outside[idx] = true;
            stack.push(idx);
        }
    };

    for z in 0..dim_z {
        for y in 0..dim_y {
            for x in 0..dim_x {
                if x != 0 && x + 1 != dim_x && y != 0 && y + 1 != dim_y && z != 0 && z + 1 != dim_z
                {
                    continue;
                }
                let idx = x + y * dim_x + z * stride_xy;
                push_if_outside(idx, unsigned, &mut outside, &mut stack);
            }
        }
    }

    while let Some(idx) = stack.pop() {
        let z = idx / stride_xy;
        let rem = idx - z * stride_xy;
        let y = rem / dim_x;
        let x = rem - y * dim_x;
        let mut try_neighbor = |nx: isize, ny: isize, nz: isize| {
            if nx < 0 || ny < 0 || nz < 0 {
                return;
            }
            let nx = nx as usize;
            let ny = ny as usize;
            let nz = nz as usize;
            if nx >= dim_x || ny >= dim_y || nz >= dim_z {
                return;
            }
            let nidx = nx + ny * dim_x + nz * stride_xy;
            push_if_outside(nidx, unsigned, &mut outside, &mut stack);
        };
        try_neighbor(x as isize - 1, y as isize, z as isize);
        try_neighbor(x as isize + 1, y as isize, z as isize);
        try_neighbor(x as isize, y as isize - 1, z as isize);
        try_neighbor(x as isize, y as isize + 1, z as isize);
        try_neighbor(x as isize, y as isize, z as isize - 1);
        try_neighbor(x as isize, y as isize, z as isize + 1);
    }

    outside.iter().map(|v| !*v).collect()
}

#[derive(Clone, Copy)]
struct SplatShape {
    center: Vec3,
    radius: f32,
    radii: Vec3,
    inv_radii: Vec3,
    inv_radii_sq: Vec3,
    rotation: glam::Mat3,
    is_ellipsoid: bool,
    bound_radius: f32,
    ellipsoid_blend: f32,
}

#[derive(Clone, Copy)]
struct SplatCandidate {
    center: Vec3,
    opacity: f32,
    radius: f32,
    radii: Vec3,
    inv_radii: Vec3,
    inv_radii_sq: Vec3,
    rotation: glam::Mat3,
    bound_radius: f32,
    is_ellipsoid: bool,
    ellipsoid_blend: f32,
}

fn splat_rotation(rotation: Option<[f32; 4]>) -> glam::Mat3 {
    let rotation = rotation.unwrap_or([0.0, 0.0, 0.0, 1.0]);
    let mut quat = glam::Quat::from_xyzw(rotation[1], rotation[2], rotation[3], rotation[0]);
    let len2 = quat.length_squared();
    if !quat.is_finite() || len2 < 1.0e-8 {
        quat = glam::Quat::IDENTITY;
    } else {
        quat = quat / len2.sqrt();
    }
    glam::Mat3::from_quat(quat)
}

fn ellipsoid_signed_distance(
    mut p_local: Vec3,
    inv_radii: Vec3,
    inv_radii_sq: Vec3,
    radii: Vec3,
    refine_steps: u32,
) -> f32 {
    let k0 = (p_local * inv_radii).length();
    let k1 = (p_local * inv_radii_sq).length();
    if k1 <= 1.0e-8 || !k1.is_finite() {
        return -radii.min_element();
    }
    let mut dist = k0 * (k0 - 1.0) / k1;
    if refine_steps == 0 {
        return dist;
    }
    let mut f = (p_local.x * inv_radii.x).powi(2)
        + (p_local.y * inv_radii.y).powi(2)
        + (p_local.z * inv_radii.z).powi(2)
        - 1.0;
    let sign = if f < 0.0 { -1.0 } else { 1.0 };
    let p_start = p_local;
    let mut grad = Vec3::new(
        2.0 * p_local.x * inv_radii_sq.x,
        2.0 * p_local.y * inv_radii_sq.y,
        2.0 * p_local.z * inv_radii_sq.z,
    );
    for _ in 0..refine_steps {
        let g2 = grad.length_squared();
        if g2 < 1.0e-12 || !g2.is_finite() {
            break;
        }
        let step = f / g2;
        if !step.is_finite() {
            break;
        }
        p_local -= grad * step;
        f = (p_local.x * inv_radii.x).powi(2)
            + (p_local.y * inv_radii.y).powi(2)
            + (p_local.z * inv_radii.z).powi(2)
            - 1.0;
        grad = Vec3::new(
            2.0 * p_local.x * inv_radii_sq.x,
            2.0 * p_local.y * inv_radii_sq.y,
            2.0 * p_local.z * inv_radii_sq.z,
        );
    }
    let refined = sign * (p_start - p_local).length();
    if refined.is_finite() {
        dist = refined;
    }
    dist
}

fn distance_gradient_magnitude(unsigned: &[f32], dims: [u32; 3], voxel: f32) -> Vec<f32> {
    let dim_x = dims[0] as usize;
    let dim_y = dims[1] as usize;
    let dim_z = dims[2] as usize;
    let stride_xy = dim_x.saturating_mul(dim_y).max(1);
    let mut grad = vec![0.0; unsigned.len()];
    let inv = if voxel > 1.0e-8 { 0.5 / voxel } else { 1.0 };
    for z in 0..dim_z {
        for y in 0..dim_y {
            for x in 0..dim_x {
                let idx = x + y * dim_x + z * stride_xy;
                let xm = x.saturating_sub(1);
                let xp = (x + 1).min(dim_x.saturating_sub(1));
                let ym = y.saturating_sub(1);
                let yp = (y + 1).min(dim_y.saturating_sub(1));
                let zm = z.saturating_sub(1);
                let zp = (z + 1).min(dim_z.saturating_sub(1));
                let idx_xm = xm + y * dim_x + z * stride_xy;
                let idx_xp = xp + y * dim_x + z * stride_xy;
                let idx_ym = x + ym * dim_x + z * stride_xy;
                let idx_yp = x + yp * dim_x + z * stride_xy;
                let idx_zm = x + y * dim_x + zm * stride_xy;
                let idx_zp = x + y * dim_x + zp * stride_xy;
                let dx = (unsigned.get(idx_xp).copied().unwrap_or(0.0)
                    - unsigned.get(idx_xm).copied().unwrap_or(0.0))
                    * inv;
                let dy = (unsigned.get(idx_yp).copied().unwrap_or(0.0)
                    - unsigned.get(idx_ym).copied().unwrap_or(0.0))
                    * inv;
                let dz = (unsigned.get(idx_zp).copied().unwrap_or(0.0)
                    - unsigned.get(idx_zm).copied().unwrap_or(0.0))
                    * inv;
                let g = Vec3::new(dx, dy, dz).length();
                grad[idx] = if g.is_finite() { g } else { 1.0 };
            }
        }
    }
    grad
}

fn filter_outliers(
    candidates: &[SplatCandidate],
    radius: f32,
    min_neighbors: usize,
    min_opacity: f32,
) -> Vec<usize> {
    let cell = radius.max(1.0e-6);
    let mut grid: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();
    for (idx, splat) in candidates.iter().enumerate() {
        if splat.opacity < min_opacity {
            continue;
        }
        let key = cell_key(splat.center, cell);
        grid.entry(key).or_default().push(idx);
    }
    let mut kept = Vec::new();
    for (idx, splat) in candidates.iter().enumerate() {
        if splat.opacity < min_opacity {
            continue;
        }
        let key = cell_key(splat.center, cell);
        let mut neighbors = 0usize;
        for dz in -1..=1 {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let neighbor_key = (key.0 + dx, key.1 + dy, key.2 + dz);
                    let Some(list) = grid.get(&neighbor_key) else {
                        continue;
                    };
                    for &other in list {
                        if other == idx {
                            continue;
                        }
                        let dist = (candidates[other].center - splat.center).length();
                        if dist <= radius {
                            neighbors += 1;
                            if neighbors >= min_neighbors {
                                break;
                            }
                        }
                    }
                    if neighbors >= min_neighbors {
                        break;
                    }
                }
                if neighbors >= min_neighbors {
                    break;
                }
            }
            if neighbors >= min_neighbors {
                break;
            }
        }
        if neighbors >= min_neighbors {
            kept.push(idx);
        }
    }
    kept
}

fn cell_key(center: Vec3, cell: f32) -> (i32, i32, i32) {
    let inv = 1.0 / cell.max(1.0e-6);
    (
        (center.x * inv).floor() as i32,
        (center.y * inv).floor() as i32,
        (center.z * inv).floor() as i32,
    )
}
