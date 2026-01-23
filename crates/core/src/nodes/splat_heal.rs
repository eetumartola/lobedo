use std::collections::BTreeMap;

use glam::Vec3;

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::splat_to_mesh::{build_splat_grid, GridSpec, SplatOutputMode};
use crate::nodes::splat_utils::{split_splats_by_group, SpatialHash};
use crate::nodes::{geometry_in, geometry_out, require_mesh_input};
use crate::parallel;
use crate::param_spec::ParamSpec;
use crate::splat::SplatGeo;

pub const NAME: &str = "Splat Heal";

const DEFAULT_METHOD: i32 = 0;
const DEFAULT_VOXEL_SIZE: f32 = 0.1;
const DEFAULT_MAX_VOXEL_DIM: i32 = 256;
const DEFAULT_N_SIGMA: f32 = 3.0;
const DEFAULT_DENSITY_ISO: f32 = 0.5;
const DEFAULT_BOUNDS_PADDING: f32 = 3.0;
const DEFAULT_CLOSE_RADIUS: i32 = 1;
const DEFAULT_FILL_STRIDE: i32 = 1;
const DEFAULT_MAX_NEW: i32 = 5000;
const DEFAULT_SDF_BAND: f32 = 0.0;
const DEFAULT_SDF_CLOSE: f32 = 0.0;
const DEFAULT_SEARCH_RADIUS: f32 = 0.0;
const DEFAULT_MIN_DISTANCE: f32 = 0.0;
const DEFAULT_SCALE_MUL: f32 = 1.0;
const DEFAULT_OPACITY_MUL: f32 = 1.0;
const DEFAULT_COPY_SH: bool = true;
const DEFAULT_MAX_M2: f32 = 3.0;
const DEFAULT_SMOOTH_K: f32 = 0.1;
const DEFAULT_SHELL_RADIUS: f32 = 1.0;
const DEFAULT_BLUR_ITERS: i32 = 1;
const DEFAULT_HEAL_SHAPE: &str = "all";
const DEFAULT_HEAL_CENTER: [f32; 3] = [0.0, 0.0, 0.0];
const DEFAULT_HEAL_SIZE: [f32; 3] = [1.0, 1.0, 1.0];
const DEFAULT_PREVIEW_SURFACE: bool = false;

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
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
            (
                "heal_shape".to_string(),
                ParamValue::String(DEFAULT_HEAL_SHAPE.to_string()),
            ),
            ("heal_center".to_string(), ParamValue::Vec3(DEFAULT_HEAL_CENTER)),
            ("heal_size".to_string(), ParamValue::Vec3(DEFAULT_HEAL_SIZE)),
            ("method".to_string(), ParamValue::Int(DEFAULT_METHOD)),
            (
                "preview_surface".to_string(),
                ParamValue::Bool(DEFAULT_PREVIEW_SURFACE),
            ),
            ("voxel_size".to_string(), ParamValue::Float(DEFAULT_VOXEL_SIZE)),
            (
                "voxel_size_max".to_string(),
                ParamValue::Int(DEFAULT_MAX_VOXEL_DIM),
            ),
            ("n_sigma".to_string(), ParamValue::Float(DEFAULT_N_SIGMA)),
            ("density_iso".to_string(), ParamValue::Float(DEFAULT_DENSITY_ISO)),
            (
                "bounds_padding".to_string(),
                ParamValue::Float(DEFAULT_BOUNDS_PADDING),
            ),
            ("close_radius".to_string(), ParamValue::Int(DEFAULT_CLOSE_RADIUS)),
            ("fill_stride".to_string(), ParamValue::Int(DEFAULT_FILL_STRIDE)),
            ("max_new".to_string(), ParamValue::Int(DEFAULT_MAX_NEW)),
            ("sdf_band".to_string(), ParamValue::Float(DEFAULT_SDF_BAND)),
            ("sdf_close".to_string(), ParamValue::Float(DEFAULT_SDF_CLOSE)),
            (
                "search_radius".to_string(),
                ParamValue::Float(DEFAULT_SEARCH_RADIUS),
            ),
            (
                "min_distance".to_string(),
                ParamValue::Float(DEFAULT_MIN_DISTANCE),
            ),
            ("scale_mul".to_string(), ParamValue::Float(DEFAULT_SCALE_MUL)),
            (
                "opacity_mul".to_string(),
                ParamValue::Float(DEFAULT_OPACITY_MUL),
            ),
            ("copy_sh".to_string(), ParamValue::Bool(DEFAULT_COPY_SH)),
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
        ParamSpec::string("group", "Group")
            .with_help("Optional group to restrict healing."),
        ParamSpec::int_enum(
            "group_type",
            "Group Type",
            vec![
                (0, "Auto"),
                (1, "Vertex"),
                (2, "Point"),
                (3, "Primitive"),
            ],
        )
        .with_help("Group domain to use."),
        ParamSpec::string_enum(
            "heal_shape",
            "Heal Bounds",
            vec![("all", "All"), ("box", "Box"), ("sphere", "Sphere")],
        )
        .with_help("Heal bounds: All, Box, or Sphere."),
        ParamSpec::vec3("heal_center", "Heal Center")
            .with_help("Heal bounds center.")
            .hidden(),
        ParamSpec::vec3("heal_size", "Heal Size")
            .with_help("Heal bounds size.")
            .hidden(),
        ParamSpec::int_enum(
            "method",
            "Method",
            vec![(0, "Voxel Close"), (1, "SDF Patch")],
        )
        .with_help("Healing method to apply."),
        ParamSpec::bool("preview_surface", "Preview Surface")
            .with_help("Preview the healed surface as a mesh in the viewport."),
        ParamSpec::float_slider("voxel_size", "Voxel Size", 0.0, 10.0)
            .with_help("Voxel size for the density grid."),
        ParamSpec::int_slider("voxel_size_max", "Max Voxel Dim", 8, 2048)
            .with_help("Max voxel dimension (safety clamp)."),
        ParamSpec::float_slider("n_sigma", "Support Sigma", 0.0, 6.0)
            .with_help("Gaussian support radius in sigmas."),
        ParamSpec::float_slider("density_iso", "Density Threshold", 0.0, 10.0)
            .with_help("Density threshold for occupancy."),
        ParamSpec::float_slider("bounds_padding", "Bounds Padding", 0.0, 10.0)
            .with_help("Padding around bounds in sigmas."),
        ParamSpec::int_slider("close_radius", "Close Radius", 0, 6)
            .with_help("Closing radius in voxels.")
            .visible_when_int("method", 0),
        ParamSpec::int_slider("fill_stride", "Fill Stride", 1, 8)
            .with_help("Subsample candidates (higher = fewer splats)."),
        ParamSpec::int_slider("max_new", "Max New", 0, 100_000)
            .with_help("Maximum number of new splats."),
        ParamSpec::float_slider("sdf_band", "SDF Band", 0.0, 5.0)
            .with_help("SDF band thickness around the surface.")
            .visible_when_int("method", 1),
        ParamSpec::float_slider("sdf_close", "SDF Close", -2.0, 2.0)
            .with_help("SDF offset to close small gaps.")
            .visible_when_int("method", 1),
        ParamSpec::float_slider("search_radius", "Search Radius", 0.0, 10.0).with_help(
            "Neighbor search radius for copying attributes (<=0 = auto).",
        ),
        ParamSpec::float_slider("min_distance", "Min Distance", 0.0, 10.0).with_help(
            "Minimum distance to existing splats (<=0 = auto).",
        ),
        ParamSpec::float_slider("scale_mul", "Scale Mult", 0.1, 10.0)
            .with_help("Scale multiplier for new splats."),
        ParamSpec::float_slider("opacity_mul", "Opacity Mult", 0.0, 2.0)
            .with_help("Opacity multiplier for new splats."),
        ParamSpec::bool("copy_sh", "Copy SH")
            .with_help("Copy full SH coefficients (else DC only)."),
        ParamSpec::float_slider("max_m2", "Exponent Clamp", 0.0, 10.0)
            .with_help("Exponent clamp for SDF method."),
        ParamSpec::float_slider("smooth_k", "Blend Sharpness", 0.001, 2.0)
            .with_help("Smooth-min blend sharpness for SDF method.")
            .visible_when_int("method", 1),
        ParamSpec::float_slider("shell_radius", "Shell Radius", 0.1, 4.0)
            .with_help("Ellipsoid shell radius for SDF method.")
            .visible_when_int("method", 1),
        ParamSpec::int_slider("blur_iters", "Density Blur", 0, 6)
            .with_help("Density blur iterations."),
    ]
}

