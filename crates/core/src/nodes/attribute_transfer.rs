use std::collections::{BTreeMap, HashMap};

use glam::Vec3;

use crate::attributes::{AttributeDomain, AttributeRef, AttributeStorage, StringTableAttribute};
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    attribute_utils::{
        domain_from_params, existing_float_attr_mesh, existing_float_attr_splats,
        existing_int_attr_mesh, existing_int_attr_splats, existing_vec2_attr_mesh,
        existing_vec2_attr_splats, existing_vec3_attr_mesh, existing_vec3_attr_splats,
        existing_vec4_attr_mesh, existing_vec4_attr_splats, mesh_positions_for_domain,
        parse_attribute_list, splat_positions_for_domain,
    },
    geometry_in,
    geometry_out,
    group_utils::{mask_has_any, mesh_group_mask, splat_group_mask},
    require_mesh_input,
};
use crate::splat::SplatGeo;

pub const NAME: &str = "Attribute Transfer";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![geometry_in("target"), geometry_in("source")],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            ("attr".to_string(), ParamValue::String(String::new())),
            ("domain".to_string(), ParamValue::Int(0)),
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut target = require_mesh_input(
        inputs,
        0,
        "Attribute Transfer requires a target mesh input",
    )?;
    let source = require_mesh_input(
        inputs,
        1,
        "Attribute Transfer requires a source mesh input",
    )?;
    let attr_names = parse_attribute_list(params.get_string("attr", ""));
    if attr_names.is_empty() {
        return Ok(target);
    }
    let domain = domain_from_params(params);
    let samples = build_source_samples_mesh(&source, domain, &attr_names);
    apply_transfer_to_mesh(params, &samples, domain, &mut target)?;
    Ok(target)
}

pub fn apply_to_geometry(
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    let Some(target) = inputs.first() else {
        return Ok(Geometry::default());
    };
    let Some(source) = inputs.get(1) else {
        return Err("Attribute Transfer requires a source input".to_string());
    };
    let attr_names = parse_attribute_list(params.get_string("attr", ""));
    if attr_names.is_empty() {
        return Ok(target.clone());
    }
    let domain = domain_from_params(params);
    let samples = build_source_samples_geometry(source, domain, &attr_names);

    let mut meshes = Vec::with_capacity(target.meshes.len());
    for mesh in &target.meshes {
        let mut mesh = mesh.clone();
        apply_transfer_to_mesh(params, &samples, domain, &mut mesh)?;
        meshes.push(mesh);
    }

    let mut splats = Vec::with_capacity(target.splats.len());
    for splat in &target.splats {
        let mut splat = splat.clone();
        apply_transfer_to_splats(params, &samples, domain, &mut splat)?;
        splats.push(splat);
    }

    Ok(Geometry {
        meshes,
        splats,
        materials: target.materials.clone(),
    })
}

#[derive(Debug, Clone)]
enum AttributeSamples {
    Float { positions: Vec<Vec3>, values: Vec<f32> },
    Int { positions: Vec<Vec3>, values: Vec<i32> },
    Vec2 { positions: Vec<Vec3>, values: Vec<[f32; 2]> },
    Vec3 { positions: Vec<Vec3>, values: Vec<[f32; 3]> },
    Vec4 { positions: Vec<Vec3>, values: Vec<[f32; 4]> },
    StringTable { positions: Vec<Vec3>, values: StringTableAttribute },
}

impl AttributeSamples {
    fn len(&self) -> usize {
        match self {
            AttributeSamples::Float { values, .. } => values.len(),
            AttributeSamples::Int { values, .. } => values.len(),
            AttributeSamples::Vec2 { values, .. } => values.len(),
            AttributeSamples::Vec3 { values, .. } => values.len(),
            AttributeSamples::Vec4 { values, .. } => values.len(),
            AttributeSamples::StringTable { values, .. } => values.len(),
        }
    }

}

fn build_source_samples_geometry(
    source: &Geometry,
    domain: AttributeDomain,
    attr_names: &[String],
) -> HashMap<String, AttributeSamples> {
    let mut samples = HashMap::new();
    for mesh in &source.meshes {
        let positions = mesh_positions_for_domain(mesh, domain);
        append_samples_from_mesh(mesh, domain, &positions, attr_names, &mut samples);
    }
    for splat in &source.splats {
        let positions = splat_positions_for_domain(splat, domain);
        append_samples_from_splats(splat, domain, &positions, attr_names, &mut samples);
    }
    samples
}

