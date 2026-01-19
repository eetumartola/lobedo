use std::collections::BTreeMap;

use glam::{Mat4, Vec3};

use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::nodes::{geometry_in, geometry_out};
use crate::volume::{try_alloc_f32, Volume, VolumeKind};
use crate::volume_sampling::VolumeSampler;

pub const NAME: &str = "Volume Combine";

const DEFAULT_OP: i32 = 0;
const DEFAULT_RESOLUTION: i32 = 0;
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
            ("op".to_string(), ParamValue::Int(DEFAULT_OP)),
            ("resolution".to_string(), ParamValue::Int(DEFAULT_RESOLUTION)),
        ]),
    }
}

pub fn apply_to_geometry(
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    let Some(left) = inputs.first() else {
        return Ok(Geometry::default());
    };
    let Some(right) = inputs.get(1) else {
        return Err("Volume Combine requires two inputs".to_string());
    };
    let Some(vol_a) = left.volumes.first() else {
        return Err("Volume Combine requires a volume on input 0".to_string());
    };
    let Some(vol_b) = right.volumes.first() else {
        return Err("Volume Combine requires a volume on input 1".to_string());
    };
    if vol_a.kind != vol_b.kind {
        return Err("Volume Combine requires matching volume types".to_string());
    }

    let op = params.get_int("op", DEFAULT_OP);
    let resolution = params.get_int("resolution", DEFAULT_RESOLUTION);
    let combined = combine_volumes(vol_a, vol_b, op, resolution)?;

    let mut output = left.clone();
    output.volumes = vec![combined];
    Ok(output)
}

fn combine_volumes(
    a: &Volume,
    b: &Volume,
    op: i32,
    resolution: i32,
) -> Result<Volume, String> {
    let (a_min, a_max) = a.world_bounds();
    let (b_min, b_max) = b.world_bounds();
    let min = a_min.min(b_min);
    let max = a_max.max(b_max);
    let size = (max - min).max(Vec3::splat(1.0e-6));

    let voxel_size = match resolution {
        1 => a.voxel_size.min(b.voxel_size),
        2 => (a.voxel_size + b.voxel_size) * 0.5,
        _ => a.voxel_size.max(b.voxel_size),
    }
    .max(1.0e-6);

    let dims = dims_from_size(size, voxel_size);
    let total = dims[0] as u64 * dims[1] as u64 * dims[2] as u64;
    if total == 0 || total > MAX_GRID_POINTS {
        return Err(format!(
            "Volume grid too large ({} voxels, max {})",
            total, MAX_GRID_POINTS
        ));
    }

    let sampler_a = VolumeSampler::new(a);
    let sampler_b = VolumeSampler::new(b);
    let scale_a = if matches!(a.kind, VolumeKind::Density) {
        a.density_scale
    } else {
        1.0
    };
    let scale_b = if matches!(b.kind, VolumeKind::Density) {
        b.density_scale
    } else {
        1.0
    };

    let mut values = try_alloc_f32(total as usize, "Volume Combine")?;
    let mut idx = 0usize;
    for z in 0..dims[2] {
        let zf = min.z + (z as f32 + 0.5) * voxel_size;
        for y in 0..dims[1] {
            let yf = min.y + (y as f32 + 0.5) * voxel_size;
            for x in 0..dims[0] {
                let xf = min.x + (x as f32 + 0.5) * voxel_size;
                let world_pos = Vec3::new(xf, yf, zf);
                let av = sampler_a.sample_world(world_pos) * scale_a;
                let bv = sampler_b.sample_world(world_pos) * scale_b;
                let mut out = combine_scalar(op, av, bv);
                if matches!(a.kind, VolumeKind::Density) && (!out.is_finite() || out < 0.0) {
                    out = 0.0;
                }
                values[idx] = out;
                idx += 1;
            }
        }
    }

    let mut volume = Volume::new(a.kind, min.to_array(), dims, voxel_size, values);
    volume.transform = Mat4::IDENTITY;
    volume.density_scale = if matches!(a.kind, VolumeKind::Density) {
        1.0
    } else {
        (a.density_scale + b.density_scale) * 0.5
    };
    volume.sdf_band = (a.sdf_band + b.sdf_band) * 0.5;
    Ok(volume)
}

fn combine_scalar(op: i32, a: f32, b: f32) -> f32 {
    match op {
        1 => a - b,
        2 => a * b,
        3 => a.min(b),
        4 => a.max(b),
        5 => 0.5 * (a + b),
        _ => a + b,
    }
}

fn dims_from_size(size: Vec3, voxel_size: f32) -> [u32; 3] {
    [
        ((size.x / voxel_size).ceil().max(1.0) as u32).saturating_add(1),
        ((size.y / voxel_size).ceil().max(1.0) as u32).saturating_add(1),
        ((size.z / voxel_size).ceil().max(1.0) as u32).saturating_add(1),
    ]
}

// sampling helpers live in volume_sampling
