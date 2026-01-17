use std::collections::BTreeMap;

use glam::Vec3;

use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{geometry_in, geometry_out, require_mesh_input};
use crate::nodes::splat_to_mesh::{marching_cubes, sanitize_grid, GridSpec};
use crate::nodes::volume_from_geo;
use crate::parallel;
use crate::volume::{Volume, VolumeKind};
use crate::volume_sampling::VolumeSampler;

pub const NAME: &str = "Boolean SDF";
const DEFAULT_MODE: &str = "auto";
const DEFAULT_OP: i32 = 1;
const DEFAULT_MAX_DIM: i32 = 64;
const DEFAULT_PADDING: f32 = 0.1;
const DEFAULT_SURFACE_ISO: f32 = 0.0;
const DEFAULT_SDF_BAND: f32 = 0.2;
const MAX_GRID_POINTS: u64 = 32_000_000;

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![geometry_in("a"), geometry_in("b")],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            ("mode".to_string(), ParamValue::String(DEFAULT_MODE.to_string())),
            ("op".to_string(), ParamValue::Int(DEFAULT_OP)),
            ("max_dim".to_string(), ParamValue::Int(DEFAULT_MAX_DIM)),
            ("padding".to_string(), ParamValue::Float(DEFAULT_PADDING)),
            (
                "surface_iso".to_string(),
                ParamValue::Float(DEFAULT_SURFACE_ISO),
            ),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mesh_a = require_mesh_input(inputs, 0, "Boolean SDF requires mesh input A")?;
    let mesh_b = require_mesh_input(inputs, 1, "Boolean SDF requires mesh input B")?;
    boolean_mesh_mesh(params, &mesh_a, &mesh_b)
}

pub fn apply_to_geometry(
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    let Some(input_a) = inputs.first() else {
        return Ok(Geometry::default());
    };
    let Some(input_b) = inputs.get(1) else {
        return Err("Boolean SDF requires two inputs".to_string());
    };
    let Some(mesh_a) = input_a.merged_mesh() else {
        return Err("Boolean SDF requires a mesh on input A".to_string());
    };

    let max_dim = params.get_int("max_dim", DEFAULT_MAX_DIM).max(1) as u32;
    let padding = params.get_float("padding", DEFAULT_PADDING).max(0.0);
    let surface_iso = params.get_float("surface_iso", DEFAULT_SURFACE_ISO);
    let op = params.get_int("op", DEFAULT_OP);

    let mode = params.get_string("mode", DEFAULT_MODE).to_lowercase();
    let force_mesh_sdf = mode.contains("sdf");
    let auto = mode.contains("auto");

    let volume_a = mesh_to_sdf(&mesh_a, max_dim, padding)?;
    let mut volume_b: Option<Volume> = None;

    if force_mesh_sdf || auto {
        if let Some(vol) = input_b.volumes.first() {
            if vol.kind == VolumeKind::Sdf {
                volume_b = Some(vol.clone());
            } else if force_mesh_sdf {
                return Err("Boolean SDF requires an SDF volume on input B".to_string());
            }
        } else if force_mesh_sdf {
            return Err("Boolean SDF requires an SDF volume on input B".to_string());
        }
    }

    if volume_b.is_none() {
        let Some(mesh_b) = input_b.merged_mesh() else {
            return Err("Boolean SDF requires a mesh on input B".to_string());
        };
        volume_b = Some(mesh_to_sdf(&mesh_b, max_dim, padding)?);
    }

    let combined = combine_sdf(&volume_a, volume_b.as_ref().unwrap(), op, max_dim)?;
    let mesh = sdf_to_mesh(&combined, surface_iso)?;

    let mut output = input_a.clone();
    output.meshes = if mesh.positions.is_empty() && mesh.indices.is_empty() {
        Vec::new()
    } else {
        vec![mesh]
    };
    Ok(output)
}

fn boolean_mesh_mesh(params: &NodeParams, a: &Mesh, b: &Mesh) -> Result<Mesh, String> {
    let max_dim = params.get_int("max_dim", DEFAULT_MAX_DIM).max(1) as u32;
    let padding = params.get_float("padding", DEFAULT_PADDING).max(0.0);
    let surface_iso = params.get_float("surface_iso", DEFAULT_SURFACE_ISO);
    let op = params.get_int("op", DEFAULT_OP);

    let vol_a = mesh_to_sdf(a, max_dim, padding)?;
    let vol_b = mesh_to_sdf(b, max_dim, padding)?;
    let combined = combine_sdf(&vol_a, &vol_b, op, max_dim)?;
    sdf_to_mesh(&combined, surface_iso)
}

