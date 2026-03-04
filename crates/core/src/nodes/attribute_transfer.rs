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
    geometry_in, geometry_out,
    group_utils::{mask_has_any, mesh_group_mask, splat_group_mask},
    recompute_mesh_normals, require_mesh_input,
};
use crate::parallel;
use crate::param_spec::ParamSpec;
use crate::splat::SplatGeo;

pub const NAME: &str = "Attribute Transfer";
const DEFAULT_MAX_RADIUS: f32 = 0.0;
const DEFAULT_SAMPLE_COUNT: i32 = 1;
const DEFAULT_COMBINE: i32 = 0;

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
            (
                "max_radius".to_string(),
                ParamValue::Float(DEFAULT_MAX_RADIUS),
            ),
            (
                "sample_count".to_string(),
                ParamValue::Int(DEFAULT_SAMPLE_COUNT),
            ),
            ("combine".to_string(), ParamValue::Int(DEFAULT_COMBINE)),
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::string("attr", "Attributes").with_help("Space-delimited list of attributes."),
        ParamSpec::int_enum(
            "domain",
            "Domain",
            vec![(0, "Point"), (1, "Vertex"), (2, "Primitive"), (3, "Detail")],
        )
        .with_help("Attribute domain to transfer."),
        ParamSpec::float_slider("max_radius", "Max Radius", 0.0, 1000.0)
            .with_help("Maximum transfer distance (0 = unlimited)."),
        ParamSpec::int_slider("sample_count", "Sample Count", 1, 64)
            .with_help("Maximum number of source samples considered per target element."),
        ParamSpec::int_enum(
            "combine",
            "Combine",
            vec![
                (0, "Closest"),
                (1, "Average"),
                (2, "Distance Weighted"),
                (3, "Minimum"),
                (4, "Maximum"),
            ],
        )
        .with_help("How multiple nearby samples are combined."),
        ParamSpec::string("group", "Group").with_help("Restrict to a group."),
        ParamSpec::int_enum(
            "group_type",
            "Group Type",
            vec![(0, "Auto"), (1, "Vertex"), (2, "Point"), (3, "Primitive")],
        )
        .with_help("Group domain to use."),
    ]
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut target =
        require_mesh_input(inputs, 0, "Attribute Transfer requires a target mesh input")?;
    let source = require_mesh_input(inputs, 1, "Attribute Transfer requires a source mesh input")?;
    let attr_names = parse_attribute_list(params.get_string("attr", ""));
    if attr_names.is_empty() {
        return Ok(target);
    }
    let domain = domain_from_params(params);
    let samples = build_source_samples_mesh(&source, domain, &attr_names);
    apply_transfer_to_mesh(params, &samples, domain, &mut target)?;
    Ok(target)
}

pub fn apply_to_geometry(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
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

    let mut meshes = Vec::new();
    if let Some(mut mesh) = target.merged_mesh() {
        apply_transfer_to_mesh(params, &samples, domain, &mut mesh)?;
        meshes.push(mesh);
    }

    let mut splats = Vec::with_capacity(target.splats.len());
    for splat in &target.splats {
        let mut splat = splat.clone();
        apply_transfer_to_splats(params, &samples, domain, &mut splat)?;
        splats.push(splat);
    }

    let curves = if meshes.is_empty() {
        Vec::new()
    } else {
        target.curves.clone()
    };
    Ok(Geometry {
        meshes,
        splats,
        curves,
        volumes: target.volumes.clone(),
        materials: target.materials.clone(),
    })
}

