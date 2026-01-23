pub use lobedo_scene::{
    SceneCurve, SceneDrawable, SceneMaterial, SceneMesh, SceneSnapshot, SceneSplats, SceneVolume,
    SceneVolumeKind,
};

use crate::attributes::{AttributeDomain, AttributeRef};
use crate::curve::Curve;
use crate::geometry::Geometry;
use crate::material::Material;
use crate::mesh::Mesh;
use crate::splat::SplatGeo;
use crate::volume::{Volume, VolumeKind};

type UvData = (Option<Vec<[f32; 2]>>, Option<Vec<[f32; 2]>>);

pub fn scene_snapshot_from_mesh(mesh: &Mesh, base_color: [f32; 3]) -> SceneSnapshot {
    SceneSnapshot {
        drawables: vec![SceneDrawable::Mesh(scene_mesh_from_mesh(mesh))],
        base_color,
        materials: Vec::new(),
    }
}

pub fn scene_snapshot_from_splats(
    splats: &SplatGeo,
    base_color: [f32; 3],
) -> SceneSnapshot {
    SceneSnapshot {
        drawables: vec![SceneDrawable::Splats(scene_splats_from_splats(splats))],
        base_color,
        materials: Vec::new(),
    }
}

pub fn scene_snapshot_from_geometry(
    geometry: &Geometry,
    base_color: [f32; 3],
) -> SceneSnapshot {
    let mut drawables = Vec::new();
    let materials: Vec<SceneMaterial> = geometry
        .materials
        .iter()
        .map(scene_material_from_material)
        .collect();
    let mut material_lookup = std::collections::HashMap::new();
    for (idx, material) in geometry.materials.iter().enumerate() {
        material_lookup.insert(material.name.clone(), idx as u32);
    }
    let mesh = geometry.merged_mesh();
    if let Some(mesh) = mesh.as_ref() {
        drawables.push(SceneDrawable::Mesh(scene_mesh_from_mesh_with_materials(
            mesh,
            &material_lookup,
        )));
    }
    for splats in &geometry.splats {
        drawables.push(SceneDrawable::Splats(scene_splats_from_splats(splats)));
    }
    let curve_points = mesh
        .as_ref()
        .map(|mesh| mesh.positions.as_slice())
        .unwrap_or(&[]);
    for curve in &geometry.curves {
        drawables.push(SceneDrawable::Curve(scene_curve_from_curve(
            curve,
            curve_points,
        )));
    }
    for volume in &geometry.volumes {
        drawables.push(SceneDrawable::Volume(scene_volume_from_volume(volume)));
    }
    SceneSnapshot {
        drawables,
        base_color,
        materials,
    }
}

pub fn scene_mesh_from_mesh(mesh: &Mesh) -> SceneMesh {
    scene_mesh_from_mesh_with_materials(mesh, &std::collections::HashMap::new())
}