pub fn compute(_params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let input = require_mesh_input(inputs, 0, "Splat Heal requires a mesh input")?;
    Ok(input)
}

pub fn apply_to_geometry(
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };

    let mut meshes = Vec::new();
    if let Some(mesh) = input.merged_mesh() {
        meshes.push(mesh);
    }
    let mut splats = Vec::with_capacity(input.splats.len());
    for splat in &input.splats {
        splats.push(apply_to_splats(params, splat)?);
    }

    if params.get_bool("preview_surface", DEFAULT_PREVIEW_SURFACE) {
        if let Some(splat) = input.merged_splats() {
            if let Ok(Some(preview)) = build_preview_surface(params, &splat) {
                meshes.push(preview);
            }
        }
    }

    let curves = if meshes.is_empty() {
        Vec::new()
    } else {
        input.curves.clone()
    };
    Ok(Geometry {
        meshes,
        splats,
        curves,
        volumes: input.volumes.clone(),
        materials: input.materials.clone(),
    })
}

pub fn apply_to_splats(params: &NodeParams, splats: &SplatGeo) -> Result<SplatGeo, String> {
    if splats.is_empty() {
        return Ok(splats.clone());
    }

    let Some((selected, _unselected)) =
        split_splats_by_group(splats, params, AttributeDomain::Point)
    else {
        return Ok(splats.clone());
    };

    let source = if selected.len() == splats.len() {
        splats.clone()
    } else {
        splats.filter_by_indices(&selected)
    };

    let method = params.get_int("method", DEFAULT_METHOD).clamp(0, 1);
    let new_splats = match method {
        1 => heal_sdf_patch(params, &source)?,
        _ => heal_voxel_close(params, &source)?,
    };
    if new_splats.is_empty() {
        return Ok(splats.clone());
    }

    let mut output = splats.clone();
    let scale_mul = params.get_float("scale_mul", DEFAULT_SCALE_MUL);
    let opacity_mul = params.get_float("opacity_mul", DEFAULT_OPACITY_MUL);
    let copy_sh = params.get_bool("copy_sh", DEFAULT_COPY_SH);
    append_new_splats(
        &mut output,
        &source,
        &new_splats,
        scale_mul,
        opacity_mul,
        copy_sh,
    );
    Ok(output)
}

