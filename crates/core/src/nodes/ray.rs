use std::collections::BTreeMap;

use glam::Vec3;

use crate::attributes::{AttributeDomain, AttributeRef, AttributeStorage, AttributeType};
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    attribute_utils::{
        existing_float_attr_mesh, existing_float_attr_splats, existing_int_attr_mesh,
        existing_int_attr_splats, existing_vec2_attr_mesh, existing_vec2_attr_splats,
        existing_vec3_attr_mesh, existing_vec3_attr_splats, existing_vec4_attr_mesh,
        existing_vec4_attr_splats, parse_attribute_list,
    },
    geometry_in,
    geometry_out,
    group_utils::{mask_has_any, mesh_group_mask, splat_group_mask},
    recompute_mesh_normals,
    require_mesh_input,
};
use crate::param_spec::ParamSpec;
use crate::splat::SplatGeo;

pub const NAME: &str = "Ray";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![geometry_in("in"), geometry_in("target")],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            ("method".to_string(), ParamValue::Int(0)),
            ("direction".to_string(), ParamValue::Vec3([0.0, 1.0, 0.0])),
            ("max_distance".to_string(), ParamValue::Float(1.0)),
            ("splat_density".to_string(), ParamValue::Float(0.0)),
            ("apply_transform".to_string(), ParamValue::Bool(true)),
            ("attr".to_string(), ParamValue::String(String::new())),
            ("hit_group".to_string(), ParamValue::String(String::new())),
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::int_enum(
            "method",
            "Method",
            vec![(0, "Normal"), (1, "Direction"), (2, "Closest")],
        )
        .with_help("Ray direction mode."),
        ParamSpec::vec3("direction", "Direction").with_help("Ray direction (Direction mode)."),
        ParamSpec::float_slider("max_distance", "Max Distance", 0.0, 1000.0)
            .with_help("Max ray distance."),
        ParamSpec::float_slider("splat_density", "Splat Density", 0.0, 10.0)
            .with_help("Accumulated splat density threshold (0 disables)."),
        ParamSpec::bool("apply_transform", "Apply Transform")
            .with_help("Move points to hit location."),
        ParamSpec::string("attr", "Attributes")
            .with_help("Attribute(s) to import from the hit."),
        ParamSpec::string("hit_group", "Hit Group").with_help("Group name to mark hits."),
        ParamSpec::string("group", "Group").with_help("Restrict source points to a group."),
        ParamSpec::int_enum(
            "group_type",
            "Group Type",
            vec![
                (0, "Auto"),
                (1, "Vertex"),
                (2, "Point"),
                (3, "Primitive"),
            ],
        )
        .with_help("Group domain to use."),
    ]
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut source = require_mesh_input(inputs, 0, "Ray requires a mesh input")?;
    let target = require_mesh_input(inputs, 1, "Ray requires a target mesh")?;
    apply_to_mesh_with_targets(params, &mut source, std::slice::from_ref(&target), &[])?;
    Ok(source)
}

pub fn apply_to_geometry(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let Some(source) = inputs.first() else {
        return Ok(Geometry::default());
    };
    let Some(target) = inputs.get(1) else {
        return Err("Ray requires a target input".to_string());
    };

    let target_meshes = target
        .merged_mesh()
        .map(|mesh| vec![mesh])
        .unwrap_or_default();
    let target_meshes_ref = target_meshes.as_slice();

    let mut meshes = Vec::new();
    if let Some(mut mesh) = source.merged_mesh() {
        apply_to_mesh_with_targets(params, &mut mesh, target_meshes_ref, &target.splats)?;
        meshes.push(mesh);
    }

    let mut splats = Vec::with_capacity(source.splats.len());
    for splat in &source.splats {
        let mut splat = splat.clone();
        apply_to_splats_with_targets(params, &mut splat, target_meshes_ref, &target.splats)?;
        splats.push(splat);
    }

    let curves = if meshes.is_empty() { Vec::new() } else { source.curves.clone() };
    Ok(Geometry {
        meshes,
        splats,
        curves,
        volumes: source.volumes.clone(),
        materials: source.materials.clone(),
    })
}

#[derive(Clone, Copy, Debug)]
enum RayMethod {
    Normal,
    Direction,
    Closest,
}

fn method_from_params(params: &NodeParams) -> RayMethod {
    match params.get_int("method", 0).clamp(0, 2) {
        1 => RayMethod::Direction,
        2 => RayMethod::Closest,
        _ => RayMethod::Normal,
    }
}

#[derive(Clone, Copy, Debug)]
struct HitInfo {
    position: Vec3,
    normal: Vec3,
    distance: f32,
    source: HitSource,
}

#[derive(Clone, Copy, Debug)]
enum HitSource {
    Mesh {
        mesh_index: usize,
        face_index: usize,
        point_indices: [usize; 3],
        corner_indices: [usize; 3],
        barycentric: [f32; 3],
    },
    Splat {
        splat_set: usize,
        splat_index: usize,
    },
}

