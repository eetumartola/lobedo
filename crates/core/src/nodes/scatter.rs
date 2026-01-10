use std::collections::BTreeMap;

use glam::Vec3;

use crate::attributes::{AttributeDomain, AttributeRef, AttributeStorage, StringTableAttribute};
use crate::curve::Curve;
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    attribute_utils::parse_attribute_list, geometry_in, geometry_out, group_utils::mesh_group_mask,
    require_mesh_input,
};
use crate::volume::{Volume, VolumeKind};
use crate::volume_sampling::VolumeSampler;

pub const NAME: &str = "Scatter";

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
            ("count".to_string(), ParamValue::Int(100)),
            ("seed".to_string(), ParamValue::Int(1)),
            ("density_attr".to_string(), ParamValue::String("density".to_string())),
            ("density_min".to_string(), ParamValue::Float(0.0)),
            ("density_max".to_string(), ParamValue::Float(1.0)),
            ("inherit".to_string(), ParamValue::String("Cd".to_string())),
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let input = require_mesh_input(inputs, 0, "Scatter requires a mesh input")?;
    let count = params.get_int("count", 200).max(0) as usize;
    let seed = params.get_int("seed", 1).max(0) as u32;
    let density_attr = params.get_string("density_attr", "").trim().to_string();
    let density_min = params.get_float("density_min", 0.0);
    let density_max = params.get_float("density_max", 1.0);
    let inherit = parse_attribute_list(params.get_string("inherit", "Cd"));
    let mask = mesh_group_mask(&input, params, AttributeDomain::Primitive);
    scatter_points(
        &input,
        count,
        seed,
        mask.as_deref(),
        if density_attr.is_empty() {
            None
        } else {
            Some(density_attr.as_str())
        },
        density_min,
        density_max,
        &inherit,
    )
}

pub fn apply_to_geometry(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };
    let count = params.get_int("count", 200).max(0) as usize;
    if count == 0 {
        return Ok(Geometry::default());
    }
    let seed = params.get_int("seed", 1).max(0) as u32;
    let density_attr = params.get_string("density_attr", "").trim().to_string();
    let density_min = params.get_float("density_min", 0.0);
    let density_max = params.get_float("density_max", 1.0);
    let inherit = parse_attribute_list(params.get_string("inherit", "Cd"));

    let merged_mesh = input.merged_mesh();
    if let Some(mesh) = merged_mesh.as_ref() {
        if !mesh.positions.is_empty()
            && mesh.indices.len() >= 3
            && mesh.indices.len().is_multiple_of(3)
        {
            let mask = mesh_group_mask(mesh, params, AttributeDomain::Primitive);
            let mesh = scatter_points(
                mesh,
                count,
                seed,
                mask.as_deref(),
                if density_attr.is_empty() {
                    None
                } else {
                    Some(density_attr.as_str())
                },
                density_min,
                density_max,
                &inherit,
            )?;
            return Ok(Geometry::with_mesh(mesh));
        }
    }

    if !input.curves.is_empty() {
        if let Some(mesh_source) = merged_mesh.as_ref() {
            let mesh = scatter_curves(
                mesh_source,
                &input.curves,
                count,
                seed,
                if density_attr.is_empty() {
                    None
                } else {
                    Some(density_attr.as_str())
                },
                density_min,
                density_max,
                &inherit,
            )?;
            return Ok(Geometry::with_mesh(mesh));
        }
    }

    if let Some(volume) = input.volumes.first() {
        let mesh = scatter_volume(volume, count, seed)?;
        return Ok(Geometry::with_mesh(mesh));
    }

    Ok(Geometry::default())
}

