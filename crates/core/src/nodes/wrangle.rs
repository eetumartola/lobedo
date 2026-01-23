use std::collections::BTreeMap;

use crate::attributes::AttributeDomain;
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    geometry_in,
    geometry_out,
    group_utils::{mesh_group_mask, splat_group_mask},
    require_mesh_input,
};
use crate::param_spec::ParamSpec;
use crate::splat::SplatGeo;
use crate::wrangle::{apply_wrangle, apply_wrangle_splats};

pub const NAME: &str = "Wrangle";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![geometry_in("in"), geometry_in("input1")],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            ("mode".to_string(), ParamValue::Int(0)),
            (
                "code".to_string(),
                ParamValue::String("@Cd = vec3(1.0, 1.0, 1.0);".to_string()),
            ),
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::int_enum(
            "mode",
            "Mode",
            vec![
                (0, "Point"),
                (1, "Vertex"),
                (2, "Primitive"),
                (3, "Detail"),
            ],
        )
        .with_help("Domain to iterate over."),
        ParamSpec::code("code", "Code").with_help("Wrangle code snippet."),
        ParamSpec::string("group", "Group").with_help("Restrict to a group."),
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
    ]
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut input = require_mesh_input(inputs, 0, "Wrangle requires a mesh input")?;
    let secondary = inputs.get(1);
    let code = params.get_string("code", "");
    let domain = match params.get_int("mode", 0).clamp(0, 3) {
        0 => AttributeDomain::Point,
        1 => AttributeDomain::Vertex,
        2 => AttributeDomain::Primitive,
        _ => AttributeDomain::Detail,
    };
    if !code.trim().is_empty() {
        let mask = mesh_group_mask(&input, params, domain);
        apply_wrangle(
            &mut input,
            domain,
            code,
            mask.as_deref(),
            secondary,
            None,
            None,
            None,
            None,
        )?;
    }
    Ok(input)
}

pub(crate) fn apply_to_splats(
    params: &NodeParams,
    splats: &mut SplatGeo,
    secondary: Option<&SplatGeo>,
    primary_volume: Option<&crate::volume::Volume>,
    secondary_volume: Option<&crate::volume::Volume>,
) -> Result<(), String> {
    let code = params.get_string("code", "");
    let domain = match params.get_int("mode", 0).clamp(0, 3) {
        0 => AttributeDomain::Point,
        1 => AttributeDomain::Vertex,
        2 => AttributeDomain::Primitive,
        _ => AttributeDomain::Detail,
    };
    if !code.trim().is_empty() {
        let mask = splat_group_mask(splats, params, domain);
        apply_wrangle_splats(
            splats,
            domain,
            code,
            mask.as_deref(),
            secondary,
            primary_volume,
            secondary_volume,
        )?;
    }
    Ok(())
}

pub fn apply_to_geometry(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };
    let secondary = inputs.get(1);
    let secondary_mesh = secondary.and_then(|geo| geo.merged_mesh());
    let secondary_splats = secondary.and_then(|geo| geo.merged_splats());
    let primary_splats = input.merged_splats();
    let primary_volume = input.volumes.first();
    let secondary_volume = secondary.and_then(|geo| geo.volumes.first());
    let code = params.get_string("code", "");
    if code.trim().is_empty() {
        return Ok(input.clone());
    }

    let domain = match params.get_int("mode", 0).clamp(0, 3) {
        0 => AttributeDomain::Point,
        1 => AttributeDomain::Vertex,
        2 => AttributeDomain::Primitive,
        _ => AttributeDomain::Detail,
    };

    let mut meshes = Vec::new();
    if let Some(mut mesh) = input.merged_mesh() {
        let mask = mesh_group_mask(&mesh, params, domain);
        apply_wrangle(
            &mut mesh,
            domain,
            code,
            mask.as_deref(),
            secondary_mesh.as_ref(),
            primary_splats.as_ref(),
            secondary_splats.as_ref(),
            primary_volume,
            secondary_volume,
        )?;
        meshes.push(mesh);
    }

    let mut splats = Vec::with_capacity(input.splats.len());
    for splat in &input.splats {
        let mut splat = splat.clone();
        let mask = splat_group_mask(&splat, params, domain);
        apply_wrangle_splats(
            &mut splat,
            domain,
            code,
            mask.as_deref(),
            secondary_splats.as_ref(),
            primary_volume,
            secondary_volume,
        )?;
        splats.push(splat);
    }

    let curves = if meshes.is_empty() { Vec::new() } else { input.curves.clone() };
    Ok(Geometry {
        meshes,
        splats,
        curves,
        volumes: input.volumes.clone(),
        materials: input.materials.clone(),
    })
}