#[derive(Clone, Copy)]
struct NewSplat {
    position: [f32; 3],
    source_index: usize,
}

fn heal_voxel_close(params: &NodeParams, source: &SplatGeo) -> Result<Vec<NewSplat>, String> {
    let grid = build_density_grid(params, source)?;
    if grid.values.is_empty() {
        return Ok(Vec::new());
    }
    let occ = occupancy_from_grid(&grid.values, grid.iso, grid.inside_is_greater);
    let close_radius = params.get_int("close_radius", DEFAULT_CLOSE_RADIUS);
    let closed = close_occupancy(&occ, &grid.spec, close_radius);
    let mut candidates = vec![0u8; occ.len()];
    parallel::for_each_indexed_mut(&mut candidates, |idx, slot| {
        let filled = closed.get(idx).copied().unwrap_or(0) != 0;
        let was_inside = occ.get(idx).copied().unwrap_or(0) != 0;
        if filled && !was_inside && is_surface_voxel(&closed, &grid.spec, idx) {
            *slot = 1;
        } else {
            *slot = 0;
        }
    });
    let closed_values: Vec<f32> = closed
        .iter()
        .map(|value| if *value != 0 { 1.0 } else { 0.0 })
        .collect();
    let surface = SurfaceGrid {
        values: &closed_values,
        iso: 0.5,
        inside_is_greater: true,
    };
    Ok(collect_new_splats(
        params,
        source,
        &grid.spec,
        &candidates,
        Some(&surface),
    ))
}

fn heal_sdf_patch(params: &NodeParams, source: &SplatGeo) -> Result<Vec<NewSplat>, String> {
    let density = build_density_grid(params, source)?;
    if density.values.is_empty() {
        return Ok(Vec::new());
    }
    let sdf = build_sdf_grid(params, source)?;
    if !grid_spec_matches(&density.spec, &sdf.spec) {
        return Err("Splat Heal: density/SDF grids have mismatched resolution".to_string());
    }

    let mut candidates = vec![0u8; density.values.len()];
    let mut band = params.get_float("sdf_band", DEFAULT_SDF_BAND);
    if band <= 0.0 {
        band = density.spec.dx * 1.5;
    }
    let sdf_close = params.get_float("sdf_close", DEFAULT_SDF_CLOSE);
    parallel::for_each_indexed_mut(&mut candidates, |idx, slot| {
        let density_val = density.values.get(idx).copied().unwrap_or(0.0);
        let inside_density = if density.inside_is_greater {
            density_val >= density.iso
        } else {
            density_val <= density.iso
        };
        if inside_density {
            *slot = 0;
            return;
        }
        let mut sdf_val = sdf.values.get(idx).copied().unwrap_or(f32::INFINITY);
        if !sdf_val.is_finite() {
            *slot = 0;
            return;
        }
        sdf_val -= sdf_close;
        *slot = if sdf_val.abs() <= band { 1 } else { 0 };
    });
    let surface = SurfaceGrid {
        values: &sdf.values,
        iso: sdf.iso,
        inside_is_greater: sdf.inside_is_greater,
    };
    Ok(collect_new_splats(
        params,
        source,
        &density.spec,
        &candidates,
        Some(&surface),
    ))
}

