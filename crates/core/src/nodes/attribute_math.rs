use std::collections::BTreeMap;

use tracing::warn;

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{mesh_in, mesh_out, require_mesh_input};

pub const NAME: &str = "Attribute Math";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![mesh_in("in")],
        outputs: vec![mesh_out("out")],
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
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut input = require_mesh_input(inputs, 0, "Attribute Math requires a mesh input")?;
    let attr = params.get_string("attr", "Cd");
    let result = params.get_string("result", attr);
    let domain = match params.get_int("domain", 0).clamp(0, 3) {
        0 => AttributeDomain::Point,
        1 => AttributeDomain::Vertex,
        2 => AttributeDomain::Primitive,
        _ => AttributeDomain::Detail,
    };
    let op = params.get_int("op", 0).clamp(0, 3);
    let value_f = params.get_float("value_f", 0.0);
    let value_v3 = params.get_vec3("value_v3", [0.0, 0.0, 0.0]);

    let attr_ref = match input.attribute(domain, attr) {
        Some(attr_ref) => attr_ref,
        None => {
            warn!(
                "Attribute Math: '{}' not found on {:?}; passing input through",
                attr, domain
            );
            return Ok(input);
        }
    };
    match attr_ref {
        crate::attributes::AttributeRef::Float(values) => {
            let mut next = Vec::with_capacity(values.len());
            for &v in values {
                next.push(apply_op_f(v, value_f, op));
            }
            input
                .set_attribute(domain, result, AttributeStorage::Float(next))
                .map_err(|err| format!("Attribute Math error: {:?}", err))?;
        }
        crate::attributes::AttributeRef::Int(values) => {
            let mut next = Vec::with_capacity(values.len());
            let value_i = value_f.round() as i32;
            for &v in values {
                next.push(apply_op_i(v, value_i, op));
            }
            input
                .set_attribute(domain, result, AttributeStorage::Int(next))
                .map_err(|err| format!("Attribute Math error: {:?}", err))?;
        }
        crate::attributes::AttributeRef::Vec2(values) => {
            let mut next = Vec::with_capacity(values.len());
            for &v in values {
                next.push([apply_op_f(v[0], value_f, op), apply_op_f(v[1], value_f, op)]);
            }
            input
                .set_attribute(domain, result, AttributeStorage::Vec2(next))
                .map_err(|err| format!("Attribute Math error: {:?}", err))?;
        }
        crate::attributes::AttributeRef::Vec3(values) => {
            let mut next = Vec::with_capacity(values.len());
            for &v in values {
                next.push([
                    apply_op_f(v[0], value_v3[0], op),
                    apply_op_f(v[1], value_v3[1], op),
                    apply_op_f(v[2], value_v3[2], op),
                ]);
            }
            input
                .set_attribute(domain, result, AttributeStorage::Vec3(next))
                .map_err(|err| format!("Attribute Math error: {:?}", err))?;
        }
        crate::attributes::AttributeRef::Vec4(values) => {
            let mut next = Vec::with_capacity(values.len());
            for &v in values {
                next.push([
                    apply_op_f(v[0], value_f, op),
                    apply_op_f(v[1], value_f, op),
                    apply_op_f(v[2], value_f, op),
                    apply_op_f(v[3], value_f, op),
                ]);
            }
            input
                .set_attribute(domain, result, AttributeStorage::Vec4(next))
                .map_err(|err| format!("Attribute Math error: {:?}", err))?;
        }
    }
    Ok(input)
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
