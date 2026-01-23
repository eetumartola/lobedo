use std::collections::BTreeMap;

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    attribute_utils::{
        domain_from_params, existing_float_attr_mesh, existing_float_attr_splats, mesh_sample_position,
        splat_sample_position,
    },
    geometry_in,
    geometry_out,
    group_utils::{mask_has_any, mesh_group_mask, splat_group_mask},
};
use crate::param_spec::ParamSpec;
use crate::parallel;
use crate::splat::SplatGeo;
use crate::volume::Volume;
use crate::volume_sampling::VolumeSampler;

pub const NAME: &str = "Attribute from Volume";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![geometry_in("geo"), geometry_in("volume")],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            ("attr".to_string(), ParamValue::String(String::new())),
            ("domain".to_string(), ParamValue::Int(0)),
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::string("attr", "Attribute")
            .with_help("Attribute name (empty = volume)."),
        ParamSpec::int_enum(
            "domain",
            "Domain",
            vec![
                (0, "Point"),
                (1, "Vertex"),
                (2, "Primitive"),
                (3, "Detail"),
            ],
        )
        .with_help("Attribute domain to write."),
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

pub fn apply_to_geometry(
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    let Some(target) = inputs.first() else {
        return Ok(Geometry::default());
    };
    let Some(source) = inputs.get(1) else {
        return Err("Attribute from Volume requires a volume input".to_string());
    };
    let Some(volume) = source.volumes.first() else {
        return Err("Attribute from Volume requires a volume on input 1".to_string());
    };
    let domain = domain_from_params(params);
    let attr_name = target_attr_name(params);

    let mut meshes = Vec::new();
    if let Some(mut mesh) = target.merged_mesh() {
        apply_to_mesh(params, &attr_name, volume, &mut mesh, domain)?;
        meshes.push(mesh);
    }

    let mut splats = Vec::with_capacity(target.splats.len());
    for splat in &target.splats {
        let mut splat = splat.clone();
        apply_to_splats(params, &attr_name, volume, &mut splat, domain)?;
        splats.push(splat);
    }

    let curves = if meshes.is_empty() {
        Vec::new()
    } else {
        target.curves.clone()
    };

    Ok(Geometry {
        meshes,
        splats,
        curves,
        volumes: target.volumes.clone(),
        materials: target.materials.clone(),
    })
}

fn target_attr_name(params: &NodeParams) -> String {
    let name = params.get_string("attr", "");
    if name.trim().is_empty() {
        "volume".to_string()
    } else {
        name.to_string()
    }
}

fn apply_to_mesh(
    params: &NodeParams,
    attr: &str,
    volume: &Volume,
    mesh: &mut Mesh,
    domain: AttributeDomain,
) -> Result<(), String> {
    let count = mesh.attribute_domain_len(domain);
    if count == 0 && domain != AttributeDomain::Detail {
        return Ok(());
    }
    let mask = mesh_group_mask(mesh, params, domain);
    let mask_ref = mask.as_deref();
    if !mask_has_any(mask.as_deref()) {
        return Ok(());
    }

    let mut values = existing_float_attr_mesh(mesh, domain, attr, count);
    let sampler = VolumeSampler::new(volume);
    let mesh_ref = &*mesh;
    parallel::for_each_indexed_mut(&mut values, |index, slot| {
        if mask_ref
            .is_some_and(|mask| !mask.get(index).copied().unwrap_or(false))
        {
            return;
        }
        let pos = mesh_sample_position(mesh_ref, domain, index);
        *slot = sampler.sample_world(pos);
    });

    mesh.set_attribute(domain, attr, AttributeStorage::Float(values))
        .map_err(|err| format!("Attribute from Volume error: {:?}", err))?;
    Ok(())
}

fn apply_to_splats(
    params: &NodeParams,
    attr: &str,
    volume: &Volume,
    splats: &mut SplatGeo,
    domain: AttributeDomain,
) -> Result<(), String> {
    let count = splats.attribute_domain_len(domain);
    if count == 0 && domain != AttributeDomain::Detail {
        return Ok(());
    }
    let mask = splat_group_mask(splats, params, domain);
    let mask_ref = mask.as_deref();
    if !mask_has_any(mask.as_deref()) {
        return Ok(());
    }

    let mut values = existing_float_attr_splats(splats, domain, attr, count);
    let sampler = VolumeSampler::new(volume);
    let splats_ref = &*splats;
    parallel::for_each_indexed_mut(&mut values, |index, slot| {
        if mask_ref
            .is_some_and(|mask| !mask.get(index).copied().unwrap_or(false))
        {
            return;
        }
        let pos = splat_sample_position(splats_ref, domain, index);
        *slot = sampler.sample_world(pos);
    });

    splats
        .set_attribute(domain, attr, AttributeStorage::Float(values))
        .map_err(|err| format!("Attribute from Volume error: {:?}", err))?;
    Ok(())
}

// sampling helpers live in volume_sampling