fn build_preview_surface(
    params: &NodeParams,
    splats: &SplatGeo,
) -> Result<Option<Mesh>, String> {
    if splats.is_empty() {
        return Ok(None);
    }
    let method = params.get_int("method", DEFAULT_METHOD).clamp(0, 1);
    match method {
        1 => {
            let sdf = build_sdf_grid(params, splats)?;
            if sdf.values.is_empty() {
                return Ok(None);
            }
            let mesh = crate::nodes::splat_to_mesh::marching_cubes(
                &sdf.values,
                &sdf.spec,
                sdf.iso,
                sdf.inside_is_greater,
            )?;
            Ok(Some(mesh))
        }
        _ => {
            let density = build_density_grid(params, splats)?;
            if density.values.is_empty() {
                return Ok(None);
            }
            let occ = occupancy_from_grid(&density.values, density.iso, density.inside_is_greater);
            let close_radius = params.get_int("close_radius", DEFAULT_CLOSE_RADIUS);
            let closed = close_occupancy(&occ, &density.spec, close_radius);
            let values: Vec<f32> = closed
                .iter()
                .map(|value| if *value != 0 { 1.0 } else { 0.0 })
                .collect();
            let mesh = crate::nodes::splat_to_mesh::marching_cubes(
                &values,
                &density.spec,
                0.5,
                true,
            )?;
            Ok(Some(mesh))
        }
    }
}

fn build_density_grid(params: &NodeParams, source: &SplatGeo) -> Result<crate::nodes::splat_to_mesh::SplatGrid, String> {
    let grid_params = grid_params_from(params, 0);
    build_splat_grid(&grid_params, source, SplatOutputMode::Mesh)
}

fn build_sdf_grid(params: &NodeParams, source: &SplatGeo) -> Result<crate::nodes::splat_to_mesh::SplatGrid, String> {
    let grid_params = grid_params_from(params, 1);
    build_splat_grid(&grid_params, source, SplatOutputMode::Sdf)
}

fn grid_params_from(params: &NodeParams, algorithm: i32) -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            ("algorithm".to_string(), ParamValue::Int(algorithm)),
            (
                "voxel_size".to_string(),
                ParamValue::Float(params.get_float("voxel_size", DEFAULT_VOXEL_SIZE)),
            ),
            (
                "voxel_size_max".to_string(),
                ParamValue::Int(params.get_int("voxel_size_max", DEFAULT_MAX_VOXEL_DIM)),
            ),
            (
                "n_sigma".to_string(),
                ParamValue::Float(params.get_float("n_sigma", DEFAULT_N_SIGMA)),
            ),
            (
                "density_iso".to_string(),
                ParamValue::Float(params.get_float("density_iso", DEFAULT_DENSITY_ISO)),
            ),
            ("surface_iso".to_string(), ParamValue::Float(0.0)),
            (
                "bounds_padding".to_string(),
                ParamValue::Float(params.get_float("bounds_padding", DEFAULT_BOUNDS_PADDING)),
            ),
            ("transfer_color".to_string(), ParamValue::Bool(false)),
            (
                "max_m2".to_string(),
                ParamValue::Float(params.get_float("max_m2", DEFAULT_MAX_M2)),
            ),
            (
                "smooth_k".to_string(),
                ParamValue::Float(params.get_float("smooth_k", DEFAULT_SMOOTH_K)),
            ),
            (
                "shell_radius".to_string(),
                ParamValue::Float(params.get_float("shell_radius", DEFAULT_SHELL_RADIUS)),
            ),
            (
                "blur_iters".to_string(),
                ParamValue::Int(params.get_int("blur_iters", DEFAULT_BLUR_ITERS)),
            ),
        ]),
    }
}

fn grid_spec_matches(a: &GridSpec, b: &GridSpec) -> bool {
    (a.nx == b.nx)
        && (a.ny == b.ny)
        && (a.nz == b.nz)
        && (a.min - b.min).length() < 1.0e-4
        && (a.dx - b.dx).abs() < 1.0e-6
}