fn mesh_to_sdf(mesh: &Mesh, max_dim: u32, padding: f32) -> Result<Volume, String> {
    let params = NodeParams {
        values: BTreeMap::from([
            ("mode".to_string(), ParamValue::String("sdf".to_string())),
            ("max_dim".to_string(), ParamValue::Int(max_dim as i32)),
            ("padding".to_string(), ParamValue::Float(padding)),
            ("density_scale".to_string(), ParamValue::Float(1.0)),
            ("sdf_band".to_string(), ParamValue::Float(DEFAULT_SDF_BAND)),
        ]),
    };
    let geom = Geometry::with_mesh(mesh.clone());
    let out = volume_from_geo::apply_to_geometry(&params, std::slice::from_ref(&geom))?;
    out.volumes
        .first()
        .cloned()
        .ok_or_else(|| "Boolean SDF failed to generate SDF volume".to_string())
}

fn combine_sdf(a: &Volume, b: &Volume, op: i32, max_dim: u32) -> Result<Volume, String> {
    if a.kind != VolumeKind::Sdf || b.kind != VolumeKind::Sdf {
        return Err("Boolean SDF requires SDF volumes".to_string());
    }

    let (a_min, a_max) = a.world_bounds();
    let (b_min, b_max) = b.world_bounds();
    let mut min = a_min.min(b_min);
    let mut max = a_max.max(b_max);
    if (max - min).length_squared() < 1.0e-8 {
        max += Vec3::splat(1.0e-3);
        min -= Vec3::splat(1.0e-3);
    }

    let size = (max - min).max(Vec3::splat(1.0e-6));
    let max_axis = size.x.max(size.y.max(size.z)).max(1.0e-6);
    let voxel_size = (max_axis / max_dim as f32).max(1.0e-6);
    let dims = dims_from_size(size, voxel_size);

    let total = dims[0] as u64 * dims[1] as u64 * dims[2] as u64;
    if total == 0 || total > MAX_GRID_POINTS {
        return Err(format!(
            "Boolean SDF volume grid too large ({} voxels, max {})",
            total, MAX_GRID_POINTS
        ));
    }

    let sampler_a = VolumeSampler::new(a);
    let sampler_b = VolumeSampler::new(b);
    let mut values = vec![0.0f32; total as usize];
    let dim_x = dims[0] as usize;
    let dim_y = dims[1] as usize;
    let dim_z = dims[2] as usize;
    let stride_xy = dim_x.saturating_mul(dim_y).max(1);
    parallel::for_each_indexed_mut(&mut values, |idx, slot| {
        let z = idx / stride_xy;
        let rem = idx - z * stride_xy;
        let y = rem / dim_x;
        let x = rem - y * dim_x;
        if z >= dim_z {
            return;
        }
        let world = Vec3::new(
            min.x + (x as f32 + 0.5) * voxel_size,
            min.y + (y as f32 + 0.5) * voxel_size,
            min.z + (z as f32 + 0.5) * voxel_size,
        );
        let av = sampler_a.sample_world(world);
        let bv = sampler_b.sample_world(world);
        let mut out = match op {
            1 => av.max(-bv),
            2 => av.max(bv),
            _ => av.min(bv),
        };
        if !out.is_finite() {
            out = 0.0;
        }
        *slot = out;
    });

    let mut volume = Volume::new(VolumeKind::Sdf, min.to_array(), dims, voxel_size, values);
    volume.sdf_band = (a.sdf_band + b.sdf_band) * 0.5;
    Ok(volume)
}

fn sdf_to_mesh(volume: &Volume, iso: f32) -> Result<Mesh, String> {
    let dims = volume.dims;
    if dims[0] < 2 || dims[1] < 2 || dims[2] < 2 {
        return Ok(Mesh::default());
    }
    let mut grid = volume.values.clone();
    let inside_is_greater = false;
    sanitize_grid(&mut grid, iso, inside_is_greater);
    let spec = GridSpec {
        min: Vec3::from(volume.origin),
        dx: volume.voxel_size.max(1.0e-6),
        nx: dims[0] as usize,
        ny: dims[1] as usize,
        nz: dims[2] as usize,
    };
    let mut mesh = marching_cubes(&grid, &spec, iso, inside_is_greater)?;
    if mesh.positions.is_empty() && mesh.indices.is_empty() {
        return Ok(mesh);
    }
    if volume.transform != glam::Mat4::IDENTITY {
        for pos in &mut mesh.positions {
            let world = volume.transform.transform_point3(Vec3::from(*pos));
            *pos = world.to_array();
        }
        let _ = mesh.compute_normals();
    }
    Ok(mesh)
}

fn dims_from_size(size: Vec3, voxel_size: f32) -> [u32; 3] {
    [
        ((size.x / voxel_size).ceil().max(1.0) as u32).saturating_add(1),
        ((size.y / voxel_size).ceil().max(1.0) as u32).saturating_add(1),
        ((size.z / voxel_size).ceil().max(1.0) as u32).saturating_add(1),
    ]
}
