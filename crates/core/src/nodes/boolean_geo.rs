use std::collections::{BTreeMap, BTreeSet, HashMap};

use boolmesh::prelude::{compute_boolean, Manifold, OpType};
use glam::Vec3;

use crate::attributes::{AttributeDomain, AttributeStorage, AttributeType, MeshAttributes, StringTableAttribute};
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::{Mesh, MeshGroups};
use crate::nodes::{geometry_in, geometry_out, recompute_mesh_normals, require_mesh_input};
use crate::nodes::volume_to_mesh::volume_to_mesh;
use crate::volume::{Volume, VolumeKind};
use crate::volume_sampling::VolumeSampler;

pub const NAME: &str = "Boolean Geo";
const DEFAULT_MODE: &str = "auto";
const DEFAULT_OP: i32 = 1;

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
        let mesh = if op == 1 || op == 2 {
            let mut combined = mesh;
            let base_prims = combined.face_count();
            if let Some(inner) = cutter_inner_surface(volume, &mesh_a, op) {
                let inner_prims = inner.face_count();
                append_mesh_with_defaults(&mut combined, &inner);
                let total = combined.face_count();
                if total == base_prims + inner_prims && total > 0 {
                    let mut outside = vec![false; total];
                    let mut inside = vec![false; total];
                    for value in &mut outside[..base_prims] {
                        *value = true;
                    }
                    for value in &mut inside[base_prims..] {
                        *value = true;
                    }
                    combined
                        .groups
                        .map_mut(AttributeDomain::Primitive)
                        .insert("outside".to_string(), outside);
                    combined
                        .groups
                        .map_mut(AttributeDomain::Primitive)
                        .insert("inside".to_string(), inside);
                }
            } else if base_prims > 0 {
                combined
                    .groups
                    .map_mut(AttributeDomain::Primitive)
                    .insert("outside".to_string(), vec![true; base_prims]);
            }
            recompute_mesh_normals(&mut combined);
            combined
        } else {
            mesh
        };
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

fn cutter_inner_surface(volume: &Volume, mesh_a: &Mesh, op: i32) -> Option<Mesh> {
    if volume.kind != VolumeKind::Sdf {
        return None;
    }
    let mut cutter = volume_to_mesh(volume, 0.0, false).ok()?;
    if cutter.indices.is_empty() || cutter.positions.is_empty() {
        return None;
    }
    if mesh_a.indices.is_empty() || mesh_a.positions.is_empty() {
        return None;
    }

    let triangles_a = build_triangle_list(mesh_a);
    if triangles_a.is_empty() {
        return None;
    }
    let mut kept_indices = Vec::new();
    for tri in cutter.indices.chunks_exact(3) {
        let a = Vec3::from(cutter.positions[tri[0] as usize]);
        let b = Vec3::from(cutter.positions[tri[1] as usize]);
        let c = Vec3::from(cutter.positions[tri[2] as usize]);
        let centroid = (a + b + c) / 3.0;
        if is_inside_mesh(centroid, &triangles_a) {
            if op == 1 {
                kept_indices.extend_from_slice(&[tri[0], tri[2], tri[1]]);
            } else {
                kept_indices.extend_from_slice(tri);
            }
        }
    }
    cutter.indices = kept_indices;
    if cutter.indices.is_empty() {
        return None;
    }
    Some(cutter)
}