fn apply_to_mesh_with_targets(
    params: &NodeParams,
    mesh: &mut Mesh,
    target_meshes: &[Mesh],
    target_splats: &[SplatGeo],
) -> Result<(), String> {
    if mesh.positions.is_empty() || (target_meshes.is_empty() && target_splats.is_empty()) {
        return Ok(());
    }
    let mask = mesh_group_mask(mesh, params, AttributeDomain::Point);
    if !mask_has_any(mask.as_deref()) {
        return Ok(());
    }

    let method = method_from_params(params);
    let max_distance = params.get_float("max_distance", 1.0);
    let splat_density = params.get_float("splat_density", 0.0);
    let apply_transform = params.get_bool("apply_transform", true);
    let direction_param = Vec3::from(params.get_vec3("direction", [0.0, 1.0, 0.0]));
    let point_normals = if matches!(method, RayMethod::Normal) {
        mesh_point_normals(mesh)
    } else {
        None
    };

    let mut hits = vec![None; mesh.positions.len()];
    for (idx, position) in mesh.positions.iter().enumerate() {
        if mask
            .as_ref()
            .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
        {
            continue;
        }
        let origin = Vec3::from(*position);
        let dir = match method {
            RayMethod::Closest => None,
            RayMethod::Direction => Some(direction_param),
            RayMethod::Normal => {
                let normal = point_normals
                    .as_ref()
                    .and_then(|values| values.get(idx).copied())
                    .unwrap_or(direction_param);
                Some(normal)
            }
        };
        let hit = match method {
            RayMethod::Closest => {
                find_closest_hit(origin, max_distance, target_meshes, target_splats)
            }
            _ => dir
                .and_then(normalize_vec)
                .and_then(|dir| {
                    find_ray_hit(
                        origin,
                        dir,
                        max_distance,
                        splat_density,
                        target_meshes,
                        target_splats,
                    )
                }),
        };
        hits[idx] = hit;
    }

    if apply_transform {
        for (idx, hit) in hits.iter().enumerate() {
            if let Some(hit) = hit {
                if let Some(slot) = mesh.positions.get_mut(idx) {
                    *slot = hit.position.into();
                }
            }
        }
        recompute_mesh_normals(mesh);
    }

    apply_hit_group(mesh.groups.map_mut(AttributeDomain::Point), params, &hits);
    apply_hit_attributes_mesh(
        mesh,
        &hits,
        target_meshes,
        target_splats,
        params,
    )?;
    Ok(())
}

fn apply_to_splats_with_targets(
    params: &NodeParams,
    splats: &mut SplatGeo,
    target_meshes: &[Mesh],
    target_splats: &[SplatGeo],
) -> Result<(), String> {
    if splats.positions.is_empty() || (target_meshes.is_empty() && target_splats.is_empty()) {
        return Ok(());
    }
    let mask = splat_group_mask(splats, params, AttributeDomain::Point);
    if !mask_has_any(mask.as_deref()) {
        return Ok(());
    }

    let method = method_from_params(params);
    let max_distance = params.get_float("max_distance", 1.0);
    let splat_density = params.get_float("splat_density", 0.0);
    let apply_transform = params.get_bool("apply_transform", true);
    let direction_param = Vec3::from(params.get_vec3("direction", [0.0, 1.0, 0.0]));
    let point_normals = if matches!(method, RayMethod::Normal) {
        splat_point_normals(splats)
    } else {
        None
    };

    let mut hits = vec![None; splats.positions.len()];
    for (idx, position) in splats.positions.iter().enumerate() {
        if mask
            .as_ref()
            .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
        {
            continue;
        }
        let origin = Vec3::from(*position);
        let dir = match method {
            RayMethod::Closest => None,
            RayMethod::Direction => Some(direction_param),
            RayMethod::Normal => {
                let normal = point_normals
                    .as_ref()
                    .and_then(|values| values.get(idx).copied())
                    .unwrap_or(direction_param);
                Some(normal)
            }
        };
        let hit = match method {
            RayMethod::Closest => {
                find_closest_hit(origin, max_distance, target_meshes, target_splats)
            }
            _ => dir
                .and_then(normalize_vec)
                .and_then(|dir| {
                    find_ray_hit(
                        origin,
                        dir,
                        max_distance,
                        splat_density,
                        target_meshes,
                        target_splats,
                    )
                }),
        };
        hits[idx] = hit;
    }

    if apply_transform {
        for (idx, hit) in hits.iter().enumerate() {
            if let Some(hit) = hit {
                if let Some(slot) = splats.positions.get_mut(idx) {
                    *slot = hit.position.into();
                }
            }
        }
    }

    apply_hit_group(splats.groups.map_mut(AttributeDomain::Point), params, &hits);
    apply_hit_attributes_splats(
        splats,
        &hits,
        target_meshes,
        target_splats,
        params,
    )?;
    Ok(())
}

fn apply_hit_group(
    groups: &mut std::collections::BTreeMap<String, Vec<bool>>,
    params: &NodeParams,
    hits: &[Option<HitInfo>],
) {
    let name = params.get_string("hit_group", "");
    if name.trim().is_empty() {
        return;
    }
    let mut values = vec![false; hits.len()];
    for (idx, hit) in hits.iter().enumerate() {
        if hit.is_some() {
            if let Some(slot) = values.get_mut(idx) {
                *slot = true;
            }
        }
    }
    groups.insert(name.to_string(), values);
}

