use std::collections::BTreeMap;

use glam::Vec3;

use crate::attributes::AttributeDomain;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::groups::build_group_mask;
use crate::mesh::Mesh;
use crate::nodes::{geometry_in, geometry_out, require_mesh_input};
use crate::splat::SplatGeo;

pub const NAME: &str = "Group";

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
            ("group".to_string(), ParamValue::String("group1".to_string())),
            ("domain".to_string(), ParamValue::Int(2)),
            ("shape".to_string(), ParamValue::String("box".to_string())),
            ("invert".to_string(), ParamValue::Bool(false)),
            ("base_group".to_string(), ParamValue::String(String::new())),
            ("center".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0])),
            ("size".to_string(), ParamValue::Vec3([1.0, 1.0, 1.0])),
            ("radius".to_string(), ParamValue::Float(1.0)),
            (
                "plane_origin".to_string(),
                ParamValue::Vec3([0.0, 0.0, 0.0]),
            ),
            (
                "plane_normal".to_string(),
                ParamValue::Vec3([0.0, 1.0, 0.0]),
            ),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut input = require_mesh_input(inputs, 0, "Group requires a mesh input")?;
    apply_to_mesh(params, &mut input)?;
    Ok(input)
}

pub(crate) fn apply_to_mesh(params: &NodeParams, mesh: &mut Mesh) -> Result<(), String> {
    let group_name = params.get_string("group", "").trim();
    if group_name.is_empty() {
        return Err("Group requires a group name".to_string());
    }
    let domain = match params.get_int("domain", 2).clamp(0, 2) {
        0 => AttributeDomain::Point,
        1 => AttributeDomain::Vertex,
        _ => AttributeDomain::Primitive,
    };
    let shape = params.get_string("shape", "box");
    let invert = params.get_bool("invert", false);
    let base_expr = params.get_string("base_group", "").trim();

    let len = mesh.attribute_domain_len(domain);
    let base_mask = if base_expr.is_empty() {
        None
    } else {
        build_group_mask(mesh.groups.map(domain), base_expr, len)
    };

    let mut mask = if shape.eq_ignore_ascii_case("group") {
        base_mask.clone().unwrap_or_else(|| vec![false; len])
    } else {
        let mut mask = Vec::with_capacity(len);
        for idx in 0..len {
            let keep = element_inside_mesh(mesh, domain, idx, params, shape);
            mask.push(keep);
        }
        if let Some(base) = &base_mask {
            for (dst, base_keep) in mask.iter_mut().zip(base.iter()) {
                *dst &= *base_keep;
            }
        }
        mask
    };

    if invert {
        for value in &mut mask {
            *value = !*value;
        }
    }

    mesh.groups
        .map_mut(domain)
        .insert(group_name.to_string(), mask);
    Ok(())
}

pub(crate) fn apply_to_splats(params: &NodeParams, splats: &mut SplatGeo) -> Result<(), String> {
    let domain = params.get_int("domain", 2);
    if domain != 2 {
        return Ok(());
    }

    let group_name = params.get_string("group", "").trim();
    if group_name.is_empty() {
        return Err("Group requires a group name".to_string());
    }
    let shape = params.get_string("shape", "box");
    let invert = params.get_bool("invert", false);
    let base_expr = params.get_string("base_group", "").trim();

    let len = splats.len();
    let base_mask = if base_expr.is_empty() {
        None
    } else {
        build_group_mask(&splats.groups, base_expr, len)
    };

    let mut mask = if shape.eq_ignore_ascii_case("group") {
        base_mask.clone().unwrap_or_else(|| vec![false; len])
    } else {
        let mut mask = Vec::with_capacity(len);
        for position in &splats.positions {
            let keep = crate::nodes::delete::is_inside(params, shape, Vec3::from(*position));
            mask.push(keep);
        }
        if let Some(base) = &base_mask {
            for (dst, base_keep) in mask.iter_mut().zip(base.iter()) {
                *dst &= *base_keep;
            }
        }
        mask
    };

    if invert {
        for value in &mut mask {
            *value = !*value;
        }
    }

    splats.groups.insert(group_name.to_string(), mask);
    Ok(())
}

fn element_inside_mesh(
    mesh: &Mesh,
    domain: AttributeDomain,
    index: usize,
    params: &NodeParams,
    shape: &str,
) -> bool {
    let position = match domain {
        AttributeDomain::Point => mesh.positions.get(index).copied(),
        AttributeDomain::Vertex => mesh
            .indices
            .get(index)
            .and_then(|idx| mesh.positions.get(*idx as usize))
            .copied(),
        AttributeDomain::Primitive => {
            let tri = match mesh.indices.get(index * 3..index * 3 + 3) {
                Some(tri) => tri,
                None => return false,
            };
            let p0 = match mesh.positions.get(tri[0] as usize) {
                Some(p0) => p0,
                None => return false,
            };
            let p1 = match mesh.positions.get(tri[1] as usize) {
                Some(p1) => p1,
                None => return false,
            };
            let p2 = match mesh.positions.get(tri[2] as usize) {
                Some(p2) => p2,
                None => return false,
            };
            Some([
                (p0[0] + p1[0] + p2[0]) / 3.0,
                (p0[1] + p1[1] + p2[1]) / 3.0,
                (p0[2] + p1[2] + p2[2]) / 3.0,
            ])
        }
        AttributeDomain::Detail => None,
    };

    position
        .map(|pos| crate::nodes::delete::is_inside(params, shape, Vec3::from(pos)))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::make_box;

    #[test]
    fn group_box_includes_primitives() {
        let mut mesh = make_box([2.0, 2.0, 2.0]);
        let params = NodeParams {
            values: BTreeMap::from([
                ("group".to_string(), ParamValue::String("keep".to_string())),
                ("domain".to_string(), ParamValue::Int(2)),
                ("shape".to_string(), ParamValue::String("box".to_string())),
                ("center".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0])),
                ("size".to_string(), ParamValue::Vec3([10.0, 10.0, 10.0])),
            ]),
        };

        apply_to_mesh(&params, &mut mesh).expect("group");
        let group = mesh.groups.map(AttributeDomain::Primitive).get("keep").unwrap();
        let count = group.iter().filter(|v| **v).count();
        assert_eq!(count, mesh.indices.len() / 3);
    }
}