fn occupancy_from_grid(values: &[f32], iso: f32, inside_is_greater: bool) -> Vec<u8> {
    let mut occ = vec![0u8; values.len()];
    parallel::for_each_indexed_mut(&mut occ, |idx, slot| {
        let value = values.get(idx).copied().unwrap_or(0.0);
        let inside = if inside_is_greater { value >= iso } else { value <= iso };
        *slot = if inside { 1 } else { 0 };
    });
    occ
}

fn close_occupancy(occ: &[u8], spec: &GridSpec, radius: i32) -> Vec<u8> {
    if radius <= 0 || occ.is_empty() {
        return occ.to_vec();
    }
    let dilated = dilate_occupancy(occ, spec, radius);
    erode_occupancy(&dilated, spec, radius)
}

fn dilate_occupancy(occ: &[u8], spec: &GridSpec, radius: i32) -> Vec<u8> {
    let mut out = vec![0u8; occ.len()];
    let nx = spec.nx as isize;
    let ny = spec.ny as isize;
    let nz = spec.nz as isize;
    let r = radius as isize;
    let slice = (spec.nx * spec.ny) as isize;
    parallel::for_each_indexed_mut(&mut out, |idx, value| {
        if occ.get(idx).copied().unwrap_or(0) != 0 {
            *value = 1;
            return;
        }
        let idx = idx as isize;
        let iz = idx / slice;
        let rem = idx - iz * slice;
        let iy = rem / spec.nx as isize;
        let ix = rem - iy * spec.nx as isize;
        let mut found = false;
        'outer: for dz in -r..=r {
            let z = iz + dz;
            if z < 0 || z >= nz {
                continue;
            }
            for dy in -r..=r {
                let y = iy + dy;
                if y < 0 || y >= ny {
                    continue;
                }
                for dx in -r..=r {
                    let x = ix + dx;
                    if x < 0 || x >= nx {
                        continue;
                    }
                    let nidx = (x + spec.nx as isize * (y + spec.ny as isize * z)) as usize;
                    if occ.get(nidx).copied().unwrap_or(0) != 0 {
                        found = true;
                        break 'outer;
                    }
                }
            }
        }
        *value = if found { 1 } else { 0 };
    });
    out
}

fn erode_occupancy(occ: &[u8], spec: &GridSpec, radius: i32) -> Vec<u8> {
    let mut out = vec![0u8; occ.len()];
    let nx = spec.nx as isize;
    let ny = spec.ny as isize;
    let nz = spec.nz as isize;
    let r = radius as isize;
    let slice = (spec.nx * spec.ny) as isize;
    parallel::for_each_indexed_mut(&mut out, |idx, value| {
        if occ.get(idx).copied().unwrap_or(0) == 0 {
            *value = 0;
            return;
        }
        let idx = idx as isize;
        let iz = idx / slice;
        let rem = idx - iz * slice;
        let iy = rem / spec.nx as isize;
        let ix = rem - iy * spec.nx as isize;
        let mut keep = true;
        'outer: for dz in -r..=r {
            let z = iz + dz;
            if z < 0 || z >= nz {
                keep = false;
                break;
            }
            for dy in -r..=r {
                let y = iy + dy;
                if y < 0 || y >= ny {
                    keep = false;
                    break 'outer;
                }
                for dx in -r..=r {
                    let x = ix + dx;
                    if x < 0 || x >= nx {
                        keep = false;
                        break 'outer;
                    }
                    let nidx = (x + spec.nx as isize * (y + spec.ny as isize * z)) as usize;
                    if occ.get(nidx).copied().unwrap_or(0) == 0 {
                        keep = false;
                        break 'outer;
                    }
                }
            }
        }
        *value = if keep { 1 } else { 0 };
    });
    out
}

