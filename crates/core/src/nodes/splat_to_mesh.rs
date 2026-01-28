use std::collections::BTreeMap;

use glam::{Mat3, Mat4, Quat, Vec3};
use lin_alg::f32::Vec3 as McVec3;
use mcubes::{MarchingCubes, MeshSide};

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{geometry_in, geometry_out};
use crate::parallel;
use crate::param_spec::ParamSpec;
use crate::splat::SplatGeo;
use crate::volume::{Volume, VolumeKind};
use crate::volume_sampling::VolumeSampler;

pub const NAME: &str = "Splat to Mesh";

const DEFAULT_VOXEL_SIZE: f32 = 0.1;
const DEFAULT_N_SIGMA: f32 = 3.0;
const DEFAULT_DENSITY_ISO: f32 = 0.5;
const DEFAULT_SURFACE_ISO: f32 = 0.0;
const DEFAULT_BOUNDS_PADDING: f32 = 3.0;
const DEFAULT_MAX_M2: f32 = 3.0;
const DEFAULT_SMOOTH_K: f32 = 0.1;
const DEFAULT_SHELL_RADIUS: f32 = 1.0;
const DEFAULT_BLUR_ITERS: i32 = 1;
const DEFAULT_MAX_VOXEL_DIM: i32 = 256;
const DEFAULT_TRANSFER_COLOR: bool = true;
const DEFAULT_OUTPUT_MODE: i32 = 0;
const MAX_GRID_POINTS: u64 = 32_000_000;

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![geometry_in("splats"), geometry_in("sdf")],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            ("output".to_string(), ParamValue::Int(DEFAULT_OUTPUT_MODE)),
            ("algorithm".to_string(), ParamValue::Int(0)),
            ("voxel_size".to_string(), ParamValue::Float(DEFAULT_VOXEL_SIZE)),
            (
                "voxel_size_max".to_string(),
                ParamValue::Int(DEFAULT_MAX_VOXEL_DIM),
            ),
            ("n_sigma".to_string(), ParamValue::Float(DEFAULT_N_SIGMA)),
            ("density_iso".to_string(), ParamValue::Float(DEFAULT_DENSITY_ISO)),
            ("surface_iso".to_string(), ParamValue::Float(DEFAULT_SURFACE_ISO)),
            (
                "bounds_padding".to_string(),
                ParamValue::Float(DEFAULT_BOUNDS_PADDING),
            ),
            (
                "transfer_color".to_string(),
                ParamValue::Bool(DEFAULT_TRANSFER_COLOR),
            ),
            ("max_m2".to_string(), ParamValue::Float(DEFAULT_MAX_M2)),
            ("smooth_k".to_string(), ParamValue::Float(DEFAULT_SMOOTH_K)),
            (
                "shell_radius".to_string(),
                ParamValue::Float(DEFAULT_SHELL_RADIUS),
            ),
            ("blur_iters".to_string(), ParamValue::Int(DEFAULT_BLUR_ITERS)),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::int_enum(
            "output",
            "Output",
            vec![(0, "Mesh"), (1, "SDF Volume")],
        )
        .with_help("Output type (mesh or SDF volume)."),
        ParamSpec::int_enum(
            "algorithm",
            "Method",
            vec![(0, "Density (Iso)"), (1, "Ellipsoid (Smooth Min)")],
        )
        .with_help("Conversion method.")
        .visible_when_int("output", 0),
        ParamSpec::float_slider("voxel_size", "Voxel Size", 0.0, 10.0)
            .with_help("Voxel size for density grid."),
        ParamSpec::int_slider("voxel_size_max", "Max Voxel Dimension", 8, 2048)
            .with_help("Max voxel dimension (safety clamp)."),
        ParamSpec::float_slider("n_sigma", "Support Sigma", 0.0, 6.0)
            .with_help("Gaussian support radius in sigmas."),
        ParamSpec::float_slider("density_iso", "Density Threshold", 0.0, 10.0)
            .with_help("Density threshold for marching cubes.")
            .visible_when_int("output", 0)
            .visible_when_int("algorithm", 0),
        ParamSpec::float_slider("surface_iso", "Surface Threshold", -5.0, 5.0)
            .with_help("Surface threshold for ellipsoid method.")
            .visible_when_int("output", 0)
            .visible_when_int("algorithm", 1),
        ParamSpec::float_slider("bounds_padding", "Bounds Padding (sigma)", 0.0, 10.0)
            .with_help("Padding around bounds in sigmas."),
        ParamSpec::bool("transfer_color", "Transfer Color")
            .with_help("Transfer splat color to mesh Cd.")
            .visible_when_int("output", 0),
        ParamSpec::float_slider("max_m2", "Exponent Clamp", 0.0, 10.0)
            .with_help("Exponent clamp for ellipsoid blend."),
        ParamSpec::float_slider("smooth_k", "Blend Sharpness", 0.001, 2.0)
            .with_help("Smooth-min blend sharpness.")
            .visible_when_int("algorithm", 1),
        ParamSpec::float_slider("shell_radius", "Shell Radius", 0.1, 4.0)
            .with_help("Shell thickness for ellipsoid.")
            .visible_when_int("algorithm", 1),
        ParamSpec::int_slider("blur_iters", "Density Blur", 0, 6)
            .with_help("Density blur iterations.")
            .visible_when_int("output", 0)
            .visible_when_int("algorithm", 0),
    ]
}