fn apply_hit_attributes_mesh(
    mesh: &mut Mesh,
    hits: &[Option<HitInfo>],
    target_meshes: &[Mesh],
    target_splats: &[SplatGeo],
    params: &NodeParams,
) -> Result<(), String> {
    let attr_names = parse_attribute_list(params.get_string("attr", ""));
    if attr_names.is_empty() {
        return Ok(());
    }
    let count = mesh.positions.len();
    for name in attr_names {
        if name == "P" {
            continue;
        }
        let Some(attr_type) = target_attribute_type(&name, target_meshes, target_splats) else {
            continue;
        };
        match attr_type {
            AttributeType::Float => {
                let mut out =
                    existing_float_attr_mesh(mesh, AttributeDomain::Point, &name, count);
                for (idx, hit) in hits.iter().enumerate() {
                    let Some(hit) = hit else { continue; };
                    if let Some(AttributeValue::Float(value)) =
                        sample_hit_value(&name, hit, target_meshes, target_splats)
                    {
                        if let Some(slot) = out.get_mut(idx) {
                            *slot = value;
                        }
                    }
                }
                mesh.set_attribute(AttributeDomain::Point, name, AttributeStorage::Float(out))
                    .map_err(|err| format!("Ray error: {:?}", err))?;
            }
            AttributeType::Int => {
                let mut out = existing_int_attr_mesh(mesh, AttributeDomain::Point, &name, count);
                for (idx, hit) in hits.iter().enumerate() {
                    let Some(hit) = hit else { continue; };
                    if let Some(AttributeValue::Int(value)) =
                        sample_hit_value(&name, hit, target_meshes, target_splats)
                    {
                        if let Some(slot) = out.get_mut(idx) {
                            *slot = value;
                        }
                    }
                }
                mesh.set_attribute(AttributeDomain::Point, name, AttributeStorage::Int(out))
                    .map_err(|err| format!("Ray error: {:?}", err))?;
            }
            AttributeType::Vec2 => {
                let mut out = existing_vec2_attr_mesh(mesh, AttributeDomain::Point, &name, count);
                for (idx, hit) in hits.iter().enumerate() {
                    let Some(hit) = hit else { continue; };
                    if let Some(AttributeValue::Vec2(value)) =
                        sample_hit_value(&name, hit, target_meshes, target_splats)
                    {
                        if let Some(slot) = out.get_mut(idx) {
                            *slot = value;
                        }
                    }
                }
                mesh.set_attribute(AttributeDomain::Point, name, AttributeStorage::Vec2(out))
                    .map_err(|err| format!("Ray error: {:?}", err))?;
            }
            AttributeType::Vec3 => {
                let mut out = existing_vec3_attr_mesh(mesh, AttributeDomain::Point, &name, count);
                for (idx, hit) in hits.iter().enumerate() {
                    let Some(hit) = hit else { continue; };
                    if let Some(AttributeValue::Vec3(value)) =
                        sample_hit_value(&name, hit, target_meshes, target_splats)
                    {
                        if let Some(slot) = out.get_mut(idx) {
                            *slot = value;
                        }
                    }
                }
                mesh.set_attribute(AttributeDomain::Point, name, AttributeStorage::Vec3(out))
                    .map_err(|err| format!("Ray error: {:?}", err))?;
            }
            AttributeType::Vec4 => {
                let mut out = existing_vec4_attr_mesh(mesh, AttributeDomain::Point, &name, count);
                for (idx, hit) in hits.iter().enumerate() {
                    let Some(hit) = hit else { continue; };
                    if let Some(AttributeValue::Vec4(value)) =
                        sample_hit_value(&name, hit, target_meshes, target_splats)
                    {
                        if let Some(slot) = out.get_mut(idx) {
                            *slot = value;
                        }
                    }
                }
                mesh.set_attribute(AttributeDomain::Point, name, AttributeStorage::Vec4(out))
                    .map_err(|err| format!("Ray error: {:?}", err))?;
            }
            AttributeType::String => {}
        }
    }
    Ok(())
}

