use std::collections::{BTreeMap, HashMap};

use glam::Vec3;

use crate::attributes::{AttributeDomain, AttributeStorage, StringTableAttribute};
use crate::curve::Curve;
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{geometry_in, geometry_out, require_mesh_input};
use crate::param_spec::ParamSpec;
use crate::volume::{try_alloc_f32, Volume};
use crate::volume_sampling::VolumeSampler;

pub const NAME: &str = "Resample";
const DEFAULT_CURVE_POINTS: i32 = 16;
const DEFAULT_MESH_RATIO: f32 = 0.5;
const DEFAULT_VOLUME_MAX_DIM: i32 = 64;

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
            ("curve_points".to_string(), ParamValue::Int(DEFAULT_CURVE_POINTS)),
            ("mesh_ratio".to_string(), ParamValue::Float(DEFAULT_MESH_RATIO)),
            ("volume_max_dim".to_string(), ParamValue::Int(DEFAULT_VOLUME_MAX_DIM)),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::int_slider("curve_points", "Curve Points", 2, 512)
            .with_help("Target point count per curve."),
        ParamSpec::float_slider("mesh_ratio", "Mesh Ratio", 0.0, 1.0)
            .with_help("Target triangle ratio for mesh reduction."),
        ParamSpec::int_slider("volume_max_dim", "Volume Max Dim", 8, 512)
            .with_help("Maximum voxel dimension for resampled volumes."),
    ]
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mesh = require_mesh_input(inputs, 0, "Resample requires a mesh input")?;
    Ok(resample_mesh(params, &mesh))
}

pub fn apply_to_geometry(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };

    let source_mesh = input.merged_mesh();
    let mut meshes = Vec::new();
    let mut mesh_out = source_mesh.as_ref().map(|mesh| resample_mesh(params, mesh));

    let mut curves = Vec::new();
    if !input.curves.is_empty() {
        if let Some(mesh) = source_mesh.as_ref() {
            let (curve_points, mut new_curves) =
                resample_curves(mesh, &input.curves, params);
            if !curve_points.is_empty() {
                let base_offset = mesh_out
                    .as_ref()
                    .map(|mesh| mesh.positions.len())
                    .unwrap_or(0);
                if let Some(mesh_out) = mesh_out.as_mut() {
                    extend_mesh_point_data(mesh_out, curve_points.len());
                    mesh_out.positions.extend(curve_points);
                } else {
                    mesh_out = Some(Mesh::with_positions_indices(curve_points, Vec::new()));
                }
                for curve in &mut new_curves {
                    curve.offset_indices(base_offset as u32);
                }
                curves = new_curves;
            } else {
                curves = input.curves.clone();
            }
        } else {
            curves = input.curves.clone();
        }
    }

    if let Some(mesh) = mesh_out {
        meshes.push(mesh);
    }

    let volume_max_dim = params
        .get_int("volume_max_dim", DEFAULT_VOLUME_MAX_DIM)
        .max(1) as u32;
    let volumes = input
        .volumes
        .iter()
        .map(|volume| resample_volume(volume, volume_max_dim))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Geometry {
        meshes,
        splats: input.splats.clone(),
        curves,
        volumes,
        materials: input.materials.clone(),
    })
}

