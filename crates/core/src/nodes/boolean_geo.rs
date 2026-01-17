use std::collections::{BTreeMap, BTreeSet, HashMap};

use boolmesh::prelude::{compute_boolean, Manifold, OpType};
use glam::Vec3;

use crate::attributes::{AttributeDomain, AttributeStorage, AttributeType, MeshAttributes, StringTableAttribute};
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::{Mesh, MeshGroups};
use crate::nodes::{geometry_in, geometry_out, recompute_mesh_normals, require_mesh_input};
use crate::volume::{Volume, VolumeKind};
use crate::volume_sampling::VolumeSampler;

pub const NAME: &str = "Boolean Geo";
const DEFAULT_MODE: &str = "auto";
const DEFAULT_OP: i32 = 0;

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![geometry_in("geo"), geometry_in("cutter")],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            ("mode".to_string(), ParamValue::String(DEFAULT_MODE.to_string())),
            ("op".to_string(), ParamValue::Int(DEFAULT_OP)),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mesh_a = require_mesh_input(inputs, 0, "Boolean Geo requires mesh input A")?;
    let mesh_b = require_mesh_input(inputs, 1, "Boolean Geo requires mesh input B")?;
    boolean_mesh_mesh(params, &mesh_a, &mesh_b)
}

pub fn apply_to_geometry(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let Some(input_a) = inputs.first() else {
        return Ok(Geometry::default());
    };
    let Some(input_b) = inputs.get(1) else {
        return Err("Boolean Geo requires two inputs".to_string());
    };
    let Some(mesh_a) = input_a.merged_mesh() else {
        return Err("Boolean Geo requires a mesh on input A".to_string());
    };

    let op = params.get_int("op", DEFAULT_OP);
    let mode = params.get_string("mode", DEFAULT_MODE).to_lowercase();
    let force_mesh_sdf = mode.contains("sdf");
    let force_mesh_mesh = mode.contains("mesh_mesh");
    let auto = mode.contains("auto") || (!force_mesh_sdf && !force_mesh_mesh);

    let mut output = input_a.clone();
    output.materials.merge(&input_b.materials);

    let mesh = if force_mesh_sdf || (auto && has_sdf_volume(input_b)) {
        let volume = find_sdf_volume(input_b)
            .ok_or_else(|| "Boolean Geo requires an SDF volume on input B".to_string())?;
        let mut mesh = clip_mesh_with_sdf(&mesh_a, volume, op)?;
        let source = SourceMesh::new(&mesh_a)?;
        transfer_attributes_from_sources(&mut mesh, &[source]);
        mesh
    } else {
        let Some(mesh_b) = input_b.merged_mesh() else {
            return Err("Boolean Geo requires a mesh on input B".to_string());
        };
        boolean_mesh_mesh(params, &mesh_a, &mesh_b)?
    };

    output.meshes = if mesh.positions.is_empty() && mesh.indices.is_empty() {
        Vec::new()
    } else {
        vec![mesh]
    };
    Ok(output)
}

fn boolean_mesh_mesh(params: &NodeParams, mesh_a: &Mesh, mesh_b: &Mesh) -> Result<Mesh, String> {
    ensure_triangle_mesh(mesh_a, "Boolean Geo requires triangle mesh input A")?;
    ensure_triangle_mesh(mesh_b, "Boolean Geo requires triangle mesh input B")?;

    let op = match params.get_int("op", DEFAULT_OP) {
        1 => OpType::Subtract,
        2 => OpType::Intersect,
        _ => OpType::Add,
    };

    let pos_a = flatten_positions(mesh_a);
    let pos_b = flatten_positions(mesh_b);
    let idx_a = mesh_a.indices.iter().map(|i| *i as usize).collect::<Vec<_>>();
    let idx_b = mesh_b.indices.iter().map(|i| *i as usize).collect::<Vec<_>>();

    let manifold_a = Manifold::new(&pos_a, &idx_a)?;
    let manifold_b = Manifold::new(&pos_b, &idx_b)?;
    let manifold = compute_boolean(&manifold_a, &manifold_b, op)?;

    let mut positions = Vec::with_capacity(manifold.ps.len());
    for p in &manifold.ps {
        positions.push(p.to_array());
    }
    let indices = manifold
        .get_indices()
        .into_iter()
        .map(|idx| idx as u32)
        .collect::<Vec<_>>();
    let mut mesh = Mesh::with_positions_indices(positions, indices);

    let sources = [SourceMesh::new(mesh_a)?, SourceMesh::new(mesh_b)?];
    transfer_attributes_from_sources(&mut mesh, &sources);
    Ok(mesh)
}