fn apply_hit_attributes_splats(
    splats: &mut SplatGeo,
    hits: &[Option<HitInfo>],
    target_meshes: &[Mesh],
    target_splats: &[SplatGeo],
    params: &NodeParams,
) -> Result<(), String> {
    let attr_names = parse_attribute_list(params.get_string("attr", ""));
    if attr_names.is_empty() {
        return Ok(());
    }
    let count = splats.positions.len();
    for name in attr_names {
        if name == "P" {
            continue;
        }
        let Some(attr_type) = target_attribute_type(&name, target_meshes, target_splats) else {
            continue;
        };
        match attr_type {
            AttributeType::Float => {
                let mut out =
                    existing_float_attr_splats(splats, AttributeDomain::Point, &name, count);
                for (idx, hit) in hits.iter().enumerate() {
                    let Some(hit) = hit else { continue; };
                    if let Some(AttributeValue::Float(value)) =
                        sample_hit_value(&name, hit, target_meshes, target_splats)
                    {
                        if let Some(slot) = out.get_mut(idx) {
                            *slot = value;
                        }
                    }
                }
                splats
                    .set_attribute(AttributeDomain::Point, name, AttributeStorage::Float(out))
                    .map_err(|err| format!("Ray error: {:?}", err))?;
            }
            AttributeType::Int => {
                let mut out =
                    existing_int_attr_splats(splats, AttributeDomain::Point, &name, count);
                for (idx, hit) in hits.iter().enumerate() {
                    let Some(hit) = hit else { continue; };
                    if let Some(AttributeValue::Int(value)) =
                        sample_hit_value(&name, hit, target_meshes, target_splats)
                    {
                        if let Some(slot) = out.get_mut(idx) {
                            *slot = value;
                        }
                    }
                }
                splats
                    .set_attribute(AttributeDomain::Point, name, AttributeStorage::Int(out))
                    .map_err(|err| format!("Ray error: {:?}", err))?;
            }
            AttributeType::Vec2 => {
                let mut out =
                    existing_vec2_attr_splats(splats, AttributeDomain::Point, &name, count);
                for (idx, hit) in hits.iter().enumerate() {
                    let Some(hit) = hit else { continue; };
                    if let Some(AttributeValue::Vec2(value)) =
                        sample_hit_value(&name, hit, target_meshes, target_splats)
                    {
                        if let Some(slot) = out.get_mut(idx) {
                            *slot = value;
                        }
                    }
                }
                splats
                    .set_attribute(AttributeDomain::Point, name, AttributeStorage::Vec2(out))
                    .map_err(|err| format!("Ray error: {:?}", err))?;
            }
            AttributeType::Vec3 => {
                let mut out =
                    existing_vec3_attr_splats(splats, AttributeDomain::Point, &name, count);
                for (idx, hit) in hits.iter().enumerate() {
                    let Some(hit) = hit else { continue; };
                    if let Some(AttributeValue::Vec3(value)) =
                        sample_hit_value(&name, hit, target_meshes, target_splats)
                    {
                        if let Some(slot) = out.get_mut(idx) {
                            *slot = value;
                        }
                    }
                }
                splats
                    .set_attribute(AttributeDomain::Point, name, AttributeStorage::Vec3(out))
                    .map_err(|err| format!("Ray error: {:?}", err))?;
            }
            AttributeType::Vec4 => {
                let mut out =
                    existing_vec4_attr_splats(splats, AttributeDomain::Point, &name, count);
                for (idx, hit) in hits.iter().enumerate() {
                    let Some(hit) = hit else { continue; };
                    if let Some(AttributeValue::Vec4(value)) =
                        sample_hit_value(&name, hit, target_meshes, target_splats)
                    {
                        if let Some(slot) = out.get_mut(idx) {
                            *slot = value;
                        }
                    }
                }
                splats
                    .set_attribute(AttributeDomain::Point, name, AttributeStorage::Vec4(out))
                    .map_err(|err| format!("Ray error: {:?}", err))?;
            }
            AttributeType::String => {}
        }
    }
    Ok(())
}

fn target_attribute_type(
    name: &str,
    target_meshes: &[Mesh],
    target_splats: &[SplatGeo],
) -> Option<AttributeType> {
    if matches!(name, "P" | "N") {
        return Some(AttributeType::Vec3);
    }
    for mesh in target_meshes {
        if let Some((_domain, attr)) = mesh.attribute_with_precedence(name) {
            return Some(attr.data_type());
        }
    }
    for splats in target_splats {
        if let Some((_domain, attr)) = splats.attribute_with_precedence(name) {
            return Some(attr.data_type());
        }
    }
    None
}

fn find_closest_hit(
    origin: Vec3,
    max_distance: f32,
    target_meshes: &[Mesh],
    target_splats: &[SplatGeo],
) -> Option<HitInfo> {
    let mut best: Option<HitInfo> = None;
    for (mesh_index, mesh) in target_meshes.iter().enumerate() {
        let Some(hit) = closest_hit_mesh(origin, mesh, mesh_index) else {
            continue;
        };
        if max_distance > 0.0 && hit.distance > max_distance {
            continue;
        }
        if best.is_none() || hit.distance < best.as_ref().unwrap().distance {
            best = Some(hit);
        }
    }
    for (splat_set, splats) in target_splats.iter().enumerate() {
        let Some(hit) = closest_hit_splats(origin, splats, splat_set) else {
            continue;
        };
        if max_distance > 0.0 && hit.distance > max_distance {
            continue;
        }
        if best.is_none() || hit.distance < best.as_ref().unwrap().distance {
            best = Some(hit);
        }
    }
    best
}

