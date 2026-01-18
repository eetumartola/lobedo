use std::collections::{BTreeMap, HashMap};

use tracing::warn;

use crate::attributes::{AttributeDomain, AttributeRef, AttributeStorage};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::attribute_utils::{domain_from_params, parse_attribute_list};
use crate::nodes::{geometry_in, geometry_out, require_mesh_input};
use crate::splat::SplatGeo;

pub const NAME: &str = "Attribute Promote";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PromotionMethod {
    Max,
    Min,
    Average,
    Mode,
    Median,
    Sum,
    SumSquares,
    RootMeanSquare,
    First,
    Last,
}

impl PromotionMethod {
    fn from_params(params: &NodeParams) -> Self {
        match params.get_int("promotion", 2) {
            0 => PromotionMethod::Max,
            1 => PromotionMethod::Min,
            2 => PromotionMethod::Average,
            3 => PromotionMethod::Mode,
            4 => PromotionMethod::Median,
            5 => PromotionMethod::Sum,
            6 => PromotionMethod::SumSquares,
            7 => PromotionMethod::RootMeanSquare,
            8 => PromotionMethod::First,
            9 => PromotionMethod::Last,
            _ => PromotionMethod::Average,
        }
    }
}

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
            ("source_domain".to_string(), ParamValue::Int(0)),
            ("target_domain".to_string(), ParamValue::Int(2)),
            ("promotion".to_string(), ParamValue::Int(2)),
            ("rename".to_string(), ParamValue::Bool(false)),
            ("new_name".to_string(), ParamValue::String(String::new())),
            ("delete_original".to_string(), ParamValue::Bool(false)),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut input =
        require_mesh_input(inputs, 0, "Attribute Promote requires a mesh input")?;
    apply_to_mesh(params, &mut input)?;
    Ok(input)
}

pub(crate) fn apply_to_mesh(params: &NodeParams, mesh: &mut Mesh) -> Result<(), String> {
    let attr_expr = params.get_string("attr", "");
    let source_domain = source_domain_from_params(params);
    let target_domain = target_domain_from_params(params);
    if source_domain == target_domain {
        return Err("Attribute Promote requires different source/target classes".to_string());
    }
    let method = PromotionMethod::from_params(params);
    let delete_original = params.get_bool("delete_original", false);
    let rename = params.get_bool("rename", false);
    let new_name = params.get_string("new_name", "");

    let attr_names = collect_attribute_names_mesh(mesh, source_domain, &attr_expr);
    if attr_names.is_empty() {
        return Err("Attribute Promote requires an attribute name".to_string());
    }
    let source_len = mesh.attribute_domain_len(source_domain);
    let target_len = mesh.attribute_domain_len(target_domain);
    if source_len == 0 || target_len == 0 {
        return Ok(());
    }
    let mapping = build_mapping(mesh, source_domain, target_domain);
    if mapping.is_empty() {
        return Ok(());
    }

    for attr_name in attr_names {
        let Some(attr_ref) = mesh.attribute(source_domain, &attr_name) else {
            warn!(
                "Attribute Promote: '{}' not found on {:?}; skipping",
                attr_name, source_domain
            );
            continue;
        };
        let out_name = resolve_output_name(&attr_name, &new_name, rename, mapping.len());
        let storage = promote_attribute(attr_ref, &mapping, method);
        if let Some(storage) = storage {
            mesh.set_attribute(target_domain, out_name, storage)
                .map_err(|err| format!("Attribute Promote error: {:?}", err))?;
            if delete_original {
                let _ = mesh.remove_attribute(source_domain, &attr_name);
            }
        }
    }
    Ok(())
}

