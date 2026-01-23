use std::collections::BTreeMap;

use glam::Vec3;

use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{geometry_in, geometry_out};
use crate::nodes::splat_to_mesh::{marching_cubes, sanitize_grid, GridSpec};
use crate::param_spec::ParamSpec;
use crate::volume::{try_alloc_f32, Volume};

pub const NAME: &str = "Volume to Mesh";

const DEFAULT_DENSITY_ISO: f32 = 0.5;
const DEFAULT_SURFACE_ISO: f32 = 0.0;

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
            (
                "density_iso".to_string(),
                ParamValue::Float(DEFAULT_DENSITY_ISO),
            ),
            (
                "surface_iso".to_string(),
                ParamValue::Float(DEFAULT_SURFACE_ISO),
            ),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::string_enum("mode", "Mode", vec![("density", "Density"), ("sdf", "SDF")])
            .with_help("Treat input volume as density or SDF."),
        ParamSpec::float_slider("density_iso", "Density Iso", 0.0, 1.0)
            .with_help("Isovalue for density surfaces."),
        ParamSpec::float_slider("surface_iso", "Surface Iso", -1.0, 1.0)
            .with_help("Isovalue for SDF surfaces."),
    ]
}

pub fn apply_to_geometry(
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };
    let Some(volume) = input.volumes.first() else {
        return Err("Volume to Mesh requires a volume input".to_string());
    };

    let mode = params.get_string("mode", "sdf").to_lowercase();
    let is_density = !mode.contains("sdf");
    let iso = if is_density {
        params.get_float("density_iso", DEFAULT_DENSITY_ISO)
    } else {
        params.get_float("surface_iso", DEFAULT_SURFACE_ISO)
    };

    let mesh = volume_to_mesh(volume, iso, is_density)?;

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
    let mut volumes = input.volumes.clone();
    if !volumes.is_empty() {
        volumes.remove(0);
    }

    Ok(Geometry {
        meshes,
        splats: input.splats.clone(),
        curves,
        volumes,
        materials: input.materials.clone(),
    })
}

pub(crate) fn volume_to_mesh(
    volume: &Volume,
    iso: f32,
    inside_is_greater: bool,
) -> Result<Mesh, String> {
    let dims = volume.dims;
    if dims[0] < 2 || dims[1] < 2 || dims[2] < 2 {
        return Ok(Mesh::default());
    }
    let mut grid = try_alloc_f32(volume.values.len(), "Volume to Mesh")?;
    if !grid.is_empty() {
        grid.copy_from_slice(&volume.values);
    }
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
