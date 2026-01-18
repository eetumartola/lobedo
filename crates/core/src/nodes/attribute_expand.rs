use std::collections::BTreeMap;

use glam::{Vec2, Vec3, Vec4};
use tracing::warn;

use crate::attributes::{AttributeDomain, AttributeRef, AttributeStorage};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::attribute_utils::domain_from_params;
use crate::nodes::expand_utils::{mesh_adjacency, ExpandMode};
use crate::nodes::group_utils::{mask_has_any, mesh_group_mask, splat_group_mask};
use crate::nodes::{geometry_in, geometry_out, recompute_mesh_normals, require_mesh_input};
use crate::splat::SplatGeo;

pub const NAME: &str = "Attribute Expand";

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
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
            ("attr".to_string(), ParamValue::String("Cd".to_string())),
            ("domain".to_string(), ParamValue::Int(0)),
            ("expand_mode".to_string(), ParamValue::Int(0)),
            ("iterations".to_string(), ParamValue::Int(1)),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut input =
        require_mesh_input(inputs, 0, "Attribute Expand requires a mesh input")?;
    apply_to_mesh(params, &mut input)?;
    Ok(input)
}

pub(crate) fn apply_to_mesh(params: &NodeParams, mesh: &mut Mesh) -> Result<(), String> {
    let attr = params.get_string("attr", "Cd").trim().to_string();
    if attr.is_empty() {
        return Ok(());
    }
    let domain = domain_from_params(params);
    if domain == AttributeDomain::Detail {
        return Ok(());
    }
    let Some(attr_ref) = mesh.attribute(domain, &attr) else {
        warn!(
            "Attribute Expand: '{}' not found on {:?}; passing input through",
            attr, domain
        );
        return Ok(());
    };
    let count = mesh.attribute_domain_len(domain);
    if count == 0 {
        return Ok(());
    }
    let mask = mesh_group_mask(mesh, params, domain);
    if !mask_has_any(mask.as_deref()) {
        return Ok(());
    }
    let iterations = params.get_int("iterations", 1).max(0) as usize;
    let mode = expand_mode_from_params(params);
    let neighbors = mesh_adjacency(mesh, domain);

    let storage = match attr_ref {
        AttributeRef::Float(values) => {
            AttributeStorage::Float(expand_scalar(values, &neighbors, mask.as_deref(), iterations, mode))
        }
        AttributeRef::Int(values) => {
            AttributeStorage::Int(expand_int(values, &neighbors, mask.as_deref(), iterations, mode))
        }
        AttributeRef::Vec2(values) => AttributeStorage::Vec2(expand_vec2(
            values,
            &neighbors,
            mask.as_deref(),
            iterations,
            mode,
        )),
        AttributeRef::Vec3(values) => AttributeStorage::Vec3(expand_vec3(
            values,
            &neighbors,
            mask.as_deref(),
            iterations,
            mode,
        )),
        AttributeRef::Vec4(values) => AttributeStorage::Vec4(expand_vec4(
            values,
            &neighbors,
            mask.as_deref(),
            iterations,
            mode,
        )),
        AttributeRef::StringTable(_) => {
            warn!("Attribute Expand: string attributes are not supported; passing input through");
            return Ok(());
        }
    };

    let modifies_positions = domain == AttributeDomain::Point && attr == "P";
    mesh.set_attribute(domain, attr, storage)
        .map_err(|err| format!("Attribute Expand error: {:?}", err))?;
    if modifies_positions {
        recompute_mesh_normals(mesh);
    }
    Ok(())
}