fn scene_mesh_from_mesh_with_materials(
    mesh: &Mesh,
    material_lookup: &std::collections::HashMap<String, u32>,
) -> SceneMesh {
    let triangulation = mesh.triangulate();
    let tri_indices = triangulation.indices;
    let tri_to_face: Vec<u32> = triangulation
        .tri_to_face
        .iter()
        .map(|&value| value as u32)
        .collect();
    let corner_indices: Vec<u32> = triangulation
        .corner_indices
        .iter()
        .map(|&value| value as u32)
        .collect();
    let poly_indices = mesh.indices.clone();
    let poly_face_counts = if mesh.face_counts.is_empty() {
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
    let mut normals = fallback_normals(mesh);
    let mut corner_normals = mesh.corner_normals.clone();
    if let Some((domain, attr)) = mesh.attribute_with_precedence("N") {
        if let Some(values) = attr_vec3(attr) {
            match domain {
                AttributeDomain::Vertex => {
                    if values.len() == mesh.indices.len() {
                        corner_normals =
                            expand_corner_attribute(&values, &triangulation.corner_indices);
                    }
                }
                AttributeDomain::Point => {
                    if values.len() == mesh.positions.len() {
                        normals = values;
                        corner_normals = None;
                    }
                }
                AttributeDomain::Primitive => {
                    if let Some(expanded) =
                        expand_primitive_vec3(mesh, &values, &triangulation.tri_to_face)
                    {
                        corner_normals = Some(expanded);
                    }
                }
                AttributeDomain::Detail => {
                    if let Some(value) = values.first().copied() {
                        if tri_indices.is_empty() {
                            normals = vec![value; mesh.positions.len()];
                            corner_normals = None;
                        } else {
                            corner_normals = Some(vec![value; tri_indices.len()]);
                        }
                    }
                }
            }
        }
    }

    let mut colors = None;
    let mut corner_colors = None;
    if let Some((domain, attr)) = mesh.attribute_with_precedence("Cd") {
        if let Some(values) = attr_vec3(attr) {
            match domain {
                AttributeDomain::Vertex => {
                    if values.len() == mesh.indices.len() {
                        corner_colors =
                            expand_corner_attribute(&values, &triangulation.corner_indices);
                    }
                }
                AttributeDomain::Point => {
                    if values.len() == mesh.positions.len() {
                        colors = Some(values);
                    }
                }
                AttributeDomain::Primitive => {
                    if let Some(expanded) =
                        expand_primitive_vec3(mesh, &values, &triangulation.tri_to_face)
                    {
                        corner_colors = Some(expanded);
                    }
                }
                AttributeDomain::Detail => {
                    if let Some(value) = values.first().copied() {
                        if tri_indices.is_empty() {
                            colors = Some(vec![value; mesh.positions.len()]);
                        } else {
                            corner_colors = Some(vec![value; tri_indices.len()]);
                        }
                    }
                }
            }
        }
    }

    let (uvs, corner_uvs) = mesh_uvs(mesh, &tri_indices, &triangulation.corner_indices);
    let corner_materials = mesh_materials(mesh, material_lookup, &triangulation.tri_to_face);

    SceneMesh {
        positions: mesh.positions.clone(),
        normals,
        indices: tri_indices,
        tri_to_face,
        corner_indices,
        poly_indices,
        poly_face_counts,
        corner_normals,
        colors,
        corner_colors,
        uvs,
        corner_uvs,
        corner_materials,
    }
}

fn scene_splats_from_splats(splats: &SplatGeo) -> SceneSplats {
    let sh0 = splats.sh0.clone();
    let mut opacity = splats.opacity.clone();
    let mut scales = splats.scales.clone();
    let sh_coeffs = splats.sh_coeffs;
    let sh_rest = splats.sh_rest.clone();

    for value in &mut opacity {
        let logit = value.clamp(-9.21034, 9.21034);
        *value = 1.0 / (1.0 + (-logit).exp());
    }

    for value in &mut scales {
        let sx = value[0].clamp(-10.0, 10.0).exp();
        let sy = value[1].clamp(-10.0, 10.0).exp();
        let sz = value[2].clamp(-10.0, 10.0).exp();
        *value = [sx, sy, sz];
    }

    let sh0_is_coeff = sh_coeffs > 0
        || sh0
            .iter()
            .any(|value| value[0] < 0.0 || value[1] < 0.0 || value[2] < 0.0);

    SceneSplats {
        positions: splats.positions.clone(),
        sh0,
        sh_coeffs,
        sh_rest,
        sh0_is_coeff,
        opacity,
        scales,
        rotations: splats.rotations.clone(),
    }
}

fn scene_curve_from_curve(curve: &Curve, positions: &[[f32; 3]]) -> SceneCurve {
    SceneCurve {
        points: curve.resolved_points(positions),
        closed: curve.closed,
    }
}

fn scene_volume_from_volume(volume: &Volume) -> SceneVolume {
    let kind = match volume.kind {
        VolumeKind::Density => SceneVolumeKind::Density,
        VolumeKind::Sdf => SceneVolumeKind::Sdf,
    };
    SceneVolume {
        kind,
        origin: volume.origin,
        dims: volume.dims,
        voxel_size: volume.voxel_size,
        values: volume.values.clone(),
        transform: volume.transform,
        density_scale: volume.density_scale,
        sdf_band: volume.sdf_band,
    }
}

fn scene_material_from_material(material: &Material) -> SceneMaterial {
    SceneMaterial {
        name: material.name.clone(),
        base_color: material.base_color,
        metallic: material.metallic,
        roughness: material.roughness,
        base_color_texture: material.base_color_texture.clone(),
    }
}

fn fallback_normals(mesh: &Mesh) -> Vec<[f32; 3]> {
    match &mesh.normals {
        Some(normals) => normals.clone(),
        None => {
            let mut temp = mesh.clone();
            temp.compute_normals();
            temp.normals
                .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; mesh.positions.len()])
        }
    }
}