fn boolean_mesh_mesh(params: &NodeParams, mesh_a: &Mesh, mesh_b: &Mesh) -> Result<Mesh, String> {
    let op = match params.get_int("op", DEFAULT_OP) {
        1 => OpType::Subtract,
        2 => OpType::Intersect,
        _ => OpType::Add,
    };

    let manifold_a = manifold_from_mesh(mesh_a)
        .map_err(|err| format!("Boolean Geo input A: {err}"))?;
    let manifold_b = manifold_from_mesh(mesh_b)
        .map_err(|err| format!("Boolean Geo input B: {err}"))?;
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
    let voxel = volume.voxel_size.max(1.0e-4);
    let mut out_positions = Vec::new();
    let mut out_indices = Vec::new();
    let mut out_face_counts = Vec::new();
    let mut point_map: HashMap<usize, u32> = HashMap::new();
    let max_edge_samples = 8usize;

    let face_counts = if mesh.face_counts.is_empty() {
        if mesh.indices.len().is_multiple_of(3) {
            vec![3u32; mesh.indices.len() / 3]
        } else {
            vec![mesh.indices.len() as u32]
        }
    } else {
        mesh.face_counts.clone()
    };

    let mut cursor = 0usize;
    for &count in &face_counts {
        let count = count as usize;
        if count < 3 || cursor + count > mesh.indices.len() {
            cursor = cursor.saturating_add(count);
            continue;
        }

        let mut face_positions = Vec::with_capacity(count);
        let mut face_dists = Vec::with_capacity(count);
        for i in 0..count {
            let idx = mesh.indices[cursor + i] as usize;
            let pos = Vec3::from(*mesh.positions.get(idx).unwrap_or(&[0.0, 0.0, 0.0]));
            let mut dist = sampler.sample_world(pos);
            if !dist.is_finite() {
                dist = 1.0e6;
            }
            face_positions.push(pos);
            face_dists.push(dist);
        }

        let mut any_keep = false;
        let mut any_drop = false;
        for &dist in &face_dists {
            let keep = if keep_inside { dist <= 0.0 } else { dist >= 0.0 };
            if keep {
                any_keep = true;
            } else {
                any_drop = true;
            }
        }

        let all_keep = !any_drop;
        let all_drop = !any_keep;
        if all_keep || all_drop {
            let boundary_keep = all_keep;
            let v0 = face_positions[0];
            let mut interior_cut = false;
            for i in 1..(count - 1) {
                let centroid = (v0 + face_positions[i] + face_positions[i + 1]) / 3.0;
                let mut cd = sampler.sample_world(centroid);
                if !cd.is_finite() {
                    cd = 1.0e6;
                }
                let centroid_keep = if keep_inside { cd <= 0.0 } else { cd >= 0.0 };
                if centroid_keep != boundary_keep {
                    interior_cut = true;
                    break;
                }
            }
            if interior_cut {
                for i in 1..(count - 1) {
                    let tri = [
                        ClipVertex {
                            pos: v0,
                            dist: face_dists[0],
                        },
                        ClipVertex {
                            pos: face_positions[i],
                            dist: face_dists[i],
                        },
                        ClipVertex {
                            pos: face_positions[i + 1],
                            dist: face_dists[i + 1],
                        },
                    ];
                    let polygon =
                        build_polygon_samples(&tri, &sampler, voxel, max_edge_samples);
                    let clipped = clip_polygon(&polygon, keep_inside);
                    if clipped.len() < 3 {
                        continue;
                    }
                    let base = out_positions.len() as u32;
                    for vertex in &clipped {
                        out_positions.push(vertex.pos.to_array());
                    }
                    out_indices.extend((0..clipped.len()).map(|idx| base + idx as u32));
                    out_face_counts.push(clipped.len() as u32);
                }
                cursor += count;
                continue;
            }
        }

        if all_keep {
            let mut face_indices = Vec::with_capacity(count);
            for i in 0..count {
                let src_idx = mesh.indices[cursor + i] as usize;
                let out_idx = if let Some(existing) = point_map.get(&src_idx) {
                    *existing
                } else {
                    let pos = mesh.positions.get(src_idx).copied().unwrap_or([0.0, 0.0, 0.0]);
                    out_positions.push(pos);
                    let idx = (out_positions.len() - 1) as u32;
                    point_map.insert(src_idx, idx);
                    idx
                };
                face_indices.push(out_idx);
            }
            out_indices.extend(face_indices);
            out_face_counts.push(count as u32);
            cursor += count;
            continue;
        }

        if all_drop {
            cursor += count;
            continue;
        }

        let mut verts = Vec::with_capacity(count);
        for i in 0..count {
            verts.push(ClipVertex {
                pos: face_positions[i],
                dist: face_dists[i],
            });
        }
        let polygon = build_polygon_samples(&verts, &sampler, voxel, max_edge_samples);
        let clipped = clip_polygon(&polygon, keep_inside);
        if clipped.len() < 3 {
            cursor += count;
            continue;
        }

        let base = out_positions.len() as u32;
        for vertex in &clipped {
            out_positions.push(vertex.pos.to_array());
        }
        out_indices.extend((0..clipped.len()).map(|i| base + i as u32));
        out_face_counts.push(clipped.len() as u32);
        cursor += count;
    }

    Ok(Mesh::with_positions_faces(
        out_positions,
        out_indices,
        out_face_counts,
    ))
}