pub(crate) fn apply_to_splats(params: &NodeParams, splats: &mut SplatGeo) -> Result<(), String> {
    let attr = params.get_string("attr", "Cd").trim().to_string();
    if attr.is_empty() {
        return Ok(());
    }
    let domain = domain_from_params(params);
    if domain == AttributeDomain::Detail || domain == AttributeDomain::Vertex {
        return Ok(());
    }
    let Some(attr_ref) = splats.attribute(domain, &attr) else {
        warn!(
            "Attribute Expand: '{}' not found on {:?}; passing input through",
            attr, domain
        );
        return Ok(());
    };
    let count = splats.attribute_domain_len(domain);
    if count == 0 {
        return Ok(());
    }
    let mask = splat_group_mask(splats, params, domain);
    if !mask_has_any(mask.as_deref()) {
        return Ok(());
    }
    let iterations = params.get_int("iterations", 1).max(0) as usize;
    let mode = expand_mode_from_params(params);
    let neighbors = vec![Vec::new(); count];

    let storage = match attr_ref {
        AttributeRef::Float(values) => {
            AttributeStorage::Float(expand_scalar(values, &neighbors, mask.as_deref(), iterations, mode))
        }
        AttributeRef::Int(values) => {
            AttributeStorage::Int(expand_int(values, &neighbors, mask.as_deref(), iterations, mode))
        }
        AttributeRef::Vec2(values) => AttributeStorage::Vec2(expand_vec2(
            values,
            &neighbors,
            mask.as_deref(),
            iterations,
            mode,
        )),
        AttributeRef::Vec3(values) => AttributeStorage::Vec3(expand_vec3(
            values,
            &neighbors,
            mask.as_deref(),
            iterations,
            mode,
        )),
        AttributeRef::Vec4(values) => AttributeStorage::Vec4(expand_vec4(
            values,
            &neighbors,
            mask.as_deref(),
            iterations,
            mode,
        )),
        AttributeRef::StringTable(_) => {
            warn!("Attribute Expand: string attributes are not supported; passing input through");
            return Ok(());
        }
    };

    splats
        .set_attribute(domain, attr, storage)
        .map_err(|err| format!("Attribute Expand error: {:?}", err))?;
    Ok(())
}

fn expand_mode_from_params(params: &NodeParams) -> ExpandMode {
    match params.get_int("expand_mode", 0) {
        1 => ExpandMode::Contract,
        _ => ExpandMode::Expand,
    }
}

fn expand_scalar(
    values: &[f32],
    neighbors: &[Vec<usize>],
    mask: Option<&[bool]>,
    iterations: usize,
    mode: ExpandMode,
) -> Vec<f32> {
    if iterations == 0 || values.is_empty() {
        return values.to_vec();
    }
    let len = values.len().min(neighbors.len());
    if len == 0 {
        return values.to_vec();
    }
    let mut current = values.to_vec();
    for _ in 0..iterations {
        let mut next = current.clone();
        for i in 0..len {
            if let Some(mask) = mask {
                if !mask.get(i).copied().unwrap_or(false) {
                    continue;
                }
            }
            let list = neighbors.get(i).map(|list| list.as_slice()).unwrap_or(&[]);
            if list.is_empty() {
                continue;
            }
            let mut candidate = current[i];
            for &n in list {
                if let Some(value) = current.get(n).copied() {
                    candidate = match mode {
                        ExpandMode::Expand => candidate.max(value),
                        ExpandMode::Contract => candidate.min(value),
                    };
                }
            }
            next[i] = candidate;
        }
        current = next;
    }
    current
}

fn expand_int(
    values: &[i32],
    neighbors: &[Vec<usize>],
    mask: Option<&[bool]>,
    iterations: usize,
    mode: ExpandMode,
) -> Vec<i32> {
    if iterations == 0 || values.is_empty() {
        return values.to_vec();
    }
    let len = values.len().min(neighbors.len());
    if len == 0 {
        return values.to_vec();
    }
    let mut current = values.to_vec();
    for _ in 0..iterations {
        let mut next = current.clone();
        for i in 0..len {
            if let Some(mask) = mask {
                if !mask.get(i).copied().unwrap_or(false) {
                    continue;
                }
            }
            let list = neighbors.get(i).map(|list| list.as_slice()).unwrap_or(&[]);
            if list.is_empty() {
                continue;
            }
            let mut candidate = current[i];
            for &n in list {
                if let Some(value) = current.get(n).copied() {
                    candidate = match mode {
                        ExpandMode::Expand => candidate.max(value),
                        ExpandMode::Contract => candidate.min(value),
                    };
                }
            }
            next[i] = candidate;
        }
        current = next;
    }
    current
}