pub(crate) fn apply_to_splats(
    params: &NodeParams,
    splats: &mut SplatGeo,
) -> Result<(), String> {
    let attr_expr = params.get_string("attr", "");
    let source_domain = source_domain_from_params(params);
    let target_domain = target_domain_from_params(params);
    if source_domain == AttributeDomain::Vertex || target_domain == AttributeDomain::Vertex {
        return Err("Attribute Promote does not support vertex class for splats".to_string());
    }
    if source_domain == target_domain {
        return Err("Attribute Promote requires different source/target classes".to_string());
    }
    let method = PromotionMethod::from_params(params);
    let delete_original = params.get_bool("delete_original", false);
    let rename = params.get_bool("rename", false);
    let new_name = params.get_string("new_name", "");

    let attr_names = collect_attribute_names_splats(splats, source_domain, &attr_expr);
    if attr_names.is_empty() {
        return Err("Attribute Promote requires an attribute name".to_string());
    }
    let source_len = splats.attribute_domain_len(source_domain);
    let target_len = splats.attribute_domain_len(target_domain);
    if source_len == 0 || target_len == 0 {
        return Ok(());
    }
    let mapping = build_mapping_splats(source_len, target_len, source_domain, target_domain);
    if mapping.is_empty() {
        return Ok(());
    }

    for attr_name in attr_names {
        let Some(attr_ref) = splats.attribute(source_domain, &attr_name) else {
            warn!(
                "Attribute Promote: '{}' not found on {:?}; skipping",
                attr_name, source_domain
            );
            continue;
        };
        let out_name = resolve_output_name(&attr_name, &new_name, rename, mapping.len());
        let storage = promote_attribute(attr_ref, &mapping, method);
        if let Some(storage) = storage {
            splats
                .set_attribute(target_domain, out_name, storage)
                .map_err(|err| format!("Attribute Promote error: {:?}", err))?;
            if delete_original {
                let _ = splats.attributes.remove(source_domain, &attr_name);
            }
        }
    }
    Ok(())
}

fn source_domain_from_params(params: &NodeParams) -> AttributeDomain {
    match params.get_int("source_domain", 0).clamp(0, 3) {
        0 => AttributeDomain::Point,
        1 => AttributeDomain::Vertex,
        2 => AttributeDomain::Primitive,
        _ => AttributeDomain::Detail,
    }
}

fn target_domain_from_params(params: &NodeParams) -> AttributeDomain {
    match params.get_int("target_domain", 2).clamp(0, 3) {
        0 => AttributeDomain::Point,
        1 => AttributeDomain::Vertex,
        2 => AttributeDomain::Primitive,
        _ => AttributeDomain::Detail,
    }
}

fn collect_attribute_names_mesh(
    mesh: &Mesh,
    domain: AttributeDomain,
    expr: &str,
) -> Vec<String> {
    let patterns = parse_attribute_list(expr);
    if patterns.is_empty() {
        return Vec::new();
    }
    let available: Vec<String> = mesh
        .list_attributes()
        .into_iter()
        .filter(|info| info.domain == domain)
        .map(|info| info.name)
        .collect();
    resolve_attribute_patterns(&available, &patterns)
}

fn collect_attribute_names_splats(
    splats: &SplatGeo,
    domain: AttributeDomain,
    expr: &str,
) -> Vec<String> {
    let patterns = parse_attribute_list(expr);
    if patterns.is_empty() {
        return Vec::new();
    }
    let available: Vec<String> = splats
        .list_attributes()
        .into_iter()
        .filter(|info| info.domain == domain)
        .map(|info| info.name)
        .collect();
    resolve_attribute_patterns(&available, &patterns)
}

fn resolve_attribute_patterns(available: &[String], patterns: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for pattern in patterns {
        if pattern.contains('*') || pattern.contains('?') {
            for name in available {
                if glob_match(pattern, name) {
                    out.push(name.clone());
                }
            }
        } else {
            out.push(pattern.clone());
        }
    }
    out.sort();
    out.dedup();
    out
}

fn resolve_output_name(original: &str, new_name: &str, rename: bool, total: usize) -> String {
    if !rename {
        return original.to_string();
    }
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return original.to_string();
    }
    if trimmed.contains('*') {
        return trimmed.replace('*', original);
    }
    if total > 1 {
        format!("{trimmed}_{original}")
    } else {
        trimmed.to_string()
    }
}