pub fn apply_to_geometry(
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };
    let Some(splats) = input.merged_splats() else {
        return Err("Splat to Mesh requires splat geometry on input 0".to_string());
    };
    let external_sdf = if let Some(geo) = inputs.get(1) {
        let Some(volume) = geo.volumes.first() else {
            return Err("Splat to Mesh SDF input requires a volume".to_string());
        };
        if volume.kind != VolumeKind::Sdf {
            return Err("Splat to Mesh SDF input requires an SDF volume".to_string());
        }
        Some(volume)
    } else {
        None
    };

    let output_mode = params.get_int("output", DEFAULT_OUTPUT_MODE).clamp(0, 1);
    if output_mode == 1 {
        let volume = if let Some(volume) = external_sdf {
            volume.clone()
        } else {
            splats_to_sdf(params, &splats)?
        };
        let mut meshes = Vec::new();
        if let Some(existing) = input.merged_mesh() {
            meshes.push(existing);
        }
        let curves = if meshes.is_empty() {
            Vec::new()
        } else {
            input.curves.clone()
        };
        let mut volumes = Vec::with_capacity(input.volumes.len() + 1);
        volumes.push(volume);
        volumes.extend(input.volumes.clone());
        return Ok(Geometry {
            meshes,
            splats: Vec::new(),
            curves,
            volumes,
            materials: input.materials.clone(),
        });
    }

    let mesh = if let Some(volume) = external_sdf {
        let iso = params.get_float("surface_iso", DEFAULT_SURFACE_ISO);
        crate::nodes::volume_to_mesh::volume_to_mesh(volume, iso, false)?
    } else {
        splats_to_mesh(params, &splats)?
    };
    let mut meshes = Vec::new();
    if let Some(existing) = input.merged_mesh() {
        if mesh.positions.is_empty() && mesh.indices.is_empty() {
            meshes.push(existing);
        } else {
            meshes.push(Mesh::merge(&[existing, mesh]));
        }
    } else if !mesh.positions.is_empty() || !mesh.indices.is_empty() {
        meshes.push(mesh);
    }

    let curves = if meshes.is_empty() {
        Vec::new()
    } else {
        input.curves.clone()
    };

    Ok(Geometry {
        meshes,
        splats: Vec::new(),
        curves,
        volumes: input.volumes.clone(),
        materials: input.materials.clone(),
    })
}

#[derive(Clone)]
struct SplatSample {
    mu: Vec3,
    rt: Mat3,
    sigma: Vec3,
    alpha: f32,
    max_sigma: f32,
    color: [f32; 3],
}

impl SplatSample {
    fn m2(&self, position: Vec3) -> f32 {
        let dx = position - self.mu;
        let u = self.rt * dx;
        let sx = u.x / self.sigma.x;
        let sy = u.y / self.sigma.y;
        let sz = u.z / self.sigma.z;
        sx * sx + sy * sy + sz * sz
    }
}

impl Default for SplatSample {
    fn default() -> Self {
        Self {
            mu: Vec3::ZERO,
            rt: Mat3::IDENTITY,
            sigma: Vec3::ZERO,
            alpha: 0.0,
            max_sigma: 0.0,
            color: [0.0, 0.0, 0.0],
        }
    }
}

pub(crate) enum SplatOutputMode {
    Mesh,
    Sdf,
}

pub(crate) struct SplatGrid {
    pub(crate) values: Vec<f32>,
    color_grid: Option<ColorGrid>,
    pub(crate) spec: GridSpec,
    pub(crate) iso: f32,
    pub(crate) inside_is_greater: bool,
}

fn splats_to_mesh(params: &NodeParams, splats: &SplatGeo) -> Result<Mesh, String> {
    let grid = build_splat_grid(params, splats, SplatOutputMode::Mesh)?;
    let mut mesh =
        marching_cubes(&grid.values, &grid.spec, grid.iso, grid.inside_is_greater)?;
    if let Some(color_grid) = grid.color_grid {
        if !mesh.positions.is_empty() {
            let positions = &mesh.positions;
            let mut colors = vec![[1.0, 1.0, 1.0]; positions.len()];
            parallel::for_each_indexed_mut(&mut colors, |idx, slot| {
                let color = sample_color_grid(&color_grid, &grid.spec, Vec3::from(positions[idx]));
                *slot = color;
            });
            mesh.set_attribute(
                AttributeDomain::Point,
                "Cd",
                AttributeStorage::Vec3(colors),
            )
            .map_err(|err| format!("Failed to set Cd attribute: {err:?}"))?;
        }
    }
    let _ = mesh.compute_normals();
    Ok(mesh)
}

