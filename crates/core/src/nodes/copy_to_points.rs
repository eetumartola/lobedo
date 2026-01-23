use std::collections::BTreeMap;

use glam::{EulerRot, Mat4, Quat, Vec3};

use crate::attributes::{
    AttributeDomain, AttributeRef, AttributeStorage, StringTableAttribute,
};
use crate::geometry::merge_splats;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    attribute_utils::parse_attribute_list, geometry_in, geometry_out,
    group_utils::{mesh_group_mask, splat_group_mask},
    require_mesh_input,
};
use crate::parallel;
use crate::param_spec::ParamSpec;
use crate::param_templates;
use crate::splat::SplatGeo;

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

pub fn param_specs() -> Vec<ParamSpec> {
    let mut specs = vec![
        ParamSpec::bool("align_to_normals", "Align to Normals")
            .with_help("Align copies to template normals."),
    ];
    specs.extend(param_templates::transform_params(false));
    specs.push(
        ParamSpec::string("inherit", "Inherit Attributes")
            .with_help("Template point attributes to copy onto each instance."),
    );
    specs.push(
        ParamSpec::string("copy_attr", "Copy Attribute")
            .with_help("Name of the per-copy index attribute."),
    );
    specs.push(ParamSpec::int_enum(
        "copy_attr_class",
        "Copy Attribute Class",
        vec![(0, "Point"), (1, "Vertex"), (2, "Primitive")],
    )
    .with_help("Attribute class for the per-copy index."));
    specs.push(
        ParamSpec::string("group", "Group")
            .with_help("Restrict to a template point group."),
    );
    specs.push(ParamSpec::int_enum(
        "group_type",
        "Group Type",
        vec![
            (0, "Auto"),
            (1, "Vertex"),
            (2, "Point"),
            (3, "Primitive"),
        ],
    )
    .with_help("Group domain to use."));
    specs
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let source = require_mesh_input(inputs, 0, "Copy to Points requires a source mesh")?;
    let template = require_mesh_input(inputs, 1, "Copy to Points requires a template mesh")?;
    let align_to_normals = params.get_bool("align_to_normals", true);
    let inherit = parse_attribute_list(params.get_string("inherit", "Cd"));
    let template = template_from_mesh(params, &template, align_to_normals, &inherit)?;
    compute_mesh_from_template(params, &source, template)
}

pub fn compute_mesh_from_splats(
    params: &NodeParams,
    source: &Mesh,
    template: &SplatGeo,
) -> Result<Mesh, String> {
    let align_to_normals = params.get_bool("align_to_normals", true);
    let inherit = parse_attribute_list(params.get_string("inherit", "Cd"));
    let template = template_from_splats(params, template, align_to_normals, &inherit)?;
    compute_mesh_from_template(params, source, template)
}

pub fn compute_splats_from_mesh(
    params: &NodeParams,
    source: &SplatGeo,
    template: &Mesh,
) -> Result<SplatGeo, String> {
    let align_to_normals = params.get_bool("align_to_normals", true);
    let inherit = parse_attribute_list(params.get_string("inherit", "Cd"));
    let template = template_from_mesh(params, template, align_to_normals, &inherit)?;
    compute_splats_from_template(params, source, template)
}

pub fn compute_splats_from_splats(
    params: &NodeParams,
    source: &SplatGeo,
    template: &SplatGeo,
) -> Result<SplatGeo, String> {
    let align_to_normals = params.get_bool("align_to_normals", true);
    let inherit = parse_attribute_list(params.get_string("inherit", "Cd"));
    let template = template_from_splats(params, template, align_to_normals, &inherit)?;
    compute_splats_from_template(params, source, template)
}

struct TemplateData<'a> {
    positions: &'a [[f32; 3]],
    normals: Option<Vec<[f32; 3]>>,
    selected: Vec<usize>,
    pscale_point: Option<AttributeRef<'a>>,
    pscale_detail: Option<AttributeRef<'a>>,
    inherit_sources: Vec<InheritSource<'a>>,
}

struct CopySettings {
    align_to_normals: bool,
    user_quat: Quat,
    base_scale: Vec3,
    translate: Vec3,
}

fn copy_settings(params: &NodeParams) -> CopySettings {
    let rot = Vec3::from(params.get_vec3("rotate_deg", [0.0, 0.0, 0.0]))
        * std::f32::consts::PI
        / 180.0;
    CopySettings {
        align_to_normals: params.get_bool("align_to_normals", true),
        user_quat: Quat::from_euler(EulerRot::XYZ, rot.x, rot.y, rot.z),
        base_scale: Vec3::from(params.get_vec3("scale", [1.0, 1.0, 1.0])),
        translate: Vec3::from(params.get_vec3("translate", [0.0, 0.0, 0.0])),
    }
}