fn build_source_samples_mesh(
    source: &Mesh,
    domain: AttributeDomain,
    attr_names: &[String],
) -> HashMap<String, AttributeSamples> {
    let mut samples = HashMap::new();
    let positions = mesh_positions_for_domain(source, domain);
    append_samples_from_mesh(source, domain, &positions, attr_names, &mut samples);
    samples
}

fn append_samples_from_mesh(
    mesh: &Mesh,
    domain: AttributeDomain,
    positions: &[Vec3],
    attr_names: &[String],
    samples: &mut HashMap<String, AttributeSamples>,
) {
    if positions.is_empty() {
        return;
    }
    for name in attr_names {
        let Some(attr) = mesh.attribute(domain, name) else {
            continue;
        };
        append_samples(samples, name, positions, attr);
    }
}

fn append_samples_from_splats(
    splats: &SplatGeo,
    domain: AttributeDomain,
    positions: &[Vec3],
    attr_names: &[String],
    samples: &mut HashMap<String, AttributeSamples>,
) {
    if positions.is_empty() {
        return;
    }
    for name in attr_names {
        let Some(attr) = splats.attribute(domain, name) else {
            continue;
        };
        append_samples(samples, name, positions, attr);
    }
}

fn append_samples(
    samples: &mut HashMap<String, AttributeSamples>,
    name: &str,
    positions: &[Vec3],
    attr: AttributeRef<'_>,
) {
    if attr.len() != positions.len() || positions.is_empty() {
        return;
    }
    match attr {
        AttributeRef::Float(values) => match samples.get_mut(name) {
            None => {
                samples.insert(
                    name.to_string(),
                    AttributeSamples::Float {
                        positions: positions.to_vec(),
                        values: values.to_vec(),
                    },
                );
            }
            Some(AttributeSamples::Float {
                positions: out_positions,
                values: out_values,
            }) => {
                out_positions.extend_from_slice(positions);
                out_values.extend_from_slice(values);
            }
            _ => {}
        },
        AttributeRef::Int(values) => match samples.get_mut(name) {
            None => {
                samples.insert(
                    name.to_string(),
                    AttributeSamples::Int {
                        positions: positions.to_vec(),
                        values: values.to_vec(),
                    },
                );
            }
            Some(AttributeSamples::Int {
                positions: out_positions,
                values: out_values,
            }) => {
                out_positions.extend_from_slice(positions);
                out_values.extend_from_slice(values);
            }
            _ => {}
        },
        AttributeRef::Vec2(values) => match samples.get_mut(name) {
            None => {
                samples.insert(
                    name.to_string(),
                    AttributeSamples::Vec2 {
                        positions: positions.to_vec(),
                        values: values.to_vec(),
                    },
                );
            }
            Some(AttributeSamples::Vec2 {
                positions: out_positions,
                values: out_values,
            }) => {
                out_positions.extend_from_slice(positions);
                out_values.extend_from_slice(values);
            }
            _ => {}
        },
        AttributeRef::Vec3(values) => match samples.get_mut(name) {
            None => {
                samples.insert(
                    name.to_string(),
                    AttributeSamples::Vec3 {
                        positions: positions.to_vec(),
                        values: values.to_vec(),
                    },
                );
            }
            Some(AttributeSamples::Vec3 {
                positions: out_positions,
                values: out_values,
            }) => {
                out_positions.extend_from_slice(positions);
                out_values.extend_from_slice(values);
            }
            _ => {}
        },
        AttributeRef::Vec4(values) => match samples.get_mut(name) {
            None => {
                samples.insert(
                    name.to_string(),
                    AttributeSamples::Vec4 {
                        positions: positions.to_vec(),
                        values: values.to_vec(),
                    },
                );
            }
            Some(AttributeSamples::Vec4 {
                positions: out_positions,
                values: out_values,
            }) => {
                out_positions.extend_from_slice(positions);
                out_values.extend_from_slice(values);
            }
            _ => {}
        },
        AttributeRef::StringTable(values) => match samples.get_mut(name) {
            None => {
                samples.insert(
                    name.to_string(),
                    AttributeSamples::StringTable {
                        positions: positions.to_vec(),
                        values: values.clone(),
                    },
                );
            }
            Some(AttributeSamples::StringTable {
                positions: out_positions,
                values: out_values,
            }) => {
                out_positions.extend_from_slice(positions);
                append_string_table_values(out_values, values);
            }
            _ => {}
        },
    }
}