fn promote_attribute(
    attr: AttributeRef<'_>,
    mapping: &[Vec<usize>],
    method: PromotionMethod,
) -> Option<AttributeStorage> {
    match attr {
        AttributeRef::Float(values) => Some(AttributeStorage::Float(promote_f32(
            values,
            mapping,
            method,
        ))),
        AttributeRef::Int(values) => Some(AttributeStorage::Int(promote_i32(
            values,
            mapping,
            method,
        ))),
        AttributeRef::Vec2(values) => Some(AttributeStorage::Vec2(promote_vec2(
            values,
            mapping,
            method,
        ))),
        AttributeRef::Vec3(values) => Some(AttributeStorage::Vec3(promote_vec3(
            values,
            mapping,
            method,
        ))),
        AttributeRef::Vec4(values) => Some(AttributeStorage::Vec4(promote_vec4(
            values,
            mapping,
            method,
        ))),
        AttributeRef::StringTable(values) => {
            let indices = promote_u32(values.indices.as_slice(), mapping, method);
            Some(AttributeStorage::StringTable(
                crate::attributes::StringTableAttribute::new(values.values.clone(), indices),
            ))
        }
    }
}

fn promote_f32(values: &[f32], mapping: &[Vec<usize>], method: PromotionMethod) -> Vec<f32> {
    let mut out = vec![0.0; mapping.len()];
    for (i, sources) in mapping.iter().enumerate() {
        if sources.is_empty() {
            continue;
        }
        let mut list = Vec::with_capacity(sources.len());
        for &idx in sources {
            if let Some(v) = values.get(idx).copied() {
                if v.is_finite() {
                    list.push(v);
                }
            }
        }
        if list.is_empty() {
            continue;
        }
        out[i] = match method {
            PromotionMethod::Max => list
                .iter()
                .copied()
                .fold(f32::NEG_INFINITY, f32::max),
            PromotionMethod::Min => list
                .iter()
                .copied()
                .fold(f32::INFINITY, f32::min),
            PromotionMethod::Average => list.iter().sum::<f32>() / list.len() as f32,
            PromotionMethod::Mode => mode_f32(&list),
            PromotionMethod::Median => median_f32(&mut list),
            PromotionMethod::Sum => list.iter().sum(),
            PromotionMethod::SumSquares => list.iter().map(|v| v * v).sum(),
            PromotionMethod::RootMeanSquare => {
                (list.iter().map(|v| v * v).sum::<f32>() / list.len() as f32).sqrt()
            }
            PromotionMethod::First => list.first().copied().unwrap_or(0.0),
            PromotionMethod::Last => list.last().copied().unwrap_or(0.0),
        };
    }
    out
}

fn promote_i32(values: &[i32], mapping: &[Vec<usize>], method: PromotionMethod) -> Vec<i32> {
    let mut out = vec![0; mapping.len()];
    for (i, sources) in mapping.iter().enumerate() {
        if sources.is_empty() {
            continue;
        }
        let mut list = Vec::with_capacity(sources.len());
        for &idx in sources {
            if let Some(v) = values.get(idx).copied() {
                list.push(v);
            }
        }
        if list.is_empty() {
            continue;
        }
        out[i] = match method {
            PromotionMethod::Max => *list.iter().max().unwrap_or(&0),
            PromotionMethod::Min => *list.iter().min().unwrap_or(&0),
            PromotionMethod::Average => {
                let sum: i64 = list.iter().map(|v| *v as i64).sum();
                (sum as f32 / list.len() as f32).round() as i32
            }
            PromotionMethod::Mode => mode_i32(&list),
            PromotionMethod::Median => median_i32(&mut list),
            PromotionMethod::Sum => list.iter().map(|v| *v as i64).sum::<i64>() as i32,
            PromotionMethod::SumSquares => list
                .iter()
                .map(|v| (*v as i64) * (*v as i64))
                .sum::<i64>() as i32,
            PromotionMethod::RootMeanSquare => {
                let sum_sq = list
                    .iter()
                    .map(|v| (*v as f32) * (*v as f32))
                    .sum::<f32>();
                (sum_sq / list.len() as f32).sqrt().round() as i32
            }
            PromotionMethod::First => *list.first().unwrap_or(&0),
            PromotionMethod::Last => *list.last().unwrap_or(&0),
        };
    }
    out
}

