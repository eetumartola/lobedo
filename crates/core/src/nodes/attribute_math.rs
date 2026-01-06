use std::collections::BTreeMap;

use tracing::warn;

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    attribute_utils::domain_from_params,
    geometry_in,
    geometry_out,
    group_utils::{mesh_group_mask, splat_group_mask},
    require_mesh_input,
};
use crate::splat::SplatGeo;

pub const NAME: &str = "Attribute Math";

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
            ("attr".to_string(), ParamValue::String("Cd".to_string())),
            ("result".to_string(), ParamValue::String("Cd".to_string())),
            ("domain".to_string(), ParamValue::Int(0)),
            ("op".to_string(), ParamValue::Int(0)),
            ("value_f".to_string(), ParamValue::Float(1.0)),
            ("value_v3".to_string(), ParamValue::Vec3([1.0, 1.0, 1.0])),
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
        ]),
    }
}

struct AttributeMathSettings {
    attr: String,
    result: String,
    domain: AttributeDomain,
    op: i32,
    value_f: f32,
    value_v3: [f32; 3],
}

fn attribute_math_settings(params: &NodeParams) -> AttributeMathSettings {
    let attr = params.get_string("attr", "Cd").to_string();
    let result = params.get_string("result", &attr).to_string();
    AttributeMathSettings {
        attr,
        result,
        domain: domain_from_params(params),
        op: params.get_int("op", 0).clamp(0, 3),
        value_f: params.get_float("value_f", 0.0),
        value_v3: params.get_vec3("value_v3", [0.0, 0.0, 0.0]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut input = require_mesh_input(inputs, 0, "Attribute Math requires a mesh input")?;
    let settings = attribute_math_settings(params);

    let attr_ref = match input.attribute(settings.domain, &settings.attr) {
        Some(attr_ref) => attr_ref,
        None => {
            warn!(
                "Attribute Math: '{}' not found on {:?}; passing input through",
                settings.attr, settings.domain
            );
            return Ok(input);
        }
    };
    let mask = mesh_group_mask(&input, params, settings.domain);
    let storage = {
        let existing = input.attribute(settings.domain, &settings.result);
        build_attribute_math_storage(
            attr_ref,
            existing,
            mask.as_deref(),
            settings.op,
            settings.value_f,
            settings.value_v3,
        )
    };
    input
        .set_attribute(settings.domain, settings.result, storage)
        .map_err(|err| format!("Attribute Math error: {:?}", err))?;
    Ok(input)
}

pub(crate) fn apply_to_splats(
    params: &NodeParams,
    splats: &mut SplatGeo,
) -> Result<(), String> {
    let settings = attribute_math_settings(params);

    let attr_ref = match splats.attribute(settings.domain, &settings.attr) {
        Some(attr_ref) => attr_ref,
        None => {
            warn!(
                "Attribute Math: '{}' not found on {:?}; passing input through",
                settings.attr, settings.domain
            );
            return Ok(());
        }
    };
    let mask = splat_group_mask(splats, params, settings.domain);
    let storage = {
        let existing = splats.attribute(settings.domain, &settings.result);
        build_attribute_math_storage(
            attr_ref,
            existing,
            mask.as_deref(),
            settings.op,
            settings.value_f,
            settings.value_v3,
        )
    };
    splats
        .set_attribute(settings.domain, settings.result, storage)
        .map_err(|err| format!("Attribute Math error: {:?}", err))?;
    Ok(())
}

fn build_attribute_math_storage(
    attr_ref: crate::attributes::AttributeRef<'_>,
    existing: Option<crate::attributes::AttributeRef<'_>>,
    mask: Option<&[bool]>,
    op: i32,
    value_f: f32,
    value_v3: [f32; 3],
) -> AttributeStorage {
    match attr_ref {
        crate::attributes::AttributeRef::Float(values) => {
            let expected_len = values.len();
            let mut next = match existing {
                Some(crate::attributes::AttributeRef::Float(existing))
                    if existing.len() == expected_len =>
                {
                    existing.to_vec()
                }
                _ => values.to_vec(),
            };
            for (idx, &v) in values.iter().enumerate() {
                if mask.is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false)) {
                    continue;
                }
                if let Some(slot) = next.get_mut(idx) {
                    *slot = apply_op_f(v, value_f, op);
                }
            }
            AttributeStorage::Float(next)
        }
        crate::attributes::AttributeRef::Int(values) => {
            let expected_len = values.len();
            let mut next = match existing {
                Some(crate::attributes::AttributeRef::Int(existing))
                    if existing.len() == expected_len =>
                {
                    existing.to_vec()
                }
                _ => values.to_vec(),
            };
            let value_i = value_f.round() as i32;
            for (idx, &v) in values.iter().enumerate() {
                if mask.is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false)) {
                    continue;
                }
                if let Some(slot) = next.get_mut(idx) {
                    *slot = apply_op_i(v, value_i, op);
                }
            }
            AttributeStorage::Int(next)
        }
        crate::attributes::AttributeRef::Vec2(values) => {
            let expected_len = values.len();
            let mut next = match existing {
                Some(crate::attributes::AttributeRef::Vec2(existing))
                    if existing.len() == expected_len =>
                {
                    existing.to_vec()
                }
                _ => values.to_vec(),
            };
            for (idx, &v) in values.iter().enumerate() {
                if mask.is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false)) {
                    continue;
                }
                if let Some(slot) = next.get_mut(idx) {
                    *slot = [
                        apply_op_f(v[0], value_f, op),
                        apply_op_f(v[1], value_f, op),
                    ];
                }
            }
            AttributeStorage::Vec2(next)
        }
        crate::attributes::AttributeRef::Vec3(values) => {
            let expected_len = values.len();
            let mut next = match existing {
                Some(crate::attributes::AttributeRef::Vec3(existing))
                    if existing.len() == expected_len =>
                {
                    existing.to_vec()
                }
                _ => values.to_vec(),
            };
            for (idx, &v) in values.iter().enumerate() {
                if mask.is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false)) {
                    continue;
                }
                if let Some(slot) = next.get_mut(idx) {
                    *slot = [
                        apply_op_f(v[0], value_v3[0], op),
                        apply_op_f(v[1], value_v3[1], op),
                        apply_op_f(v[2], value_v3[2], op),
                    ];
                }
            }
            AttributeStorage::Vec3(next)
        }
        crate::attributes::AttributeRef::Vec4(values) => {
            let expected_len = values.len();
            let mut next = match existing {
                Some(crate::attributes::AttributeRef::Vec4(existing))
                    if existing.len() == expected_len =>
                {
                    existing.to_vec()
                }
                _ => values.to_vec(),
            };
            for (idx, &v) in values.iter().enumerate() {
                if mask.is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false)) {
                    continue;
                }
                if let Some(slot) = next.get_mut(idx) {
                    *slot = [
                        apply_op_f(v[0], value_f, op),
                        apply_op_f(v[1], value_f, op),
                        apply_op_f(v[2], value_f, op),
                        apply_op_f(v[3], value_f, op),
                    ];
                }
            }
            AttributeStorage::Vec4(next)
        }
    }
}

fn apply_op_f(value: f32, rhs: f32, op: i32) -> f32 {
    match op {
        0 => value + rhs,
        1 => value - rhs,
        2 => value * rhs,
        3 => {
            if rhs.abs() < 1.0e-6 {
                value
            } else {
                value / rhs
            }
        }
        _ => value,
    }
}

fn apply_op_i(value: i32, rhs: i32, op: i32) -> i32 {
    match op {
        0 => value.saturating_add(rhs),
        1 => value.saturating_sub(rhs),
        2 => value.saturating_mul(rhs),
        3 => {
            if rhs == 0 {
                value
            } else {
                value / rhs
            }
        }
        _ => value,
    }
}