fn apply_transfer_to_mesh(
    params: &NodeParams,
    samples: &HashMap<String, AttributeSamples>,
    domain: AttributeDomain,
    mesh: &mut Mesh,
) -> Result<(), String> {
    if samples.is_empty() {
        return Ok(());
    }
    let count = mesh.attribute_domain_len(domain);
    if count == 0 && domain != AttributeDomain::Detail {
        return Ok(());
    }

    let mask = mesh_group_mask(mesh, params, domain);
    if !mask_has_any(mask.as_deref()) {
        return Ok(());
    }

    for (name, samples) in samples {
        if samples.len() == 0 {
            continue;
        }
        let positions = mesh_positions_for_domain(mesh, domain);
        if positions.is_empty() {
            continue;
        }
        match samples {
            AttributeSamples::Float { positions: src_pos, values } => {
                let mut out = existing_float_attr_mesh(mesh, domain, name, count);
                transfer_values(
                    &positions,
                    src_pos,
                    values,
                    mask.as_deref(),
                    |idx, value| {
                        if let Some(slot) = out.get_mut(idx) {
                            *slot = value;
                        }
                    },
                );
                mesh.set_attribute(domain, name, AttributeStorage::Float(out))
                    .map_err(|err| format!("Attribute Transfer error: {:?}", err))?;
            }
            AttributeSamples::Int { positions: src_pos, values } => {
                let mut out = existing_int_attr_mesh(mesh, domain, name, count);
                transfer_values(
                    &positions,
                    src_pos,
                    values,
                    mask.as_deref(),
                    |idx, value| {
                        if let Some(slot) = out.get_mut(idx) {
                            *slot = value;
                        }
                    },
                );
                mesh.set_attribute(domain, name, AttributeStorage::Int(out))
                    .map_err(|err| format!("Attribute Transfer error: {:?}", err))?;
            }
            AttributeSamples::Vec2 { positions: src_pos, values } => {
                let mut out = existing_vec2_attr_mesh(mesh, domain, name, count);
                transfer_values(
                    &positions,
                    src_pos,
                    values,
                    mask.as_deref(),
                    |idx, value| {
                        if let Some(slot) = out.get_mut(idx) {
                            *slot = value;
                        }
                    },
                );
                mesh.set_attribute(domain, name, AttributeStorage::Vec2(out))
                    .map_err(|err| format!("Attribute Transfer error: {:?}", err))?;
            }
            AttributeSamples::Vec3 { positions: src_pos, values } => {
                let mut out = existing_vec3_attr_mesh(mesh, domain, name, count);
                transfer_values(
                    &positions,
                    src_pos,
                    values,
                    mask.as_deref(),
                    |idx, value| {
                        if let Some(slot) = out.get_mut(idx) {
                            *slot = value;
                        }
                    },
                );
                mesh.set_attribute(domain, name, AttributeStorage::Vec3(out))
                    .map_err(|err| format!("Attribute Transfer error: {:?}", err))?;
            }
            AttributeSamples::Vec4 { positions: src_pos, values } => {
                let mut out = existing_vec4_attr_mesh(mesh, domain, name, count);
                transfer_values(
                    &positions,
                    src_pos,
                    values,
                    mask.as_deref(),
                    |idx, value| {
                        if let Some(slot) = out.get_mut(idx) {
                            *slot = value;
                        }
                    },
                );
                mesh.set_attribute(domain, name, AttributeStorage::Vec4(out))
                    .map_err(|err| format!("Attribute Transfer error: {:?}", err))?;
            }
            AttributeSamples::StringTable { positions: src_pos, values } => {
                let existing = mesh.attribute(domain, name);
                let existing_table = match existing {
                    Some(AttributeRef::StringTable(table)) => Some(table),
                    _ => None,
                };
                let (combined_values, map_existing, map_source) =
                    merge_string_tables(existing_table, values);
                let mut out = vec![0u32; count.max(1)];
                if let Some(table) = existing_table {
                    if table.indices.len() == count {
                        for (idx, &old) in table.indices.iter().enumerate() {
                            let mapped = map_existing.get(old as usize).copied().unwrap_or(0);
                            if let Some(slot) = out.get_mut(idx) {
                                *slot = mapped;
                            }
                        }
                    }
                }
                let source_indices: Vec<u32> = values
                    .indices
                    .iter()
                    .map(|idx| map_source.get(*idx as usize).copied().unwrap_or(0))
                    .collect();
                transfer_values(
                    &positions,
                    src_pos,
                    &source_indices,
                    mask.as_deref(),
                    |idx, value| {
                        if let Some(slot) = out.get_mut(idx) {
                            *slot = value;
                        }
                    },
                );
                let mut table = combined_values;
                if table.is_empty() && !out.is_empty() {
                    table.push(String::new());
                    out.fill(0);
                }
                mesh.set_attribute(
                    domain,
                    name,
                    AttributeStorage::StringTable(StringTableAttribute::new(table, out)),
                )
                .map_err(|err| format!("Attribute Transfer error: {:?}", err))?;
            }
        }
    }

    Ok(())
}