#[derive(Debug, Clone)]
enum AttributeSamples {
    Float {
        positions: Vec<Vec3>,
        values: Vec<f32>,
    },
    Int {
        positions: Vec<Vec3>,
        values: Vec<i32>,
    },
    Vec2 {
        positions: Vec<Vec3>,
        values: Vec<[f32; 2]>,
    },
    Vec3 {
        positions: Vec<Vec3>,
        values: Vec<[f32; 3]>,
    },
    Vec4 {
        positions: Vec<Vec3>,
        values: Vec<[f32; 4]>,
    },
    StringTable {
        positions: Vec<Vec3>,
        values: StringTableAttribute,
    },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransferCombineMode {
    Closest,
    Average,
    DistanceWeighted,
    Minimum,
    Maximum,
}

#[derive(Debug, Clone, Copy)]
struct TransferSettings {
    max_radius_sq: Option<f32>,
    sample_count: usize,
    combine: TransferCombineMode,
}

#[derive(Debug, Clone, Copy)]
struct NeighborSample {
    index: usize,
    dist_sq: f32,
}

fn transfer_settings(params: &NodeParams) -> TransferSettings {
    let max_radius = params.get_float("max_radius", DEFAULT_MAX_RADIUS);
    let max_radius_sq = if max_radius.is_finite() && max_radius > 0.0 {
        Some(max_radius * max_radius)
    } else {
        None
    };
    let sample_count = params
        .get_int("sample_count", DEFAULT_SAMPLE_COUNT)
        .clamp(1, 64) as usize;
    let combine = match params.get_int("combine", DEFAULT_COMBINE).clamp(0, 4) {
        1 => TransferCombineMode::Average,
        2 => TransferCombineMode::DistanceWeighted,
        3 => TransferCombineMode::Minimum,
        4 => TransferCombineMode::Maximum,
        _ => TransferCombineMode::Closest,
    };
    TransferSettings {
        max_radius_sq,
        sample_count,
        combine,
    }
}

fn build_source_samples_geometry(
    source: &Geometry,
    domain: AttributeDomain,
    attr_names: &[String],
) -> HashMap<String, AttributeSamples> {
    let mut samples = HashMap::new();
    if let Some(mesh) = source.merged_mesh() {
        let positions = mesh_positions_for_domain(&mesh, domain);
        append_samples_from_mesh(&mesh, domain, &positions, attr_names, &mut samples);
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
    let settings = transfer_settings(params);

    for (name, samples) in samples {
        if samples.len() == 0 {
            continue;
        }
        let positions = mesh_positions_for_domain(mesh, domain);
        if positions.is_empty() {
            continue;
        }
        match samples {
            AttributeSamples::Float {
                positions: src_pos,
                values,
            } => {
                let mut out = existing_float_attr_mesh(mesh, domain, name, count);
                transfer_values_with_options(
                    &positions,
                    src_pos,
                    values,
                    mask.as_deref(),
                    &mut out,
                    settings,
                    combine_float,
                );
                mesh.set_attribute(domain, name, AttributeStorage::Float(out))
                    .map_err(|err| format!("Attribute Transfer error: {err:?}"))?;
            }
            AttributeSamples::Int {
                positions: src_pos,
                values,
            } => {
                let mut out = existing_int_attr_mesh(mesh, domain, name, count);
                transfer_values_with_options(
                    &positions,
                    src_pos,
                    values,
                    mask.as_deref(),
                    &mut out,
                    settings,
                    combine_int,
                );
                mesh.set_attribute(domain, name, AttributeStorage::Int(out))
                    .map_err(|err| format!("Attribute Transfer error: {err:?}"))?;
            }
            AttributeSamples::Vec2 {
                positions: src_pos,
                values,
            } => {
                let mut out = existing_vec2_attr_mesh(mesh, domain, name, count);
                transfer_values_with_options(
                    &positions,
                    src_pos,
                    values,
                    mask.as_deref(),
                    &mut out,
                    settings,
                    combine_vec2,
                );
                mesh.set_attribute(domain, name, AttributeStorage::Vec2(out))
                    .map_err(|err| format!("Attribute Transfer error: {err:?}"))?;
            }
            AttributeSamples::Vec3 {
                positions: src_pos,
                values,
            } => {
                let mut out = existing_vec3_attr_mesh(mesh, domain, name, count);
                transfer_values_with_options(
                    &positions,
                    src_pos,
                    values,
                    mask.as_deref(),
                    &mut out,
                    settings,
                    combine_vec3,
                );
                mesh.set_attribute(domain, name, AttributeStorage::Vec3(out))
                    .map_err(|err| format!("Attribute Transfer error: {err:?}"))?;
            }
            AttributeSamples::Vec4 {
                positions: src_pos,
                values,
            } => {
                let mut out = existing_vec4_attr_mesh(mesh, domain, name, count);
                transfer_values_with_options(
                    &positions,
                    src_pos,
                    values,
                    mask.as_deref(),
                    &mut out,
                    settings,
                    combine_vec4,
                );
                mesh.set_attribute(domain, name, AttributeStorage::Vec4(out))
                    .map_err(|err| format!("Attribute Transfer error: {err:?}"))?;
            }
            AttributeSamples::StringTable {
                positions: src_pos,
                values,
            } => {
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
                transfer_values_with_options(
                    &positions,
                    src_pos,
                    &source_indices,
                    mask.as_deref(),
                    &mut out,
                    settings,
                    combine_string_index,
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
                .map_err(|err| format!("Attribute Transfer error: {err:?}"))?;
            }
        }
    }

    if domain == AttributeDomain::Point && samples.contains_key("P") {
        recompute_mesh_normals(mesh);
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
    let settings = transfer_settings(params);

    for (name, samples) in samples {
        if samples.len() == 0 {
            continue;
        }
        let positions = splat_positions_for_domain(splats, domain);
        if positions.is_empty() {
            continue;
        }
        match samples {
            AttributeSamples::Float {
                positions: src_pos,
                values,
            } => {
                let mut out = existing_float_attr_splats(splats, domain, name, count);
                transfer_values_with_options(
                    &positions,
                    src_pos,
                    values,
                    mask.as_deref(),
                    &mut out,
                    settings,
                    combine_float,
                );
                splats
                    .set_attribute(domain, name, AttributeStorage::Float(out))
                    .map_err(|err| format!("Attribute Transfer error: {err:?}"))?;
            }
            AttributeSamples::Int {
                positions: src_pos,
                values,
            } => {
                let mut out = existing_int_attr_splats(splats, domain, name, count);
                transfer_values_with_options(
                    &positions,
                    src_pos,
                    values,
                    mask.as_deref(),
                    &mut out,
                    settings,
                    combine_int,
                );
                splats
                    .set_attribute(domain, name, AttributeStorage::Int(out))
                    .map_err(|err| format!("Attribute Transfer error: {err:?}"))?;
            }
            AttributeSamples::Vec2 {
                positions: src_pos,
                values,
            } => {
                let mut out = existing_vec2_attr_splats(splats, domain, name, count);
                transfer_values_with_options(
                    &positions,
                    src_pos,
                    values,
                    mask.as_deref(),
                    &mut out,
                    settings,
                    combine_vec2,
                );
                splats
                    .set_attribute(domain, name, AttributeStorage::Vec2(out))
                    .map_err(|err| format!("Attribute Transfer error: {err:?}"))?;
            }
            AttributeSamples::Vec3 {
                positions: src_pos,
                values,
            } => {
                let mut out = existing_vec3_attr_splats(splats, domain, name, count);
                transfer_values_with_options(
                    &positions,
                    src_pos,
                    values,
                    mask.as_deref(),
                    &mut out,
                    settings,
                    combine_vec3,
                );
                splats
                    .set_attribute(domain, name, AttributeStorage::Vec3(out))
                    .map_err(|err| format!("Attribute Transfer error: {err:?}"))?;
            }
            AttributeSamples::Vec4 {
                positions: src_pos,
                values,
            } => {
                let mut out = existing_vec4_attr_splats(splats, domain, name, count);
                transfer_values_with_options(
                    &positions,
                    src_pos,
                    values,
                    mask.as_deref(),
                    &mut out,
                    settings,
                    combine_vec4,
                );
                splats
                    .set_attribute(domain, name, AttributeStorage::Vec4(out))
                    .map_err(|err| format!("Attribute Transfer error: {err:?}"))?;
            }
            AttributeSamples::StringTable {
                positions: src_pos,
                values,
            } => {
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
                transfer_values_with_options(
                    &positions,
                    src_pos,
                    &source_indices,
                    mask.as_deref(),
                    &mut out,
                    settings,
                    combine_string_index,
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
                    .map_err(|err| format!("Attribute Transfer error: {err:?}"))?;
            }
        }
    }

    Ok(())
}

fn transfer_values_with_options<T: Copy + Send + Sync>(
    target_positions: &[Vec3],
    source_positions: &[Vec3],
    source_values: &[T],
    mask: Option<&[bool]>,
    out: &mut [T],
    settings: TransferSettings,
    combine: fn(&[NeighborSample], &[T], TransferSettings) -> Option<T>,
) {
    if source_positions.is_empty() || source_values.is_empty() {
        return;
    }
    let mask_ref = mask;
    parallel::for_each_indexed_mut(out, |idx, slot| {
        if mask_ref
            .as_ref()
            .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
        {
            return;
        }
        let position = target_positions.get(idx).copied().unwrap_or(Vec3::ZERO);
        let neighbors = nearest_neighbors(position, source_positions, settings);
        if let Some(value) = combine(&neighbors, source_values, settings) {
            *slot = value;
        }
    });
}

fn nearest_neighbors(
    position: Vec3,
    samples: &[Vec3],
    settings: TransferSettings,
) -> Vec<NeighborSample> {
    let max_samples = settings.sample_count.max(1);
    let mut nearest = Vec::with_capacity(max_samples.min(samples.len()));
    for (index, sample) in samples.iter().enumerate() {
        let dist_sq = position.distance_squared(*sample);
        if let Some(max_radius_sq) = settings.max_radius_sq {
            if dist_sq > max_radius_sq {
                continue;
            }
        }
        if nearest.len() < max_samples {
            nearest.push(NeighborSample { index, dist_sq });
            if nearest.len() == max_samples {
                nearest.sort_by(|a, b| a.dist_sq.total_cmp(&b.dist_sq));
            }
            continue;
        }
        if let Some(last) = nearest.last_mut() {
            if dist_sq < last.dist_sq {
                *last = NeighborSample { index, dist_sq };
                let mut cursor = nearest.len().saturating_sub(1);
                while cursor > 0 && nearest[cursor].dist_sq < nearest[cursor - 1].dist_sq {
                    nearest.swap(cursor, cursor - 1);
                    cursor -= 1;
                }
            }
        }
    }
    if nearest.len() > 1 {
        nearest.sort_by(|a, b| a.dist_sq.total_cmp(&b.dist_sq));
    }
    nearest
}

fn combine_float(
    samples: &[NeighborSample],
    source_values: &[f32],
    settings: TransferSettings,
) -> Option<f32> {
    if samples.is_empty() {
        return None;
    }
    match settings.combine {
        TransferCombineMode::Closest => source_values.get(samples[0].index).copied(),
        TransferCombineMode::Average => {
            let mut sum = 0.0f32;
            let mut count = 0usize;
            for sample in samples {
                if let Some(value) = source_values.get(sample.index).copied() {
                    sum += value;
                    count += 1;
                }
            }
            if count == 0 {
                None
            } else {
                Some(sum / count as f32)
            }
        }
        TransferCombineMode::DistanceWeighted => {
            let mut weighted_sum = 0.0f32;
            let mut weight_sum = 0.0f32;
            for sample in samples {
                if let Some(value) = source_values.get(sample.index).copied() {
                    let weight = 1.0 / sample.dist_sq.max(1.0e-12);
                    weighted_sum += value * weight;
                    weight_sum += weight;
                }
            }
            if weight_sum <= 0.0 {
                None
            } else {
                Some(weighted_sum / weight_sum)
            }
        }
        TransferCombineMode::Minimum => {
            let mut value = f32::INFINITY;
            let mut any = false;
            for sample in samples {
                if let Some(current) = source_values.get(sample.index).copied() {
                    value = value.min(current);
                    any = true;
                }
            }
            any.then_some(value)
        }
        TransferCombineMode::Maximum => {
            let mut value = f32::NEG_INFINITY;
            let mut any = false;
            for sample in samples {
                if let Some(current) = source_values.get(sample.index).copied() {
                    value = value.max(current);
                    any = true;
                }
            }
            any.then_some(value)
        }
    }
}

fn combine_int(
    samples: &[NeighborSample],
    source_values: &[i32],
    settings: TransferSettings,
) -> Option<i32> {
    if samples.is_empty() {
        return None;
    }
    match settings.combine {
        TransferCombineMode::Closest => source_values.get(samples[0].index).copied(),
        TransferCombineMode::Average => {
            let mut sum = 0.0f32;
            let mut count = 0usize;
            for sample in samples {
                if let Some(value) = source_values.get(sample.index).copied() {
                    sum += value as f32;
                    count += 1;
                }
            }
            if count == 0 {
                None
            } else {
                Some((sum / count as f32).round() as i32)
            }
        }
        TransferCombineMode::DistanceWeighted => {
            let mut weighted_sum = 0.0f32;
            let mut weight_sum = 0.0f32;
            for sample in samples {
                if let Some(value) = source_values.get(sample.index).copied() {
                    let weight = 1.0 / sample.dist_sq.max(1.0e-12);
                    weighted_sum += value as f32 * weight;
                    weight_sum += weight;
                }
            }
            if weight_sum <= 0.0 {
                None
            } else {
                Some((weighted_sum / weight_sum).round() as i32)
            }
        }
        TransferCombineMode::Minimum => {
            let mut value = i32::MAX;
            let mut any = false;
            for sample in samples {
                if let Some(current) = source_values.get(sample.index).copied() {
                    value = value.min(current);
                    any = true;
                }
            }
            any.then_some(value)
        }
        TransferCombineMode::Maximum => {
            let mut value = i32::MIN;
            let mut any = false;
            for sample in samples {
                if let Some(current) = source_values.get(sample.index).copied() {
                    value = value.max(current);
                    any = true;
                }
            }
            any.then_some(value)
        }
    }
}

fn combine_vec2(
    samples: &[NeighborSample],
    source_values: &[[f32; 2]],
    settings: TransferSettings,
) -> Option<[f32; 2]> {
    if samples.is_empty() {
        return None;
    }
    match settings.combine {
        TransferCombineMode::Closest => source_values.get(samples[0].index).copied(),
        TransferCombineMode::Average => {
            let mut sum = [0.0f32; 2];
            let mut count = 0usize;
            for sample in samples {
                if let Some(value) = source_values.get(sample.index).copied() {
                    sum[0] += value[0];
                    sum[1] += value[1];
                    count += 1;
                }
            }
            if count == 0 {
                None
            } else {
                Some([sum[0] / count as f32, sum[1] / count as f32])
            }
        }
        TransferCombineMode::DistanceWeighted => {
            let mut sum = [0.0f32; 2];
            let mut weight_sum = 0.0f32;
            for sample in samples {
                if let Some(value) = source_values.get(sample.index).copied() {
                    let weight = 1.0 / sample.dist_sq.max(1.0e-12);
                    sum[0] += value[0] * weight;
                    sum[1] += value[1] * weight;
                    weight_sum += weight;
                }
            }
            if weight_sum <= 0.0 {
                None
            } else {
                Some([sum[0] / weight_sum, sum[1] / weight_sum])
            }
        }
        TransferCombineMode::Minimum => {
            let mut value = [f32::INFINITY; 2];
            let mut any = false;
            for sample in samples {
                if let Some(current) = source_values.get(sample.index).copied() {
                    value[0] = value[0].min(current[0]);
                    value[1] = value[1].min(current[1]);
                    any = true;
                }
            }
            any.then_some(value)
        }
        TransferCombineMode::Maximum => {
            let mut value = [f32::NEG_INFINITY; 2];
            let mut any = false;
            for sample in samples {
                if let Some(current) = source_values.get(sample.index).copied() {
                    value[0] = value[0].max(current[0]);
                    value[1] = value[1].max(current[1]);
                    any = true;
                }
            }
            any.then_some(value)
        }
    }
}

fn combine_vec3(
    samples: &[NeighborSample],
    source_values: &[[f32; 3]],
    settings: TransferSettings,
) -> Option<[f32; 3]> {
    if samples.is_empty() {
        return None;
    }
    match settings.combine {
        TransferCombineMode::Closest => source_values.get(samples[0].index).copied(),
        TransferCombineMode::Average => {
            let mut sum = [0.0f32; 3];
            let mut count = 0usize;
            for sample in samples {
                if let Some(value) = source_values.get(sample.index).copied() {
                    sum[0] += value[0];
                    sum[1] += value[1];
                    sum[2] += value[2];
                    count += 1;
                }
            }
            if count == 0 {
                None
            } else {
                Some([
                    sum[0] / count as f32,
                    sum[1] / count as f32,
                    sum[2] / count as f32,
                ])
            }
        }
        TransferCombineMode::DistanceWeighted => {
            let mut sum = [0.0f32; 3];
            let mut weight_sum = 0.0f32;
            for sample in samples {
                if let Some(value) = source_values.get(sample.index).copied() {
                    let weight = 1.0 / sample.dist_sq.max(1.0e-12);
                    sum[0] += value[0] * weight;
                    sum[1] += value[1] * weight;
                    sum[2] += value[2] * weight;
                    weight_sum += weight;
                }
            }
            if weight_sum <= 0.0 {
                None
            } else {
                Some([
                    sum[0] / weight_sum,
                    sum[1] / weight_sum,
                    sum[2] / weight_sum,
                ])
            }
        }
        TransferCombineMode::Minimum => {
            let mut value = [f32::INFINITY; 3];
            let mut any = false;
            for sample in samples {
                if let Some(current) = source_values.get(sample.index).copied() {
                    value[0] = value[0].min(current[0]);
                    value[1] = value[1].min(current[1]);
                    value[2] = value[2].min(current[2]);
                    any = true;
                }
            }
            any.then_some(value)
        }
        TransferCombineMode::Maximum => {
            let mut value = [f32::NEG_INFINITY; 3];
            let mut any = false;
            for sample in samples {
                if let Some(current) = source_values.get(sample.index).copied() {
                    value[0] = value[0].max(current[0]);
                    value[1] = value[1].max(current[1]);
                    value[2] = value[2].max(current[2]);
                    any = true;
                }
            }
            any.then_some(value)
        }
    }
}

fn combine_vec4(
    samples: &[NeighborSample],
    source_values: &[[f32; 4]],
    settings: TransferSettings,
) -> Option<[f32; 4]> {
    if samples.is_empty() {
        return None;
    }
    match settings.combine {
        TransferCombineMode::Closest => source_values.get(samples[0].index).copied(),
        TransferCombineMode::Average => {
            let mut sum = [0.0f32; 4];
            let mut count = 0usize;
            for sample in samples {
                if let Some(value) = source_values.get(sample.index).copied() {
                    sum[0] += value[0];
                    sum[1] += value[1];
                    sum[2] += value[2];
                    sum[3] += value[3];
                    count += 1;
                }
            }
            if count == 0 {
                None
            } else {
                Some([
                    sum[0] / count as f32,
                    sum[1] / count as f32,
                    sum[2] / count as f32,
                    sum[3] / count as f32,
                ])
            }
        }
        TransferCombineMode::DistanceWeighted => {
            let mut sum = [0.0f32; 4];
            let mut weight_sum = 0.0f32;
            for sample in samples {
                if let Some(value) = source_values.get(sample.index).copied() {
                    let weight = 1.0 / sample.dist_sq.max(1.0e-12);
                    sum[0] += value[0] * weight;
                    sum[1] += value[1] * weight;
                    sum[2] += value[2] * weight;
                    sum[3] += value[3] * weight;
                    weight_sum += weight;
                }
            }
            if weight_sum <= 0.0 {
                None
            } else {
                Some([
                    sum[0] / weight_sum,
                    sum[1] / weight_sum,
                    sum[2] / weight_sum,
                    sum[3] / weight_sum,
                ])
            }
        }
        TransferCombineMode::Minimum => {
            let mut value = [f32::INFINITY; 4];
            let mut any = false;
            for sample in samples {
                if let Some(current) = source_values.get(sample.index).copied() {
                    value[0] = value[0].min(current[0]);
                    value[1] = value[1].min(current[1]);
                    value[2] = value[2].min(current[2]);
                    value[3] = value[3].min(current[3]);
                    any = true;
                }
            }
            any.then_some(value)
        }
        TransferCombineMode::Maximum => {
            let mut value = [f32::NEG_INFINITY; 4];
            let mut any = false;
            for sample in samples {
                if let Some(current) = source_values.get(sample.index).copied() {
                    value[0] = value[0].max(current[0]);
                    value[1] = value[1].max(current[1]);
                    value[2] = value[2].max(current[2]);
                    value[3] = value[3].max(current[3]);
                    any = true;
                }
            }
            any.then_some(value)
        }
    }
}

fn combine_string_index(
    samples: &[NeighborSample],
    source_values: &[u32],
    settings: TransferSettings,
) -> Option<u32> {
    if samples.is_empty() {
        return None;
    }
    if settings.combine == TransferCombineMode::Closest {
        return source_values.get(samples[0].index).copied();
    }

    if settings.combine == TransferCombineMode::DistanceWeighted {
        let mut weighted: HashMap<u32, f32> = HashMap::new();
        for sample in samples {
            if let Some(value) = source_values.get(sample.index).copied() {
                let weight = 1.0 / sample.dist_sq.max(1.0e-12);
                *weighted.entry(value).or_insert(0.0) += weight;
            }
        }
        let mut best = source_values.get(samples[0].index).copied().unwrap_or(0);
        let mut best_score = f32::NEG_INFINITY;
        for sample in samples {
            if let Some(value) = source_values.get(sample.index).copied() {
                let score = weighted.get(&value).copied().unwrap_or(0.0);
                if score > best_score {
                    best_score = score;
                    best = value;
                }
            }
        }
        return Some(best);
    }

    let mut counts: HashMap<u32, u32> = HashMap::new();
    for sample in samples {
        if let Some(value) = source_values.get(sample.index).copied() {
            *counts.entry(value).or_insert(0) += 1;
        }
    }
    let mut best = source_values.get(samples[0].index).copied().unwrap_or(0);
    let mut best_count = 0u32;
    for sample in samples {
        if let Some(value) = source_values.get(sample.index).copied() {
            let count = counts.get(&value).copied().unwrap_or(0);
            if count > best_count {
                best_count = count;
                best = value;
            }
        }
    }
    Some(best)
}

fn append_string_table_values(combined: &mut StringTableAttribute, source: &StringTableAttribute) {
    if source.indices.is_empty() {
        return;
    }
    let mut lookup: HashMap<String, u32> = HashMap::new();
    for (idx, value) in combined.values.iter().enumerate() {
        lookup.insert(value.clone(), idx as u32);
    }
    for &index in &source.indices {
        let value = source
            .values
            .get(index as usize)
            .cloned()
            .unwrap_or_default();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::{AttributeDomain, AttributeRef, AttributeStorage};

    fn params_with_overrides(overrides: &[(&str, ParamValue)]) -> NodeParams {
        let mut params = default_params();
        for (key, value) in overrides {
            params.values.insert((*key).to_string(), value.clone());
        }
        params
    }

    #[test]
    fn transfer_average_respects_sample_count() {
        let mut source =
            Mesh::with_positions_indices(vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]], vec![0, 1, 1]);
        source
            .set_attribute(
                AttributeDomain::Point,
                "w",
                AttributeStorage::Float(vec![0.0, 2.0]),
            )
            .expect("set source attr");

        let target = Mesh::with_positions_indices(vec![[0.5, 0.0, 0.0]], vec![0, 0, 0]);
        let params = params_with_overrides(&[
            ("attr", ParamValue::String("w".to_string())),
            ("domain", ParamValue::Int(0)),
            ("sample_count", ParamValue::Int(2)),
            ("combine", ParamValue::Int(1)),
        ]);
        let out = compute(&params, &[target, source]).expect("transfer");

        let Some(AttributeRef::Float(values)) = out.attribute(AttributeDomain::Point, "w") else {
            panic!("missing float attribute");
        };
        assert_eq!(values.len(), 1);
        assert!((values[0] - 1.0).abs() < 1.0e-6);
    }

    #[test]
    fn transfer_max_radius_keeps_existing_value_when_no_neighbor() {
        let mut source = Mesh::with_positions_indices(vec![[0.0, 0.0, 0.0]], vec![0, 0, 0]);
        source
            .set_attribute(
                AttributeDomain::Point,
                "w",
                AttributeStorage::Float(vec![3.0]),
            )
            .expect("set source attr");

        let mut target = Mesh::with_positions_indices(vec![[10.0, 0.0, 0.0]], vec![0, 0, 0]);
        target
            .set_attribute(
                AttributeDomain::Point,
                "w",
                AttributeStorage::Float(vec![7.0]),
            )
            .expect("set target attr");

        let params = params_with_overrides(&[
            ("attr", ParamValue::String("w".to_string())),
            ("domain", ParamValue::Int(0)),
            ("max_radius", ParamValue::Float(1.0)),
        ]);
        let out = compute(&params, &[target, source]).expect("transfer");

        let Some(AttributeRef::Float(values)) = out.attribute(AttributeDomain::Point, "w") else {
            panic!("missing float attribute");
        };
        assert_eq!(values, &[7.0]);
    }

    #[test]
    fn transfer_max_mode_uses_largest_of_neighbors() {
        let mut source =
            Mesh::with_positions_indices(vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]], vec![0, 1, 1]);
        source
            .set_attribute(
                AttributeDomain::Point,
                "w",
                AttributeStorage::Float(vec![1.0, 5.0]),
            )
            .expect("set source attr");

        let target = Mesh::with_positions_indices(vec![[0.5, 0.0, 0.0]], vec![0, 0, 0]);
        let params = params_with_overrides(&[
            ("attr", ParamValue::String("w".to_string())),
            ("domain", ParamValue::Int(0)),
            ("sample_count", ParamValue::Int(2)),
            ("combine", ParamValue::Int(4)),
        ]);
        let out = compute(&params, &[target, source]).expect("transfer");

        let Some(AttributeRef::Float(values)) = out.attribute(AttributeDomain::Point, "w") else {
            panic!("missing float attribute");
        };
        assert_eq!(values, &[5.0]);
    }
}