fn promote_u32(values: &[u32], mapping: &[Vec<usize>], method: PromotionMethod) -> Vec<u32> {
    let mut out = vec![0; mapping.len()];
    for (i, sources) in mapping.iter().enumerate() {
        if sources.is_empty() {
            continue;
        }
        let mut list = Vec::with_capacity(sources.len());
        for &idx in sources {
            if let Some(v) = values.get(idx).copied() {
                list.push(v);
            }
        }
        if list.is_empty() {
            continue;
        }
        out[i] = match method {
            PromotionMethod::Max => *list.iter().max().unwrap_or(&0),
            PromotionMethod::Min => *list.iter().min().unwrap_or(&0),
            PromotionMethod::Average => {
                let sum: u64 = list.iter().map(|v| *v as u64).sum();
                ((sum as f32 / list.len() as f32).round() as i32).max(0) as u32
            }
            PromotionMethod::Mode => mode_u32(&list),
            PromotionMethod::Median => median_u32(&mut list),
            PromotionMethod::Sum => list.iter().map(|v| *v as u64).sum::<u64>() as u32,
            PromotionMethod::SumSquares => list
                .iter()
                .map(|v| (*v as u64) * (*v as u64))
                .sum::<u64>() as u32,
            PromotionMethod::RootMeanSquare => {
                let sum_sq = list
                    .iter()
                    .map(|v| (*v as f32) * (*v as f32))
                    .sum::<f32>();
                (sum_sq / list.len() as f32).sqrt().round() as u32
            }
            PromotionMethod::First => *list.first().unwrap_or(&0),
            PromotionMethod::Last => *list.last().unwrap_or(&0),
        };
    }
    out
}

fn promote_vec2(
    values: &[[f32; 2]],
    mapping: &[Vec<usize>],
    method: PromotionMethod,
) -> Vec<[f32; 2]> {
    let mut out = vec![[0.0; 2]; mapping.len()];
    for (i, sources) in mapping.iter().enumerate() {
        if sources.is_empty() {
            continue;
        }
        let mut list_x = Vec::with_capacity(sources.len());
        let mut list_y = Vec::with_capacity(sources.len());
        for &idx in sources {
            if let Some(v) = values.get(idx).copied() {
                list_x.push(v[0]);
                list_y.push(v[1]);
            }
        }
        out[i] = [
            promote_scalar(&mut list_x, method),
            promote_scalar(&mut list_y, method),
        ];
    }
    out
}

fn promote_vec3(
    values: &[[f32; 3]],
    mapping: &[Vec<usize>],
    method: PromotionMethod,
) -> Vec<[f32; 3]> {
    let mut out = vec![[0.0; 3]; mapping.len()];
    for (i, sources) in mapping.iter().enumerate() {
        if sources.is_empty() {
            continue;
        }
        let mut list_x = Vec::with_capacity(sources.len());
        let mut list_y = Vec::with_capacity(sources.len());
        let mut list_z = Vec::with_capacity(sources.len());
        for &idx in sources {
            if let Some(v) = values.get(idx).copied() {
                list_x.push(v[0]);
                list_y.push(v[1]);
                list_z.push(v[2]);
            }
        }
        out[i] = [
            promote_scalar(&mut list_x, method),
            promote_scalar(&mut list_y, method),
            promote_scalar(&mut list_z, method),
        ];
    }
    out
}

fn promote_vec4(
    values: &[[f32; 4]],
    mapping: &[Vec<usize>],
    method: PromotionMethod,
) -> Vec<[f32; 4]> {
    let mut out = vec![[0.0; 4]; mapping.len()];
    for (i, sources) in mapping.iter().enumerate() {
        if sources.is_empty() {
            continue;
        }
        let mut list_x = Vec::with_capacity(sources.len());
        let mut list_y = Vec::with_capacity(sources.len());
        let mut list_z = Vec::with_capacity(sources.len());
        let mut list_w = Vec::with_capacity(sources.len());
        for &idx in sources {
            if let Some(v) = values.get(idx).copied() {
                list_x.push(v[0]);
                list_y.push(v[1]);
                list_z.push(v[2]);
                list_w.push(v[3]);
            }
        }
        out[i] = [
            promote_scalar(&mut list_x, method),
            promote_scalar(&mut list_y, method),
            promote_scalar(&mut list_z, method),
            promote_scalar(&mut list_w, method),
        ];
    }
    out
}

