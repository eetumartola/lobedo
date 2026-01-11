use std::collections::{BTreeMap, HashMap};

use glam::Vec3;

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::group_utils::{mask_has_any, splat_group_mask};
use crate::nodes::splat_to_mesh::{build_splat_grid, GridSpec, SplatOutputMode};
use crate::nodes::splat_utils::splat_cell_key;
use crate::nodes::{geometry_in, geometry_out, require_mesh_input};
use crate::parallel;
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
            ("method".to_string(), ParamValue::Int(DEFAULT_METHOD)),
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

    let group_mask = splat_group_mask(splats, params, AttributeDomain::Point);
    if !mask_has_any(group_mask.as_deref()) {
        return Ok(splats.clone());
    }

    let mut selected = Vec::new();
    for idx in 0..splats.len() {
        let keep = group_mask
            .as_ref()
            .map(|mask| mask.get(idx).copied().unwrap_or(false))
            .unwrap_or(true);
        if keep {
            selected.push(idx);
        }
    }
    if selected.is_empty() {
        return Ok(splats.clone());
    }

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
        *slot = if filled && !was_inside { 1 } else { 0 };
    });
    Ok(collect_new_splats(params, source, &grid.spec, &candidates))
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
    Ok(collect_new_splats(params, source, &density.spec, &candidates))
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
        let pos_grid = Vec3::new(
            spec.min.x + ix as f32 * spec.dx,
            spec.min.y + iy as f32 * spec.dx,
            spec.min.z + iz as f32 * spec.dx,
        );
        let pos_world = Vec3::new(pos_grid.x, -pos_grid.y, pos_grid.z);
        if let Some((hit_idx, dist)) = hash.nearest(pos_world, &source.positions, search_radius)
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

struct SpatialHash {
    min: Vec3,
    inv_cell: f32,
    cells: HashMap<(i32, i32, i32), Vec<usize>>,
}

impl SpatialHash {
    fn build(positions: &[[f32; 3]], cell_size: f32) -> Option<Self> {
        if positions.is_empty() || !cell_size.is_finite() || cell_size <= 0.0 {
            return None;
        }
        let mut iter = positions.iter();
        let first = Vec3::from(*iter.next()?);
        let mut min = first;
        for pos in iter {
            min = min.min(Vec3::from(*pos));
        }
        let inv_cell = 1.0 / cell_size;
        let mut cells: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();
        for (idx, pos) in positions.iter().enumerate() {
            let key = splat_cell_key(Vec3::from(*pos), min, inv_cell);
            cells.entry(key).or_default().push(idx);
        }
        Some(Self { min, inv_cell, cells })
    }

    fn nearest(
        &self,
        position: Vec3,
        positions: &[[f32; 3]],
        max_dist: f32,
    ) -> Option<(usize, f32)> {
        let max_dist = if max_dist.is_finite() && max_dist > 0.0 {
            max_dist
        } else {
            f32::INFINITY
        };
        let max_dist_sq = max_dist * max_dist;
        let base = splat_cell_key(position, self.min, self.inv_cell);
        let mut best = None;
        let mut best_dist = max_dist_sq;
        for dz in -1..=1 {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let key = (base.0 + dx, base.1 + dy, base.2 + dz);
                    let Some(list) = self.cells.get(&key) else { continue };
                    for &idx in list {
                        let pos = Vec3::from(positions[idx]);
                        let dist_sq = position.distance_squared(pos);
                        if dist_sq < best_dist {
                            best_dist = dist_sq;
                            best = Some(idx);
                        }
                    }
                }
            }
        }
        best.map(|idx| (idx, best_dist.sqrt()))
    }
}

fn sigmoid(value: f32) -> f32 {
    1.0 / (1.0 + (-value).exp())
}

fn logit(value: f32) -> f32 {
    let clamped = value.clamp(1.0e-6, 1.0 - 1.0e-6);
    (clamped / (1.0 - clamped)).ln()
}