fn collect_new_splats(
    params: &NodeParams,
    source: &SplatGeo,
    spec: &GridSpec,
    candidates: &[u8],
    surface: Option<&SurfaceGrid<'_>>,
) -> Vec<NewSplat> {
    let max_new = params.get_int("max_new", DEFAULT_MAX_NEW).max(0) as usize;
    if max_new == 0 || candidates.is_empty() {
        return Vec::new();
    }
    let fill_stride = params.get_int("fill_stride", DEFAULT_FILL_STRIDE).max(1) as usize;
    let mut search_radius = params.get_float("search_radius", DEFAULT_SEARCH_RADIUS);
    if search_radius <= 0.0 {
        search_radius = spec.dx * 2.0;
    }
    if !search_radius.is_finite() || search_radius <= 1.0e-6 {
        return Vec::new();
    }
    let mut min_distance = params.get_float("min_distance", DEFAULT_MIN_DISTANCE);
    if min_distance <= 0.0 {
        min_distance = spec.dx * 0.4;
    }

    let Some(hash) = SpatialHash::build(&source.positions, search_radius) else {
        return Vec::new();
    };
    let mut new_splats = Vec::new();
    let nx = spec.nx;
    let ny = spec.ny;
    let slice = nx * ny;
    for (idx, flag) in candidates.iter().enumerate() {
        if *flag == 0 {
            continue;
        }
        let iz = idx / slice;
        let rem = idx - iz * slice;
        let iy = rem / nx;
        let ix = rem - iy * nx;
        if fill_stride > 1 && !(ix + iy + iz).is_multiple_of(fill_stride) {
            continue;
        }
        let mut pos_grid = Vec3::new(
            spec.min.x + ix as f32 * spec.dx,
            spec.min.y + iy as f32 * spec.dx,
            spec.min.z + iz as f32 * spec.dx,
        );
        if let Some(surface) = surface {
            pos_grid = project_to_surface(surface, spec, ix, iy, iz, pos_grid);
        }
        let pos_world = pos_grid;
        if !heal_bounds_contains(params, pos_world) {
            continue;
        }
        if let Some((hit_idx, dist)) = hash.nearest(&source.positions, pos_world, search_radius)
        {
            if min_distance > 0.0 && dist < min_distance {
                continue;
            }
            new_splats.push(NewSplat {
                position: pos_world.to_array(),
                source_index: hit_idx,
            });
            if new_splats.len() >= max_new {
                break;
            }
        }
    }
    new_splats
}

struct SurfaceGrid<'a> {
    values: &'a [f32],
    iso: f32,
    inside_is_greater: bool,
}

fn project_to_surface(
    surface: &SurfaceGrid<'_>,
    spec: &GridSpec,
    ix: usize,
    iy: usize,
    iz: usize,
    pos: Vec3,
) -> Vec3 {
    let idx = grid_index(spec, ix, iy, iz);
    let value = surface.values.get(idx).copied().unwrap_or(surface.iso);
    let signed = if surface.inside_is_greater {
        value - surface.iso
    } else {
        surface.iso - value
    };
    let grad = grid_gradient(surface.values, spec, ix, iy, iz);
    let grad_len2 = grad.length_squared();
    if grad_len2 > 1.0e-8 && signed.is_finite() {
        pos - grad * (signed / grad_len2)
    } else {
        pos
    }
}

fn grid_index(spec: &GridSpec, ix: usize, iy: usize, iz: usize) -> usize {
    ix + spec.nx * (iy + spec.ny * iz)
}

fn grid_sample(values: &[f32], spec: &GridSpec, ix: isize, iy: isize, iz: isize) -> f32 {
    let x = ix.clamp(0, spec.nx.saturating_sub(1) as isize) as usize;
    let y = iy.clamp(0, spec.ny.saturating_sub(1) as isize) as usize;
    let z = iz.clamp(0, spec.nz.saturating_sub(1) as isize) as usize;
    let idx = grid_index(spec, x, y, z);
    values.get(idx).copied().unwrap_or(0.0)
}

fn grid_gradient(values: &[f32], spec: &GridSpec, ix: usize, iy: usize, iz: usize) -> Vec3 {
    let ix = ix as isize;
    let iy = iy as isize;
    let iz = iz as isize;
    let dx = spec.dx.max(1.0e-6);
    let fx1 = grid_sample(values, spec, ix + 1, iy, iz);
    let fx0 = grid_sample(values, spec, ix - 1, iy, iz);
    let fy1 = grid_sample(values, spec, ix, iy + 1, iz);
    let fy0 = grid_sample(values, spec, ix, iy - 1, iz);
    let fz1 = grid_sample(values, spec, ix, iy, iz + 1);
    let fz0 = grid_sample(values, spec, ix, iy, iz - 1);
    Vec3::new(fx1 - fx0, fy1 - fy0, fz1 - fz0) * (0.5 / dx)
}

fn is_surface_voxel(filled: &[u8], spec: &GridSpec, idx: usize) -> bool {
    if filled.get(idx).copied().unwrap_or(0) == 0 {
        return false;
    }
    let nx = spec.nx as isize;
    let ny = spec.ny as isize;
    let nz = spec.nz as isize;
    let slice = (spec.nx * spec.ny) as isize;
    let idx = idx as isize;
    let iz = idx / slice;
    let rem = idx - iz * slice;
    let iy = rem / spec.nx as isize;
    let ix = rem - iy * spec.nx as isize;
    for (dx, dy, dz) in [
        (-1, 0, 0),
        (1, 0, 0),
        (0, -1, 0),
        (0, 1, 0),
        (0, 0, -1),
        (0, 0, 1),
    ] {
        let x = ix + dx;
        let y = iy + dy;
        let z = iz + dz;
        if x < 0 || y < 0 || z < 0 || x >= nx || y >= ny || z >= nz {
            return true;
        }
        let nidx = (x + nx * (y + ny * z)) as usize;
        if filled.get(nidx).copied().unwrap_or(0) == 0 {
            return true;
        }
    }
    false
}

