use std::collections::BTreeMap;

use crate::attributes::AttributeDomain;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::groups::{build_group_mask, group_expr_matches};
use crate::mesh::Mesh;
use crate::nodes::expand_utils::{expand_mask, mesh_adjacency, ExpandMode};
use crate::nodes::group_utils::{group_type_from_params, GroupType};
use crate::nodes::{geometry_in, geometry_out, require_mesh_input};
use crate::splat::SplatGeo;

pub const NAME: &str = "Group Expand";

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
            ("out_group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
            ("expand_mode".to_string(), ParamValue::Int(0)),
            ("iterations".to_string(), ParamValue::Int(1)),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut input = require_mesh_input(inputs, 0, "Group Expand requires a mesh input")?;
    apply_to_mesh(params, &mut input)?;
    Ok(input)
}

pub(crate) fn apply_to_mesh(params: &NodeParams, mesh: &mut Mesh) -> Result<(), String> {
    let group_expr = params.get_string("group", "").trim().to_string();
    if group_expr.is_empty() {
        return Err("Group Expand requires a group name".to_string());
    }
    let output_group = output_group_name(&group_expr, params)?;
    let group_type = group_type_from_params(params);
    let domain = select_group_domain_mesh(mesh, &group_expr, group_type);
    let len = mesh.attribute_domain_len(domain);
    if len == 0 {
        return Ok(());
    }
    let Some(mask) = build_group_mask(mesh.groups.map(domain), &group_expr, len) else {
        return Err("Group Expand requires an existing group".to_string());
    };
    let iterations = params.get_int("iterations", 1).max(0) as usize;
    let mode = expand_mode_from_params(params);
    let neighbors = mesh_adjacency(mesh, domain);
    let expanded = if neighbors.is_empty() {
        mask
    } else {
        expand_mask(&mask, &neighbors, iterations, mode)
    };
    mesh.groups.map_mut(domain).insert(output_group, expanded);
    Ok(())
}

pub(crate) fn apply_to_splats(
    params: &NodeParams,
    splats: &mut SplatGeo,
) -> Result<(), String> {
    let group_expr = params.get_string("group", "").trim().to_string();
    if group_expr.is_empty() {
        return Err("Group Expand requires a group name".to_string());
    }
    let output_group = output_group_name(&group_expr, params)?;
    let group_type = group_type_from_params(params);
    let domain = select_group_domain_splats(splats, &group_expr, group_type);
    let len = splats.attribute_domain_len(domain);
    if len == 0 {
        return Ok(());
    }
    let mut groups = splats.groups.map(domain).clone();
    if len > 0 {
        groups
            .entry("splats".to_string())
            .or_insert_with(|| vec![true; len]);
    }
    let Some(mask) = build_group_mask(&groups, &group_expr, len) else {
        return Err("Group Expand requires an existing group".to_string());
    };
    let expanded = mask;
    splats.groups.map_mut(domain).insert(output_group, expanded);
    Ok(())
}

fn expand_mode_from_params(params: &NodeParams) -> ExpandMode {
    match params.get_int("expand_mode", 0) {
        1 => ExpandMode::Contract,
        _ => ExpandMode::Expand,
    }
}

fn output_group_name(group_expr: &str, params: &NodeParams) -> Result<String, String> {
    let out = params.get_string("out_group", "").trim().to_string();
    if !out.is_empty() {
        return Ok(out);
    }
    let trimmed = group_expr.trim();
    let has_separators = trimmed
        .chars()
        .any(|c| c.is_whitespace() || c == ',' || c == ';');
    let has_prefix = trimmed.starts_with('^') || trimmed.starts_with('!');
    let has_wildcards = trimmed.contains('*') || trimmed.contains('?');
    if !has_separators && !has_prefix && !has_wildcards {
        return Ok(trimmed.to_string());
    }
    Err("Group Expand requires an output group name when using group patterns".to_string())
}

fn select_group_domain_mesh(mesh: &Mesh, expr: &str, group_type: GroupType) -> AttributeDomain {
    match group_type {
        GroupType::Vertex => AttributeDomain::Vertex,
        GroupType::Point => AttributeDomain::Point,
        GroupType::Primitive => AttributeDomain::Primitive,
        GroupType::Auto => {
            if group_expr_matches(mesh.groups.map(AttributeDomain::Vertex), expr) {
                AttributeDomain::Vertex
            } else if group_expr_matches(mesh.groups.map(AttributeDomain::Point), expr) {
                AttributeDomain::Point
            } else {
                AttributeDomain::Primitive
            }
        }
    }
}

fn select_group_domain_splats(
    splats: &SplatGeo,
    expr: &str,
    group_type: GroupType,
) -> AttributeDomain {
    match group_type {
        GroupType::Point => AttributeDomain::Point,
        GroupType::Primitive => AttributeDomain::Primitive,
        GroupType::Vertex => AttributeDomain::Point,
        GroupType::Auto => {
            let mut point_groups = splats.groups.map(AttributeDomain::Point).clone();
            if !splats.is_empty() {
                point_groups
                    .entry("splats".to_string())
                    .or_insert_with(|| vec![true; splats.len()]);
            }
            if group_expr_matches(&point_groups, expr) {
                AttributeDomain::Point
            } else {
                AttributeDomain::Primitive
            }
        }
    }
}