fn apply_transfer_to_splats(
    params: &NodeParams,
    samples: &HashMap<String, AttributeSamples>,
    domain: AttributeDomain,
    splats: &mut SplatGeo,
) -> Result<(), String> {
    if samples.is_empty() {
        return Ok(());
    }
    let count = splats.attribute_domain_len(domain);
    if count == 0 && domain != AttributeDomain::Detail {
        return Ok(());
    }

    let mask = splat_group_mask(splats, params, domain);
    if !mask_has_any(mask.as_deref()) {
        return Ok(());
    }

    for (name, samples) in samples {
        if samples.len() == 0 {
            continue;
        }
        let positions = splat_positions_for_domain(splats, domain);
        if positions.is_empty() {
            continue;
        }
        match samples {
            AttributeSamples::Float { positions: src_pos, values } => {
                let mut out = existing_float_attr_splats(splats, domain, name, count);
                transfer_values(
                    &positions,
                    src_pos,
                    values,
                    mask.as_deref(),
                    |idx, value| {
                        if let Some(slot) = out.get_mut(idx) {
                            *slot = value;
                        }
                    },
                );
                splats
                    .set_attribute(domain, name, AttributeStorage::Float(out))
                    .map_err(|err| format!("Attribute Transfer error: {:?}", err))?;
            }
            AttributeSamples::Int { positions: src_pos, values } => {
                let mut out = existing_int_attr_splats(splats, domain, name, count);
                transfer_values(
                    &positions,
                    src_pos,
                    values,
                    mask.as_deref(),
                    |idx, value| {
                        if let Some(slot) = out.get_mut(idx) {
                            *slot = value;
                        }
                    },
                );
                splats
                    .set_attribute(domain, name, AttributeStorage::Int(out))
                    .map_err(|err| format!("Attribute Transfer error: {:?}", err))?;
            }
            AttributeSamples::Vec2 { positions: src_pos, values } => {
                let mut out = existing_vec2_attr_splats(splats, domain, name, count);
                transfer_values(
                    &positions,
                    src_pos,
                    values,
                    mask.as_deref(),
                    |idx, value| {
                        if let Some(slot) = out.get_mut(idx) {
                            *slot = value;
                        }
                    },
                );
                splats
                    .set_attribute(domain, name, AttributeStorage::Vec2(out))
                    .map_err(|err| format!("Attribute Transfer error: {:?}", err))?;
            }
            AttributeSamples::Vec3 { positions: src_pos, values } => {
                let mut out = existing_vec3_attr_splats(splats, domain, name, count);
                transfer_values(
                    &positions,
                    src_pos,
                    values,
                    mask.as_deref(),
                    |idx, value| {
                        if let Some(slot) = out.get_mut(idx) {
                            *slot = value;
                        }
                    },
                );
                splats
                    .set_attribute(domain, name, AttributeStorage::Vec3(out))
                    .map_err(|err| format!("Attribute Transfer error: {:?}", err))?;
            }
            AttributeSamples::Vec4 { positions: src_pos, values } => {
                let mut out = existing_vec4_attr_splats(splats, domain, name, count);
                transfer_values(
                    &positions,
                    src_pos,
                    values,
                    mask.as_deref(),
                    |idx, value| {
                        if let Some(slot) = out.get_mut(idx) {
                            *slot = value;
                        }
                    },
                );
                splats
                    .set_attribute(domain, name, AttributeStorage::Vec4(out))
                    .map_err(|err| format!("Attribute Transfer error: {:?}", err))?;
            }
            AttributeSamples::StringTable { positions: src_pos, values } => {
                let existing = splats.attribute(domain, name);
                let existing_table = match existing {
                    Some(AttributeRef::StringTable(table)) => Some(table),
                    _ => None,
                };
                let (combined_values, map_existing, map_source) =
                    merge_string_tables(existing_table, values);
                let mut out = vec![0u32; count.max(1)];
                if let Some(table) = existing_table {
                    if table.indices.len() == count {
                        for (idx, &old) in table.indices.iter().enumerate() {
                            let mapped = map_existing.get(old as usize).copied().unwrap_or(0);
                            if let Some(slot) = out.get_mut(idx) {
                                *slot = mapped;
                            }
                        }
                    }
                }
                let source_indices: Vec<u32> = values
                    .indices
                    .iter()
                    .map(|idx| map_source.get(*idx as usize).copied().unwrap_or(0))
                    .collect();
                transfer_values(
                    &positions,
                    src_pos,
                    &source_indices,
                    mask.as_deref(),
                    |idx, value| {
                        if let Some(slot) = out.get_mut(idx) {
                            *slot = value;
                        }
                    },
                );
                let mut table = combined_values;
                if table.is_empty() && !out.is_empty() {
                    table.push(String::new());
                    out.fill(0);
                }
                splats
                    .set_attribute(
                        domain,
                        name,
                        AttributeStorage::StringTable(StringTableAttribute::new(table, out)),
                    )
                    .map_err(|err| format!("Attribute Transfer error: {:?}", err))?;
            }
        }
    }

    Ok(())
}

