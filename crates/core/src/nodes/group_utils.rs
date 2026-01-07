use crate::attributes::AttributeDomain;
use crate::graph::NodeParams;
use crate::groups::{build_group_mask, group_expr_matches};
use crate::mesh::Mesh;
use crate::splat::SplatGeo;

pub const GROUP_PARAM: &str = "group";
pub const GROUP_TYPE_PARAM: &str = "group_type";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupType {
    Auto,
    Vertex,
    Point,
    Primitive,
}

pub fn group_type_from_params(params: &NodeParams) -> GroupType {
    match params.get_int(GROUP_TYPE_PARAM, 0) {
        1 => GroupType::Vertex,
        2 => GroupType::Point,
        3 => GroupType::Primitive,
        _ => GroupType::Auto,
    }
}

pub fn mask_has_any(mask: Option<&[bool]>) -> bool {
    match mask {
        Some(mask) => mask.iter().any(|value| *value),
        None => true,
    }
}

pub fn mesh_group_mask(
    mesh: &Mesh,
    params: &NodeParams,
    target_domain: AttributeDomain,
) -> Option<Vec<bool>> {
    let expr = params.get_string(GROUP_PARAM, "").trim();
    if expr.is_empty() {
        return None;
    }
    let group_type = group_type_from_params(params);
    let source_domain = select_group_domain(mesh, expr, group_type);
    let source_len = mesh.attribute_domain_len(source_domain);
    let mask = build_group_mask(mesh.groups.map(source_domain), expr, source_len)?;
    Some(map_group_mask(mesh, source_domain, target_domain, &mask))
}

pub fn splat_group_mask(
    splats: &SplatGeo,
    params: &NodeParams,
    target_domain: AttributeDomain,
) -> Option<Vec<bool>> {
    let expr = params.get_string(GROUP_PARAM, "").trim();
    if expr.is_empty() {
        return None;
    }
    let group_type = group_type_from_params(params);
    let len = splats.len();
    let point_groups = splat_group_map_with_intrinsic(splats, AttributeDomain::Point);
    let prim_groups = splat_group_map_with_intrinsic(splats, AttributeDomain::Primitive);
    let mask = match group_type {
        GroupType::Point => build_group_mask(&point_groups, expr, len),
        GroupType::Primitive => build_group_mask(&prim_groups, expr, len),
        GroupType::Vertex => Some(vec![false; len]),
        GroupType::Auto => {
            let domain = if group_expr_matches(&point_groups, expr) {
                AttributeDomain::Point
            } else if group_expr_matches(&prim_groups, expr) {
                AttributeDomain::Primitive
            } else {
                AttributeDomain::Point
            };
            let groups = match domain {
                AttributeDomain::Primitive => &prim_groups,
                _ => &point_groups,
            };
            build_group_mask(groups, expr, len)
        }
    };

    match target_domain {
        AttributeDomain::Detail => mask.map(|mask| vec![mask.iter().any(|value| *value)]),
        AttributeDomain::Vertex => Some(vec![false; splats.attribute_domain_len(target_domain)]),
        _ => mask,
    }
}

fn splat_group_map_with_intrinsic(
    splats: &SplatGeo,
    domain: AttributeDomain,
) -> std::collections::BTreeMap<String, Vec<bool>> {
    let mut map = splats.groups.map(domain).clone();
    if splats.len() > 0 {
        map.entry("splats".to_string())
            .or_insert_with(|| vec![true; splats.len()]);
    }
    map
}

fn select_group_domain(mesh: &Mesh, expr: &str, group_type: GroupType) -> AttributeDomain {
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

fn map_group_mask(
    mesh: &Mesh,
    source_domain: AttributeDomain,
    target_domain: AttributeDomain,
    mask: &[bool],
) -> Vec<bool> {
    if source_domain == target_domain {
        return mask.to_vec();
    }
    let target_len = mesh.attribute_domain_len(target_domain);
    if target_len == 0 {
        return Vec::new();
    }
    if target_domain == AttributeDomain::Detail {
        let any = mask.iter().any(|value| *value);
        return vec![any];
    }

    let mut out = vec![false; target_len];
    match (source_domain, target_domain) {
        (AttributeDomain::Point, AttributeDomain::Vertex) => {
            for (vertex_index, point_index) in mesh.indices.iter().enumerate() {
                if mask.get(*point_index as usize).copied().unwrap_or(false) {
                    if let Some(slot) = out.get_mut(vertex_index) {
                        *slot = true;
                    }
                }
            }
        }
        (AttributeDomain::Point, AttributeDomain::Primitive) => {
            for (prim_index, tri) in mesh.indices.chunks_exact(3).enumerate() {
                let a = tri[0] as usize;
                let b = tri[1] as usize;
                let c = tri[2] as usize;
                if mask.get(a).copied().unwrap_or(false)
                    || mask.get(b).copied().unwrap_or(false)
                    || mask.get(c).copied().unwrap_or(false)
                {
                    if let Some(slot) = out.get_mut(prim_index) {
                        *slot = true;
                    }
                }
            }
        }
        (AttributeDomain::Vertex, AttributeDomain::Point) => {
            for (vertex_index, point_index) in mesh.indices.iter().enumerate() {
                if mask.get(vertex_index).copied().unwrap_or(false) {
                    if let Some(slot) = out.get_mut(*point_index as usize) {
                        *slot = true;
                    }
                }
            }
        }
        (AttributeDomain::Vertex, AttributeDomain::Primitive) => {
            for (prim_index, _tri) in mesh.indices.chunks_exact(3).enumerate() {
                let base = prim_index * 3;
                if mask.get(base).copied().unwrap_or(false)
                    || mask.get(base + 1).copied().unwrap_or(false)
                    || mask.get(base + 2).copied().unwrap_or(false)
                {
                    if let Some(slot) = out.get_mut(prim_index) {
                        *slot = true;
                    }
                }
            }
        }
        (AttributeDomain::Primitive, AttributeDomain::Point) => {
            for (prim_index, tri) in mesh.indices.chunks_exact(3).enumerate() {
                if mask.get(prim_index).copied().unwrap_or(false) {
                    for &idx in tri {
                        if let Some(slot) = out.get_mut(idx as usize) {
                            *slot = true;
                        }
                    }
                }
            }
        }
        (AttributeDomain::Primitive, AttributeDomain::Vertex) => {
            for (prim_index, _tri) in mesh.indices.chunks_exact(3).enumerate() {
                if mask.get(prim_index).copied().unwrap_or(false) {
                    let base = prim_index * 3;
                    if let Some(slot) = out.get_mut(base) {
                        *slot = true;
                    }
                    if let Some(slot) = out.get_mut(base + 1) {
                        *slot = true;
                    }
                    if let Some(slot) = out.get_mut(base + 2) {
                        *slot = true;
                    }
                }
            }
        }
        _ => {}
    }
    out
}