fn promote_scalar(values: &mut Vec<f32>, method: PromotionMethod) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    match method {
        PromotionMethod::Max => values.iter().copied().fold(f32::NEG_INFINITY, f32::max),
        PromotionMethod::Min => values.iter().copied().fold(f32::INFINITY, f32::min),
        PromotionMethod::Average => values.iter().sum::<f32>() / values.len() as f32,
        PromotionMethod::Mode => mode_f32(values),
        PromotionMethod::Median => median_f32(values),
        PromotionMethod::Sum => values.iter().sum(),
        PromotionMethod::SumSquares => values.iter().map(|v| v * v).sum(),
        PromotionMethod::RootMeanSquare => {
            (values.iter().map(|v| v * v).sum::<f32>() / values.len() as f32).sqrt()
        }
        PromotionMethod::First => values.first().copied().unwrap_or(0.0),
        PromotionMethod::Last => values.last().copied().unwrap_or(0.0),
    }
}

fn mode_f32(values: &[f32]) -> f32 {
    let mut counts: HashMap<u32, (usize, f32)> = HashMap::new();
    for &value in values {
        let key = value.to_bits();
        let entry = counts.entry(key).or_insert((0, value));
        entry.0 += 1;
        entry.1 = value;
    }
    let mut best = None;
    for (_, (count, value)) in counts {
        match best {
            Some((best_count, best_value)) => {
                if count > best_count || (count == best_count && value < best_value) {
                    best = Some((count, value));
                }
            }
            None => best = Some((count, value)),
        }
    }
    best.map(|(_, value)| value).unwrap_or(0.0)
}

fn median_f32(values: &mut Vec<f32>) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    values[mid]
}

fn mode_i32(values: &[i32]) -> i32 {
    let mut counts: HashMap<i32, usize> = HashMap::new();
    for &value in values {
        *counts.entry(value).or_insert(0) += 1;
    }
    let mut best = None;
    for (value, count) in counts {
        match best {
            Some((best_count, best_value)) => {
                if count > best_count || (count == best_count && value < best_value) {
                    best = Some((count, value));
                }
            }
            None => best = Some((count, value)),
        }
    }
    best.map(|(_, value)| value).unwrap_or(0)
}

fn mode_u32(values: &[u32]) -> u32 {
    let mut counts: HashMap<u32, usize> = HashMap::new();
    for &value in values {
        *counts.entry(value).or_insert(0) += 1;
    }
    let mut best = None;
    for (value, count) in counts {
        match best {
            Some((best_count, best_value)) => {
                if count > best_count || (count == best_count && value < best_value) {
                    best = Some((count, value));
                }
            }
            None => best = Some((count, value)),
        }
    }
    best.map(|(_, value)| value).unwrap_or(0)
}

fn median_i32(values: &mut Vec<i32>) -> i32 {
    if values.is_empty() {
        return 0;
    }
    values.sort_unstable();
    values[values.len() / 2]
}

fn median_u32(values: &mut Vec<u32>) -> u32 {
    if values.is_empty() {
        return 0;
    }
    values.sort_unstable();
    values[values.len() / 2]
}