fn resample_mesh(params: &NodeParams, mesh: &Mesh) -> Mesh {
    let ratio = params.get_float("mesh_ratio", DEFAULT_MESH_RATIO);
    if ratio >= 0.999 {
        return mesh.clone();
    }
    if ratio <= 0.0 {
        return Mesh::default();
    }
    let tri_count = mesh.indices.len() / 3;
    if tri_count == 0 || !mesh.indices.len().is_multiple_of(3) {
        return mesh.clone();
    }
    let target_tris = (tri_count as f32 * ratio).round().max(1.0) as usize;
    if target_tris >= tri_count {
        return mesh.clone();
    }

    let bounds = match mesh.bounds() {
        Some(bounds) => bounds,
        None => return mesh.clone(),
    };
    let min = Vec3::from(bounds.min);
    let max = Vec3::from(bounds.max);
    let size = (max - min).abs();
    let volume = size.x * size.y * size.z;
    if !volume.is_finite() || volume <= 0.0 {
        return mesh.clone();
    }

    let target_points = target_tris.max(1) as f32;
    let cell_size = (volume / target_points).cbrt().max(1.0e-6);
    let inv_cell = 1.0 / cell_size;

    #[derive(Clone)]
    struct Cell {
        sum: Vec3,
        count: u32,
        rep: usize,
    }

    let mut cell_map: HashMap<(i32, i32, i32), Cell> = HashMap::new();
    let mut point_cells = Vec::with_capacity(mesh.positions.len());
    for (idx, pos) in mesh.positions.iter().enumerate() {
        let pos = Vec3::from(*pos);
        let key = (
            ((pos.x - min.x) * inv_cell).floor() as i32,
            ((pos.y - min.y) * inv_cell).floor() as i32,
            ((pos.z - min.z) * inv_cell).floor() as i32,
        );
        let entry = cell_map.entry(key).or_insert(Cell {
            sum: Vec3::ZERO,
            count: 0,
            rep: idx,
        });
        entry.sum += pos;
        entry.count += 1;
        point_cells.push(key);
    }

    let mut cells: Vec<((i32, i32, i32), Cell)> = cell_map.into_iter().collect();
    cells.sort_by_key(|(key, _)| *key);

    let mut new_positions = Vec::with_capacity(cells.len());
    let mut rep_indices = Vec::with_capacity(cells.len());
    let mut cell_to_new: HashMap<(i32, i32, i32), usize> = HashMap::new();
    for (idx, (key, cell)) in cells.into_iter().enumerate() {
        let center = if cell.count > 0 {
            cell.sum / cell.count as f32
        } else {
            Vec3::ZERO
        };
        new_positions.push(center.to_array());
        rep_indices.push(cell.rep);
        cell_to_new.insert(key, idx);
    }

    let mut point_map = Vec::with_capacity(mesh.positions.len());
    for key in point_cells {
        let new_idx = cell_to_new.get(&key).copied().unwrap_or(0);
        point_map.push(new_idx as u32);
    }

    let mut new_indices = Vec::new();
    for tri in mesh.indices.chunks_exact(3) {
        let a = point_map.get(tri[0] as usize).copied().unwrap_or(0);
        let b = point_map.get(tri[1] as usize).copied().unwrap_or(0);
        let c = point_map.get(tri[2] as usize).copied().unwrap_or(0);
        if a == b || b == c || a == c {
            continue;
        }
        new_indices.push(a);
        new_indices.push(b);
        new_indices.push(c);
    }

    let mut out = Mesh::with_positions_indices(new_positions, new_indices);
    if let Some(uvs) = &mesh.uvs {
        if uvs.len() == mesh.positions.len() {
            let mut new_uvs = Vec::with_capacity(out.positions.len());
            for &rep in &rep_indices {
                new_uvs.push(*uvs.get(rep).unwrap_or(&[0.0, 0.0]));
            }
            out.uvs = Some(new_uvs);
        }
    }

    for (name, storage) in mesh.attributes.map(AttributeDomain::Point) {
        let mapped = remap_storage(storage, &rep_indices);
        let _ = out.set_attribute(AttributeDomain::Point, name.clone(), mapped);
    }
    for (name, storage) in mesh.attributes.map(AttributeDomain::Detail) {
        let _ = out.set_attribute(AttributeDomain::Detail, name.clone(), storage.clone());
    }
    for (name, values) in mesh.groups.map(AttributeDomain::Point) {
        let mut new_values = Vec::with_capacity(rep_indices.len());
        for &rep in &rep_indices {
            new_values.push(values.get(rep).copied().unwrap_or(false));
        }
        out.groups.map_mut(AttributeDomain::Point).insert(name.clone(), new_values);
    }

    if !out.indices.is_empty() {
        let _ = out.compute_normals();
    }

    out
}

fn remap_storage(storage: &AttributeStorage, reps: &[usize]) -> AttributeStorage {
    match storage {
        AttributeStorage::Float(values) => AttributeStorage::Float(
            reps.iter().map(|&idx| values.get(idx).copied().unwrap_or(0.0)).collect(),
        ),
        AttributeStorage::Int(values) => AttributeStorage::Int(
            reps.iter().map(|&idx| values.get(idx).copied().unwrap_or(0)).collect(),
        ),
        AttributeStorage::Vec2(values) => AttributeStorage::Vec2(
            reps.iter().map(|&idx| values.get(idx).copied().unwrap_or([0.0, 0.0])).collect(),
        ),
        AttributeStorage::Vec3(values) => AttributeStorage::Vec3(
            reps.iter().map(|&idx| values.get(idx).copied().unwrap_or([0.0, 0.0, 0.0])).collect(),
        ),
        AttributeStorage::Vec4(values) => AttributeStorage::Vec4(
            reps.iter()
                .map(|&idx| values.get(idx).copied().unwrap_or([0.0, 0.0, 0.0, 0.0]))
                .collect(),
        ),
        AttributeStorage::StringTable(values) => {
            let mut indices = Vec::with_capacity(reps.len());
            for &idx in reps {
                indices.push(values.indices.get(idx).copied().unwrap_or(0));
            }
            AttributeStorage::StringTable(StringTableAttribute::new(values.values.clone(), indices))
        }
    }
}

fn resample_curves(mesh: &Mesh, curves: &[Curve], params: &NodeParams) -> (Vec<[f32; 3]>, Vec<Curve>) {
    let target = params
        .get_int("curve_points", DEFAULT_CURVE_POINTS)
        .max(2) as usize;
    let mut positions = Vec::new();
    let mut out_curves = Vec::new();
    for curve in curves {
        let points = curve.resolved_points(&mesh.positions);
        let resampled = resample_polyline(&points, curve.closed, target);
        if resampled.len() < 2 {
            continue;
        }
        let base = positions.len() as u32;
        positions.extend_from_slice(&resampled);
        let indices = (0..resampled.len() as u32).map(|i| base + i).collect();
        out_curves.push(Curve::new(indices, curve.closed));
    }
    (positions, out_curves)
}