fn splats_to_sdf(params: &NodeParams, splats: &SplatGeo) -> Result<Volume, String> {
    let grid = build_splat_grid(params, splats, SplatOutputMode::Sdf)?;
    let dims = [grid.spec.nx as u32, grid.spec.ny as u32, grid.spec.nz as u32];
    let mut volume = Volume::new(
        VolumeKind::Sdf,
        grid.spec.min.to_array(),
        dims,
        grid.spec.dx,
        grid.values,
    );
    volume.sdf_band = grid.spec.dx.max(1.0e-6) * 2.0;
    Ok(volume)
}

pub(crate) fn sdf_grid_from_volume(
    volume: &Volume,
    target_spec: Option<&GridSpec>,
) -> Result<SplatGrid, String> {
    if volume.kind != VolumeKind::Sdf {
        return Err("Expected SDF volume".to_string());
    }
    let spec = if let Some(spec) = target_spec {
        GridSpec {
            min: spec.min,
            dx: spec.dx,
            nx: spec.nx,
            ny: spec.ny,
            nz: spec.nz,
        }
    } else {
        grid_spec_from_volume(volume)
    };
    let iso = 0.0;
    let inside_is_greater = false;
    if spec.nx == 0 || spec.ny == 0 || spec.nz == 0 {
        return Ok(SplatGrid {
            values: Vec::new(),
            color_grid: None,
            spec,
            iso,
            inside_is_greater,
        });
    }

    let values = if target_spec.is_none()
        && volume_matches_spec(volume, &spec)
        && volume.transform == Mat4::IDENTITY
    {
        volume.values.clone()
    } else {
        sample_volume_to_grid(volume, &spec)
    };

    Ok(SplatGrid {
        values,
        color_grid: None,
        spec,
        iso,
        inside_is_greater,
    })
}

fn grid_spec_from_volume(volume: &Volume) -> GridSpec {
    GridSpec {
        min: Vec3::from(volume.origin),
        dx: volume.voxel_size.max(1.0e-6),
        nx: volume.dims[0] as usize,
        ny: volume.dims[1] as usize,
        nz: volume.dims[2] as usize,
    }
}

fn volume_matches_spec(volume: &Volume, spec: &GridSpec) -> bool {
    let origin = Vec3::from(volume.origin);
    volume.dims[0] as usize == spec.nx
        && volume.dims[1] as usize == spec.ny
        && volume.dims[2] as usize == spec.nz
        && (volume.voxel_size - spec.dx).abs() < 1.0e-6
        && (origin - spec.min).length() < 1.0e-4
}

fn sample_volume_to_grid(volume: &Volume, spec: &GridSpec) -> Vec<f32> {
    let total = spec.nx * spec.ny * spec.nz;
    if total == 0 {
        return Vec::new();
    }
    let mut values = vec![0.0f32; total];
    let sampler = VolumeSampler::new(volume);
    let nx = spec.nx;
    let ny = spec.ny;
    let slice = nx * ny;
    let min = spec.min;
    let dx = spec.dx;
    parallel::for_each_indexed_mut(&mut values, |idx, slot| {
        let iz = idx / slice;
        let rem = idx - iz * slice;
        let iy = rem / nx;
        let ix = rem - iy * nx;
        let pos = min + Vec3::new(ix as f32 * dx, iy as f32 * dx, iz as f32 * dx);
        *slot = sampler.sample_world(pos);
    });
    values
}