fn build_polygon_samples(
    vertices: &[ClipVertex],
    sampler: &VolumeSampler,
    voxel: f32,
    max_edge_samples: usize,
) -> Vec<ClipVertex> {
    let mut polygon = Vec::new();
    if vertices.is_empty() {
        return polygon;
    }
    let count = vertices.len();
    for i in 0..count {
        let curr = vertices[i];
        let next = vertices[(i + 1) % count];
        polygon.push(curr);
        let edge = next.pos - curr.pos;
        let length = edge.length();
        let steps = ((length / (voxel * 2.0)).ceil() as usize).clamp(1, max_edge_samples);
        if steps > 1 {
            let inv = 1.0 / steps as f32;
            for step in 1..steps {
                let t = step as f32 * inv;
                if t >= 1.0 {
                    break;
                }
                let p = curr.pos + edge * t;
                let mut d = sampler.sample_world(p);
                if !d.is_finite() {
                    d = 1.0e6;
                }
                polygon.push(ClipVertex { pos: p, dist: d });
            }
        }
    }
    polygon
}

fn manifold_from_mesh(mesh: &Mesh) -> Result<Manifold, String> {
    if mesh.positions.is_empty() {
        return Err("mesh has no points".to_string());
    }
    let triangulation = mesh.triangulate();
    if triangulation.indices.is_empty() || !triangulation.indices.len().is_multiple_of(3) {
        return Err("mesh has no triangles".to_string());
    }
    let positions = flatten_positions(mesh);
    let indices = triangulation
        .indices
        .iter()
        .map(|i| *i as usize)
        .collect::<Vec<_>>();
    Manifold::new(&positions, &indices).map_err(|err| err.to_string())
}