fn find_ray_hit(
    origin: Vec3,
    dir: Vec3,
    max_distance: f32,
    splat_density: f32,
    target_meshes: &[Mesh],
    target_splats: &[SplatGeo],
) -> Option<HitInfo> {
    let mut best: Option<HitInfo> = None;
    for (mesh_index, mesh) in target_meshes.iter().enumerate() {
        let Some(hit) = ray_hit_mesh(origin, dir, max_distance, mesh, mesh_index) else {
            continue;
        };
        if best.is_none() || hit.distance < best.as_ref().unwrap().distance {
            best = Some(hit);
        }
    }
    for (splat_set, splats) in target_splats.iter().enumerate() {
        let Some(hit) =
            ray_hit_splats(origin, dir, max_distance, splat_density, splats, splat_set)
        else {
            continue;
        };
        if best.is_none() || hit.distance < best.as_ref().unwrap().distance {
            best = Some(hit);
        }
    }
    best
}

fn closest_hit_mesh(origin: Vec3, mesh: &Mesh, mesh_index: usize) -> Option<HitInfo> {
    let triangulation = mesh.triangulate();
    if triangulation.indices.len() < 3 {
        return None;
    }
    let tri_indices = &triangulation.indices;
    let tri_corners = &triangulation.corner_indices;
    let tri_faces = &triangulation.tri_to_face;
    let tri_count = tri_indices.len() / 3;
    let mut best: Option<HitInfo> = None;
    for tri_index in 0..tri_count {
        let base = tri_index * 3;
        let a_idx = *tri_indices.get(base)? as usize;
        let b_idx = *tri_indices.get(base + 1)? as usize;
        let c_idx = *tri_indices.get(base + 2)? as usize;
        let a = Vec3::from(*mesh.positions.get(a_idx)?);
        let b = Vec3::from(*mesh.positions.get(b_idx)?);
        let c = Vec3::from(*mesh.positions.get(c_idx)?);
        let (closest, bary) = closest_point_on_triangle(origin, a, b, c);
        if !closest.is_finite() {
            continue;
        }
        let normal = triangle_normal(a, b, c);
        let distance = (closest - origin).length();
        if !distance.is_finite() {
            continue;
        }
        let face_index = *tri_faces.get(tri_index).unwrap_or(&tri_index);
        let corner_indices = [
            *tri_corners.get(base).unwrap_or(&base),
            *tri_corners.get(base + 1).unwrap_or(&(base + 1)),
            *tri_corners.get(base + 2).unwrap_or(&(base + 2)),
        ];
        let hit = HitInfo {
            position: closest,
            normal,
            distance,
            source: HitSource::Mesh {
                mesh_index,
                face_index,
                point_indices: [a_idx, b_idx, c_idx],
                corner_indices,
                barycentric: bary,
            },
        };
        if best.is_none() || distance < best.as_ref().unwrap().distance {
            best = Some(hit);
        }
    }
    best
}

fn ray_hit_mesh(
    origin: Vec3,
    dir: Vec3,
    max_distance: f32,
    mesh: &Mesh,
    mesh_index: usize,
) -> Option<HitInfo> {
    let triangulation = mesh.triangulate();
    if triangulation.indices.len() < 3 {
        return None;
    }
    let tri_indices = &triangulation.indices;
    let tri_corners = &triangulation.corner_indices;
    let tri_faces = &triangulation.tri_to_face;
    let tri_count = tri_indices.len() / 3;
    let mut best: Option<HitInfo> = None;
    for tri_index in 0..tri_count {
        let base = tri_index * 3;
        let a_idx = *tri_indices.get(base)? as usize;
        let b_idx = *tri_indices.get(base + 1)? as usize;
        let c_idx = *tri_indices.get(base + 2)? as usize;
        let a = Vec3::from(*mesh.positions.get(a_idx)?);
        let b = Vec3::from(*mesh.positions.get(b_idx)?);
        let c = Vec3::from(*mesh.positions.get(c_idx)?);
        let Some((t, bary)) = ray_triangle_intersect(origin, dir, a, b, c) else {
            continue;
        };
        if t < 0.0 {
            continue;
        }
        if max_distance > 0.0 && t > max_distance {
            continue;
        }
        let position = origin + dir * t;
        let normal = triangle_normal(a, b, c);
        let face_index = *tri_faces.get(tri_index).unwrap_or(&tri_index);
        let corner_indices = [
            *tri_corners.get(base).unwrap_or(&base),
            *tri_corners.get(base + 1).unwrap_or(&(base + 1)),
            *tri_corners.get(base + 2).unwrap_or(&(base + 2)),
        ];
        let hit = HitInfo {
            position,
            normal,
            distance: t,
            source: HitSource::Mesh {
                mesh_index,
                face_index,
                point_indices: [a_idx, b_idx, c_idx],
                corner_indices,
                barycentric: bary,
            },
        };
        if best.is_none() || t < best.as_ref().unwrap().distance {
            best = Some(hit);
        }
    }
    best
}