pub(crate) fn build_splat_grid(
    params: &NodeParams,
    splats: &SplatGeo,
    output_mode: SplatOutputMode,
) -> Result<SplatGrid, String> {
    if splats.is_empty() {
        return Ok(SplatGrid {
            values: Vec::new(),
            color_grid: None,
            spec: GridSpec {
                min: Vec3::ZERO,
                dx: 1.0,
                nx: 0,
                ny: 0,
                nz: 0,
            },
            iso: 0.0,
            inside_is_greater: true,
        });
    }

    let mut algorithm = params.get_int("algorithm", 0).clamp(0, 1);
    if matches!(output_mode, SplatOutputMode::Sdf) {
        algorithm = 1;
    }
    let voxel_size = params
        .get_float("voxel_size", DEFAULT_VOXEL_SIZE)
        .max(1.0e-4);
    let max_voxel_dim = params
        .get_int("voxel_size_max", DEFAULT_MAX_VOXEL_DIM)
        .max(1) as usize;
    let n_sigma = params.get_float("n_sigma", DEFAULT_N_SIGMA).max(0.1);
    let density_iso = params.get_float("density_iso", DEFAULT_DENSITY_ISO);
    let surface_iso = params.get_float("surface_iso", DEFAULT_SURFACE_ISO);
    let bounds_padding = params
        .get_float("bounds_padding", DEFAULT_BOUNDS_PADDING)
        .max(0.0);
    let transfer_color = params.get_bool("transfer_color", DEFAULT_TRANSFER_COLOR);
    let max_m2 = params
        .get_float("max_m2", DEFAULT_MAX_M2)
        .clamp(0.0, 10.0);
    let smooth_k = params.get_float("smooth_k", DEFAULT_SMOOTH_K).max(0.0);
    let shell_radius = params
        .get_float("shell_radius", DEFAULT_SHELL_RADIUS)
        .max(0.01);
    let blur_iters = params.get_int("blur_iters", DEFAULT_BLUR_ITERS).max(0) as usize;

    let inside_is_greater = algorithm == 0;
    let iso = if inside_is_greater { density_iso } else { surface_iso };
    let samples = build_samples(splats);
    if samples.is_empty() {
        return Ok(SplatGrid {
            values: Vec::new(),
            color_grid: None,
            spec: GridSpec {
                min: Vec3::ZERO,
                dx: 1.0,
                nx: 0,
                ny: 0,
                nz: 0,
            },
            iso,
            inside_is_greater,
        });
    }

    let want_color = matches!(output_mode, SplatOutputMode::Mesh) && transfer_color;
    let (mut values, mut color_grid, spec) = match algorithm {
        1 => {
            let spec = build_grid_spec(&samples, voxel_size, bounds_padding, max_voxel_dim)?;
            let mut color_grid = want_color.then(|| ColorGrid::new(spec.nx * spec.ny * spec.nz));
            let grid = rasterize_smoothmin(
                &samples,
                &spec,
                n_sigma,
                max_m2,
                smooth_k,
                shell_radius,
                color_grid.as_mut(),
            );
            (grid, color_grid, spec)
        }
        _ => {
            let spec = build_grid_spec(&samples, voxel_size, bounds_padding, max_voxel_dim)?;
            let mut color_grid = want_color.then(|| ColorGrid::new(spec.nx * spec.ny * spec.nz));
            let grid = rasterize_density(&samples, &spec, n_sigma, max_m2, color_grid.as_mut());
            (grid, color_grid, spec)
        }
    };

    sanitize_grid(&mut values, iso, inside_is_greater);
    if matches!(output_mode, SplatOutputMode::Mesh) && algorithm == 0 && blur_iters > 0 {
        blur_grid(&mut values, &spec, blur_iters);
        if let Some(color_grid) = color_grid.as_mut() {
            blur_color_grid(color_grid, &spec, blur_iters);
        }
    }

    Ok(SplatGrid {
        values,
        color_grid,
        spec,
        iso,
        inside_is_greater,
    })
}

pub(crate) struct GridSpec {
    pub min: Vec3,
    pub dx: f32,
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
}

struct ColorGrid {
    sum: Vec<[f32; 3]>,
    weight: Vec<f32>,
}

impl ColorGrid {
    fn new(len: usize) -> Self {
        Self {
            sum: vec![[0.0, 0.0, 0.0]; len],
            weight: vec![0.0; len],
        }
    }

    fn add(&mut self, idx: usize, weight: f32, color: [f32; 3]) {
        let sum = &mut self.sum[idx];
        sum[0] += color[0] * weight;
        sum[1] += color[1] * weight;
        sum[2] += color[2] * weight;
        self.weight[idx] += weight;
    }
}

fn build_samples(splats: &SplatGeo) -> Vec<SplatSample> {
    let use_sh0_colors = splats.sh0.iter().any(|value| {
        value
            .iter()
            .any(|channel| channel.is_finite() && *channel < 0.0)
    });
    let mut samples = vec![SplatSample::default(); splats.len()];
    parallel::for_each_indexed_mut(&mut samples, |idx, sample| {
        let position = Vec3::from(splats.positions[idx]);
        let rotation = splats.rotations[idx];
        let mut quat = Quat::from_xyzw(rotation[1], rotation[2], rotation[3], rotation[0]);
        if quat.length_squared() > 0.0 {
            quat = quat.normalize();
        } else {
            quat = Quat::IDENTITY;
        }
        let rot = Mat3::from_quat(quat);
        let rt = rot.transpose();
        let log_scale = splats.scales[idx];
        let mut sigma = Vec3::new(log_scale[0].exp(), log_scale[1].exp(), log_scale[2].exp());
        sigma = sigma.max(Vec3::splat(1.0e-5));
        let max_sigma = sigma.x.max(sigma.y).max(sigma.z);

        let mut opacity = splats.opacity[idx];
        if !opacity.is_finite() {
            opacity = 0.0;
        }
        let opacity = opacity.clamp(-20.0, 20.0);
        let alpha = 1.0 / (1.0 + (-opacity).exp());

        let mut color = splats.sh0[idx];
        if color.iter().any(|value| !value.is_finite()) {
            color = [1.0, 1.0, 1.0];
        }
        if use_sh0_colors {
            const SH_C0: f32 = 0.2820948;
            color = [
                color[0] * SH_C0 + 0.5,
                color[1] * SH_C0 + 0.5,
                color[2] * SH_C0 + 0.5,
            ];
        }

        *sample = SplatSample {
            mu: position,
            rt,
            sigma,
            alpha,
            max_sigma,
            color,
        };
    });
    samples
}