#[allow(clippy::too_many_arguments)]
fn scatter_points(
    input: &Mesh,
    count: usize,
    seed: u32,
    mask: Option<&[bool]>,
    density_attr: Option<&str>,
    density_min: f32,
    density_max: f32,
    inherit: &[String],
) -> Result<Mesh, String> {
    if count == 0 {
        return Ok(Mesh::default());
    }
    if !input.indices.len().is_multiple_of(3) || input.positions.is_empty() {
        return Err("Scatter requires a triangle mesh input".to_string());
    }

    let density_source: Option<DensitySource<'_>> =
        density_attr.and_then(|name| mesh_density_source(input, name));
    let inherit_sources = build_mesh_inherit_sources(input, inherit);
    let mut inherit_buffers = build_inherit_buffers(&inherit_sources, count);

    let mut areas = Vec::new();
    let mut total = 0.0f32;
    for (prim_index, tri) in input.indices.chunks_exact(3).enumerate() {
        if mask
            .as_ref()
            .is_some_and(|mask| !mask.get(prim_index).copied().unwrap_or(false))
        {
            areas.push(total);
            continue;
        }
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;
        if i0 >= input.positions.len() || i1 >= input.positions.len() || i2 >= input.positions.len()
        {
            areas.push(total);
            continue;
        }
        let p0 = Vec3::from(input.positions[i0]);
        let p1 = Vec3::from(input.positions[i1]);
        let p2 = Vec3::from(input.positions[i2]);
        let area = 0.5 * (p1 - p0).cross(p2 - p0).length();
        let density = density_source
            .as_ref()
            .map(|source| {
                map_density_value(source.sample(prim_index, i0, i1, i2), density_min, density_max)
            })
            .unwrap_or(1.0);
        let weight = area.max(0.0) * density.max(0.0);
        total += weight;
        areas.push(total);
    }

    if total <= 0.0 {
        return if mask.is_some() {
            Ok(Mesh::default())
        } else {
            Err("Scatter requires non-degenerate triangles".to_string())
        };
    }

    let mut rng = XorShift32::new(seed);
    let mut positions = Vec::with_capacity(count);
    let mut normals = Vec::with_capacity(count);

    for _ in 0..count {
        let sample = rng.next_f32() * total;
        let tri_index = find_area_index(&areas, sample);
        let tri = input
            .indices
            .get(tri_index * 3..tri_index * 3 + 3)
            .ok_or_else(|| "scatter triangle index out of range".to_string())?;
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;
        let p0 = Vec3::from(input.positions[i0]);
        let p1 = Vec3::from(input.positions[i1]);
        let p2 = Vec3::from(input.positions[i2]);

        let r1 = rng.next_f32().clamp(0.0, 1.0);
        let r2 = rng.next_f32().clamp(0.0, 1.0);
        let sqrt_r1 = r1.sqrt();
        let u = 1.0 - sqrt_r1;
        let v = r2 * sqrt_r1;
        let w = 1.0 - u - v;
        let point = p0 * u + p1 * v + p2 * w;

        let normal = (p1 - p0).cross(p2 - p0);
        let normal = if normal.length_squared() > 0.0 {
            normal.normalize().to_array()
        } else {
            [0.0, 1.0, 0.0]
        };

        positions.push(point.to_array());
        normals.push(normal);
        let weights = [u, v, w];
        apply_mesh_inherit(
            &inherit_sources,
            &mut inherit_buffers,
            tri_index,
            [i0, i1, i2],
            weights,
        );
    }

    let mut mesh = Mesh {
        positions,
        indices: Vec::new(),
        normals: Some(normals),
        corner_normals: None,
        uvs: None,
        attributes: Default::default(),
        groups: Default::default(),
    };
    apply_inherit_buffers(&mut mesh, inherit_buffers)?;
    Ok(mesh)
}

#[allow(clippy::too_many_arguments)]
fn scatter_curves(
    mesh: &Mesh,
    curves: &[Curve],
    count: usize,
    seed: u32,
    density_attr: Option<&str>,
    density_min: f32,
    density_max: f32,
    inherit: &[String],
) -> Result<Mesh, String> {
    if count == 0 || mesh.positions.is_empty() {
        return Ok(Mesh::default());
    }

    let positions = mesh.positions.as_slice();
    let density_source: Option<CurveDensitySource<'_>> =
        density_attr.and_then(|name| curve_density_source(mesh, name));
    let inherit_sources = build_curve_inherit_sources(mesh, inherit);
    let mut inherit_buffers = build_inherit_buffers(&inherit_sources, count);

    let mut segments = Vec::new();
    let mut cumulative = Vec::new();
    let mut total = 0.0f32;
    for curve in curves {
        let indices = &curve.indices;
        if indices.len() < 2 {
            continue;
        }
        let seg_count = if curve.closed { indices.len() } else { indices.len() - 1 };
        for i in 0..seg_count {
            let a = indices.get(i).copied().unwrap_or(0) as usize;
            let b = indices
                .get((i + 1) % indices.len())
                .copied()
                .unwrap_or(0) as usize;
            let Some(p0) = positions.get(a) else { continue };
            let Some(p1) = positions.get(b) else { continue };
            let p0 = Vec3::from(*p0);
            let p1 = Vec3::from(*p1);
            let len = (p1 - p0).length();
            if len <= 0.0 {
                continue;
            }
            let density = density_source
                .as_ref()
                .map(|source| map_density_value(source.sample(a, b), density_min, density_max))
                .unwrap_or(1.0);
            let weight = len * density.max(0.0);
            total += weight;
            segments.push(CurveSegment { a, b, p0, p1 });
            cumulative.push(total);
        }
    }

    if total <= 0.0 || segments.is_empty() {
        return Ok(Mesh::default());
    }

    let mut rng = XorShift32::new(seed);
    let mut out_positions = Vec::with_capacity(count);
    let mut normals = Vec::with_capacity(count);
    for _ in 0..count {
        let sample = rng.next_f32() * total;
        let seg_index = find_area_index(&cumulative, sample).min(segments.len() - 1);
        let segment = &segments[seg_index];
        let t = rng.next_f32().clamp(0.0, 1.0);
        let point = segment.p0.lerp(segment.p1, t);
        out_positions.push(point.to_array());
        normals.push([0.0, 1.0, 0.0]);
        apply_curve_inherit(
            &inherit_sources,
            &mut inherit_buffers,
            segment.a,
            segment.b,
            t,
        );
    }

    let mut mesh = Mesh {
        positions: out_positions,
        indices: Vec::new(),
        normals: Some(normals),
        corner_normals: None,
        uvs: None,
        attributes: Default::default(),
        groups: Default::default(),
    };
    apply_inherit_buffers(&mut mesh, inherit_buffers)?;
    Ok(mesh)
}

