use std::collections::BTreeMap;

use glam::{EulerRot, Mat4, Quat, Vec3};

use crate::attributes::{
    AttributeDomain, AttributeRef, AttributeStorage, StringTableAttribute,
};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    attribute_utils::parse_attribute_list, geometry_in, geometry_out,
    group_utils::mesh_group_mask, require_mesh_input,
};

pub const NAME: &str = "Copy to Points";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![geometry_in("source"), geometry_in("template")],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            ("align_to_normals".to_string(), ParamValue::Bool(true)),
            ("translate".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0])),
            ("rotate_deg".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0])),
            ("scale".to_string(), ParamValue::Vec3([1.0, 1.0, 1.0])),
            ("inherit".to_string(), ParamValue::String("Cd".to_string())),
            ("copy_attr".to_string(), ParamValue::String("copynr".to_string())),
            ("copy_attr_class".to_string(), ParamValue::Int(0)),
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let source = require_mesh_input(inputs, 0, "Copy to Points requires a source mesh")?;
    let template = require_mesh_input(inputs, 1, "Copy to Points requires a template mesh")?;

    if template.positions.is_empty() {
        return Err("Copy to Points requires template points".to_string());
    }

    let align_to_normals = params.get_bool("align_to_normals", true);
    let translate = params.get_vec3("translate", [0.0, 0.0, 0.0]);
    let rotate_deg = params.get_vec3("rotate_deg", [0.0, 0.0, 0.0]);
    let scale = params.get_vec3("scale", [1.0, 1.0, 1.0]);
    let inherit = parse_attribute_list(params.get_string("inherit", "Cd"));
    let copy_attr = params.get_string("copy_attr", "copynr");
    let copy_attr = copy_attr.trim().to_string();
    let copy_attr_domain = copy_attr_domain(params.get_int("copy_attr_class", 0));

    let mask = mesh_group_mask(&template, params, AttributeDomain::Point);
    let selected: Vec<usize> = if let Some(mask) = &mask {
        mask.iter()
            .enumerate()
            .filter_map(|(idx, value)| if *value { Some(idx) } else { None })
            .collect()
    } else {
        (0..template.positions.len()).collect()
    };
    if selected.is_empty() {
        return Ok(Mesh::default());
    }

    let mut normals = template.normals.clone().unwrap_or_default();
    if align_to_normals && normals.len() != template.positions.len() {
        let mut temp = template.clone();
        if temp.normals.is_none() {
            temp.compute_normals();
        }
        normals = temp.normals.unwrap_or_default();
    }

    let rot = Vec3::from(rotate_deg) * std::f32::consts::PI / 180.0;
    let user_quat = Quat::from_euler(EulerRot::XYZ, rot.x, rot.y, rot.z);
    let base_scale = Vec3::from(scale);
    let translate = Vec3::from(translate);
    let inherit_sources = build_inherit_sources(&template, &inherit);
    let pscale_attr = template.attribute(AttributeDomain::Point, "pscale");
    let pscale_detail = template.attribute(AttributeDomain::Detail, "pscale");

    let mut copies = Vec::with_capacity(selected.len());
    for (copy_idx, idx) in selected.into_iter().enumerate() {
        let pos = template.positions.get(idx).copied().unwrap_or([0.0, 0.0, 0.0]);
        let mut rotation = user_quat;
        if align_to_normals {
            let normal = normals.get(idx).copied().unwrap_or([0.0, 1.0, 0.0]);
            let normal = Vec3::from(normal);
            if normal.length_squared() > 0.0001 {
                let align = Quat::from_rotation_arc(Vec3::Y, normal.normalize());
                rotation = align * user_quat;
            }
        }
        let pscale = sample_pscale(idx, pscale_attr, pscale_detail);
        let scale = base_scale * pscale;
        let matrix =
            Mat4::from_scale_rotation_translation(scale, rotation, Vec3::from(pos) + translate);
        let mut mesh = source.clone();
        mesh.transform(matrix);
        apply_inherit_attributes(&mut mesh, &inherit_sources, idx)?;
        if !copy_attr.is_empty() {
            apply_copy_index_attribute(&mut mesh, &copy_attr, copy_attr_domain, copy_idx)?;
        }
        copies.push(mesh);
    }
    Ok(Mesh::merge(&copies))
}

#[derive(Clone)]
struct InheritSource<'a> {
    name: String,
    domain: AttributeDomain,
    attr: AttributeRef<'a>,
}

enum InheritValue {
    Float(f32),
    Int(i32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    StringTable { values: Vec<String>, index: u32 },
}

fn copy_attr_domain(value: i32) -> AttributeDomain {
    match value.clamp(0, 2) {
        1 => AttributeDomain::Vertex,
        2 => AttributeDomain::Primitive,
        _ => AttributeDomain::Point,
    }
}

fn build_inherit_sources<'a>(mesh: &'a Mesh, names: &[String]) -> Vec<InheritSource<'a>> {
    let mut sources = Vec::new();
    for name in names {
        let Some((domain, attr)) = mesh.attribute_with_precedence(name) else {
            continue;
        };
        if attr.is_empty() {
            continue;
        }
        sources.push(InheritSource {
            name: name.clone(),
            domain,
            attr,
        });
    }
    sources
}