fn expand_vec2(
    values: &[[f32; 2]],
    neighbors: &[Vec<usize>],
    mask: Option<&[bool]>,
    iterations: usize,
    mode: ExpandMode,
) -> Vec<[f32; 2]> {
    if iterations == 0 || values.is_empty() {
        return values.to_vec();
    }
    let len = values.len().min(neighbors.len());
    if len == 0 {
        return values.to_vec();
    }
    let mut current = values.to_vec();
    for _ in 0..iterations {
        let mut next = current.clone();
        for i in 0..len {
            if let Some(mask) = mask {
                if !mask.get(i).copied().unwrap_or(false) {
                    continue;
                }
            }
            let list = neighbors.get(i).map(|list| list.as_slice()).unwrap_or(&[]);
            if list.is_empty() {
                continue;
            }
            let mut candidate = Vec2::from(current[i]);
            for &n in list {
                if let Some(value) = current.get(n).copied() {
                    let v = Vec2::from(value);
                    candidate = match mode {
                        ExpandMode::Expand => candidate.max(v),
                        ExpandMode::Contract => candidate.min(v),
                    };
                }
            }
            next[i] = candidate.to_array();
        }
        current = next;
    }
    current
}

fn expand_vec3(
    values: &[[f32; 3]],
    neighbors: &[Vec<usize>],
    mask: Option<&[bool]>,
    iterations: usize,
    mode: ExpandMode,
) -> Vec<[f32; 3]> {
    if iterations == 0 || values.is_empty() {
        return values.to_vec();
    }
    let len = values.len().min(neighbors.len());
    if len == 0 {
        return values.to_vec();
    }
    let mut current = values.to_vec();
    for _ in 0..iterations {
        let mut next = current.clone();
        for i in 0..len {
            if let Some(mask) = mask {
                if !mask.get(i).copied().unwrap_or(false) {
                    continue;
                }
            }
            let list = neighbors.get(i).map(|list| list.as_slice()).unwrap_or(&[]);
            if list.is_empty() {
                continue;
            }
            let mut candidate = Vec3::from(current[i]);
            for &n in list {
                if let Some(value) = current.get(n).copied() {
                    let v = Vec3::from(value);
                    candidate = match mode {
                        ExpandMode::Expand => candidate.max(v),
                        ExpandMode::Contract => candidate.min(v),
                    };
                }
            }
            next[i] = candidate.to_array();
        }
        current = next;
    }
    current
}

fn expand_vec4(
    values: &[[f32; 4]],
    neighbors: &[Vec<usize>],
    mask: Option<&[bool]>,
    iterations: usize,
    mode: ExpandMode,
) -> Vec<[f32; 4]> {
    if iterations == 0 || values.is_empty() {
        return values.to_vec();
    }
    let len = values.len().min(neighbors.len());
    if len == 0 {
        return values.to_vec();
    }
    let mut current = values.to_vec();
    for _ in 0..iterations {
        let mut next = current.clone();
        for i in 0..len {
            if let Some(mask) = mask {
                if !mask.get(i).copied().unwrap_or(false) {
                    continue;
                }
            }
            let list = neighbors.get(i).map(|list| list.as_slice()).unwrap_or(&[]);
            if list.is_empty() {
                continue;
            }
            let mut candidate = Vec4::from(current[i]);
            for &n in list {
                if let Some(value) = current.get(n).copied() {
                    let v = Vec4::from(value);
                    candidate = match mode {
                        ExpandMode::Expand => candidate.max(v),
                        ExpandMode::Contract => candidate.min(v),
                    };
                }
            }
            next[i] = candidate.to_array();
        }
        current = next;
    }
    current
}