fn scatter_volume(volume: &Volume, count: usize, seed: u32) -> Result<Mesh, String> {
    if count == 0 || volume.values.is_empty() {
        return Ok(Mesh::default());
    }

    let (world_min, world_max) = volume.world_bounds();
    let sampler = VolumeSampler::new(volume);
    let max_density = if matches!(volume.kind, VolumeKind::Density) {
        volume
            .values
            .iter()
            .copied()
            .fold(0.0_f32, f32::max)
            .max(1.0e-6)
    } else {
        1.0
    };

    let mut rng = XorShift32::new(seed);
    let mut out_positions = Vec::with_capacity(count);
    let mut normals = Vec::with_capacity(count);
    let mut attempts = 0usize;
    let max_attempts = (count.max(1) * 50).max(100);
    while out_positions.len() < count && attempts < max_attempts {
        attempts += 1;
        let x = world_min.x + rng.next_f32() * (world_max.x - world_min.x);
        let y = world_min.y + rng.next_f32() * (world_max.y - world_min.y);
        let z = world_min.z + rng.next_f32() * (world_max.z - world_min.z);
        let world_pos = Vec3::new(x, y, z);
        let mut density = sampler.sample_world(world_pos);
        if matches!(volume.kind, VolumeKind::Sdf) {
            density = if density <= 0.0 { 1.0 } else { 0.0 };
        }
        if density <= 0.0 {
            continue;
        }
        let accept = (density / max_density).clamp(0.0, 1.0);
        if rng.next_f32() <= accept {
            out_positions.push(world_pos.to_array());
            normals.push([0.0, 1.0, 0.0]);
        }
    }

    Ok(Mesh {
        positions: out_positions,
        indices: Vec::new(),
        normals: Some(normals),
        corner_normals: None,
        uvs: None,
        attributes: Default::default(),
        groups: Default::default(),
    })
}

fn find_area_index(cumulative: &[f32], sample: f32) -> usize {
    let mut lo = 0usize;
    let mut hi = cumulative.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        if sample < cumulative[mid] {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    lo.min(cumulative.len().saturating_sub(1))
}

fn map_density_value(value: f32, min: f32, max: f32) -> f32 {
    if !value.is_finite() {
        return 0.0;
    }
    let t = value.clamp(0.0, 1.0);
    min + (max - min) * t
}

struct DensitySource<'a> {
    domain: AttributeDomain,
    attr: AttributeRef<'a>,
}

impl<'a> DensitySource<'a> {
    fn sample(&self, prim_index: usize, i0: usize, i1: usize, i2: usize) -> f32 {
        match self.domain {
            AttributeDomain::Point => sample_numeric_point(self.attr, [i0, i1, i2]),
            AttributeDomain::Vertex => {
                let base = prim_index * 3;
                sample_numeric_point(self.attr, [base, base + 1, base + 2])
            }
            AttributeDomain::Primitive => sample_numeric_single(self.attr, prim_index),
            AttributeDomain::Detail => sample_numeric_single(self.attr, 0),
        }
    }
}

struct CurveDensitySource<'a> {
    domain: AttributeDomain,
    attr: AttributeRef<'a>,
}