fn append_mesh_with_defaults(dst: &mut Mesh, src: &Mesh) {
    dst.ensure_face_counts();
    let base = dst.positions.len() as u32;
    let src_points = src.positions.len();
    let src_corners = src.indices.len();
    let src_face_counts = if src.face_counts.is_empty() {
        if src.indices.len().is_multiple_of(3) {
            vec![3u32; src.indices.len() / 3]
        } else if src.indices.is_empty() {
            Vec::new()
        } else {
            vec![src.indices.len() as u32]
        }
    } else {
        src.face_counts.clone()
    };
    let src_prims = src_face_counts.len();

    dst.positions.extend_from_slice(&src.positions);
    dst.indices
        .extend(src.indices.iter().map(|idx| idx + base));
    dst.face_counts.extend(src_face_counts);

    if let Some(uvs) = &mut dst.uvs {
        if let Some(src_uvs) = &src.uvs {
            uvs.extend_from_slice(src_uvs);
        } else {
            uvs.extend(std::iter::repeat_n([0.0, 0.0], src_points));
        }
    }

    let point_names = dst
        .attributes
        .map(AttributeDomain::Point)
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    for name in point_names {
        if let Some(storage) = dst.attributes.map_mut(AttributeDomain::Point).get_mut(&name) {
            extend_attribute_storage(storage, src_points);
        }
    }

    let vertex_names = dst
        .attributes
        .map(AttributeDomain::Vertex)
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    for name in vertex_names {
        if let Some(storage) = dst
            .attributes
            .map_mut(AttributeDomain::Vertex)
            .get_mut(&name)
        {
            extend_attribute_storage(storage, src_corners);
        }
    }

    let prim_names = dst
        .attributes
        .map(AttributeDomain::Primitive)
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    for name in prim_names {
        if let Some(storage) = dst
            .attributes
            .map_mut(AttributeDomain::Primitive)
            .get_mut(&name)
        {
            extend_attribute_storage(storage, src_prims);
        }
    }

    for group in dst.groups.map_mut(AttributeDomain::Point).values_mut() {
        group.extend(std::iter::repeat_n(false, src_points));
    }
    for group in dst.groups.map_mut(AttributeDomain::Vertex).values_mut() {
        group.extend(std::iter::repeat_n(false, src_corners));
    }
    for group in dst.groups.map_mut(AttributeDomain::Primitive).values_mut() {
        group.extend(std::iter::repeat_n(false, src_prims));
    }
}

fn extend_attribute_storage(storage: &mut AttributeStorage, extra: usize) {
    match storage {
        AttributeStorage::Float(values) => {
            values.extend(std::iter::repeat_n(0.0, extra));
        }
        AttributeStorage::Int(values) => {
            values.extend(std::iter::repeat_n(0, extra));
        }
        AttributeStorage::Vec2(values) => {
            values.extend(std::iter::repeat_n([0.0, 0.0], extra));
        }
        AttributeStorage::Vec3(values) => {
            values.extend(std::iter::repeat_n([0.0, 0.0, 0.0], extra));
        }
        AttributeStorage::Vec4(values) => {
            values.extend(std::iter::repeat_n([0.0, 0.0, 0.0, 0.0], extra));
        }
        AttributeStorage::StringTable(values) => {
            if values.values.is_empty() {
                values.values.push(String::new());
            }
            values.indices.extend(std::iter::repeat_n(0, extra));
        }
    }
}

#[derive(Clone, Copy)]
struct MeshTriangle {
    a: Vec3,
    b: Vec3,
    c: Vec3,
}

fn build_triangle_list(mesh: &Mesh) -> Vec<MeshTriangle> {
    let mut tris = Vec::new();
    let triangulation = mesh.triangulate();
    for tri in triangulation.indices.chunks_exact(3) {
        let a = Vec3::from(mesh.positions[tri[0] as usize]);
        let b = Vec3::from(mesh.positions[tri[1] as usize]);
        let c = Vec3::from(mesh.positions[tri[2] as usize]);
        tris.push(MeshTriangle { a, b, c });
    }
    tris
}

fn is_inside_mesh(point: Vec3, triangles: &[MeshTriangle]) -> bool {
    if triangles.is_empty() {
        return false;
    }
    winding_number(point, triangles).abs() >= 0.5
}

fn winding_number(point: Vec3, triangles: &[MeshTriangle]) -> f32 {
    let mut total = 0.0f32;
    for tri in triangles {
        let a = tri.a - point;
        let b = tri.b - point;
        let c = tri.c - point;
        let la = a.length();
        let lb = b.length();
        let lc = c.length();
        if la < 1.0e-8 || lb < 1.0e-8 || lc < 1.0e-8 {
            continue;
        }
        let numerator = a.dot(b.cross(c));
        let denom = la * lb * lc + a.dot(b) * lc + b.dot(c) * la + c.dot(a) * lb;
        if denom.abs() < 1.0e-12 {
            continue;
        }
        total += 2.0 * numerator.atan2(denom);
    }
    total / (4.0 * std::f32::consts::PI)
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

#[derive(Clone)]
struct SourceMesh<'a> {
    mesh: &'a Mesh,
    positions: Vec<Vec3>,
    triangles: Vec<[u32; 3]>,
    tri_bounds: Vec<[Vec3; 2]>,
    tri_to_face: Vec<usize>,
    tri_corner_indices: Vec<[usize; 3]>,
    point_uvs: Option<Vec<[f32; 2]>>,
    vertex_uvs: Option<Vec<[f32; 2]>>,
}