fn closest_hit_splats(origin: Vec3, splats: &SplatGeo, splat_set: usize) -> Option<HitInfo> {
    if splats.positions.is_empty() {
        return None;
    }
    let mut best: Option<HitInfo> = None;
    for (idx, position) in splats.positions.iter().enumerate() {
        let pos = Vec3::from(*position);
        if !pos.is_finite() {
            continue;
        }
        let distance = (pos - origin).length();
        if !distance.is_finite() {
            continue;
        }
        let normal = normalize_vec(pos - origin).unwrap_or(Vec3::Y);
        let hit = HitInfo {
            position: pos,
            normal,
            distance,
            source: HitSource::Splat {
                splat_set,
                splat_index: idx,
            },
        };
        if best.is_none() || distance < best.as_ref().unwrap().distance {
            best = Some(hit);
        }
    }
    best
}

fn ray_hit_splats(
    origin: Vec3,
    dir: Vec3,
    max_distance: f32,
    splat_density: f32,
    splats: &SplatGeo,
    splat_set: usize,
) -> Option<HitInfo> {
    if splats.positions.is_empty() {
        return None;
    }
    if splat_density > 0.0 {
        return ray_hit_splats_accumulated(
            origin,
            dir,
            max_distance,
            splat_density,
            splats,
            splat_set,
        );
    }
    let mut best: Option<HitInfo> = None;
    for (idx, position) in splats.positions.iter().enumerate() {
        let center = Vec3::from(*position);
        let radius = splat_radius(splats.scales.get(idx).copied());
        let Some(t) = ray_sphere_intersect(origin, dir, center, radius) else {
            continue;
        };
        if t < 0.0 {
            continue;
        }
        if max_distance > 0.0 && t > max_distance {
            continue;
        }
        let position = origin + dir * t;
        let normal = normalize_vec(position - center).unwrap_or(Vec3::Y);
        let hit = HitInfo {
            position,
            normal,
            distance: t,
            source: HitSource::Splat {
                splat_set,
                splat_index: idx,
            },
        };
        if best.is_none() || t < best.as_ref().unwrap().distance {
            best = Some(hit);
        }
    }
    best
}

fn ray_hit_splats_accumulated(
    origin: Vec3,
    dir: Vec3,
    max_distance: f32,
    splat_density: f32,
    splats: &SplatGeo,
    splat_set: usize,
) -> Option<HitInfo> {
    let mut segments = Vec::new();
    for (idx, position) in splats.positions.iter().enumerate() {
        let center = Vec3::from(*position);
        if !center.is_finite() {
            continue;
        }
        let radius = splat_radius(splats.scales.get(idx).copied());
        let Some((t0, mut t1)) = ray_sphere_interval(origin, dir, center, radius) else {
            continue;
        };
        if max_distance > 0.0 {
            if t0 > max_distance {
                continue;
            }
            t1 = t1.min(max_distance);
        }
        if t1 <= t0 {
            continue;
        }
        let alpha = splat_alpha(splats.opacity.get(idx).copied().unwrap_or(0.0));
        if alpha <= 0.0 {
            continue;
        }
        segments.push((t0, t1, alpha, center, idx));
    }
    if segments.is_empty() {
        return None;
    }
    segments.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut accumulated = 0.0f32;
    for (t0, t1, alpha, center, idx) in segments {
        let span = t1 - t0;
        let contrib = alpha * span;
        if contrib <= 0.0 {
            continue;
        }
        if accumulated + contrib >= splat_density {
            let remaining = (splat_density - accumulated).max(0.0);
            let t = if alpha > 0.0 {
                (t0 + remaining / alpha).min(t1)
            } else {
                t0
            };
            let position = origin + dir * t;
            let normal = normalize_vec(position - center).unwrap_or(Vec3::Y);
            return Some(HitInfo {
                position,
                normal,
                distance: t,
                source: HitSource::Splat {
                    splat_set,
                    splat_index: idx,
                },
            });
        }
        accumulated += contrib;
    }
    None
}

fn ray_triangle_intersect(
    origin: Vec3,
    dir: Vec3,
    a: Vec3,
    b: Vec3,
    c: Vec3,
) -> Option<(f32, [f32; 3])> {
    let eps = 1.0e-6;
    let edge1 = b - a;
    let edge2 = c - a;
    let h = dir.cross(edge2);
    let det = edge1.dot(h);
    if det.abs() < eps {
        return None;
    }
    let inv_det = 1.0 / det;
    let s = origin - a;
    let u = s.dot(h) * inv_det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(edge1);
    let v = dir.dot(q) * inv_det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = edge2.dot(q) * inv_det;
    Some((t, [1.0 - u - v, u, v]))
}

fn ray_sphere_intersect(origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    let oc = origin - center;
    let b = oc.dot(dir);
    let c = oc.dot(oc) - radius * radius;
    let h = b * b - c;
    if h < 0.0 {
        return None;
    }
    let sqrt_h = h.sqrt();
    let mut t = -b - sqrt_h;
    if t < 0.0 {
        t = -b + sqrt_h;
    }
    Some(t)
}