fn heal_bounds_contains(params: &NodeParams, position: Vec3) -> bool {
    let shape = params
        .get_string("heal_shape", DEFAULT_HEAL_SHAPE)
        .to_lowercase();
    match shape.as_str() {
        "box" => {
            let center = Vec3::from(params.get_vec3("heal_center", DEFAULT_HEAL_CENTER));
            let size = Vec3::from(params.get_vec3("heal_size", DEFAULT_HEAL_SIZE)).abs();
            let half = size * 0.5;
            let delta = (position - center).abs();
            delta.x <= half.x && delta.y <= half.y && delta.z <= half.z
        }
        "sphere" => {
            let center = Vec3::from(params.get_vec3("heal_center", DEFAULT_HEAL_CENTER));
            let size = Vec3::from(params.get_vec3("heal_size", DEFAULT_HEAL_SIZE)).abs();
            let radius = 0.5 * size.max_element();
            (position - center).length_squared() <= radius * radius
        }
        _ => true,
    }
}

fn append_new_splats(
    output: &mut SplatGeo,
    source: &SplatGeo,
    new_splats: &[NewSplat],
    scale_mul: f32,
    opacity_mul: f32,
    copy_sh: bool,
) {
    if new_splats.is_empty() {
        return;
    }
    let max_coeffs = output.sh_coeffs.max(source.sh_coeffs);
    if output.sh_coeffs != max_coeffs {
        let old_coeffs = output.sh_coeffs;
        let old_len = output.len();
        let mut upgraded = Vec::with_capacity(old_len * max_coeffs);
        if old_coeffs == 0 {
            upgraded.extend(std::iter::repeat_n([0.0, 0.0, 0.0], old_len * max_coeffs));
        } else {
            for i in 0..old_len {
                let base = i * old_coeffs;
                for c in 0..max_coeffs {
                    let value = if c < old_coeffs {
                        output.sh_rest[base + c]
                    } else {
                        [0.0, 0.0, 0.0]
                    };
                    upgraded.push(value);
                }
            }
        }
        output.sh_rest = upgraded;
        output.sh_coeffs = max_coeffs;
    }

    let scale_mul = if scale_mul.is_finite() { scale_mul.max(1.0e-6) } else { 1.0 };
    let log_scale_offset = scale_mul.ln();
    let opacity_mul = if opacity_mul.is_finite() { opacity_mul.max(0.0) } else { 1.0 };

    for new_splat in new_splats {
        let src = new_splat.source_index;
        output.positions.push(new_splat.position);
        output
            .rotations
            .push(source.rotations.get(src).copied().unwrap_or([0.0, 0.0, 0.0, 1.0]));

        let log_scale = source.scales.get(src).copied().unwrap_or([0.0, 0.0, 0.0]);
        let mut log_scale = Vec3::from(log_scale) + Vec3::splat(log_scale_offset);
        log_scale = Vec3::new(
            log_scale.x.clamp(-10.0, 10.0),
            log_scale.y.clamp(-10.0, 10.0),
            log_scale.z.clamp(-10.0, 10.0),
        );
        output.scales.push([log_scale.x, log_scale.y, log_scale.z]);

        let base_opacity = source.opacity.get(src).copied().unwrap_or(0.0);
        let alpha = sigmoid(base_opacity) * opacity_mul;
        output.opacity.push(logit(alpha));

        output
            .sh0
            .push(source.sh0.get(src).copied().unwrap_or([1.0, 1.0, 1.0]));
        if max_coeffs > 0 {
            if copy_sh && source.sh_coeffs > 0 {
                let base = src * source.sh_coeffs;
                for c in 0..max_coeffs {
                    let value = if c < source.sh_coeffs {
                        source.sh_rest[base + c]
                    } else {
                        [0.0, 0.0, 0.0]
                    };
                    output.sh_rest.push(value);
                }
            } else {
                output
                    .sh_rest
                    .extend(std::iter::repeat_n([0.0, 0.0, 0.0], max_coeffs));
            }
        }
    }

    let mapping: Vec<usize> = new_splats.iter().map(|s| s.source_index).collect();
    append_attributes_from_source(output, source, &mapping);
    append_groups_from_source(output, source, &mapping);
}