fn build_grid_spec(
    samples: &[SplatSample],
    voxel_size: f32,
    bounds_padding: f32,
    max_voxel_dim: usize,
) -> Result<GridSpec, String> {
    let mut min = samples[0].mu;
    let mut max = samples[0].mu;
    let mut max_sigma = samples[0].max_sigma;
    for sample in samples.iter().skip(1) {
        min = min.min(sample.mu);
        max = max.max(sample.mu);
        max_sigma = max_sigma.max(sample.max_sigma);
    }
    let pad = bounds_padding * max_sigma;
    min -= Vec3::splat(pad);
    max += Vec3::splat(pad);

    let extent = max - min;
    let max_dim = max_voxel_dim.max(1) as f32;
    let mut dx = voxel_size;
    if extent.x > 0.0 {
        dx = dx.max(extent.x / max_dim);
    }
    if extent.y > 0.0 {
        dx = dx.max(extent.y / max_dim);
    }
    if extent.z > 0.0 {
        dx = dx.max(extent.z / max_dim);
    }
    dx = dx.max(1.0e-4);

    let cells_x = ((extent.x / dx).ceil() as isize).max(1) as usize;
    let cells_y = ((extent.y / dx).ceil() as isize).max(1) as usize;
    let cells_z = ((extent.z / dx).ceil() as isize).max(1) as usize;
    let nx = cells_x + 1;
    let ny = cells_y + 1;
    let nz = cells_z + 1;
    let total = nx as u64 * ny as u64 * nz as u64;
    if total > MAX_GRID_POINTS {
        return Err(format!(
            "Splat to Mesh grid too large ({} points). Increase voxel size.",
            total
        ));
    }

    Ok(GridSpec {
        min,
        dx,
        nx,
        ny,
        nz,
    })
}

fn rasterize_density(
    samples: &[SplatSample],
    spec: &GridSpec,
    n_sigma: f32,
    max_m2: f32,
    mut color_grid: Option<&mut ColorGrid>,
) -> Vec<f32> {
    let mut grid = vec![0.0f32; spec.nx * spec.ny * spec.nz];
    let cutoff_m2 = n_sigma * n_sigma;
    for sample in samples {
        let r = n_sigma * sample.max_sigma;
        let min = sample.mu - Vec3::splat(r);
        let max = sample.mu + Vec3::splat(r);
        let ix0 = ((min.x - spec.min.x) / spec.dx).floor() as isize;
        let iy0 = ((min.y - spec.min.y) / spec.dx).floor() as isize;
        let iz0 = ((min.z - spec.min.z) / spec.dx).floor() as isize;
        let ix1 = ((max.x - spec.min.x) / spec.dx).ceil() as isize;
        let iy1 = ((max.y - spec.min.y) / spec.dx).ceil() as isize;
        let iz1 = ((max.z - spec.min.z) / spec.dx).ceil() as isize;
        let ix0 = ix0.clamp(0, spec.nx as isize - 1) as usize;
        let iy0 = iy0.clamp(0, spec.ny as isize - 1) as usize;
        let iz0 = iz0.clamp(0, spec.nz as isize - 1) as usize;
        let ix1 = ix1.clamp(0, spec.nx as isize - 1) as usize;
        let iy1 = iy1.clamp(0, spec.ny as isize - 1) as usize;
        let iz1 = iz1.clamp(0, spec.nz as isize - 1) as usize;

        for iz in iz0..=iz1 {
            let z = spec.min.z + iz as f32 * spec.dx;
            for iy in iy0..=iy1 {
                let y = spec.min.y + iy as f32 * spec.dx;
                for ix in ix0..=ix1 {
                    let x = spec.min.x + ix as f32 * spec.dx;
                    let pos = Vec3::new(x, y, z);
                    let mut m2 = sample.m2(pos);
                    if !m2.is_finite() {
                        continue;
                    }
                    if m2 > cutoff_m2 {
                        continue;
                    }
                    m2 = m2.min(max_m2);
                    let w = (-0.5 * m2).exp();
                    let idx = grid_index(spec, ix, iy, iz);
                    let weight = sample.alpha * w;
                    grid[idx] += weight;
                    if let Some(color_grid) = color_grid.as_deref_mut() {
                        color_grid.add(idx, weight, sample.color);
                    }
                }
            }
        }
    }
    grid
}