fn attr_vec3(attr: AttributeRef<'_>) -> Option<Vec<[f32; 3]>> {
    match attr {
        AttributeRef::Vec3(values) => Some(values.to_vec()),
        AttributeRef::Vec4(values) => Some(values.iter().map(|v| [v[0], v[1], v[2]]).collect()),
        _ => None,
    }
}

fn attr_vec2(attr: AttributeRef<'_>) -> Option<Vec<[f32; 2]>> {
    match attr {
        AttributeRef::Vec2(values) => Some(values.to_vec()),
        AttributeRef::Vec3(values) => Some(values.iter().map(|v| [v[0], v[1]]).collect()),
        AttributeRef::Vec4(values) => Some(values.iter().map(|v| [v[0], v[1]]).collect()),
        _ => None,
    }
}

fn mesh_uvs(mesh: &Mesh, tri_indices: &[u32], corner_indices: &[usize]) -> UvData {
    let mut uvs = mesh.uvs.clone();
    if let Some(attr) = mesh.attribute(AttributeDomain::Point, "uv") {
        if let Some(values) = attr_vec2(attr) {
            if values.len() == mesh.positions.len() {
                uvs = Some(values);
            }
        }
    }

    let mut corner_uvs = None;
    if let Some(attr) = mesh.attribute(AttributeDomain::Vertex, "uv") {
        if let Some(values) = attr_vec2(attr) {
            if values.len() == mesh.indices.len() {
                corner_uvs = expand_corner_attribute(&values, corner_indices);
            }
        }
    }
    if corner_uvs.is_none() {
        if let Some(uvs) = &uvs {
            if uvs.len() == mesh.positions.len() && !tri_indices.is_empty() {
                let mut expanded = Vec::with_capacity(tri_indices.len());
                for &idx in tri_indices {
                    expanded.push(*uvs.get(idx as usize).unwrap_or(&[0.0, 0.0]));
                }
                corner_uvs = Some(expanded);
            }
        }
    }

    (uvs, corner_uvs)
}

fn mesh_materials(
    mesh: &Mesh,
    material_lookup: &std::collections::HashMap<String, u32>,
    tri_to_face: &[usize],
) -> Option<Vec<u32>> {
    let attr = mesh.attribute(AttributeDomain::Primitive, "material")?;
    let AttributeRef::StringTable(table) = attr else {
        return None;
    };
    let face_count = mesh.face_count();
    if face_count == 0 || table.indices.len() != face_count || tri_to_face.is_empty() {
        return None;
    }
    let mut out = Vec::with_capacity(tri_to_face.len() * 3);
    for &face_idx in tri_to_face {
        let mat_idx = *table.indices.get(face_idx)?;
        let name = table
            .values
            .get(mat_idx as usize)
            .map(|value| value.as_str())
            .unwrap_or("");
        let resolved = material_lookup.get(name).copied().unwrap_or(0);
        out.extend_from_slice(&[resolved; 3]);
    }
    Some(out)
}

fn expand_primitive_vec3(
    mesh: &Mesh,
    values: &[[f32; 3]],
    tri_to_face: &[usize],
) -> Option<Vec<[f32; 3]>> {
    let face_count = mesh.face_count();
    if values.len() != face_count || tri_to_face.is_empty() {
        return None;
    }
    let mut expanded = Vec::with_capacity(tri_to_face.len() * 3);
    for &face_idx in tri_to_face {
        let value = *values.get(face_idx)?;
        expanded.extend_from_slice(&[value; 3]);
    }
    Some(expanded)
}

fn expand_corner_attribute<T: Copy>(values: &[T], corner_indices: &[usize]) -> Option<Vec<T>> {
    if values.is_empty() {
        return None;
    }
    let mut out = Vec::with_capacity(corner_indices.len());
    for &idx in corner_indices {
        out.push(*values.get(idx)?);
    }
    Some(out)
}