fn append_attributes_from_source(output: &mut SplatGeo, source: &SplatGeo, mapping: &[usize]) {
    if mapping.is_empty() {
        return;
    }
    for domain in [AttributeDomain::Point, AttributeDomain::Primitive] {
        let source_map = source.attributes.map(domain);
        for (name, storage) in output.attributes.map_mut(domain) {
            let source_storage = source_map.get(name);
            append_attribute_storage(storage, source_storage, mapping);
        }
    }
}

fn append_attribute_storage(
    output: &mut AttributeStorage,
    source: Option<&AttributeStorage>,
    mapping: &[usize],
) {
    match source {
        Some(AttributeStorage::Float(src)) => {
            if let AttributeStorage::Float(out) = output {
                for &idx in mapping {
                    out.push(src.get(idx).copied().unwrap_or(0.0));
                }
            } else {
                append_attribute_defaults(output, mapping.len());
            }
        }
        Some(AttributeStorage::Int(src)) => {
            if let AttributeStorage::Int(out) = output {
                for &idx in mapping {
                    out.push(src.get(idx).copied().unwrap_or(0));
                }
            } else {
                append_attribute_defaults(output, mapping.len());
            }
        }
        Some(AttributeStorage::Vec2(src)) => {
            if let AttributeStorage::Vec2(out) = output {
                for &idx in mapping {
                    out.push(src.get(idx).copied().unwrap_or([0.0, 0.0]));
                }
            } else {
                append_attribute_defaults(output, mapping.len());
            }
        }
        Some(AttributeStorage::Vec3(src)) => {
            if let AttributeStorage::Vec3(out) = output {
                for &idx in mapping {
                    out.push(src.get(idx).copied().unwrap_or([0.0, 0.0, 0.0]));
                }
            } else {
                append_attribute_defaults(output, mapping.len());
            }
        }
        Some(AttributeStorage::Vec4(src)) => {
            if let AttributeStorage::Vec4(out) = output {
                for &idx in mapping {
                    out.push(src.get(idx).copied().unwrap_or([0.0, 0.0, 0.0, 0.0]));
                }
            } else {
                append_attribute_defaults(output, mapping.len());
            }
        }
        Some(AttributeStorage::StringTable(src)) => {
            if let AttributeStorage::StringTable(out) = output {
                if out.values.is_empty() && !src.values.is_empty() {
                    out.values = src.values.clone();
                }
                for &idx in mapping {
                    out.indices.push(src.indices.get(idx).copied().unwrap_or(0));
                }
                if out.values.is_empty() && !out.indices.is_empty() {
                    out.values.push(String::new());
                }
            } else {
                append_attribute_defaults(output, mapping.len());
            }
        }
        None => append_attribute_defaults(output, mapping.len()),
    }
}

fn append_attribute_defaults(storage: &mut AttributeStorage, count: usize) {
    if count == 0 {
        return;
    }
    match storage {
        AttributeStorage::Float(values) => values.extend(std::iter::repeat_n(0.0, count)),
        AttributeStorage::Int(values) => values.extend(std::iter::repeat_n(0, count)),
        AttributeStorage::Vec2(values) => {
            values.extend(std::iter::repeat_n([0.0, 0.0], count));
        }
        AttributeStorage::Vec3(values) => {
            values.extend(std::iter::repeat_n([0.0, 0.0, 0.0], count));
        }
        AttributeStorage::Vec4(values) => {
            values.extend(std::iter::repeat_n([0.0, 0.0, 0.0, 0.0], count));
        }
        AttributeStorage::StringTable(values) => {
            if values.values.is_empty() {
                values.values.push(String::new());
            }
            values.indices.extend(std::iter::repeat_n(0u32, count));
        }
    }
}

fn append_groups_from_source(output: &mut SplatGeo, source: &SplatGeo, mapping: &[usize]) {
    if mapping.is_empty() {
        return;
    }
    for domain in [AttributeDomain::Point, AttributeDomain::Primitive] {
        let source_map = source.groups.map(domain);
        for (name, values) in output.groups.map_mut(domain) {
            if let Some(src_group) = source_map.get(name) {
                for &idx in mapping {
                    values.push(src_group.get(idx).copied().unwrap_or(false));
                }
            } else {
                values.extend(std::iter::repeat_n(false, mapping.len()));
            }
        }
    }
}

fn sigmoid(value: f32) -> f32 {
    1.0 / (1.0 + (-value).exp())
}

fn logit(value: f32) -> f32 {
    let clamped = value.clamp(1.0e-6, 1.0 - 1.0e-6);
    (clamped / (1.0 - clamped)).ln()
}
