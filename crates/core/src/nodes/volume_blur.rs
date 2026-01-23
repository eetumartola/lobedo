use std::collections::BTreeMap;

use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::nodes::{geometry_in, geometry_out};
use crate::param_spec::ParamSpec;
use crate::volume::{try_alloc_f32, Volume, VolumeKind};

pub const NAME: &str = "Volume Blur";
const DEFAULT_RADIUS: f32 = 1.0;
const DEFAULT_ITERATIONS: i32 = 1;

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
            ("radius".to_string(), ParamValue::Float(DEFAULT_RADIUS)),
            ("iterations".to_string(), ParamValue::Int(DEFAULT_ITERATIONS)),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::float_slider("radius", "Radius", 0.0, 1000.0)
            .with_help("Blur radius in world units."),
        ParamSpec::int_slider("iterations", "Iterations", 0, 20)
            .with_help("Number of blur passes."),
    ]
}

pub fn apply_to_geometry(
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };
    if input.volumes.is_empty() {
        return Err("Volume Blur requires a volume input".to_string());
    }
    let radius = params.get_float("radius", DEFAULT_RADIUS).max(0.0);
    let iterations = params
        .get_int("iterations", DEFAULT_ITERATIONS)
        .max(0);
    let mut output = input.clone();
    output.volumes = input
        .volumes
        .iter()
        .map(|volume| blur_volume(volume, radius, iterations))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(output)
}

fn blur_volume(volume: &Volume, radius: f32, iterations: i32) -> Result<Volume, String> {
    if volume.is_empty() || iterations <= 0 || radius <= 0.0 {
        return Ok(volume.clone());
    }
    let voxel_size = volume.voxel_size.max(1.0e-6);
    let radius_vox = (radius / voxel_size).max(0.0);
    let radius_i = radius_vox.ceil() as i32;
    if radius_i <= 0 {
        return Ok(volume.clone());
    }

    let dims = volume.dims;
    let nx = dims[0] as i32;
    let ny = dims[1] as i32;
    let nz = dims[2] as i32;
    let mut src = try_alloc_f32(volume.values.len(), "Volume Blur")?;
    src.copy_from_slice(&volume.values);
    let mut dst = try_alloc_f32(src.len(), "Volume Blur")?;
    let radius_sq = radius_vox * radius_vox;

    for _ in 0..iterations {
        for z in 0..nz {
            for y in 0..ny {
                for x in 0..nx {
                    let mut accum = 0.0f32;
                    let mut weight = 0.0f32;
                    for dz in -radius_i..=radius_i {
                        let zz = z + dz;
                        if zz < 0 || zz >= nz {
                            continue;
                        }
                        for dy in -radius_i..=radius_i {
                            let yy = y + dy;
                            if yy < 0 || yy >= ny {
                                continue;
                            }
                            for dx in -radius_i..=radius_i {
                                let xx = x + dx;
                                if xx < 0 || xx >= nx {
                                    continue;
                                }
                                let dist_sq = (dx * dx + dy * dy + dz * dz) as f32;
                                if dist_sq > radius_sq {
                                    continue;
                                }
                                let idx = volume.value_index(xx as u32, yy as u32, zz as u32);
                                accum += src[idx];
                                weight += 1.0;
                            }
                        }
                    }
                    let idx = volume.value_index(x as u32, y as u32, z as u32);
                    let mut value = if weight > 0.0 {
                        accum / weight
                    } else {
                        src[idx]
                    };
                    if matches!(volume.kind, VolumeKind::Density)
                        && (!value.is_finite() || value < 0.0)
                    {
                        value = 0.0;
                    }
                    dst[idx] = value;
                }
            }
        }
        std::mem::swap(&mut src, &mut dst);
    }

    let mut out = volume.clone();
    out.values = src;
    Ok(out)
}