fn rasterize_smoothmin(
    samples: &[SplatSample],
    spec: &GridSpec,
    n_sigma: f32,
    max_m2: f32,
    smooth_k: f32,
    shell_radius: f32,
    mut color_grid: Option<&mut ColorGrid>,
) -> Vec<f32> {
    let mut grid = if smooth_k > 0.0 {
        vec![0.0f32; spec.nx * spec.ny * spec.nz]
    } else {
        vec![f32::INFINITY; spec.nx * spec.ny * spec.nz]
    };
    let cutoff_m2 = n_sigma * n_sigma;

    for sample in samples {
        let r = n_sigma * sample.max_sigma;
        let min = sample.mu - Vec3::splat(r);
        let max = sample.mu + Vec3::splat(r);
        let ix0 = ((min.x - spec.min.x) / spec.dx).floor() as isize;
        let iy0 = ((min.y - spec.min.y) / spec.dx).floor() as isize;
        let iz0 = ((min.z - spec.min.z) / spec.dx).floor() as isize;
        let ix1 = ((max.x - spec.min.x) / spec.dx).ceil() as isize;
        let iy1 = ((max.y - spec.min.y) / spec.dx).ceil() as isize;
        let iz1 = ((max.z - spec.min.z) / spec.dx).ceil() as isize;
        let ix0 = ix0.clamp(0, spec.nx as isize - 1) as usize;
        let iy0 = iy0.clamp(0, spec.ny as isize - 1) as usize;
        let iz0 = iz0.clamp(0, spec.nz as isize - 1) as usize;
        let ix1 = ix1.clamp(0, spec.nx as isize - 1) as usize;
        let iy1 = iy1.clamp(0, spec.ny as isize - 1) as usize;
        let iz1 = iz1.clamp(0, spec.nz as isize - 1) as usize;

        for iz in iz0..=iz1 {
            let z = spec.min.z + iz as f32 * spec.dx;
            for iy in iy0..=iy1 {
                let y = spec.min.y + iy as f32 * spec.dx;
                for ix in ix0..=ix1 {
                    let x = spec.min.x + ix as f32 * spec.dx;
                    let pos = Vec3::new(x, y, z);
                    let mut m2 = sample.m2(pos);
                    if !m2.is_finite() {
                        continue;
                    }
                    if m2 > cutoff_m2 {
                        continue;
                    }
                    m2 = m2.min(max_m2);
                    let weight = sample.alpha * (-0.5 * m2).exp();
                    let d = m2.sqrt() - shell_radius;
                    let idx = grid_index(spec, ix, iy, iz);
                    if smooth_k > 0.0 {
                        let exp_arg = (-(d / smooth_k)).clamp(-50.0, 50.0);
                        let smooth_weight = exp_arg.exp();
                        grid[idx] += sample.alpha * smooth_weight;
                    } else if d < grid[idx] {
                        grid[idx] = d;
                    }
                    if let Some(color_grid) = color_grid.as_deref_mut() {
                        color_grid.add(idx, weight, sample.color);
                    }
                }
            }
        }
    }

    if smooth_k > 0.0 {
        for value in &mut grid {
            if *value > 0.0 {
                *value = -smooth_k * value.ln();
            } else {
                *value = f32::INFINITY;
            }
        }
    }
    grid
}

fn grid_index(spec: &GridSpec, ix: usize, iy: usize, iz: usize) -> usize {
    ix + spec.nx * (iy + spec.ny * iz)
}

pub(crate) fn marching_cubes(
    grid: &[f32],
    spec: &GridSpec,
    iso: f32,
    inside_is_greater: bool,
) -> Result<Mesh, String> {
    if spec.nx < 2 || spec.ny < 2 || spec.nz < 2 {
        return Ok(Mesh::default());
    }
    if grid.is_empty() {
        return Ok(Mesh::default());
    }

    let (values, iso_level) = if inside_is_greater {
        (grid.iter().map(|v| -*v).collect::<Vec<_>>(), -iso)
    } else {
        (grid.to_vec(), iso)
    };

    let extent = Vec3::new(
        (spec.nx - 1) as f32 * spec.dx,
        (spec.ny - 1) as f32 * spec.dx,
        (spec.nz - 1) as f32 * spec.dx,
    );
    let sx = (spec.nx.saturating_sub(1)).max(1) as f32;
    let sy = (spec.ny.saturating_sub(1)).max(1) as f32;
    let sz = (spec.nz.saturating_sub(1)).max(1) as f32;
    let offset = McVec3::new(spec.min.x, spec.min.y, spec.min.z);
    let mc = MarchingCubes::new(
        (spec.nx, spec.ny, spec.nz),
        (extent.x, extent.y, extent.z),
        (sx, sy, sz),
        offset,
        values,
        iso_level,
    )
    .map_err(|err| err.to_string())?;
    let output = mc.generate(MeshSide::Both);

    let mut positions = Vec::with_capacity(output.vertices.len());
    for vertex in output.vertices {
        positions.push([vertex.posit.x, vertex.posit.y, vertex.posit.z]);
    }
    let indices = output.indices.into_iter().map(|idx| idx as u32).collect();
    Ok(Mesh::with_positions_indices(positions, indices))
}

pub(crate) fn sanitize_grid(grid: &mut [f32], iso: f32, inside_is_greater: bool) {
    let outside = if inside_is_greater { iso - 1.0 } else { iso + 1.0 };
    parallel::for_each_indexed_mut(grid, |_, value| {
        if !value.is_finite() {
            *value = outside;
        }
    });
}

fn blur_grid(grid: &mut [f32], spec: &GridSpec, iterations: usize) {
    if iterations == 0 || grid.is_empty() {
        return;
    }
    let mut max_before = 0.0f32;
    for value in grid.iter().copied() {
        if value.is_finite() && value > max_before {
            max_before = value;
        }
    }
    blur_grid_raw(grid, spec, iterations);
    if max_before > 0.0 {
        let mut max_after = 0.0f32;
        for value in grid.iter().copied() {
            if value.is_finite() && value > max_after {
                max_after = value;
            }
        }
        if max_after > 0.0 {
            let scale = max_before / max_after;
            for value in grid {
                *value *= scale;
            }
        }
    }
}