fn transfer_values<T: Copy>(
    target_positions: &[Vec3],
    source_positions: &[Vec3],
    source_values: &[T],
    mask: Option<&[bool]>,
    mut set_value: impl FnMut(usize, T),
) {
    if source_positions.is_empty() || source_values.is_empty() {
        return;
    }
    for (idx, position) in target_positions.iter().enumerate() {
        if mask
            .as_ref()
            .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
        {
            continue;
        }
        let nearest = find_nearest_index(*position, source_positions);
        if let Some(value) = source_values.get(nearest).copied() {
            set_value(idx, value);
        }
    }
}

fn append_string_table_values(
    combined: &mut StringTableAttribute,
    source: &StringTableAttribute,
) {
    if source.indices.is_empty() {
        return;
    }
    let mut lookup: HashMap<String, u32> = HashMap::new();
    for (idx, value) in combined.values.iter().enumerate() {
        lookup.insert(value.clone(), idx as u32);
    }
    for &index in &source.indices {
        let value = source.values.get(index as usize).cloned().unwrap_or_default();
        let entry = if let Some(&existing) = lookup.get(&value) {
            existing
        } else {
            let new_index = combined.values.len() as u32;
            combined.values.push(value.clone());
            lookup.insert(value, new_index);
            new_index
        };
        combined.indices.push(entry);
    }
}

fn merge_string_tables(
    existing: Option<&StringTableAttribute>,
    source: &StringTableAttribute,
) -> (Vec<String>, Vec<u32>, Vec<u32>) {
    let mut combined = Vec::new();
    let mut lookup: HashMap<String, u32> = HashMap::new();
    let mut existing_map = Vec::new();
    if let Some(existing) = existing {
        existing_map = Vec::with_capacity(existing.values.len());
        for value in &existing.values {
            let entry = lookup.get(value).copied().unwrap_or_else(|| {
                let idx = combined.len() as u32;
                combined.push(value.clone());
                lookup.insert(value.clone(), idx);
                idx
            });
            existing_map.push(entry);
        }
    }
    let mut source_map = Vec::with_capacity(source.values.len());
    for value in &source.values {
        let entry = lookup.get(value).copied().unwrap_or_else(|| {
            let idx = combined.len() as u32;
            combined.push(value.clone());
            lookup.insert(value.clone(), idx);
            idx
        });
        source_map.push(entry);
    }
    (combined, existing_map, source_map)
}

fn find_nearest_index(position: Vec3, samples: &[Vec3]) -> usize {
    let mut best = 0usize;
    let mut best_dist = f32::MAX;
    for (idx, sample) in samples.iter().enumerate() {
        let dist = position.distance_squared(*sample);
        if dist < best_dist {
            best = idx;
            best_dist = dist;
        }
    }
    best
}