impl<'a> CurveDensitySource<'a> {
    fn sample(&self, a: usize, b: usize) -> f32 {
        match self.domain {
            AttributeDomain::Point => {
                let da = sample_numeric_single(self.attr, a);
                let db = sample_numeric_single(self.attr, b);
                (da + db) * 0.5
            }
            AttributeDomain::Detail => sample_numeric_single(self.attr, 0),
            _ => 1.0,
        }
    }
}

#[derive(Clone)]
struct InheritSource<'a> {
    name: String,
    domain: AttributeDomain,
    attr: AttributeRef<'a>,
}

enum InheritBuffer {
    Float { name: String, values: Vec<f32> },
    Int { name: String, values: Vec<i32> },
    Vec2 { name: String, values: Vec<[f32; 2]> },
    Vec3 { name: String, values: Vec<[f32; 3]> },
    Vec4 { name: String, values: Vec<[f32; 4]> },
    StringTable { name: String, values: Vec<String>, indices: Vec<u32> },
}

fn mesh_density_source<'a>(mesh: &'a Mesh, name: &str) -> Option<DensitySource<'a>> {
    let (domain, attr) = mesh.attribute_with_precedence(name)?;
    if attr.is_empty() {
        return None;
    }
    Some(DensitySource { domain, attr })
}

fn curve_density_source<'a>(mesh: &'a Mesh, name: &str) -> Option<CurveDensitySource<'a>> {
    if let Some(attr) = mesh.attribute(AttributeDomain::Point, name) {
        if !attr.is_empty() {
            return Some(CurveDensitySource {
                domain: AttributeDomain::Point,
                attr,
            });
        }
    }
    if let Some(attr) = mesh.attribute(AttributeDomain::Detail, name) {
        if !attr.is_empty() {
            return Some(CurveDensitySource {
                domain: AttributeDomain::Detail,
                attr,
            });
        }
    }
    None
}

fn build_mesh_inherit_sources<'a>(mesh: &'a Mesh, names: &[String]) -> Vec<InheritSource<'a>> {
    let mut sources = Vec::new();
    for name in names {
        let Some((domain, attr)) = mesh.attribute_with_precedence(name) else {
            continue;
        };
        if attr.is_empty() {
            continue;
        }
        sources.push(InheritSource {
            name: name.clone(),
            domain,
            attr,
        });
    }
    sources
}

fn build_curve_inherit_sources<'a>(mesh: &'a Mesh, names: &[String]) -> Vec<InheritSource<'a>> {
    let mut sources = Vec::new();
    for name in names {
        if let Some(attr) = mesh.attribute(AttributeDomain::Point, name) {
            if !attr.is_empty() {
                sources.push(InheritSource {
                    name: name.clone(),
                    domain: AttributeDomain::Point,
                    attr,
                });
            }
            continue;
        }
        if let Some(attr) = mesh.attribute(AttributeDomain::Detail, name) {
            if !attr.is_empty() {
                sources.push(InheritSource {
                    name: name.clone(),
                    domain: AttributeDomain::Detail,
                    attr,
                });
            }
        }
    }
    sources
}