fn ensure_triangle_mesh(mesh: &Mesh, message: &str) -> Result<(), String> {
    if mesh.positions.is_empty()
        || mesh.indices.len() < 3
        || !mesh.indices.len().is_multiple_of(3)
    {
        return Err(message.to_string());
    }
    Ok(())
}

fn flatten_positions(mesh: &Mesh) -> Vec<f64> {
    let mut flat = Vec::with_capacity(mesh.positions.len() * 3);
    for p in &mesh.positions {
        flat.push(p[0] as f64);
        flat.push(p[1] as f64);
        flat.push(p[2] as f64);
    }
    flat
}

fn has_sdf_volume(geom: &Geometry) -> bool {
    geom.volumes.iter().any(|v| v.kind == VolumeKind::Sdf)
}

fn find_sdf_volume(geom: &Geometry) -> Option<&Volume> {
    geom.volumes.iter().find(|v| v.kind == VolumeKind::Sdf)
}

fn clip_mesh_with_sdf(mesh: &Mesh, volume: &Volume, op: i32) -> Result<Mesh, String> {
    if volume.kind != VolumeKind::Sdf {
        return Err("Boolean Geo requires an SDF volume on input B".to_string());
    }
    if op == 0 {
        return Ok(mesh.clone());
    }
    if mesh.indices.is_empty() || mesh.positions.is_empty() {
        return Ok(Mesh::default());
    }

    let keep_inside = matches!(op, 2);
    let sampler = VolumeSampler::new(volume);
    let mut out_positions = Vec::new();
    let mut out_indices = Vec::new();
    let target_len = volume.voxel_size.max(1.0e-4);
    let max_depth = 6;

    for tri in mesh.indices.chunks_exact(3) {
        let p0 = Vec3::from(mesh.positions[tri[0] as usize]);
        let p1 = Vec3::from(mesh.positions[tri[1] as usize]);
        let p2 = Vec3::from(mesh.positions[tri[2] as usize]);

        let mut stack = Vec::new();
        stack.push(SdfTri::new(&sampler, p0, p1, p2, 0));

        while let Some(tri) = stack.pop() {
            let max_len = tri.max_edge_len();
            if tri.depth < max_depth && max_len > target_len {
                let (a, b, c, d) = tri.subdivide(&sampler);
                stack.push(a);
                stack.push(b);
                stack.push(c);
                stack.push(d);
                continue;
            }

            let poly = clip_polygon(
                &[
                    ClipVertex {
                        pos: tri.p0,
                        dist: tri.d0,
                    },
                    ClipVertex {
                        pos: tri.p1,
                        dist: tri.d1,
                    },
                    ClipVertex {
                        pos: tri.p2,
                        dist: tri.d2,
                    },
                ],
                keep_inside,
            );
            if poly.len() < 3 {
                continue;
            }

            let base = out_positions.len() as u32;
            for vertex in &poly {
                out_positions.push(vertex.pos.to_array());
            }
            if poly.len() == 3 {
                out_indices.extend_from_slice(&[base, base + 1, base + 2]);
            } else if poly.len() == 4 {
                out_indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            } else {
                for i in 1..(poly.len() - 1) {
                    out_indices.extend_from_slice(&[base, base + i as u32, base + i as u32 + 1]);
                }
            }
        }
    }

    Ok(Mesh::with_positions_indices(out_positions, out_indices))
}

#[derive(Clone, Copy)]
struct ClipVertex {
    pos: Vec3,
    dist: f32,
}