fn build_mapping(mesh: &Mesh, source: AttributeDomain, target: AttributeDomain) -> Vec<Vec<usize>> {
    if source == target {
        return Vec::new();
    }
    if target == AttributeDomain::Detail {
        let source_len = mesh.attribute_domain_len(source);
        return vec![(0..source_len).collect()];
    }
    let target_len = mesh.attribute_domain_len(target);
    if target_len == 0 {
        return Vec::new();
    }
    if source == AttributeDomain::Detail {
        return vec![vec![0usize]; target_len];
    }
    let mut mapping = vec![Vec::new(); target_len];
    let face_counts = if mesh.face_counts.is_empty() {
        if mesh.indices.len().is_multiple_of(3) {
            vec![3u32; mesh.indices.len() / 3]
        } else if mesh.indices.is_empty() {
            Vec::new()
        } else {
            vec![mesh.indices.len() as u32]
        }
    } else {
        mesh.face_counts.clone()
    };
    match (source, target) {
        (AttributeDomain::Point, AttributeDomain::Vertex) => {
            for (vertex_idx, point_idx) in mesh.indices.iter().enumerate() {
                if vertex_idx < mapping.len() {
                    mapping[vertex_idx].push(*point_idx as usize);
                }
            }
        }
        (AttributeDomain::Primitive, AttributeDomain::Vertex) => {
            let mut cursor = 0usize;
            for (prim_idx, count) in face_counts.iter().enumerate() {
                let count = *count as usize;
                for corner in 0..count {
                    let vertex_idx = cursor + corner;
                    if vertex_idx < mapping.len() {
                        mapping[vertex_idx].push(prim_idx);
                    }
                }
                cursor += count;
            }
        }
        (AttributeDomain::Point, AttributeDomain::Primitive) => {
            let mut cursor = 0usize;
            for (prim_idx, count) in face_counts.iter().enumerate() {
                let count = *count as usize;
                if prim_idx >= mapping.len() || cursor + count > mesh.indices.len() {
                    cursor = cursor.saturating_add(count);
                    continue;
                }
                let face = &mesh.indices[cursor..cursor + count];
                for idx in face {
                    mapping[prim_idx].push(*idx as usize);
                }
                cursor += count;
            }
        }
        (AttributeDomain::Vertex, AttributeDomain::Primitive) => {
            let mut cursor = 0usize;
            for (prim_idx, count) in face_counts.iter().enumerate() {
                let count = *count as usize;
                if prim_idx >= mapping.len() {
                    cursor = cursor.saturating_add(count);
                    continue;
                }
                for corner in 0..count {
                    mapping[prim_idx].push(cursor + corner);
                }
                cursor += count;
            }
        }
        (AttributeDomain::Vertex, AttributeDomain::Point) => {
            for (vertex_idx, point_idx) in mesh.indices.iter().enumerate() {
                let point_idx = *point_idx as usize;
                if point_idx < mapping.len() {
                    mapping[point_idx].push(vertex_idx);
                }
            }
        }
        (AttributeDomain::Primitive, AttributeDomain::Point) => {
            let mut cursor = 0usize;
            for (prim_idx, count) in face_counts.iter().enumerate() {
                let count = *count as usize;
                if cursor + count > mesh.indices.len() {
                    cursor = cursor.saturating_add(count);
                    continue;
                }
                let face = &mesh.indices[cursor..cursor + count];
                for idx in face {
                    let point_idx = *idx as usize;
                    if point_idx < mapping.len() {
                        mapping[point_idx].push(prim_idx);
                    }
                }
                cursor += count;
            }
        }
        (AttributeDomain::Point, AttributeDomain::Point)
        | (AttributeDomain::Vertex, AttributeDomain::Vertex)
        | (AttributeDomain::Primitive, AttributeDomain::Primitive)
        | (AttributeDomain::Detail, _) => {}
        (AttributeDomain::Point, AttributeDomain::Detail)
        | (AttributeDomain::Vertex, AttributeDomain::Detail)
        | (AttributeDomain::Primitive, AttributeDomain::Detail)
        | (AttributeDomain::Detail, AttributeDomain::Detail) => {}
        _ => {}
    }
    mapping
}

fn build_mapping_splats(
    source_len: usize,
    target_len: usize,
    source: AttributeDomain,
    target: AttributeDomain,
) -> Vec<Vec<usize>> {
    if source == target {
        return Vec::new();
    }
    if target == AttributeDomain::Detail {
        return vec![(0..source_len).collect()];
    }
    if source == AttributeDomain::Detail {
        return vec![vec![0usize]; target_len];
    }
    let mut mapping = vec![Vec::new(); target_len];
    match (source, target) {
        (AttributeDomain::Point, AttributeDomain::Primitive)
        | (AttributeDomain::Primitive, AttributeDomain::Point)
        | (AttributeDomain::Point, AttributeDomain::Point)
        | (AttributeDomain::Primitive, AttributeDomain::Primitive) => {
            let count = source_len.min(target_len);
            for idx in 0..count {
                mapping[idx].push(idx);
            }
        }
        _ => {}
    }
    mapping
}

fn glob_match(pattern: &str, value: &str) -> bool {
    glob_match_inner(pattern.as_bytes(), value.as_bytes())
}

fn glob_match_inner(pattern: &[u8], value: &[u8]) -> bool {
    if pattern.is_empty() {
        return value.is_empty();
    }
    match pattern[0] {
        b'*' => {
            for idx in 0..=value.len() {
                if glob_match_inner(&pattern[1..], &value[idx..]) {
                    return true;
                }
            }
            false
        }
        b'?' => {
            if value.is_empty() {
                false
            } else {
                glob_match_inner(&pattern[1..], &value[1..])
            }
        }
        ch => {
            if value.first().copied() == Some(ch) {
                glob_match_inner(&pattern[1..], &value[1..])
            } else {
                false
            }
        }
    }
}