fn blur_grid_raw(grid: &mut [f32], spec: &GridSpec, iterations: usize) {
    if iterations == 0 || grid.is_empty() {
        return;
    }
    let mut temp = vec![0.0f32; grid.len()];
    for _ in 0..iterations {
        blur_axis_x(grid, &mut temp, spec);
        blur_axis_y(&temp, grid, spec);
        blur_axis_z(grid, &mut temp, spec);
        grid.copy_from_slice(&temp);
    }
}

fn blur_color_grid(color: &mut ColorGrid, spec: &GridSpec, iterations: usize) {
    if iterations == 0 || color.sum.is_empty() {
        return;
    }
    let mut temp = vec![[0.0f32; 3]; color.sum.len()];
    for _ in 0..iterations {
        blur_color_axis_x(&color.sum, &mut temp, spec);
        blur_color_axis_y(&temp, &mut color.sum, spec);
        blur_color_axis_z(&color.sum, &mut temp, spec);
        color.sum.copy_from_slice(&temp);
    }
    blur_grid_raw(&mut color.weight, spec, iterations);
}

fn blur_axis_x(src: &[f32], dst: &mut [f32], spec: &GridSpec) {
    let nx = spec.nx;
    let ny = spec.ny;
    let slice = nx * ny;
    let one_third = 1.0 / 3.0;
    parallel::for_each_indexed_mut(dst, |idx, value| {
        let iz = idx / slice;
        let rem = idx - iz * slice;
        let ix = rem % nx;
        let prev = if ix == 0 { src[idx] } else { src[idx - 1] };
        let next = if ix + 1 == nx { src[idx] } else { src[idx + 1] };
        *value = (prev + src[idx] + next) * one_third;
    });
}

fn blur_color_axis_x(src: &[[f32; 3]], dst: &mut [[f32; 3]], spec: &GridSpec) {
    let nx = spec.nx;
    let ny = spec.ny;
    let slice = nx * ny;
    let one_third = 1.0 / 3.0;
    parallel::for_each_indexed_mut(dst, |idx, value| {
        let iz = idx / slice;
        let rem = idx - iz * slice;
        let ix = rem % nx;
        let prev = if ix == 0 { src[idx] } else { src[idx - 1] };
        let next = if ix + 1 == nx { src[idx] } else { src[idx + 1] };
        let current = src[idx];
        *value = [
            (prev[0] + current[0] + next[0]) * one_third,
            (prev[1] + current[1] + next[1]) * one_third,
            (prev[2] + current[2] + next[2]) * one_third,
        ];
    });
}

fn blur_axis_y(src: &[f32], dst: &mut [f32], spec: &GridSpec) {
    let nx = spec.nx;
    let ny = spec.ny;
    let slice = nx * ny;
    let one_third = 1.0 / 3.0;
    parallel::for_each_indexed_mut(dst, |idx, value| {
        let iz = idx / slice;
        let rem = idx - iz * slice;
        let iy = rem / nx;
        let prev = if iy == 0 { src[idx] } else { src[idx - nx] };
        let next = if iy + 1 == ny { src[idx] } else { src[idx + nx] };
        *value = (prev + src[idx] + next) * one_third;
    });
}

fn blur_color_axis_y(src: &[[f32; 3]], dst: &mut [[f32; 3]], spec: &GridSpec) {
    let nx = spec.nx;
    let ny = spec.ny;
    let slice = nx * ny;
    let one_third = 1.0 / 3.0;
    parallel::for_each_indexed_mut(dst, |idx, value| {
        let iz = idx / slice;
        let rem = idx - iz * slice;
        let iy = rem / nx;
        let prev = if iy == 0 { src[idx] } else { src[idx - nx] };
        let next = if iy + 1 == ny { src[idx] } else { src[idx + nx] };
        let current = src[idx];
        *value = [
            (prev[0] + current[0] + next[0]) * one_third,
            (prev[1] + current[1] + next[1]) * one_third,
            (prev[2] + current[2] + next[2]) * one_third,
        ];
    });
}

fn blur_axis_z(src: &[f32], dst: &mut [f32], spec: &GridSpec) {
    let nx = spec.nx;
    let ny = spec.ny;
    let nz = spec.nz;
    let slice = nx * ny;
    let one_third = 1.0 / 3.0;
    parallel::for_each_indexed_mut(dst, |idx, value| {
        let iz = idx / slice;
        let prev = if iz == 0 { src[idx] } else { src[idx - slice] };
        let next = if iz + 1 == nz { src[idx] } else { src[idx + slice] };
        *value = (prev + src[idx] + next) * one_third;
    });
}

fn blur_color_axis_z(src: &[[f32; 3]], dst: &mut [[f32; 3]], spec: &GridSpec) {
    let nx = spec.nx;
    let ny = spec.ny;
    let nz = spec.nz;
    let slice = nx * ny;
    let one_third = 1.0 / 3.0;
    parallel::for_each_indexed_mut(dst, |idx, value| {
        let iz = idx / slice;
        let prev = if iz == 0 { src[idx] } else { src[idx - slice] };
        let next = if iz + 1 == nz { src[idx] } else { src[idx + slice] };
        let current = src[idx];
        *value = [
            (prev[0] + current[0] + next[0]) * one_third,
            (prev[1] + current[1] + next[1]) * one_third,
            (prev[2] + current[2] + next[2]) * one_third,
        ];
    });
}

