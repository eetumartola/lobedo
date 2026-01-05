use std::collections::{BTreeMap, HashMap};

use glam::Vec3;

use crate::attributes::{AttributeDomain, AttributeRef, AttributeStorage};
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    geometry_in,
    geometry_out,
    group_utils::{mesh_group_mask, splat_group_mask},
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

    Ok(Geometry { meshes, splats })
}

fn domain_from_params(params: &NodeParams) -> AttributeDomain {
    match params.get_int("domain", 0).clamp(0, 3) {
        0 => AttributeDomain::Point,
        1 => AttributeDomain::Vertex,
        2 => AttributeDomain::Primitive,
        _ => AttributeDomain::Detail,
    }
}

fn parse_attribute_list(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string())
        .collect()
}

#[derive(Debug, Clone)]
enum AttributeSamples {
    Float { positions: Vec<Vec3>, values: Vec<f32> },
    Int { positions: Vec<Vec3>, values: Vec<i32> },
    Vec2 { positions: Vec<Vec3>, values: Vec<[f32; 2]> },
    Vec3 { positions: Vec<Vec3>, values: Vec<[f32; 3]> },
    Vec4 { positions: Vec<Vec3>, values: Vec<[f32; 4]> },
}

impl AttributeSamples {
    fn len(&self) -> usize {
        match self {
            AttributeSamples::Float { values, .. } => values.len(),
            AttributeSamples::Int { values, .. } => values.len(),
            AttributeSamples::Vec2 { values, .. } => values.len(),
            AttributeSamples::Vec3 { values, .. } => values.len(),
            AttributeSamples::Vec4 { values, .. } => values.len(),
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
    if let Some(mask) = &mask {
        if !mask.iter().any(|value| *value) {
            return Ok(());
        }
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
    if let Some(mask) = &mask {
        if !mask.iter().any(|value| *value) {
            return Ok(());
        }
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

fn mesh_positions_for_domain(mesh: &Mesh, domain: AttributeDomain) -> Vec<Vec3> {
    match domain {
        AttributeDomain::Point => mesh.positions.iter().copied().map(Vec3::from).collect(),
        AttributeDomain::Vertex => mesh
            .indices
            .iter()
            .filter_map(|idx| mesh.positions.get(*idx as usize))
            .copied()
            .map(Vec3::from)
            .collect(),
        AttributeDomain::Primitive => {
            let mut positions = Vec::with_capacity(mesh.indices.len() / 3);
            for tri in mesh.indices.chunks_exact(3) {
                let p0 = mesh.positions.get(tri[0] as usize).copied().unwrap_or([0.0; 3]);
                let p1 = mesh.positions.get(tri[1] as usize).copied().unwrap_or([0.0; 3]);
                let p2 = mesh.positions.get(tri[2] as usize).copied().unwrap_or([0.0; 3]);
                let center = (Vec3::from(p0) + Vec3::from(p1) + Vec3::from(p2)) / 3.0;
                positions.push(center);
            }
            positions
        }
        AttributeDomain::Detail => mesh
            .bounds()
            .map(|bounds| {
                Vec3::new(
                    (bounds.min[0] + bounds.max[0]) * 0.5,
                    (bounds.min[1] + bounds.max[1]) * 0.5,
                    (bounds.min[2] + bounds.max[2]) * 0.5,
                )
            })
            .into_iter()
            .collect(),
    }
}

fn splat_positions_for_domain(splats: &SplatGeo, domain: AttributeDomain) -> Vec<Vec3> {
    match domain {
        AttributeDomain::Point | AttributeDomain::Primitive => {
            splats.positions.iter().copied().map(Vec3::from).collect()
        }
        AttributeDomain::Detail => {
            let mut iter = splats.positions.iter();
            let Some(first) = iter.next().copied() else {
                return Vec::new();
            };
            let mut min = first;
            let mut max = first;
            for p in iter {
                min[0] = min[0].min(p[0]);
                min[1] = min[1].min(p[1]);
                min[2] = min[2].min(p[2]);
                max[0] = max[0].max(p[0]);
                max[1] = max[1].max(p[1]);
                max[2] = max[2].max(p[2]);
            }
            vec![Vec3::new(
                (min[0] + max[0]) * 0.5,
                (min[1] + max[1]) * 0.5,
                (min[2] + max[2]) * 0.5,
            )]
        }
        AttributeDomain::Vertex => Vec::new(),
    }
}

fn existing_float_attr_mesh(
    mesh: &Mesh,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<f32> {
    if let Some(AttributeRef::Float(values)) = mesh.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![0.0; count.max(1)]
}

fn existing_int_attr_mesh(
    mesh: &Mesh,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<i32> {
    if let Some(AttributeRef::Int(values)) = mesh.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![0; count.max(1)]
}

fn existing_vec2_attr_mesh(
    mesh: &Mesh,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<[f32; 2]> {
    if let Some(AttributeRef::Vec2(values)) = mesh.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![[0.0, 0.0]; count.max(1)]
}

fn existing_vec3_attr_mesh(
    mesh: &Mesh,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<[f32; 3]> {
    if let Some(AttributeRef::Vec3(values)) = mesh.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![[0.0, 0.0, 0.0]; count.max(1)]
}

fn existing_vec4_attr_mesh(
    mesh: &Mesh,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<[f32; 4]> {
    if let Some(AttributeRef::Vec4(values)) = mesh.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![[0.0, 0.0, 0.0, 0.0]; count.max(1)]
}

fn existing_float_attr_splats(
    splats: &SplatGeo,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<f32> {
    if let Some(AttributeRef::Float(values)) = splats.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![0.0; count.max(1)]
}

fn existing_int_attr_splats(
    splats: &SplatGeo,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<i32> {
    if let Some(AttributeRef::Int(values)) = splats.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![0; count.max(1)]
}

fn existing_vec2_attr_splats(
    splats: &SplatGeo,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<[f32; 2]> {
    if let Some(AttributeRef::Vec2(values)) = splats.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![[0.0, 0.0]; count.max(1)]
}

fn existing_vec3_attr_splats(
    splats: &SplatGeo,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<[f32; 3]> {
    if let Some(AttributeRef::Vec3(values)) = splats.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![[0.0, 0.0, 0.0]; count.max(1)]
}

fn existing_vec4_attr_splats(
    splats: &SplatGeo,
    domain: AttributeDomain,
    name: &str,
    count: usize,
) -> Vec<[f32; 4]> {
    if let Some(AttributeRef::Vec4(values)) = splats.attribute(domain, name) {
        if values.len() == count {
            return values.to_vec();
        }
    }
    vec![[0.0, 0.0, 0.0, 0.0]; count.max(1)]
}
