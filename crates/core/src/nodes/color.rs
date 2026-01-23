use std::collections::BTreeMap;

use crate::attributes::{AttributeDomain, AttributeRef, AttributeStorage};
use crate::gradient::{parse_color_gradient, ColorGradient};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    attribute_utils::{domain_from_params, existing_vec3_attr_mesh, existing_vec3_attr_splats},
    geometry_in,
    geometry_out,
    group_utils::{mask_has_any, mesh_group_mask, splat_group_mask},
    require_mesh_input,
};
use crate::param_spec::ParamSpec;
use crate::splat::SplatGeo;

pub const NAME: &str = "Color";

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
            ("color_mode".to_string(), ParamValue::Int(0)),
            ("color".to_string(), ParamValue::Vec3([1.0, 1.0, 1.0])),
            ("attr".to_string(), ParamValue::String("mask".to_string())),
            (
                "gradient".to_string(),
                ParamValue::String(ColorGradient::default().to_string()),
            ),
            ("domain".to_string(), ParamValue::Int(0)),
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::int_enum(
            "color_mode",
            "Mode",
            vec![(0, "Constant"), (1, "From Attribute")],
        )
        .with_help("Constant color or color from attribute."),
        ParamSpec::vec3("color", "Color")
            .with_help("Color value (RGB).")
            .visible_when_int("color_mode", 0),
        ParamSpec::string("attr", "Attribute")
            .with_help("Attribute to map into the gradient.")
            .visible_when_int("color_mode", 1),
        ParamSpec::gradient("gradient", "Gradient")
            .with_help("Gradient stops like 0:#000000;1:#ffffff.")
            .visible_when_int("color_mode", 1),
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
        ParamSpec::string("group", "Group")
            .with_help("Restrict to a group."),
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
    let mut input = require_mesh_input(inputs, 0, "Color requires a mesh input")?;
    let mode = params.get_int("color_mode", 0).clamp(0, 1);
    let color = params.get_vec3("color", [1.0, 1.0, 1.0]);
    let attr = params.get_string("attr", "mask");
    let gradient = parse_color_gradient(params.get_string("gradient", ""));
    let domain = domain_from_params(params);
    let count = input.attribute_domain_len(domain);
    if count == 0 && domain != AttributeDomain::Detail {
        return Ok(input);
    }
    let mask = mesh_group_mask(&input, params, domain);
    if !mask_has_any(mask.as_deref()) {
        return Ok(input);
    }
    let mut values = existing_vec3_attr_mesh(&input, domain, "Cd", count);
    if mode == 0 {
        apply_color_to_values(&mut values, color, mask.as_deref());
    } else {
        let Some(samples) = mesh_attribute_samples(&input, domain, attr, count) else {
            return Ok(input);
        };
        apply_gradient_to_values(&mut values, &samples, &gradient, mask.as_deref());
    }
    input
        .set_attribute(domain, "Cd", AttributeStorage::Vec3(values))
        .map_err(|err| format!("Color attribute error: {:?}", err))?;
    Ok(input)
}

pub(crate) fn apply_to_splats(params: &NodeParams, splats: &mut SplatGeo) -> Result<(), String> {
    let mode = params.get_int("color_mode", 0).clamp(0, 1);
    let color = params.get_vec3("color", [1.0, 1.0, 1.0]);
    let attr = params.get_string("attr", "mask");
    let gradient = parse_color_gradient(params.get_string("gradient", ""));
    let domain = domain_from_params(params);
    let count = splats.attribute_domain_len(domain);
    if count == 0 {
        return Ok(());
    }

    let mask = splat_group_mask(splats, params, domain);
    if !mask_has_any(mask.as_deref()) {
        return Ok(());
    }

    let mut values = existing_vec3_attr_splats(splats, domain, "Cd", count);
    if mode == 0 {
        apply_color_to_values(&mut values, color, mask.as_deref());
    } else {
        let Some(samples) = splat_attribute_samples(splats, domain, attr, count) else {
            return Ok(());
        };
        apply_gradient_to_values(&mut values, &samples, &gradient, mask.as_deref());
    }

    splats
        .set_attribute(domain, "Cd", AttributeStorage::Vec3(values))
        .map_err(|err| format!("Color attribute error: {:?}", err))?;
    Ok(())
}

fn apply_color_to_values(
    values: &mut [[f32; 3]],
    color: [f32; 3],
    mask: Option<&[bool]>,
) {
    if let Some(mask) = mask {
        for (idx, value) in values.iter_mut().enumerate() {
            if mask.get(idx).copied().unwrap_or(false) {
                *value = color;
            }
        }
    } else {
        values.iter_mut().for_each(|value| *value = color);
    }
}

fn apply_gradient_to_values(
    values: &mut [[f32; 3]],
    samples: &[f32],
    gradient: &ColorGradient,
    mask: Option<&[bool]>,
) {
    if values.len() != samples.len() {
        return;
    }
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for (idx, value) in samples.iter().enumerate() {
        if mask
            .as_ref()
            .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
        {
            continue;
        }
        if value.is_finite() {
            min = min.min(*value);
            max = max.max(*value);
        }
    }
    if !min.is_finite() || !max.is_finite() {
        return;
    }
    let denom = (max - min).abs();
    let inv = if denom > 1.0e-6 { 1.0 / denom } else { 0.0 };
    for (idx, value) in samples.iter().enumerate() {
        if mask
            .as_ref()
            .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
        {
            continue;
        }
        let t = if !value.is_finite() || inv == 0.0 {
            0.0
        } else {
            ((*value - min) * inv).clamp(0.0, 1.0)
        };
        if let Some(slot) = values.get_mut(idx) {
            *slot = gradient.sample(t);
        }
    }
}

fn mesh_attribute_samples(
    mesh: &Mesh,
    domain: AttributeDomain,
    attr: &str,
    count: usize,
) -> Option<Vec<f32>> {
    let attr_ref = mesh.attribute(domain, attr)?;
    attribute_samples(attr_ref, count)
}

fn splat_attribute_samples(
    splats: &SplatGeo,
    domain: AttributeDomain,
    attr: &str,
    count: usize,
) -> Option<Vec<f32>> {
    let attr_ref = splats.attribute(domain, attr)?;
    attribute_samples(attr_ref, count)
}

fn attribute_samples(attr_ref: AttributeRef<'_>, count: usize) -> Option<Vec<f32>> {
    match attr_ref {
        AttributeRef::Float(values) => {
            if values.len() == count {
                Some(values.to_vec())
            } else {
                None
            }
        }
        AttributeRef::Int(values) => {
            if values.len() == count {
                Some(values.iter().map(|v| *v as f32).collect())
            } else {
                None
            }
        }
        _ => None,
    }
}