fn sample_inherit_value(source: &InheritSource<'_>, point_index: usize) -> Option<InheritValue> {
    let attr_len = source.attr.len();
    if attr_len == 0 {
        return None;
    }
    let index = match source.domain {
        AttributeDomain::Point => point_index.min(attr_len.saturating_sub(1)),
        AttributeDomain::Detail => 0,
        AttributeDomain::Vertex | AttributeDomain::Primitive => {
            if point_index < attr_len {
                point_index
            } else {
                0
            }
        }
    };
    Some(match source.attr {
        AttributeRef::Float(values) => {
            InheritValue::Float(values.get(index).copied().unwrap_or(0.0))
        }
        AttributeRef::Int(values) => InheritValue::Int(values.get(index).copied().unwrap_or(0)),
        AttributeRef::Vec2(values) => {
            InheritValue::Vec2(values.get(index).copied().unwrap_or([0.0, 0.0]))
        }
        AttributeRef::Vec3(values) => InheritValue::Vec3(
            values.get(index).copied().unwrap_or([0.0, 0.0, 0.0]),
        ),
        AttributeRef::Vec4(values) => InheritValue::Vec4(
            values.get(index).copied().unwrap_or([0.0, 0.0, 0.0, 0.0]),
        ),
        AttributeRef::StringTable(values) => InheritValue::StringTable {
            values: values.values.clone(),
            index: values.indices.get(index).copied().unwrap_or(0),
        },
    })
}

fn apply_inherit_attributes(
    mesh: &mut Mesh,
    sources: &[InheritSource<'_>],
    point_index: usize,
) -> Result<(), String> {
    let point_count = mesh.attribute_domain_len(AttributeDomain::Point);
    for source in sources {
        let Some(value) = sample_inherit_value(source, point_index) else {
            continue;
        };
        match value {
            InheritValue::Float(value) => {
                mesh.set_attribute(
                    AttributeDomain::Point,
                    source.name.clone(),
                    AttributeStorage::Float(vec![value; point_count]),
                )
                .map_err(|err| format!("Copy to Points inherit error: {:?}", err))?;
            }
            InheritValue::Int(value) => {
                mesh.set_attribute(
                    AttributeDomain::Point,
                    source.name.clone(),
                    AttributeStorage::Int(vec![value; point_count]),
                )
                .map_err(|err| format!("Copy to Points inherit error: {:?}", err))?;
            }
            InheritValue::Vec2(value) => {
                mesh.set_attribute(
                    AttributeDomain::Point,
                    source.name.clone(),
                    AttributeStorage::Vec2(vec![value; point_count]),
                )
                .map_err(|err| format!("Copy to Points inherit error: {:?}", err))?;
            }
            InheritValue::Vec3(value) => {
                mesh.set_attribute(
                    AttributeDomain::Point,
                    source.name.clone(),
                    AttributeStorage::Vec3(vec![value; point_count]),
                )
                .map_err(|err| format!("Copy to Points inherit error: {:?}", err))?;
            }
            InheritValue::Vec4(value) => {
                mesh.set_attribute(
                    AttributeDomain::Point,
                    source.name.clone(),
                    AttributeStorage::Vec4(vec![value; point_count]),
                )
                .map_err(|err| format!("Copy to Points inherit error: {:?}", err))?;
            }
            InheritValue::StringTable { values, index } => {
                mesh.set_attribute(
                    AttributeDomain::Point,
                    source.name.clone(),
                    AttributeStorage::StringTable(StringTableAttribute::new(
                        values,
                        vec![index; point_count],
                    )),
                )
                .map_err(|err| format!("Copy to Points inherit error: {:?}", err))?;
            }
        }
    }
    Ok(())
}

fn apply_copy_index_attribute(
    mesh: &mut Mesh,
    name: &str,
    domain: AttributeDomain,
    copy_idx: usize,
) -> Result<(), String> {
    let count = mesh.attribute_domain_len(domain);
    if count == 0 {
        return Ok(());
    }
    let values = vec![copy_idx as i32; count];
    mesh.set_attribute(domain, name, AttributeStorage::Int(values))
        .map_err(|err| format!("Copy to Points attribute error: {:?}", err))?;
    Ok(())
}

fn sample_pscale(
    point_index: usize,
    point_attr: Option<AttributeRef<'_>>,
    detail_attr: Option<AttributeRef<'_>>,
) -> f32 {
    if let Some(AttributeRef::Float(values)) = point_attr {
        if let Some(value) = values.get(point_index) {
            return if value.is_finite() { *value } else { 1.0 };
        }
    }
    if let Some(AttributeRef::Float(values)) = detail_attr {
        if let Some(value) = values.first() {
            return if value.is_finite() { *value } else { 1.0 };
        }
    }
    1.0
}


