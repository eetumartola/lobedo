use std::collections::BTreeMap;

use crate::attributes::{AttributeDomain, AttributeStorage, StringTableAttribute};
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::material::Material;
use crate::mesh::Mesh;
use crate::nodes::{geometry_in, geometry_out, require_mesh_input};
use crate::splat::SplatGeo;

pub const NAME: &str = "Material";

const DEFAULT_NAME: &str = "material1";

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
            ("name".to_string(), ParamValue::String(DEFAULT_NAME.to_string())),
            ("base_color".to_string(), ParamValue::Vec3([1.0, 1.0, 1.0])),
            ("metallic".to_string(), ParamValue::Float(0.0)),
            ("roughness".to_string(), ParamValue::Float(0.5)),
            ("base_color_tex".to_string(), ParamValue::String(String::new())),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut mesh = require_mesh_input(inputs, 0, "Material requires a mesh input")?;
    let name = params.get_string("name", DEFAULT_NAME);
    assign_material_mesh(&mut mesh, name);
    Ok(mesh)
}

pub fn apply_to_geometry(
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    let input = inputs.first().cloned().unwrap_or_default();
    let material = build_material(params);
    let name = material.name.clone();
    let mut materials = input.materials.clone();
    materials.insert(material);

    let mut meshes = Vec::new();
    if let Some(mut mesh) = input.merged_mesh() {
        assign_material_mesh(&mut mesh, &name);
        meshes.push(mesh);
    }

    let mut splats = input.splats.clone();
    for splat in &mut splats {
        assign_material_splats(splat, &name);
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
        materials,
    })
}

fn build_material(params: &NodeParams) -> Material {
    let name = params.get_string("name", DEFAULT_NAME).to_string();
    let mut material = Material::new(name);
    material.base_color = params.get_vec3("base_color", [1.0, 1.0, 1.0]);
    material.metallic = params.get_float("metallic", 0.0);
    material.roughness = params.get_float("roughness", 0.5).clamp(0.0, 1.0);
    let tex = params.get_string("base_color_tex", "");
    if !tex.trim().is_empty() {
        material.base_color_texture = Some(tex.to_string());
    }
    material
}

fn assign_material_mesh(mesh: &mut Mesh, name: &str) {
    let prim_count = mesh.attribute_domain_len(AttributeDomain::Primitive);
    if prim_count == 0 {
        return;
    }
    let values = vec![name.to_string()];
    let indices = vec![0u32; prim_count];
    let storage = AttributeStorage::StringTable(StringTableAttribute::new(values, indices));
    let _ = mesh.set_attribute(AttributeDomain::Primitive, "material", storage);
}

fn assign_material_splats(splats: &mut SplatGeo, name: &str) {
    let prim_count = splats.attribute_domain_len(AttributeDomain::Primitive);
    if prim_count == 0 {
        return;
    }
    let values = vec![name.to_string()];
    let indices = vec![0u32; prim_count];
    let storage = AttributeStorage::StringTable(StringTableAttribute::new(values, indices));
    let _ = splats.set_attribute(AttributeDomain::Primitive, "material", storage);
}