fn sample_color_grid(color: &ColorGrid, spec: &GridSpec, position: Vec3) -> [f32; 3] {
    let max_x = (spec.nx.saturating_sub(1)) as f32;
    let max_y = (spec.ny.saturating_sub(1)) as f32;
    let max_z = (spec.nz.saturating_sub(1)) as f32;
    let fx = ((position.x - spec.min.x) / spec.dx).clamp(0.0, max_x);
    let fy = ((position.y - spec.min.y) / spec.dx).clamp(0.0, max_y);
    let fz = ((position.z - spec.min.z) / spec.dx).clamp(0.0, max_z);

    let ix0 = fx.floor() as usize;
    let iy0 = fy.floor() as usize;
    let iz0 = fz.floor() as usize;
    let ix1 = (ix0 + 1).min(spec.nx - 1);
    let iy1 = (iy0 + 1).min(spec.ny - 1);
    let iz1 = (iz0 + 1).min(spec.nz - 1);
    let tx = fx - ix0 as f32;
    let ty = fy - iy0 as f32;
    let tz = fz - iz0 as f32;

    let idx000 = grid_index(spec, ix0, iy0, iz0);
    let idx100 = grid_index(spec, ix1, iy0, iz0);
    let idx010 = grid_index(spec, ix0, iy1, iz0);
    let idx110 = grid_index(spec, ix1, iy1, iz0);
    let idx001 = grid_index(spec, ix0, iy0, iz1);
    let idx101 = grid_index(spec, ix1, iy0, iz1);
    let idx011 = grid_index(spec, ix0, iy1, iz1);
    let idx111 = grid_index(spec, ix1, iy1, iz1);

    let c000 = Vec3::from(color.sum[idx000]);
    let c100 = Vec3::from(color.sum[idx100]);
    let c010 = Vec3::from(color.sum[idx010]);
    let c110 = Vec3::from(color.sum[idx110]);
    let c001 = Vec3::from(color.sum[idx001]);
    let c101 = Vec3::from(color.sum[idx101]);
    let c011 = Vec3::from(color.sum[idx011]);
    let c111 = Vec3::from(color.sum[idx111]);

    let c00 = c000.lerp(c100, tx);
    let c10 = c010.lerp(c110, tx);
    let c01 = c001.lerp(c101, tx);
    let c11 = c011.lerp(c111, tx);
    let c0 = c00.lerp(c10, ty);
    let c1 = c01.lerp(c11, ty);
    let sum = c0.lerp(c1, tz);

    let w000 = color.weight[idx000];
    let w100 = color.weight[idx100];
    let w010 = color.weight[idx010];
    let w110 = color.weight[idx110];
    let w001 = color.weight[idx001];
    let w101 = color.weight[idx101];
    let w011 = color.weight[idx011];
    let w111 = color.weight[idx111];
    let w00 = w000 + (w100 - w000) * tx;
    let w10 = w010 + (w110 - w010) * tx;
    let w01 = w001 + (w101 - w001) * tx;
    let w11 = w011 + (w111 - w011) * tx;
    let w0 = w00 + (w10 - w00) * ty;
    let w1 = w01 + (w11 - w01) * ty;
    let weight = w0 + (w1 - w0) * tz;

    if weight > 1.0e-6 {
        (sum / weight).to_array()
    } else {
        [1.0, 1.0, 1.0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{NodeParams, ParamValue};
    use crate::splat::SplatGeo;
    use crate::volume::VolumeKind;
    use std::collections::BTreeMap;

    #[test]
    fn marching_cubes_extracts_surface() {
        let spec = GridSpec {
            min: Vec3::new(-1.0, -1.0, -1.0),
            dx: 1.0,
            nx: 3,
            ny: 3,
            nz: 3,
        };
        let mut grid = vec![0.0f32; spec.nx * spec.ny * spec.nz];
        for iz in 0..spec.nz {
            for iy in 0..spec.ny {
                for ix in 0..spec.nx {
                    let pos = Vec3::new(
                        spec.min.x + ix as f32 * spec.dx,
                        spec.min.y + iy as f32 * spec.dx,
                        spec.min.z + iz as f32 * spec.dx,
                    );
                    let d = pos.length() - 1.0;
                    grid[grid_index(&spec, ix, iy, iz)] = d;
                }
            }
        }

        let mesh = marching_cubes(&grid, &spec, 0.0, false).expect("mesh");
        assert!(!mesh.indices.is_empty());
        assert!(!mesh.positions.is_empty());
    }

    #[test]
    fn splat_to_sdf_outputs_volume() {
        let splats = SplatGeo::with_len(1);
        let params = NodeParams {
            values: BTreeMap::from([("output".to_string(), ParamValue::Int(1))]),
        };
        let volume = splats_to_sdf(&params, &splats).expect("sdf volume");
        assert_eq!(volume.kind, VolumeKind::Sdf);
        assert!(!volume.values.is_empty());
    }
}