fn copy_attr_info(params: &NodeParams) -> (String, AttributeDomain) {
    let copy_attr = params.get_string("copy_attr", "copynr");
    let copy_attr = copy_attr.trim().to_string();
    let copy_attr_domain = copy_attr_domain(params.get_int("copy_attr_class", 0));
    (copy_attr, copy_attr_domain)
}

fn template_from_mesh<'a>(
    params: &NodeParams,
    template: &'a Mesh,
    align_to_normals: bool,
    inherit: &[String],
) -> Result<TemplateData<'a>, String> {
    if template.positions.is_empty() {
        return Err("Copy to Points requires template points".to_string());
    }

    let mask = mesh_group_mask(template, params, AttributeDomain::Point);
    let selected = selected_indices(template.positions.len(), mask.as_deref());
    let mut normals = None;
    if align_to_normals {
        let mut values = template.normals.clone().unwrap_or_default();
        if values.len() != template.positions.len() {
            let mut temp = template.clone();
            if temp.normals.is_none() {
                temp.compute_normals();
            }
            values = temp.normals.unwrap_or_default();
        }
        if values.len() == template.positions.len() {
            normals = Some(values);
        }
    }
    Ok(TemplateData {
        positions: template.positions.as_slice(),
        normals,
        selected,
        pscale_point: template.attribute(AttributeDomain::Point, "pscale"),
        pscale_detail: template.attribute(AttributeDomain::Detail, "pscale"),
        inherit_sources: build_inherit_sources(template, inherit),
    })
}

fn template_from_splats<'a>(
    params: &NodeParams,
    template: &'a SplatGeo,
    align_to_normals: bool,
    inherit: &[String],
) -> Result<TemplateData<'a>, String> {
    if template.positions.is_empty() {
        return Err("Copy to Points requires template points".to_string());
    }

    let mask = splat_group_mask(template, params, AttributeDomain::Point);
    let selected = selected_indices(template.positions.len(), mask.as_deref());
    let normals = if align_to_normals {
        match template.attribute(AttributeDomain::Point, "N") {
            Some(AttributeRef::Vec3(values)) if values.len() == template.positions.len() => {
                Some(values.to_vec())
            }
            _ => None,
        }
    } else {
        None
    };

    Ok(TemplateData {
        positions: template.positions.as_slice(),
        normals,
        selected,
        pscale_point: template.attribute(AttributeDomain::Point, "pscale"),
        pscale_detail: template.attribute(AttributeDomain::Detail, "pscale"),
        inherit_sources: build_inherit_sources_splats(template, inherit),
    })
}

fn selected_indices(len: usize, mask: Option<&[bool]>) -> Vec<usize> {
    if let Some(mask) = mask {
        mask.iter()
            .enumerate()
            .filter_map(|(idx, value)| if *value { Some(idx) } else { None })
            .collect()
    } else {
        (0..len).collect()
    }
}

fn compute_mesh_from_template(
    params: &NodeParams,
    source: &Mesh,
    template: TemplateData<'_>,
) -> Result<Mesh, String> {
    if template.selected.is_empty() {
        return Ok(Mesh::default());
    }

    let settings = copy_settings(params);
    let (copy_attr, copy_attr_domain) = copy_attr_info(params);
    let TemplateData {
        positions,
        normals,
        selected,
        pscale_point,
        pscale_detail,
        inherit_sources,
    } = template;
    let normals = normals.as_deref();
    let pscale_point = pscale_point.as_ref();
    let pscale_detail = pscale_detail.as_ref();
    let mut copies: Vec<Mesh> = (0..selected.len()).map(|_| Mesh::default()).collect();
    parallel::try_for_each_indexed_mut(&mut copies, |copy_idx, slot| {
        let idx = selected[copy_idx];
        let matrix = build_copy_matrix(
            &settings,
            positions,
            normals,
            idx,
            pscale_point,
            pscale_detail,
        );
        let mut mesh = source.clone();
        mesh.transform(matrix);
        apply_inherit_attributes(&mut mesh, &inherit_sources, idx)?;
        if !copy_attr.is_empty() {
            apply_copy_index_attribute(&mut mesh, &copy_attr, copy_attr_domain, copy_idx)?;
        }
        *slot = mesh;
        Ok::<(), String>(())
    })?;
    Ok(Mesh::merge(&copies))
}