fn clip_polygon(input: &[ClipVertex], keep_inside: bool) -> Vec<ClipVertex> {
    if input.is_empty() {
        return Vec::new();
    }
    let mut output = Vec::new();
    let len = input.len();
    for i in 0..len {
        let curr = input[i];
        let next = input[(i + 1) % len];
        let curr_inside = if keep_inside { curr.dist <= 0.0 } else { curr.dist >= 0.0 };
        let next_inside = if keep_inside { next.dist <= 0.0 } else { next.dist >= 0.0 };
        if curr_inside && next_inside {
            output.push(next);
        } else if curr_inside && !next_inside {
            if let Some(intersection) = clip_intersection(curr, next) {
                output.push(intersection);
            }
        } else if !curr_inside && next_inside {
            if let Some(intersection) = clip_intersection(curr, next) {
                output.push(intersection);
            }
            output.push(next);
        }
    }
    output
}

fn clip_intersection(a: ClipVertex, b: ClipVertex) -> Option<ClipVertex> {
    let denom = a.dist - b.dist;
    if denom.abs() <= 1.0e-8 {
        return None;
    }
    let t = a.dist / denom;
    let pos = a.pos + (b.pos - a.pos) * t;
    Some(ClipVertex { pos, dist: 0.0 })
}

#[derive(Clone, Copy)]
struct SdfTri {
    p0: Vec3,
    p1: Vec3,
    p2: Vec3,
    d0: f32,
    d1: f32,
    d2: f32,
    depth: u8,
}

impl SdfTri {
    fn new(sampler: &VolumeSampler<'_>, p0: Vec3, p1: Vec3, p2: Vec3, depth: u8) -> Self {
        let mut d0 = sampler.sample_world(p0);
        let mut d1 = sampler.sample_world(p1);
        let mut d2 = sampler.sample_world(p2);
        if !d0.is_finite() {
            d0 = 1.0e6;
        }
        if !d1.is_finite() {
            d1 = 1.0e6;
        }
        if !d2.is_finite() {
            d2 = 1.0e6;
        }
        Self {
            p0,
            p1,
            p2,
            d0,
            d1,
            d2,
            depth,
        }
    }

    fn max_edge_len(&self) -> f32 {
        let l01 = (self.p1 - self.p0).length();
        let l12 = (self.p2 - self.p1).length();
        let l20 = (self.p0 - self.p2).length();
        l01.max(l12).max(l20)
    }

    fn subdivide(&self, sampler: &VolumeSampler<'_>) -> (Self, Self, Self, Self) {
        let m01 = (self.p0 + self.p1) * 0.5;
        let m12 = (self.p1 + self.p2) * 0.5;
        let m20 = (self.p2 + self.p0) * 0.5;
        let depth = self.depth + 1;
        let t0 = Self::new(sampler, self.p0, m01, m20, depth);
        let t1 = Self::new(sampler, m01, self.p1, m12, depth);
        let t2 = Self::new(sampler, m20, m12, self.p2, depth);
        let t3 = Self::new(sampler, m01, m12, m20, depth);
        (t0, t1, t2, t3)
    }
}

#[derive(Clone)]
struct SourceMesh<'a> {
    mesh: &'a Mesh,
    positions: Vec<Vec3>,
    triangles: Vec<[u32; 3]>,
    tri_bounds: Vec<[Vec3; 2]>,
    point_uvs: Option<Vec<[f32; 2]>>,
    vertex_uvs: Option<Vec<[f32; 2]>>,
}

impl<'a> SourceMesh<'a> {
    fn new(mesh: &'a Mesh) -> Result<Self, String> {
        ensure_triangle_mesh(mesh, "Boolean Geo requires triangle mesh inputs")?;
        let positions = mesh.positions.iter().copied().map(Vec3::from).collect::<Vec<_>>();
        let mut triangles = Vec::new();
        let mut tri_bounds = Vec::new();
        for tri in mesh.indices.chunks_exact(3) {
            let tri_idx = [tri[0], tri[1], tri[2]];
            let a = positions[tri_idx[0] as usize];
            let b = positions[tri_idx[1] as usize];
            let c = positions[tri_idx[2] as usize];
            let min = a.min(b).min(c);
            let max = a.max(b).max(c);
            triangles.push(tri_idx);
            tri_bounds.push([min, max]);
        }
        let point_uvs = mesh.uvs.as_ref().and_then(|uvs| {
            if uvs.len() == mesh.positions.len() {
                Some(uvs.clone())
            } else {
                None
            }
        });
        let vertex_uvs = mesh
            .attributes
            .get(AttributeDomain::Vertex, "uv")
            .and_then(|storage| match storage {
                AttributeStorage::Vec2(values) if values.len() == mesh.indices.len() => {
                    Some(values.clone())
                }
                _ => None,
            });
        Ok(Self {
            mesh,
            positions,
            triangles,
            tri_bounds,
            point_uvs,
            vertex_uvs,
        })
    }
}