impl<'a> SourceMesh<'a> {
    fn new(mesh: &'a Mesh) -> Result<Self, String> {
        let triangulation = mesh.triangulate();
        if triangulation.indices.is_empty() {
            return Err("Boolean Geo requires mesh inputs".to_string());
        }
        let positions = mesh.positions.iter().copied().map(Vec3::from).collect::<Vec<_>>();
        let mut triangles = Vec::new();
        let mut tri_bounds = Vec::new();
        let mut tri_to_face = Vec::new();
        let mut tri_corner_indices = Vec::new();
        let tri_count = triangulation.indices.len() / 3;
        for tri_index in 0..tri_count {
            let base = tri_index * 3;
            let tri_idx = [
                *triangulation.indices.get(base).unwrap_or(&0),
                *triangulation.indices.get(base + 1).unwrap_or(&0),
                *triangulation.indices.get(base + 2).unwrap_or(&0),
            ];
            let a = positions[tri_idx[0] as usize];
            let b = positions[tri_idx[1] as usize];
            let c = positions[tri_idx[2] as usize];
            let min = a.min(b).min(c);
            let max = a.max(b).max(c);
            triangles.push(tri_idx);
            tri_bounds.push([min, max]);
            tri_to_face.push(*triangulation.tri_to_face.get(tri_index).unwrap_or(&tri_index));
            tri_corner_indices.push([
                *triangulation.corner_indices.get(base).unwrap_or(&base),
                *triangulation.corner_indices.get(base + 1).unwrap_or(&(base + 1)),
                *triangulation.corner_indices.get(base + 2).unwrap_or(&(base + 2)),
            ]);
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
            tri_to_face,
            tri_corner_indices,
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
    let prim_count = output.face_count();

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
        prim_count,
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
        prim_count,
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
    let prim_positions = crate::nodes::attribute_utils::mesh_positions_for_domain(
        output,
        AttributeDomain::Primitive,
    );
    prim_positions
        .iter()
        .map(|pos| nearest_triangle(*pos, sources))
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
            AttributeDomain::Primitive => {
                let face_index = sample_face_index(source, sample.tri_index);
                values.get(face_index).copied()
            }
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
            AttributeDomain::Primitive => {
                let face_index = sample_face_index(source, sample.tri_index);
                values.get(face_index).copied()
            }
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
            AttributeDomain::Primitive => {
                let face_index = sample_face_index(source, sample.tri_index);
                values.get(face_index).copied()
            }
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
            AttributeDomain::Primitive => {
                let face_index = sample_face_index(source, sample.tri_index);
                values.get(face_index).copied()
            }
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
            AttributeDomain::Primitive => {
                let face_index = sample_face_index(source, sample.tri_index);
                values.get(face_index).copied()
            }
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
            AttributeDomain::Primitive => {
                let face_index = sample_face_index(source, sample.tri_index);
                values.value(face_index).map(|v| v.to_string())
            }
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
        AttributeDomain::Primitive => {
            let face_index = sample_face_index(source, sample.tri_index);
            values.get(face_index).copied()
        }
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
            let corners = source.tri_corner_indices.get(tri_index)?;
            Some(*corners)
        }
        _ => None,
    }
}

fn sample_face_index(source: &SourceMesh<'_>, tri_index: usize) -> usize {
    *source.tri_to_face.get(tri_index).unwrap_or(&tri_index)
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