fn build_inherit_buffers(sources: &[InheritSource<'_>], count: usize) -> Vec<InheritBuffer> {
    sources
        .iter()
        .map(|source| match source.attr {
            AttributeRef::Float(_) => InheritBuffer::Float {
                name: source.name.clone(),
                values: Vec::with_capacity(count),
            },
            AttributeRef::Int(_) => InheritBuffer::Int {
                name: source.name.clone(),
                values: Vec::with_capacity(count),
            },
            AttributeRef::Vec2(_) => InheritBuffer::Vec2 {
                name: source.name.clone(),
                values: Vec::with_capacity(count),
            },
            AttributeRef::Vec3(_) => InheritBuffer::Vec3 {
                name: source.name.clone(),
                values: Vec::with_capacity(count),
            },
            AttributeRef::Vec4(_) => InheritBuffer::Vec4 {
                name: source.name.clone(),
                values: Vec::with_capacity(count),
            },
            AttributeRef::StringTable(values) => InheritBuffer::StringTable {
                name: source.name.clone(),
                values: values.values.clone(),
                indices: Vec::with_capacity(count),
            },
        })
        .collect()
}

fn apply_mesh_inherit(
    sources: &[InheritSource<'_>],
    buffers: &mut [InheritBuffer],
    prim_index: usize,
    point_indices: [usize; 3],
    weights: [f32; 3],
) {
    for (source, buffer) in sources.iter().zip(buffers.iter_mut()) {
        match (source.domain, &source.attr, buffer) {
            (AttributeDomain::Point, attr, InheritBuffer::Float { values, .. }) => {
                values.push(sample_numeric_weighted(*attr, point_indices, weights));
            }
            (AttributeDomain::Vertex, attr, InheritBuffer::Float { values, .. }) => {
                let base = prim_index * 3;
                values.push(sample_numeric_weighted(*attr, [base, base + 1, base + 2], weights));
            }
            (AttributeDomain::Primitive, attr, InheritBuffer::Float { values, .. }) => {
                values.push(sample_numeric_single(*attr, prim_index));
            }
            (AttributeDomain::Detail, attr, InheritBuffer::Float { values, .. }) => {
                values.push(sample_numeric_single(*attr, 0));
            }
            (AttributeDomain::Point, attr, InheritBuffer::Int { values, .. }) => {
                values.push(sample_int_weighted(*attr, point_indices, weights));
            }
            (AttributeDomain::Vertex, attr, InheritBuffer::Int { values, .. }) => {
                let base = prim_index * 3;
                values.push(sample_int_weighted(*attr, [base, base + 1, base + 2], weights));
            }
            (AttributeDomain::Primitive, attr, InheritBuffer::Int { values, .. }) => {
                values.push(sample_int_single(*attr, prim_index));
            }
            (AttributeDomain::Detail, attr, InheritBuffer::Int { values, .. }) => {
                values.push(sample_int_single(*attr, 0));
            }
            (AttributeDomain::Point, attr, InheritBuffer::Vec2 { values, .. }) => {
                values.push(sample_vec2_weighted(*attr, point_indices, weights));
            }
            (AttributeDomain::Vertex, attr, InheritBuffer::Vec2 { values, .. }) => {
                let base = prim_index * 3;
                values.push(sample_vec2_weighted(*attr, [base, base + 1, base + 2], weights));
            }
            (AttributeDomain::Primitive, attr, InheritBuffer::Vec2 { values, .. }) => {
                values.push(sample_vec2_single(*attr, prim_index));
            }
            (AttributeDomain::Detail, attr, InheritBuffer::Vec2 { values, .. }) => {
                values.push(sample_vec2_single(*attr, 0));
            }
            (AttributeDomain::Point, attr, InheritBuffer::Vec3 { values, .. }) => {
                values.push(sample_vec3_weighted(*attr, point_indices, weights));
            }
            (AttributeDomain::Vertex, attr, InheritBuffer::Vec3 { values, .. }) => {
                let base = prim_index * 3;
                values.push(sample_vec3_weighted(*attr, [base, base + 1, base + 2], weights));
            }
            (AttributeDomain::Primitive, attr, InheritBuffer::Vec3 { values, .. }) => {
                values.push(sample_vec3_single(*attr, prim_index));
            }
            (AttributeDomain::Detail, attr, InheritBuffer::Vec3 { values, .. }) => {
                values.push(sample_vec3_single(*attr, 0));
            }
            (AttributeDomain::Point, attr, InheritBuffer::Vec4 { values, .. }) => {
                values.push(sample_vec4_weighted(*attr, point_indices, weights));
            }
            (AttributeDomain::Vertex, attr, InheritBuffer::Vec4 { values, .. }) => {
                let base = prim_index * 3;
                values.push(sample_vec4_weighted(*attr, [base, base + 1, base + 2], weights));
            }
            (AttributeDomain::Primitive, attr, InheritBuffer::Vec4 { values, .. }) => {
                values.push(sample_vec4_single(*attr, prim_index));
            }
            (AttributeDomain::Detail, attr, InheritBuffer::Vec4 { values, .. }) => {
                values.push(sample_vec4_single(*attr, 0));
            }
            (AttributeDomain::Point, AttributeRef::StringTable(values), InheritBuffer::StringTable { indices, .. }) => {
                let idx = select_string_index(values, point_indices, weights);
                indices.push(idx);
            }
            (AttributeDomain::Vertex, AttributeRef::StringTable(values), InheritBuffer::StringTable { indices, .. }) => {
                let base = prim_index * 3;
                let idx = select_string_index(values, [base, base + 1, base + 2], weights);
                indices.push(idx);
            }
            (AttributeDomain::Primitive, AttributeRef::StringTable(values), InheritBuffer::StringTable { indices, .. }) => {
                indices.push(select_string_single(values, prim_index));
            }
            (AttributeDomain::Detail, AttributeRef::StringTable(values), InheritBuffer::StringTable { indices, .. }) => {
                indices.push(select_string_single(values, 0));
            }
            _ => {}
        }
    }
}

fn apply_curve_inherit(
    sources: &[InheritSource<'_>],
    buffers: &mut [InheritBuffer],
    a: usize,
    b: usize,
    t: f32,
) {
    for (source, buffer) in sources.iter().zip(buffers.iter_mut()) {
        match (source.domain, &source.attr, buffer) {
            (AttributeDomain::Point, attr, InheritBuffer::Float { values, .. }) => {
                values.push(sample_numeric_line(*attr, a, b, t));
            }
            (AttributeDomain::Detail, attr, InheritBuffer::Float { values, .. }) => {
                values.push(sample_numeric_single(*attr, 0));
            }
            (AttributeDomain::Point, attr, InheritBuffer::Int { values, .. }) => {
                values.push(sample_int_line(*attr, a, b, t));
            }
            (AttributeDomain::Detail, attr, InheritBuffer::Int { values, .. }) => {
                values.push(sample_int_single(*attr, 0));
            }
            (AttributeDomain::Point, attr, InheritBuffer::Vec2 { values, .. }) => {
                values.push(sample_vec2_line(*attr, a, b, t));
            }
            (AttributeDomain::Detail, attr, InheritBuffer::Vec2 { values, .. }) => {
                values.push(sample_vec2_single(*attr, 0));
            }
            (AttributeDomain::Point, attr, InheritBuffer::Vec3 { values, .. }) => {
                values.push(sample_vec3_line(*attr, a, b, t));
            }
            (AttributeDomain::Detail, attr, InheritBuffer::Vec3 { values, .. }) => {
                values.push(sample_vec3_single(*attr, 0));
            }
            (AttributeDomain::Point, attr, InheritBuffer::Vec4 { values, .. }) => {
                values.push(sample_vec4_line(*attr, a, b, t));
            }
            (AttributeDomain::Detail, attr, InheritBuffer::Vec4 { values, .. }) => {
                values.push(sample_vec4_single(*attr, 0));
            }
            (AttributeDomain::Point, AttributeRef::StringTable(values), InheritBuffer::StringTable { indices, .. }) => {
                let idx = if t < 0.5 {
                    select_string_single(values, a)
                } else {
                    select_string_single(values, b)
                };
                indices.push(idx);
            }
            (AttributeDomain::Detail, AttributeRef::StringTable(values), InheritBuffer::StringTable { indices, .. }) => {
                indices.push(select_string_single(values, 0));
            }
            _ => {}
        }
    }
}

fn apply_inherit_buffers(mesh: &mut Mesh, buffers: Vec<InheritBuffer>) -> Result<(), String> {
    for buffer in buffers {
        match buffer {
            InheritBuffer::Float { name, values } => {
                mesh.set_attribute(AttributeDomain::Point, name, AttributeStorage::Float(values))
                    .map_err(|err| format!("Scatter inherit error: {:?}", err))?;
            }
            InheritBuffer::Int { name, values } => {
                mesh.set_attribute(AttributeDomain::Point, name, AttributeStorage::Int(values))
                    .map_err(|err| format!("Scatter inherit error: {:?}", err))?;
            }
            InheritBuffer::Vec2 { name, values } => {
                mesh.set_attribute(AttributeDomain::Point, name, AttributeStorage::Vec2(values))
                    .map_err(|err| format!("Scatter inherit error: {:?}", err))?;
            }
            InheritBuffer::Vec3 { name, values } => {
                mesh.set_attribute(AttributeDomain::Point, name, AttributeStorage::Vec3(values))
                    .map_err(|err| format!("Scatter inherit error: {:?}", err))?;
            }
            InheritBuffer::Vec4 { name, values } => {
                mesh.set_attribute(AttributeDomain::Point, name, AttributeStorage::Vec4(values))
                    .map_err(|err| format!("Scatter inherit error: {:?}", err))?;
            }
            InheritBuffer::StringTable { name, values, indices } => {
                mesh.set_attribute(
                    AttributeDomain::Point,
                    name,
                    AttributeStorage::StringTable(StringTableAttribute::new(values, indices)),
                )
                .map_err(|err| format!("Scatter inherit error: {:?}", err))?;
            }
        }
    }
    Ok(())
}

fn sample_numeric_single(attr: AttributeRef<'_>, index: usize) -> f32 {
    match attr {
        AttributeRef::Float(values) => values.get(index).copied().unwrap_or(0.0),
        AttributeRef::Int(values) => values.get(index).copied().unwrap_or(0) as f32,
        AttributeRef::Vec2(values) => values
            .get(index)
            .map(|v| Vec3::new(v[0], v[1], 0.0).length())
            .unwrap_or(0.0),
        AttributeRef::Vec3(values) => values
            .get(index)
            .map(|v| Vec3::from(*v).length())
            .unwrap_or(0.0),
        AttributeRef::Vec4(values) => values
            .get(index)
            .map(|v| Vec3::new(v[0], v[1], v[2]).length())
            .unwrap_or(0.0),
        AttributeRef::StringTable(_) => 0.0,
    }
}

fn sample_numeric_point(attr: AttributeRef<'_>, indices: [usize; 3]) -> f32 {
    let v0 = sample_numeric_single(attr, indices[0]);
    let v1 = sample_numeric_single(attr, indices[1]);
    let v2 = sample_numeric_single(attr, indices[2]);
    (v0 + v1 + v2) / 3.0
}

fn sample_numeric_weighted(attr: AttributeRef<'_>, indices: [usize; 3], weights: [f32; 3]) -> f32 {
    let v0 = sample_numeric_single(attr, indices[0]);
    let v1 = sample_numeric_single(attr, indices[1]);
    let v2 = sample_numeric_single(attr, indices[2]);
    v0 * weights[0] + v1 * weights[1] + v2 * weights[2]
}

fn sample_int_single(attr: AttributeRef<'_>, index: usize) -> i32 {
    match attr {
        AttributeRef::Int(values) => values.get(index).copied().unwrap_or(0),
        _ => sample_numeric_single(attr, index).round() as i32,
    }
}

fn sample_int_weighted(attr: AttributeRef<'_>, indices: [usize; 3], weights: [f32; 3]) -> i32 {
    sample_numeric_weighted(attr, indices, weights).round() as i32
}

fn sample_vec2_single(attr: AttributeRef<'_>, index: usize) -> [f32; 2] {
    match attr {
        AttributeRef::Vec2(values) => values.get(index).copied().unwrap_or([0.0; 2]),
        AttributeRef::Float(values) => values
            .get(index)
            .copied()
            .map(|v| [v, v])
            .unwrap_or([0.0; 2]),
        AttributeRef::Int(values) => values
            .get(index)
            .copied()
            .map(|v| [v as f32, v as f32])
            .unwrap_or([0.0; 2]),
        AttributeRef::Vec3(values) => values
            .get(index)
            .map(|v| [v[0], v[1]])
            .unwrap_or([0.0; 2]),
        AttributeRef::Vec4(values) => values
            .get(index)
            .map(|v| [v[0], v[1]])
            .unwrap_or([0.0; 2]),
        AttributeRef::StringTable(_) => [0.0; 2],
    }
}

fn sample_vec2_weighted(attr: AttributeRef<'_>, indices: [usize; 3], weights: [f32; 3]) -> [f32; 2] {
    let a = sample_vec2_single(attr, indices[0]);
    let b = sample_vec2_single(attr, indices[1]);
    let c = sample_vec2_single(attr, indices[2]);
    [
        a[0] * weights[0] + b[0] * weights[1] + c[0] * weights[2],
        a[1] * weights[0] + b[1] * weights[1] + c[1] * weights[2],
    ]
}

fn sample_vec2_line(attr: AttributeRef<'_>, a: usize, b: usize, t: f32) -> [f32; 2] {
    let va = sample_vec2_single(attr, a);
    let vb = sample_vec2_single(attr, b);
    [
        va[0] * (1.0 - t) + vb[0] * t,
        va[1] * (1.0 - t) + vb[1] * t,
    ]
}

fn sample_vec3_single(attr: AttributeRef<'_>, index: usize) -> [f32; 3] {
    match attr {
        AttributeRef::Vec3(values) => values.get(index).copied().unwrap_or([0.0; 3]),
        AttributeRef::Float(values) => values
            .get(index)
            .copied()
            .map(|v| [v, v, v])
            .unwrap_or([0.0; 3]),
        AttributeRef::Int(values) => values
            .get(index)
            .copied()
            .map(|v| [v as f32, v as f32, v as f32])
            .unwrap_or([0.0; 3]),
        AttributeRef::Vec2(values) => values
            .get(index)
            .map(|v| [v[0], v[1], 0.0])
            .unwrap_or([0.0; 3]),
        AttributeRef::Vec4(values) => values
            .get(index)
            .map(|v| [v[0], v[1], v[2]])
            .unwrap_or([0.0; 3]),
        AttributeRef::StringTable(_) => [0.0; 3],
    }
}

fn sample_vec3_weighted(attr: AttributeRef<'_>, indices: [usize; 3], weights: [f32; 3]) -> [f32; 3] {
    let a = sample_vec3_single(attr, indices[0]);
    let b = sample_vec3_single(attr, indices[1]);
    let c = sample_vec3_single(attr, indices[2]);
    [
        a[0] * weights[0] + b[0] * weights[1] + c[0] * weights[2],
        a[1] * weights[0] + b[1] * weights[1] + c[1] * weights[2],
        a[2] * weights[0] + b[2] * weights[1] + c[2] * weights[2],
    ]
}

fn sample_vec3_line(attr: AttributeRef<'_>, a: usize, b: usize, t: f32) -> [f32; 3] {
    let va = sample_vec3_single(attr, a);
    let vb = sample_vec3_single(attr, b);
    [
        va[0] * (1.0 - t) + vb[0] * t,
        va[1] * (1.0 - t) + vb[1] * t,
        va[2] * (1.0 - t) + vb[2] * t,
    ]
}

fn sample_vec4_single(attr: AttributeRef<'_>, index: usize) -> [f32; 4] {
    match attr {
        AttributeRef::Vec4(values) => values.get(index).copied().unwrap_or([0.0; 4]),
        AttributeRef::Float(values) => values
            .get(index)
            .copied()
            .map(|v| [v, v, v, v])
            .unwrap_or([0.0; 4]),
        AttributeRef::Int(values) => values
            .get(index)
            .copied()
            .map(|v| [v as f32, v as f32, v as f32, v as f32])
            .unwrap_or([0.0; 4]),
        AttributeRef::Vec3(values) => values
            .get(index)
            .map(|v| [v[0], v[1], v[2], 0.0])
            .unwrap_or([0.0; 4]),
        AttributeRef::Vec2(values) => values
            .get(index)
            .map(|v| [v[0], v[1], 0.0, 0.0])
            .unwrap_or([0.0; 4]),
        AttributeRef::StringTable(_) => [0.0; 4],
    }
}

fn sample_vec4_weighted(attr: AttributeRef<'_>, indices: [usize; 3], weights: [f32; 3]) -> [f32; 4] {
    let a = sample_vec4_single(attr, indices[0]);
    let b = sample_vec4_single(attr, indices[1]);
    let c = sample_vec4_single(attr, indices[2]);
    [
        a[0] * weights[0] + b[0] * weights[1] + c[0] * weights[2],
        a[1] * weights[0] + b[1] * weights[1] + c[1] * weights[2],
        a[2] * weights[0] + b[2] * weights[1] + c[2] * weights[2],
        a[3] * weights[0] + b[3] * weights[1] + c[3] * weights[2],
    ]
}

fn sample_vec4_line(attr: AttributeRef<'_>, a: usize, b: usize, t: f32) -> [f32; 4] {
    let va = sample_vec4_single(attr, a);
    let vb = sample_vec4_single(attr, b);
    [
        va[0] * (1.0 - t) + vb[0] * t,
        va[1] * (1.0 - t) + vb[1] * t,
        va[2] * (1.0 - t) + vb[2] * t,
        va[3] * (1.0 - t) + vb[3] * t,
    ]
}

fn select_string_single(values: &StringTableAttribute, index: usize) -> u32 {
    values.indices.get(index).copied().unwrap_or(0)
}

fn select_string_index(values: &StringTableAttribute, indices: [usize; 3], weights: [f32; 3]) -> u32 {
    let mut max_i = 0;
    if weights[1] > weights[max_i] {
        max_i = 1;
    }
    if weights[2] > weights[max_i] {
        max_i = 2;
    }
    let idx = indices[max_i];
    select_string_single(values, idx)
}

fn sample_numeric_line(attr: AttributeRef<'_>, a: usize, b: usize, t: f32) -> f32 {
    let va = sample_numeric_single(attr, a);
    let vb = sample_numeric_single(attr, b);
    va * (1.0 - t) + vb * t
}

fn sample_int_line(attr: AttributeRef<'_>, a: usize, b: usize, t: f32) -> i32 {
    sample_numeric_line(attr, a, b, t).round() as i32
}

struct CurveSegment {
    a: usize,
    b: usize,
    p0: Vec3,
    p1: Vec3,
}

struct XorShift32 {
    state: u32,
}

impl XorShift32 {
    fn new(seed: u32) -> Self {
        let seed = if seed == 0 { 0x12345678 } else { seed };
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    fn next_f32(&mut self) -> f32 {
        let value = self.next_u32();
        value as f32 / u32::MAX as f32
    }
}