fn compute_splats_from_template(
    params: &NodeParams,
    source: &SplatGeo,
    template: TemplateData<'_>,
) -> Result<SplatGeo, String> {
    if template.selected.is_empty() {
        return Ok(SplatGeo::default());
    }

    let settings = copy_settings(params);
    let (copy_attr, copy_attr_domain) = copy_attr_info(params);
    let TemplateData {
        positions,
        normals,
        selected,
        pscale_point,
        pscale_detail,
        inherit_sources,
    } = template;
    let normals = normals.as_deref();
    let pscale_point = pscale_point.as_ref();
    let pscale_detail = pscale_detail.as_ref();
    let mut copies: Vec<SplatGeo> = (0..selected.len()).map(|_| SplatGeo::default()).collect();
    parallel::try_for_each_indexed_mut(&mut copies, |copy_idx, slot| {
        let idx = selected[copy_idx];
        let matrix = build_copy_matrix(
            &settings,
            positions,
            normals,
            idx,
            pscale_point,
            pscale_detail,
        );
        let mut splats = source.clone();
        splats.transform(matrix);
        apply_inherit_attributes_splats(&mut splats, &inherit_sources, idx)?;
        if !copy_attr.is_empty() {
            apply_copy_index_attribute_splats(
                &mut splats,
                &copy_attr,
                copy_attr_domain,
                copy_idx,
            )?;
        }
        *slot = splats;
        Ok::<(), String>(())
    })?;
    Ok(merge_splats(&copies))
}

fn build_copy_matrix(
    settings: &CopySettings,
    positions: &[[f32; 3]],
    normals: Option<&[[f32; 3]]>,
    idx: usize,
    pscale_point: Option<&AttributeRef<'_>>,
    pscale_detail: Option<&AttributeRef<'_>>,
) -> Mat4 {
    let pos = positions.get(idx).copied().unwrap_or([0.0, 0.0, 0.0]);
    let mut rotation = settings.user_quat;
    if settings.align_to_normals {
        let normal = normals
            .and_then(|values| values.get(idx).copied())
            .unwrap_or([0.0, 1.0, 0.0]);
        let normal = Vec3::from(normal);
        if normal.length_squared() > 0.0001 {
            let align = Quat::from_rotation_arc(Vec3::Y, normal.normalize());
            rotation = align * settings.user_quat;
        }
    }
    let pscale = sample_pscale(idx, pscale_point, pscale_detail);
    let scale = settings.base_scale * pscale;
    Mat4::from_scale_rotation_translation(scale, rotation, Vec3::from(pos) + settings.translate)
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

fn build_inherit_sources_splats<'a>(
    splats: &'a SplatGeo,
    names: &[String],
) -> Vec<InheritSource<'a>> {
    let mut sources = Vec::new();
    for name in names {
        let Some((domain, attr)) = splats.attribute_with_precedence(name) else {
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

fn apply_inherit_attributes_splats(
    splats: &mut SplatGeo,
    sources: &[InheritSource<'_>],
    point_index: usize,
) -> Result<(), String> {
    let point_count = splats.attribute_domain_len(AttributeDomain::Point);
    for source in sources {
        let Some(value) = sample_inherit_value(source, point_index) else {
            continue;
        };
        match value {
            InheritValue::Float(value) => {
                splats
                    .set_attribute(
                        AttributeDomain::Point,
                        source.name.clone(),
                        AttributeStorage::Float(vec![value; point_count]),
                    )
                    .map_err(|err| format!("Copy to Points inherit error: {:?}", err))?;
            }
            InheritValue::Int(value) => {
                splats
                    .set_attribute(
                        AttributeDomain::Point,
                        source.name.clone(),
                        AttributeStorage::Int(vec![value; point_count]),
                    )
                    .map_err(|err| format!("Copy to Points inherit error: {:?}", err))?;
            }
            InheritValue::Vec2(value) => {
                splats
                    .set_attribute(
                        AttributeDomain::Point,
                        source.name.clone(),
                        AttributeStorage::Vec2(vec![value; point_count]),
                    )
                    .map_err(|err| format!("Copy to Points inherit error: {:?}", err))?;
            }
            InheritValue::Vec3(value) => {
                splats
                    .set_attribute(
                        AttributeDomain::Point,
                        source.name.clone(),
                        AttributeStorage::Vec3(vec![value; point_count]),
                    )
                    .map_err(|err| format!("Copy to Points inherit error: {:?}", err))?;
            }
            InheritValue::Vec4(value) => {
                splats
                    .set_attribute(
                        AttributeDomain::Point,
                        source.name.clone(),
                        AttributeStorage::Vec4(vec![value; point_count]),
                    )
                    .map_err(|err| format!("Copy to Points inherit error: {:?}", err))?;
            }
            InheritValue::StringTable { values, index } => {
                splats
                    .set_attribute(
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

fn apply_copy_index_attribute_splats(
    splats: &mut SplatGeo,
    name: &str,
    domain: AttributeDomain,
    copy_idx: usize,
) -> Result<(), String> {
    let domain = if domain == AttributeDomain::Vertex {
        AttributeDomain::Point
    } else {
        domain
    };
    let count = splats.attribute_domain_len(domain);
    if count == 0 {
        return Ok(());
    }
    let values = vec![copy_idx as i32; count];
    splats
        .set_attribute(domain, name, AttributeStorage::Int(values))
        .map_err(|err| format!("Copy to Points attribute error: {:?}", err))?;
    Ok(())
}

fn sample_pscale(
    point_index: usize,
    point_attr: Option<&AttributeRef<'_>>,
    detail_attr: Option<&AttributeRef<'_>>,
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