#[derive(Clone, Copy)]
struct SampleRef {
    source: usize,
    tri_index: usize,
    barycentric: [f32; 3],
}

fn transfer_attributes_from_sources(output: &mut Mesh, sources: &[SourceMesh<'_>]) {
    if output.positions.is_empty() || output.indices.len() < 3 {
        return;
    }

    let point_samples = build_point_samples(output, sources);
    let corner_samples = build_corner_samples(output, sources);
    let prim_samples = build_prim_samples(output, sources);

    let mut attributes = MeshAttributes::default();
    let mut groups = MeshGroups::default();

    transfer_domain_attributes(
        output.positions.len(),
        AttributeDomain::Point,
        sources,
        &point_samples,
        &mut attributes,
    );
    transfer_domain_attributes(
        output.indices.len(),
        AttributeDomain::Vertex,
        sources,
        &corner_samples,
        &mut attributes,
    );
    transfer_domain_attributes(
        output.indices.len() / 3,
        AttributeDomain::Primitive,
        sources,
        &prim_samples,
        &mut attributes,
    );
    transfer_detail_attributes(sources, &mut attributes);

    transfer_groups(
        output.positions.len(),
        AttributeDomain::Point,
        sources,
        &point_samples,
        &mut groups,
    );
    transfer_groups(
        output.indices.len(),
        AttributeDomain::Vertex,
        sources,
        &corner_samples,
        &mut groups,
    );
    transfer_groups(
        output.indices.len() / 3,
        AttributeDomain::Primitive,
        sources,
        &prim_samples,
        &mut groups,
    );

    output.attributes = attributes;
    output.groups = groups;

    if let Some(AttributeStorage::Vec2(values)) = output.attributes.get(AttributeDomain::Point, "uv") {
        if values.len() == output.positions.len() {
            output.uvs = Some(values.clone());
        }
    }

    recompute_mesh_normals(output);
}

fn build_point_samples(output: &Mesh, sources: &[SourceMesh<'_>]) -> Vec<Option<SampleRef>> {
    output
        .positions
        .iter()
        .map(|pos| nearest_triangle(Vec3::from(*pos), sources))
        .collect()
}

fn build_corner_samples(output: &Mesh, sources: &[SourceMesh<'_>]) -> Vec<Option<SampleRef>> {
    output
        .indices
        .iter()
        .map(|idx| {
            let pos = output.positions.get(*idx as usize).copied().unwrap_or([0.0, 0.0, 0.0]);
            nearest_triangle(Vec3::from(pos), sources)
        })
        .collect()
}

fn build_prim_samples(output: &Mesh, sources: &[SourceMesh<'_>]) -> Vec<Option<SampleRef>> {
    output
        .indices
        .chunks_exact(3)
        .map(|tri| {
            let a = Vec3::from(output.positions[tri[0] as usize]);
            let b = Vec3::from(output.positions[tri[1] as usize]);
            let c = Vec3::from(output.positions[tri[2] as usize]);
            nearest_triangle((a + b + c) / 3.0, sources)
        })
        .collect()
}

fn nearest_triangle(point: Vec3, sources: &[SourceMesh<'_>]) -> Option<SampleRef> {
    let mut best: Option<(SampleRef, f32)> = None;
    for (source_idx, source) in sources.iter().enumerate() {
        for (tri_idx, tri) in source.triangles.iter().enumerate() {
            let bounds = source.tri_bounds[tri_idx];
            let bound_dist = distance2_point_aabb(point, bounds[0], bounds[1]);
            if let Some((_, best_dist)) = best {
                if bound_dist >= best_dist {
                    continue;
                }
            }
            let a = source.positions[tri[0] as usize];
            let b = source.positions[tri[1] as usize];
            let c = source.positions[tri[2] as usize];
            let (closest, barycentric) = closest_point_on_triangle(point, a, b, c);
            let dist = (point - closest).length_squared();
            if best.map(|(_, best_dist)| dist < best_dist).unwrap_or(true) {
                best = Some((
                    SampleRef {
                        source: source_idx,
                        tri_index: tri_idx,
                        barycentric,
                    },
                    dist,
                ));
            }
        }
    }
    best.map(|(sample, _)| sample)
}

fn distance2_point_aabb(p: Vec3, min: Vec3, max: Vec3) -> f32 {
    let clamped = Vec3::new(
        p.x.clamp(min.x, max.x),
        p.y.clamp(min.y, max.y),
        p.z.clamp(min.z, max.z),
    );
    (p - clamped).length_squared()
}

fn transfer_domain_attributes(
    len: usize,
    domain: AttributeDomain,
    sources: &[SourceMesh<'_>],
    samples: &[Option<SampleRef>],
    attributes: &mut MeshAttributes,
) {
    let schema = collect_attribute_schema(sources, domain);
    for (name, data_type) in schema {
        match data_type {
            AttributeType::Float => {
                let mut values = vec![0.0f32; len];
                for (idx, sample) in samples.iter().enumerate().take(len) {
                    if let Some(sample) = sample {
                        if let Some(value) = sample_float(sources, *sample, domain, &name) {
                            values[idx] = value;
                        }
                    }
                }
                attributes
                    .map_mut(domain)
                    .insert(name, AttributeStorage::Float(values));
            }
            AttributeType::Int => {
                let mut values = vec![0i32; len];
                for (idx, sample) in samples.iter().enumerate().take(len) {
                    if let Some(sample) = sample {
                        if let Some(value) = sample_int(sources, *sample, domain, &name) {
                            values[idx] = value;
                        }
                    }
                }
                attributes
                    .map_mut(domain)
                    .insert(name, AttributeStorage::Int(values));
            }
            AttributeType::Vec2 => {
                let mut values = vec![[0.0f32, 0.0f32]; len];
                for (idx, sample) in samples.iter().enumerate().take(len) {
                    if let Some(sample) = sample {
                        if let Some(value) = sample_vec2(sources, *sample, domain, &name) {
                            values[idx] = value;
                        }
                    }
                }
                attributes
                    .map_mut(domain)
                    .insert(name, AttributeStorage::Vec2(values));
            }
            AttributeType::Vec3 => {
                let mut values = vec![[0.0f32, 0.0f32, 0.0f32]; len];
                for (idx, sample) in samples.iter().enumerate().take(len) {
                    if let Some(sample) = sample {
                        if let Some(value) = sample_vec3(sources, *sample, domain, &name) {
                            values[idx] = value;
                        }
                    }
                }
                attributes
                    .map_mut(domain)
                    .insert(name, AttributeStorage::Vec3(values));
            }
            AttributeType::Vec4 => {
                let mut values = vec![[0.0f32, 0.0f32, 0.0f32, 0.0f32]; len];
                for (idx, sample) in samples.iter().enumerate().take(len) {
                    if let Some(sample) = sample {
                        if let Some(value) = sample_vec4(sources, *sample, domain, &name) {
                            values[idx] = value;
                        }
                    }
                }
                attributes
                    .map_mut(domain)
                    .insert(name, AttributeStorage::Vec4(values));
            }
            AttributeType::String => {
                let mut builder = StringTableBuilder::default();
                for sample in samples.iter().take(len) {
                    let value = sample
                        .and_then(|sample| sample_string(sources, sample, domain, &name))
                        .unwrap_or_default();
                    builder.push(&value);
                }
                if !builder.indices.is_empty() {
                    attributes.map_mut(domain).insert(
                        name,
                        AttributeStorage::StringTable(StringTableAttribute::new(
                            builder.values,
                            builder.indices,
                        )),
                    );
                }
            }
        }
    }
}

fn transfer_detail_attributes(sources: &[SourceMesh<'_>], attributes: &mut MeshAttributes) {
    let Some(source) = sources.first() else {
        return;
    };
    for (name, storage) in source.mesh.attributes.map(AttributeDomain::Detail) {
        if storage.len() == 1 {
            attributes
                .map_mut(AttributeDomain::Detail)
                .insert(name.clone(), storage.clone());
        }
    }
}

fn collect_attribute_schema(
    sources: &[SourceMesh<'_>],
    domain: AttributeDomain,
) -> Vec<(String, AttributeType)> {
    let mut types: BTreeMap<String, AttributeType> = BTreeMap::new();
    let mut conflicts: BTreeSet<String> = BTreeSet::new();

    for source in sources {
        if domain == AttributeDomain::Point && source.point_uvs.is_some() {
            register_attr("uv", AttributeType::Vec2, &mut types, &mut conflicts);
        }
        if domain == AttributeDomain::Vertex && source.vertex_uvs.is_some() {
            register_attr("uv", AttributeType::Vec2, &mut types, &mut conflicts);
        }
        for (name, storage) in source.mesh.attributes.map(domain) {
            if name == "P" || name == "N" {
                continue;
            }
            register_attr(name, storage.data_type(), &mut types, &mut conflicts);
        }
    }

    for name in conflicts {
        types.remove(&name);
    }
    types.into_iter().collect()
}

fn register_attr(
    name: &str,
    data_type: AttributeType,
    types: &mut BTreeMap<String, AttributeType>,
    conflicts: &mut BTreeSet<String>,
) {
    if conflicts.contains(name) {
        return;
    }
    match types.get(name) {
        Some(existing) if *existing != data_type => {
            conflicts.insert(name.to_string());
        }
        Some(_) => {}
        None => {
            types.insert(name.to_string(), data_type);
        }
    }
}

#[derive(Default)]
struct StringTableBuilder {
    values: Vec<String>,
    indices: Vec<u32>,
    lookup: HashMap<String, u32>,
}

impl StringTableBuilder {
    fn push(&mut self, value: &str) {
        let index = if let Some(&idx) = self.lookup.get(value) {
            idx
        } else {
            let idx = self.values.len() as u32;
            self.values.push(value.to_string());
            self.lookup.insert(value.to_string(), idx);
            idx
        };
        self.indices.push(index);
    }
}

fn sample_float(
    sources: &[SourceMesh<'_>],
    sample: SampleRef,
    domain: AttributeDomain,
    name: &str,
) -> Option<f32> {
    let source = sources.get(sample.source)?;
    let attr = source.mesh.attributes.get(domain, name)?;
    match attr {
        AttributeStorage::Float(values) => match domain {
            AttributeDomain::Point | AttributeDomain::Vertex => {
                let indices = mesh_attribute_indices(source, domain, sample.tri_index)?;
                lerp_f32(values, indices, sample.barycentric)
            }
            AttributeDomain::Primitive => values.get(sample.tri_index).copied(),
            AttributeDomain::Detail => values.first().copied(),
        },
        _ => None,
    }
}

fn sample_int(
    sources: &[SourceMesh<'_>],
    sample: SampleRef,
    domain: AttributeDomain,
    name: &str,
) -> Option<i32> {
    let source = sources.get(sample.source)?;
    let attr = source.mesh.attributes.get(domain, name)?;
    match attr {
        AttributeStorage::Int(values) => match domain {
            AttributeDomain::Point | AttributeDomain::Vertex => {
                let indices = mesh_attribute_indices(source, domain, sample.tri_index)?;
                let idx = barycentric_max_index(sample.barycentric);
                let corner = indices.get(idx)?;
                values.get(*corner).copied()
            }
            AttributeDomain::Primitive => values.get(sample.tri_index).copied(),
            AttributeDomain::Detail => values.first().copied(),
        },
        _ => None,
    }
}

fn sample_vec2(
    sources: &[SourceMesh<'_>],
    sample: SampleRef,
    domain: AttributeDomain,
    name: &str,
) -> Option<[f32; 2]> {
    let source = sources.get(sample.source)?;
    if name == "uv" {
        if domain == AttributeDomain::Point {
            if let Some(values) = &source.point_uvs {
                let indices = mesh_attribute_indices(source, domain, sample.tri_index)?;
                return lerp_vec2(values, indices, sample.barycentric);
            }
        }
        if domain == AttributeDomain::Vertex {
            if let Some(values) = &source.vertex_uvs {
                let indices = mesh_attribute_indices(source, domain, sample.tri_index)?;
                return lerp_vec2(values, indices, sample.barycentric);
            }
        }
    }
    let attr = source.mesh.attributes.get(domain, name)?;
    match attr {
        AttributeStorage::Vec2(values) => match domain {
            AttributeDomain::Point | AttributeDomain::Vertex => {
                let indices = mesh_attribute_indices(source, domain, sample.tri_index)?;
                lerp_vec2(values, indices, sample.barycentric)
            }
            AttributeDomain::Primitive => values.get(sample.tri_index).copied(),
            AttributeDomain::Detail => values.first().copied(),
        },
        _ => None,
    }
}

fn sample_vec3(
    sources: &[SourceMesh<'_>],
    sample: SampleRef,
    domain: AttributeDomain,
    name: &str,
) -> Option<[f32; 3]> {
    let source = sources.get(sample.source)?;
    let attr = source.mesh.attributes.get(domain, name)?;
    match attr {
        AttributeStorage::Vec3(values) => match domain {
            AttributeDomain::Point | AttributeDomain::Vertex => {
                let indices = mesh_attribute_indices(source, domain, sample.tri_index)?;
                lerp_vec3(values, indices, sample.barycentric)
            }
            AttributeDomain::Primitive => values.get(sample.tri_index).copied(),
            AttributeDomain::Detail => values.first().copied(),
        },
        _ => None,
    }
}

fn sample_vec4(
    sources: &[SourceMesh<'_>],
    sample: SampleRef,
    domain: AttributeDomain,
    name: &str,
) -> Option<[f32; 4]> {
    let source = sources.get(sample.source)?;
    let attr = source.mesh.attributes.get(domain, name)?;
    match attr {
        AttributeStorage::Vec4(values) => match domain {
            AttributeDomain::Point | AttributeDomain::Vertex => {
                let indices = mesh_attribute_indices(source, domain, sample.tri_index)?;
                lerp_vec4(values, indices, sample.barycentric)
            }
            AttributeDomain::Primitive => values.get(sample.tri_index).copied(),
            AttributeDomain::Detail => values.first().copied(),
        },
        _ => None,
    }
}

fn sample_string(
    sources: &[SourceMesh<'_>],
    sample: SampleRef,
    domain: AttributeDomain,
    name: &str,
) -> Option<String> {
    let source = sources.get(sample.source)?;
    let attr = source.mesh.attributes.get(domain, name)?;
    match attr {
        AttributeStorage::StringTable(values) => match domain {
            AttributeDomain::Point | AttributeDomain::Vertex => {
                let indices = mesh_attribute_indices(source, domain, sample.tri_index)?;
                let idx = barycentric_max_index(sample.barycentric);
                let corner = *indices.get(idx)?;
                values.value(corner).map(|v| v.to_string())
            }
            AttributeDomain::Primitive => values.value(sample.tri_index).map(|v| v.to_string()),
            AttributeDomain::Detail => values.value(0).map(|v| v.to_string()),
        },
        _ => None,
    }
}

fn transfer_groups(
    len: usize,
    domain: AttributeDomain,
    sources: &[SourceMesh<'_>],
    samples: &[Option<SampleRef>],
    groups: &mut MeshGroups,
) {
    let mut names = BTreeSet::new();
    for source in sources {
        names.extend(source.mesh.groups.map(domain).keys().cloned());
    }
    for name in names {
        let mut values = vec![false; len];
        for (idx, sample) in samples.iter().enumerate().take(len) {
            if let Some(sample) = sample {
                if let Some(value) = sample_group(sources, *sample, domain, &name) {
                    values[idx] = value;
                }
            }
        }
        groups.map_mut(domain).insert(name, values);
    }
}

fn sample_group(
    sources: &[SourceMesh<'_>],
    sample: SampleRef,
    domain: AttributeDomain,
    name: &str,
) -> Option<bool> {
    let source = sources.get(sample.source)?;
    let values = source.mesh.groups.map(domain).get(name)?;
    match domain {
        AttributeDomain::Point => {
            let indices = mesh_attribute_indices(source, domain, sample.tri_index)?;
            let idx = barycentric_max_index(sample.barycentric);
            let point = *indices.get(idx)?;
            values.get(point).copied()
        }
        AttributeDomain::Vertex => {
            let indices = mesh_attribute_indices(source, domain, sample.tri_index)?;
            let idx = barycentric_max_index(sample.barycentric);
            let corner = *indices.get(idx)?;
            values.get(corner).copied()
        }
        AttributeDomain::Primitive => values.get(sample.tri_index).copied(),
        AttributeDomain::Detail => None,
    }
}

fn mesh_attribute_indices(
    source: &SourceMesh<'_>,
    domain: AttributeDomain,
    tri_index: usize,
) -> Option<[usize; 3]> {
    match domain {
        AttributeDomain::Point => {
            let tri = source.triangles.get(tri_index)?;
            Some([
                tri[0] as usize,
                tri[1] as usize,
                tri[2] as usize,
            ])
        }
        AttributeDomain::Vertex => {
            let base = tri_index * 3;
            Some([base, base + 1, base + 2])
        }
        _ => None,
    }
}

fn barycentric_max_index(barycentric: [f32; 3]) -> usize {
    let mut idx = 0;
    let mut best = barycentric[0];
    if barycentric[1] > best {
        best = barycentric[1];
        idx = 1;
    }
    if barycentric[2] > best {
        idx = 2;
    }
    idx
}

fn lerp_f32(values: &[f32], indices: [usize; 3], barycentric: [f32; 3]) -> Option<f32> {
    let a = *values.get(indices[0])?;
    let b = *values.get(indices[1])?;
    let c = *values.get(indices[2])?;
    Some(a * barycentric[0] + b * barycentric[1] + c * barycentric[2])
}

fn lerp_vec2(
    values: &[[f32; 2]],
    indices: [usize; 3],
    barycentric: [f32; 3],
) -> Option<[f32; 2]> {
    let a = values.get(indices[0])?;
    let b = values.get(indices[1])?;
    let c = values.get(indices[2])?;
    Some([
        a[0] * barycentric[0] + b[0] * barycentric[1] + c[0] * barycentric[2],
        a[1] * barycentric[0] + b[1] * barycentric[1] + c[1] * barycentric[2],
    ])
}

fn lerp_vec3(
    values: &[[f32; 3]],
    indices: [usize; 3],
    barycentric: [f32; 3],
) -> Option<[f32; 3]> {
    let a = values.get(indices[0])?;
    let b = values.get(indices[1])?;
    let c = values.get(indices[2])?;
    Some([
        a[0] * barycentric[0] + b[0] * barycentric[1] + c[0] * barycentric[2],
        a[1] * barycentric[0] + b[1] * barycentric[1] + c[1] * barycentric[2],
        a[2] * barycentric[0] + b[2] * barycentric[1] + c[2] * barycentric[2],
    ])
}

fn lerp_vec4(
    values: &[[f32; 4]],
    indices: [usize; 3],
    barycentric: [f32; 3],
) -> Option<[f32; 4]> {
    let a = values.get(indices[0])?;
    let b = values.get(indices[1])?;
    let c = values.get(indices[2])?;
    Some([
        a[0] * barycentric[0] + b[0] * barycentric[1] + c[0] * barycentric[2],
        a[1] * barycentric[0] + b[1] * barycentric[1] + c[1] * barycentric[2],
        a[2] * barycentric[0] + b[2] * barycentric[1] + c[2] * barycentric[2],
        a[3] * barycentric[0] + b[3] * barycentric[1] + c[3] * barycentric[2],
    ])
}

fn closest_point_on_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> (Vec3, [f32; 3]) {
    let ab = b - a;
    let ac = c - a;
    let area = ab.cross(ac).length_squared();
    if area <= 1.0e-12 {
        let mut best = a;
        let mut bary = [1.0, 0.0, 0.0];
        let mut best_dist = (p - a).length_squared();
        let dist_b = (p - b).length_squared();
        if dist_b < best_dist {
            best = b;
            bary = [0.0, 1.0, 0.0];
            best_dist = dist_b;
        }
        let dist_c = (p - c).length_squared();
        if dist_c < best_dist {
            best = c;
            bary = [0.0, 0.0, 1.0];
        }
        return (best, bary);
    }
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return (a, [1.0, 0.0, 0.0]);
    }

    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return (b, [0.0, 1.0, 0.0]);
    }

    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return (a + ab * v, [1.0 - v, v, 0.0]);
    }

    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return (c, [0.0, 0.0, 1.0]);
    }

    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return (a + ac * w, [1.0 - w, 0.0, w]);
    }

    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        let point = b + (c - b) * w;
        return (point, [0.0, 1.0 - w, w]);
    }

    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    let u = 1.0 - v - w;
    let point = a + ab * v + ac * w;
    (point, [u, v, w])
}