fn ray_sphere_interval(origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<(f32, f32)> {
    if radius <= 0.0 {
        return None;
    }
    let oc = origin - center;
    let b = oc.dot(dir);
    let c = oc.dot(oc) - radius * radius;
    let h = b * b - c;
    if h < 0.0 {
        return None;
    }
    let sqrt_h = h.sqrt();
    let mut t0 = -b - sqrt_h;
    let mut t1 = -b + sqrt_h;
    if t1 < 0.0 {
        return None;
    }
    if t0 < 0.0 {
        t0 = 0.0;
    }
    if t1 < t0 {
        std::mem::swap(&mut t0, &mut t1);
    }
    Some((t0, t1))
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

fn triangle_normal(a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let n = (b - a).cross(c - a);
    normalize_vec(n).unwrap_or(Vec3::Y)
}

fn normalize_vec(v: Vec3) -> Option<Vec3> {
    if v.length_squared() <= 1.0e-8 {
        None
    } else {
        Some(v.normalize())
    }
}

fn splat_alpha(opacity: f32) -> f32 {
    let mut alpha = 1.0 / (1.0 + (-opacity).exp());
    if !alpha.is_finite() {
        alpha = 0.0;
    }
    alpha.clamp(0.0, 1.0)
}

fn mesh_point_normals(mesh: &Mesh) -> Option<Vec<Vec3>> {
    if let Some(normals) = &mesh.normals {
        if normals.len() == mesh.positions.len() {
            return Some(normals.iter().copied().map(Vec3::from).collect());
        }
    }
    if mesh.indices.is_empty() || mesh.positions.is_empty() {
        return None;
    }
    let mut normals = vec![Vec3::ZERO; mesh.positions.len()];
    let triangulation = mesh.triangulate();
    for tri in triangulation.indices.chunks_exact(3) {
        let a = Vec3::from(mesh.positions[tri[0] as usize]);
        let b = Vec3::from(mesh.positions[tri[1] as usize]);
        let c = Vec3::from(mesh.positions[tri[2] as usize]);
        let n = (b - a).cross(c - a);
        for &idx in tri {
            if let Some(slot) = normals.get_mut(idx as usize) {
                *slot += n;
            }
        }
    }
    for normal in &mut normals {
        if let Some(n) = normalize_vec(*normal) {
            *normal = n;
        } else {
            *normal = Vec3::Y;
        }
    }
    Some(normals)
}

fn splat_point_normals(splats: &SplatGeo) -> Option<Vec<Vec3>> {
    let Some(AttributeRef::Vec3(values)) =
        splats.attribute(AttributeDomain::Point, "N")
    else {
        return None;
    };
    if values.len() != splats.positions.len() {
        return None;
    }
    Some(values.iter().copied().map(Vec3::from).collect())
}

fn splat_radius(scale: Option<[f32; 3]>) -> f32 {
    let Some(scale) = scale else {
        return 1.0;
    };
    let s = Vec3::new(
        if scale[0].is_finite() { scale[0].exp() } else { 0.0 },
        if scale[1].is_finite() { scale[1].exp() } else { 0.0 },
        if scale[2].is_finite() { scale[2].exp() } else { 0.0 },
    );
    let max_sigma = s.x.max(s.y).max(s.z);
    let radius = max_sigma * 3.0;
    if radius.is_finite() {
        radius.max(1.0e-4)
    } else {
        1.0e-4
    }
}

#[derive(Clone, Copy, Debug)]
enum AttributeValue {
    Float(f32),
    Int(i32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
}

fn sample_hit_value(
    name: &str,
    hit: &HitInfo,
    target_meshes: &[Mesh],
    target_splats: &[SplatGeo],
) -> Option<AttributeValue> {
    if name == "P" {
        return Some(AttributeValue::Vec3(hit.position.into()));
    }
    if name == "N" {
        return Some(AttributeValue::Vec3(hit.normal.into()));
    }
    match hit.source {
        HitSource::Mesh {
            mesh_index,
            face_index,
            point_indices,
            corner_indices,
            barycentric,
        } => {
            let mesh = target_meshes.get(mesh_index)?;
            sample_mesh_attribute(
                mesh,
                name,
                face_index,
                point_indices,
                corner_indices,
                barycentric,
                hit.position,
                hit.normal,
            )
        }
        HitSource::Splat {
            splat_set,
            splat_index,
        } => {
            let splats = target_splats.get(splat_set)?;
            sample_splat_attribute(splats, name, splat_index, hit.position, hit.normal)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn sample_mesh_attribute(
    mesh: &Mesh,
    name: &str,
    face_index: usize,
    point_indices: [usize; 3],
    corner_indices: [usize; 3],
    barycentric: [f32; 3],
    hit_pos: Vec3,
    hit_normal: Vec3,
) -> Option<AttributeValue> {
    if name == "P" {
        return Some(AttributeValue::Vec3(hit_pos.into()));
    }
    if name == "N" {
        return Some(AttributeValue::Vec3(hit_normal.into()));
    }
    let (domain, attr) = mesh.attribute_with_precedence(name)?;
    match attr {
        AttributeRef::Float(values) => match domain {
            AttributeDomain::Point | AttributeDomain::Vertex => {
                let indices = match domain {
                    AttributeDomain::Point => point_indices,
                    AttributeDomain::Vertex => corner_indices,
                    _ => point_indices,
                };
                let value = lerp_f32(values, indices, barycentric)?;
                Some(AttributeValue::Float(value))
            }
            AttributeDomain::Primitive => {
                let value = values.get(face_index).copied()?;
                Some(AttributeValue::Float(value))
            }
            AttributeDomain::Detail => values.first().copied().map(AttributeValue::Float),
        },
        AttributeRef::Int(values) => match domain {
            AttributeDomain::Point | AttributeDomain::Vertex => {
                let indices = match domain {
                    AttributeDomain::Point => point_indices,
                    AttributeDomain::Vertex => corner_indices,
                    _ => point_indices,
                };
                let idx = barycentric_max_index(barycentric);
                let value = *indices.get(idx).and_then(|i| values.get(*i))?;
                Some(AttributeValue::Int(value))
            }
            AttributeDomain::Primitive => {
                let value = values.get(face_index).copied()?;
                Some(AttributeValue::Int(value))
            }
            AttributeDomain::Detail => values.first().copied().map(AttributeValue::Int),
        },
        AttributeRef::Vec2(values) => match domain {
            AttributeDomain::Point | AttributeDomain::Vertex => {
                let indices = match domain {
                    AttributeDomain::Point => point_indices,
                    AttributeDomain::Vertex => corner_indices,
                    _ => point_indices,
                };
                let value = lerp_vec2(values, indices, barycentric)?;
                Some(AttributeValue::Vec2(value))
            }
            AttributeDomain::Primitive => {
                let value = values.get(face_index).copied()?;
                Some(AttributeValue::Vec2(value))
            }
            AttributeDomain::Detail => values.first().copied().map(AttributeValue::Vec2),
        },
        AttributeRef::Vec3(values) => match domain {
            AttributeDomain::Point | AttributeDomain::Vertex => {
                let indices = match domain {
                    AttributeDomain::Point => point_indices,
                    AttributeDomain::Vertex => corner_indices,
                    _ => point_indices,
                };
                let value = lerp_vec3(values, indices, barycentric)?;
                Some(AttributeValue::Vec3(value))
            }
            AttributeDomain::Primitive => {
                let value = values.get(face_index).copied()?;
                Some(AttributeValue::Vec3(value))
            }
            AttributeDomain::Detail => values.first().copied().map(AttributeValue::Vec3),
        },
        AttributeRef::Vec4(values) => match domain {
            AttributeDomain::Point | AttributeDomain::Vertex => {
                let indices = match domain {
                    AttributeDomain::Point => point_indices,
                    AttributeDomain::Vertex => corner_indices,
                    _ => point_indices,
                };
                let value = lerp_vec4(values, indices, barycentric)?;
                Some(AttributeValue::Vec4(value))
            }
            AttributeDomain::Primitive => {
                let value = values.get(face_index).copied()?;
                Some(AttributeValue::Vec4(value))
            }
            AttributeDomain::Detail => values.first().copied().map(AttributeValue::Vec4),
        },
        AttributeRef::StringTable(_) => None,
    }
}

fn sample_splat_attribute(
    splats: &SplatGeo,
    name: &str,
    splat_index: usize,
    hit_pos: Vec3,
    hit_normal: Vec3,
) -> Option<AttributeValue> {
    if name == "P" {
        return Some(AttributeValue::Vec3(hit_pos.into()));
    }
    if name == "N" {
        return Some(AttributeValue::Vec3(hit_normal.into()));
    }
    let (domain, attr) = splats.attribute_with_precedence(name)?;
    match attr {
        AttributeRef::Float(values) => match domain {
            AttributeDomain::Point | AttributeDomain::Primitive => {
                values.get(splat_index).copied().map(AttributeValue::Float)
            }
            AttributeDomain::Detail => values.first().copied().map(AttributeValue::Float),
            AttributeDomain::Vertex => None,
        },
        AttributeRef::Int(values) => match domain {
            AttributeDomain::Point | AttributeDomain::Primitive => {
                values.get(splat_index).copied().map(AttributeValue::Int)
            }
            AttributeDomain::Detail => values.first().copied().map(AttributeValue::Int),
            AttributeDomain::Vertex => None,
        },
        AttributeRef::Vec2(values) => match domain {
            AttributeDomain::Point | AttributeDomain::Primitive => {
                values.get(splat_index).copied().map(AttributeValue::Vec2)
            }
            AttributeDomain::Detail => values.first().copied().map(AttributeValue::Vec2),
            AttributeDomain::Vertex => None,
        },
        AttributeRef::Vec3(values) => match domain {
            AttributeDomain::Point | AttributeDomain::Primitive => {
                values.get(splat_index).copied().map(AttributeValue::Vec3)
            }
            AttributeDomain::Detail => values.first().copied().map(AttributeValue::Vec3),
            AttributeDomain::Vertex => None,
        },
        AttributeRef::Vec4(values) => match domain {
            AttributeDomain::Point | AttributeDomain::Primitive => {
                values.get(splat_index).copied().map(AttributeValue::Vec4)
            }
            AttributeDomain::Detail => values.first().copied().map(AttributeValue::Vec4),
            AttributeDomain::Vertex => None,
        },
        AttributeRef::StringTable(_) => None,
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