fn resample_polyline(points: &[[f32; 3]], closed: bool, target: usize) -> Vec<[f32; 3]> {
    if points.len() < 2 || target < 2 {
        return points.to_vec();
    }
    let target = if closed { target.max(3) } else { target.max(2) };
    let segment_count = if closed { points.len() } else { points.len() - 1 };
    let mut lengths = Vec::with_capacity(segment_count);
    let mut total = 0.0f32;
    for i in 0..segment_count {
        let a = Vec3::from(points[i]);
        let b = Vec3::from(points[(i + 1) % points.len()]);
        let len = (b - a).length().max(0.0);
        total += len;
        lengths.push(len);
    }
    if total <= 1.0e-6 {
        return vec![points[0]; target];
    }

    let step = if closed {
        total / target as f32
    } else {
        total / (target - 1) as f32
    };
    let mut samples = Vec::with_capacity(target);
    let mut seg_index = 0usize;
    let mut seg_start = Vec3::from(points[0]);
    let mut seg_end = Vec3::from(points[1 % points.len()]);
    let mut seg_len = lengths[0].max(1.0e-6);
    let mut seg_accum = 0.0f32;

    for i in 0..target {
        let dist = if closed { i as f32 * step } else { (i as f32 * step).min(total) };
        while dist > seg_accum + seg_len && seg_index + 1 < lengths.len() {
            seg_accum += seg_len;
            seg_index += 1;
            seg_start = Vec3::from(points[seg_index]);
            seg_end = Vec3::from(points[(seg_index + 1) % points.len()]);
            seg_len = lengths[seg_index].max(1.0e-6);
        }
        let t = ((dist - seg_accum) / seg_len).clamp(0.0, 1.0);
        let pos = seg_start + (seg_end - seg_start) * t;
        samples.push(pos.to_array());
    }
    if !closed {
        if let Some(last) = points.last() {
            if let Some(last_sample) = samples.last_mut() {
                *last_sample = *last;
            }
        }
    }
    samples
}

fn extend_mesh_point_data(mesh: &mut Mesh, extra: usize) {
    if extra == 0 {
        return;
    }
    if let Some(normals) = &mut mesh.normals {
        normals.extend(std::iter::repeat_n([0.0, 1.0, 0.0], extra));
    }
    if let Some(uvs) = &mut mesh.uvs {
        uvs.extend(std::iter::repeat_n([0.0, 0.0], extra));
    }
    for storage in mesh.attributes.map_mut(AttributeDomain::Point).values_mut() {
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
                values.indices.extend(std::iter::repeat_n(0, extra));
            }
        }
    }
    for group in mesh.groups.map_mut(AttributeDomain::Point).values_mut() {
        group.extend(std::iter::repeat_n(false, extra));
    }
}

fn resample_volume(volume: &Volume, max_dim: u32) -> Result<Volume, String> {
    if volume.is_empty() || max_dim == 0 {
        return Ok(volume.clone());
    }
    let old_dims = volume.dims;
    let max_old = old_dims[0].max(old_dims[1]).max(old_dims[2]);
    if max_old == 0 || max_old == max_dim {
        return Ok(volume.clone());
    }

    let voxel_size = volume.voxel_size.max(1.0e-6);
    let size = Vec3::new(
        old_dims[0].saturating_sub(1) as f32 * voxel_size,
        old_dims[1].saturating_sub(1) as f32 * voxel_size,
        old_dims[2].saturating_sub(1) as f32 * voxel_size,
    );
    let max_extent = size.x.max(size.y).max(size.z).max(1.0e-6);
    let new_voxel = max_extent / max_dim as f32;
    let new_dims = [
        (size.x / new_voxel).round().max(1.0) as u32,
        (size.y / new_voxel).round().max(1.0) as u32,
        (size.z / new_voxel).round().max(1.0) as u32,
    ];

    let total = (new_dims[0] * new_dims[1] * new_dims[2]) as usize;
    let mut values = try_alloc_f32(total, "Volume Resample")?;
    let sampler = VolumeSampler::new(volume);
    let origin = Vec3::from(volume.origin);
    for z in 0..new_dims[2] {
        for y in 0..new_dims[1] {
            for x in 0..new_dims[0] {
                let local = origin
                    + Vec3::new(x as f32 * new_voxel, y as f32 * new_voxel, z as f32 * new_voxel);
                let world = volume.transform.transform_point3(local);
                let value = sampler.sample_world(world);
                let idx = (z * new_dims[0] * new_dims[1] + y * new_dims[0] + x) as usize;
                values[idx] = value;
            }
        }
    }

    let mut out = volume.clone();
    out.dims = new_dims;
    out.voxel_size = new_voxel;
    out.values = values;
    out.sdf_band = new_voxel.max(1.0e-6) * 2.0;
    Ok(out)
}
