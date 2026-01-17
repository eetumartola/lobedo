use std::collections::HashMap;
use std::path::Path;

use crate::attributes::{AttributeDomain, AttributeRef, AttributeStorage, StringTableAttribute};
use crate::mesh::Mesh;

pub fn load_gltf_mesh(path: &str) -> Result<Mesh, String> {
    if let Some(data) = crate::assets::load_bytes(path) {
        return load_gltf_mesh_bytes(&data);
    }
    #[cfg(target_arch = "wasm32")]
    {
        return Err("GLTF import is not supported in web builds without a picked file".to_string());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let (document, buffers, _) =
            gltf::import(path).map_err(|err| format!("glTF load failed: {err}"))?;
        build_mesh_from_gltf(&document, &buffers)
    }
}

pub fn load_gltf_mesh_bytes(data: &[u8]) -> Result<Mesh, String> {
    let (document, buffers, _) =
        gltf::import_slice(data).map_err(|err| format!("glTF load failed: {err}"))?;
    build_mesh_from_gltf(&document, &buffers)
}

fn build_mesh_from_gltf(
    document: &gltf::Document,
    buffers: &[gltf::buffer::Data],
) -> Result<Mesh, String> {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut colors: Vec<[f32; 3]> = Vec::new();
    let mut material_values: Vec<String> = Vec::new();
    let mut material_lookup: HashMap<String, u32> = HashMap::new();
    let mut material_indices: Vec<u32> = Vec::new();
    let mut include_normals = true;
    let mut include_uvs = true;
    let mut include_colors = true;

    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            if primitive.mode() != gltf::mesh::Mode::Triangles {
                continue;
            }
            let reader =
                primitive.reader(|buffer| buffers.get(buffer.index()).map(|data| data.0.as_slice()));
            let prim_positions: Vec<[f32; 3]> = reader
                .read_positions()
                .ok_or_else(|| "glTF primitive missing POSITION attribute".to_string())?
                .collect();
            if prim_positions.is_empty() {
                continue;
            }
            let base = positions.len() as u32;
            positions.extend(prim_positions.iter().copied());

            if let Some(iter) = reader.read_normals() {
                normals.extend(iter);
            } else {
                include_normals = false;
            }

            if let Some(iter) = reader.read_tex_coords(0) {
                uvs.extend(iter.into_f32());
            } else {
                include_uvs = false;
            }

            if let Some(iter) = reader.read_colors(0) {
                colors.extend(iter.into_rgba_f32().map(|c| [c[0], c[1], c[2]]));
            } else {
                include_colors = false;
            }

            let prim_indices: Vec<u32> = match reader.read_indices() {
                Some(indices) => indices.into_u32().collect(),
                None => (0..prim_positions.len() as u32).collect(),
            };
            let tri_count = prim_indices.len() / 3;
            indices.extend(prim_indices.into_iter().map(|idx| idx + base));
            if tri_count > 0 {
                let material = primitive.material();
                let mut name = material
                    .name()
                    .map(|name| name.to_string())
                    .unwrap_or_else(|| {
                        if let Some(index) = material.index() {
                            format!("material_{index}")
                        } else {
                            "material_default".to_string()
                        }
                    });
                if name.trim().is_empty() {
                    name = "material_default".to_string();
                }
                let entry = material_lookup.entry(name.clone()).or_insert_with(|| {
                    let idx = material_values.len() as u32;
                    material_values.push(name);
                    idx
                });
                material_indices.extend(std::iter::repeat_n(*entry, tri_count));
            }
        }
    }

    if positions.is_empty() {
        return Err("glTF has no triangle geometry".to_string());
    }

    let mut mesh = Mesh::with_positions_indices(positions, indices);
    if include_normals && normals.len() == mesh.positions.len() {
        mesh.normals = Some(normals);
    }
    if include_uvs && uvs.len() == mesh.positions.len() {
        mesh.uvs = Some(uvs.clone());
        let _ = mesh.set_attribute(
            AttributeDomain::Point,
            "uv",
            AttributeStorage::Vec2(uvs),
        );
    }
    if include_colors && colors.len() == mesh.positions.len() {
        let _ = mesh.set_attribute(
            AttributeDomain::Point,
            "Cd",
            AttributeStorage::Vec3(colors),
        );
    }
    if !material_indices.is_empty() && material_indices.len() == mesh.indices.len() / 3 {
        let _ = mesh.set_attribute(
            AttributeDomain::Primitive,
            "material",
            AttributeStorage::StringTable(StringTableAttribute::new(
                material_values,
                material_indices,
            )),
        );
    }
    if mesh.normals.is_none() && mesh.corner_normals.is_none() {
        mesh.compute_normals();
    }
    Ok(mesh)
}

pub fn write_gltf(path: &str, mesh: &Mesh) -> Result<(), String> {
    let export = build_export_mesh(mesh)?;
    let (json, bin) = build_gltf_payload(&export, Path::new(path))?;
    let json_bytes = serde_json::to_vec(&json).map_err(|err| err.to_string())?;
    let extension = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();
    if extension == "gltf" {
        let bin_name = Path::new(path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(|stem| format!("{stem}.bin"))
            .unwrap_or_else(|| "buffer.bin".to_string());
        let bin_path = Path::new(path)
            .with_file_name(bin_name);
        std::fs::write(path, &json_bytes).map_err(|err| err.to_string())?;
        std::fs::write(bin_path, bin).map_err(|err| err.to_string())?;
        Ok(())
    } else {
        let glb = gltf::binary::Glb {
            header: gltf::binary::Header {
                magic: *b"glTF",
                version: 2,
                length: 0,
            },
            json: std::borrow::Cow::Owned(json_bytes),
            bin: Some(std::borrow::Cow::Owned(bin)),
        };
        let mut file = std::fs::File::create(path).map_err(|err| err.to_string())?;
        glb.to_writer(&mut file).map_err(|err| err.to_string())?;
        Ok(())
    }
}

struct ExportMesh {
    positions: Vec<[f32; 3]>,
    indices: Vec<u32>,
    normals: Option<Vec<[f32; 3]>>,
    uvs: Option<Vec<[f32; 2]>>,
    colors: Option<Vec<[f32; 3]>>,
}

fn build_export_mesh(mesh: &Mesh) -> Result<ExportMesh, String> {
    let point_uvs = point_uvs(mesh);
    let vertex_uvs = vertex_uvs(mesh);
    let point_colors = point_colors(mesh);
    let vertex_colors = vertex_colors(mesh);
    let corner_normals = mesh
        .corner_normals
        .as_ref()
        .filter(|normals| normals.len() == mesh.indices.len());
    let point_normals = mesh
        .normals
        .as_ref()
        .filter(|normals| normals.len() == mesh.positions.len());
    let needs_expand =
        corner_normals.is_some() || vertex_uvs.is_some() || vertex_colors.is_some();

    let (tri_indices, tri_corners) = if mesh.indices.is_empty() {
        if !mesh.positions.len().is_multiple_of(3) {
            return Err("Mesh has no indices and non-triangular vertex count".to_string());
        }
        let indices = (0..mesh.positions.len() as u32).collect::<Vec<_>>();
        let corners = (0..mesh.positions.len()).collect::<Vec<_>>();
        (indices, corners)
    } else {
        let triangulation = mesh.triangulate();
        if triangulation.indices.is_empty() {
            return Err("Mesh has no triangles to export".to_string());
        }
        (triangulation.indices, triangulation.corner_indices)
    };

    if needs_expand {
        let mut positions = Vec::with_capacity(tri_indices.len());
        let mut indices = Vec::with_capacity(tri_indices.len());
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut colors = Vec::new();

        for (corner_idx, &idx) in tri_indices.iter().enumerate() {
            let pos = mesh
                .positions
                .get(idx as usize)
                .copied()
                .unwrap_or([0.0, 0.0, 0.0]);
            positions.push(pos);
            let corner = *tri_corners.get(corner_idx).unwrap_or(&corner_idx);
            if let Some(corner_normals) = corner_normals {
                if let Some(normal) = corner_normals.get(corner) {
                    normals.push(*normal);
                }
            } else if let Some(point) = point_normals {
                if let Some(normal) = point.get(idx as usize) {
                    normals.push(*normal);
                }
            }
            if let Some(uvs_corner) = &vertex_uvs {
                if let Some(uv) = uvs_corner.get(corner) {
                    uvs.push(*uv);
                }
            } else if let Some(uvs_point) = &point_uvs {
                if let Some(uv) = uvs_point.get(idx as usize) {
                    uvs.push(*uv);
                }
            }
            if let Some(colors_corner) = &vertex_colors {
                if let Some(color) = colors_corner.get(corner) {
                    colors.push(*color);
                }
            } else if let Some(colors_point) = &point_colors {
                if let Some(color) = colors_point.get(idx as usize) {
                    colors.push(*color);
                }
            }
            indices.push(corner_idx as u32);
        }

        Ok(ExportMesh {
            positions,
            indices,
            normals: if normals.is_empty() { None } else { Some(normals) },
            uvs: if uvs.is_empty() { None } else { Some(uvs) },
            colors: if colors.is_empty() { None } else { Some(colors) },
        })
    } else {
        Ok(ExportMesh {
            positions: mesh.positions.clone(),
            indices: tri_indices,
            normals: point_normals.cloned(),
            uvs: point_uvs,
            colors: point_colors,
        })
    }
}

fn point_uvs(mesh: &Mesh) -> Option<Vec<[f32; 2]>> {
    if let Some(uvs) = &mesh.uvs {
        if uvs.len() == mesh.positions.len() {
            return Some(uvs.clone());
        }
    }
    if let Some(AttributeRef::Vec2(values)) = mesh.attribute(AttributeDomain::Point, "uv") {
        if values.len() == mesh.positions.len() {
            return Some(values.to_vec());
        }
    }
    None
}

fn vertex_uvs(mesh: &Mesh) -> Option<Vec<[f32; 2]>> {
    if let Some(AttributeRef::Vec2(values)) = mesh.attribute(AttributeDomain::Vertex, "uv") {
        if values.len() == mesh.indices.len() {
            return Some(values.to_vec());
        }
    }
    None
}

fn point_colors(mesh: &Mesh) -> Option<Vec<[f32; 3]>> {
    if let Some(AttributeRef::Vec3(values)) = mesh.attribute(AttributeDomain::Point, "Cd") {
        if values.len() == mesh.positions.len() {
            return Some(values.to_vec());
        }
    }
    None
}

fn vertex_colors(mesh: &Mesh) -> Option<Vec<[f32; 3]>> {
    if let Some(AttributeRef::Vec3(values)) = mesh.attribute(AttributeDomain::Vertex, "Cd") {
        if values.len() == mesh.indices.len() {
            return Some(values.to_vec());
        }
    }
    None
}

fn build_gltf_payload(
    mesh: &ExportMesh,
    path: &Path,
) -> Result<(serde_json::Value, Vec<u8>), String> {
    let mut buffer = Vec::new();
    let mut buffer_views = Vec::new();
    let mut accessors = Vec::new();
    let mut attributes = serde_json::Map::new();

    let pos_view = push_vec3(&mut buffer, &mut buffer_views, &mesh.positions, 34962);
    let (pos_min, pos_max) = min_max_vec3(&mesh.positions);
    let pos_accessor = push_accessor(
        &mut accessors,
        pos_view,
        5126,
        mesh.positions.len(),
        "VEC3",
        Some(pos_min),
        Some(pos_max),
    );
    attributes.insert("POSITION".to_string(), serde_json::json!(pos_accessor));

    if let Some(normals) = &mesh.normals {
        let normal_view = push_vec3(&mut buffer, &mut buffer_views, normals, 34962);
        let normal_accessor =
            push_accessor(&mut accessors, normal_view, 5126, normals.len(), "VEC3", None, None);
        attributes.insert("NORMAL".to_string(), serde_json::json!(normal_accessor));
    }
    if let Some(uvs) = &mesh.uvs {
        let uv_view = push_vec2(&mut buffer, &mut buffer_views, uvs, 34962);
        let uv_accessor =
            push_accessor(&mut accessors, uv_view, 5126, uvs.len(), "VEC2", None, None);
        attributes.insert("TEXCOORD_0".to_string(), serde_json::json!(uv_accessor));
    }
    if let Some(colors) = &mesh.colors {
        let color_view = push_vec3(&mut buffer, &mut buffer_views, colors, 34962);
        let color_accessor =
            push_accessor(&mut accessors, color_view, 5126, colors.len(), "VEC3", None, None);
        attributes.insert("COLOR_0".to_string(), serde_json::json!(color_accessor));
    }

    let (index_bytes, index_component_type) =
        encode_indices(&mesh.indices, mesh.positions.len())?;
    let index_view = push_bytes(&mut buffer, &mut buffer_views, &index_bytes, 34963);
    let index_accessor = push_accessor(
        &mut accessors,
        index_view,
        index_component_type,
        mesh.indices.len(),
        "SCALAR",
        None,
        None,
    );

    let bin_uri = if path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("gltf"))
        .unwrap_or(false)
    {
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(|stem| format!("{stem}.bin"))
            .unwrap_or_else(|| "buffer.bin".to_string());
        Some(name)
    } else {
        None
    };

    let buffer_obj = if let Some(uri) = bin_uri {
        serde_json::json!({
            "byteLength": buffer.len(),
            "uri": uri
        })
    } else {
        serde_json::json!({
            "byteLength": buffer.len()
        })
    };

    let gltf = serde_json::json!({
        "asset": {
            "version": "2.0",
            "generator": "Lobedo"
        },
        "scenes": [
            { "nodes": [0] }
        ],
        "scene": 0,
        "nodes": [
            { "mesh": 0 }
        ],
        "meshes": [
            {
                "primitives": [
                    {
                        "attributes": attributes,
                        "indices": index_accessor,
                        "mode": 4
                    }
                ]
            }
        ],
        "buffers": [buffer_obj],
        "bufferViews": buffer_views,
        "accessors": accessors
    });

    Ok((gltf, buffer))
}

fn push_vec3(
    buffer: &mut Vec<u8>,
    buffer_views: &mut Vec<serde_json::Value>,
    data: &[[f32; 3]],
    target: u32,
) -> usize {
    let mut flat = Vec::with_capacity(data.len() * 3);
    for item in data {
        flat.extend_from_slice(item);
    }
    push_f32(buffer, buffer_views, &flat, target)
}

fn push_vec2(
    buffer: &mut Vec<u8>,
    buffer_views: &mut Vec<serde_json::Value>,
    data: &[[f32; 2]],
    target: u32,
) -> usize {
    let mut flat = Vec::with_capacity(data.len() * 2);
    for item in data {
        flat.extend_from_slice(item);
    }
    push_f32(buffer, buffer_views, &flat, target)
}

fn push_f32(
    buffer: &mut Vec<u8>,
    buffer_views: &mut Vec<serde_json::Value>,
    data: &[f32],
    target: u32,
) -> usize {
    align_to_four(buffer);
    let offset = buffer.len();
    for value in data {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    let length = buffer.len() - offset;
    let view = serde_json::json!({
        "buffer": 0,
        "byteOffset": offset,
        "byteLength": length,
        "target": target
    });
    buffer_views.push(view);
    buffer_views.len() - 1
}

fn push_bytes(
    buffer: &mut Vec<u8>,
    buffer_views: &mut Vec<serde_json::Value>,
    data: &[u8],
    target: u32,
) -> usize {
    align_to_four(buffer);
    let offset = buffer.len();
    buffer.extend_from_slice(data);
    let length = buffer.len() - offset;
    let view = serde_json::json!({
        "buffer": 0,
        "byteOffset": offset,
        "byteLength": length,
        "target": target
    });
    buffer_views.push(view);
    buffer_views.len() - 1
}

fn push_accessor(
    accessors: &mut Vec<serde_json::Value>,
    view: usize,
    component_type: u32,
    count: usize,
    ty: &str,
    min: Option<Vec<f32>>,
    max: Option<Vec<f32>>,
) -> usize {
    let mut obj = serde_json::Map::new();
    obj.insert("bufferView".to_string(), serde_json::json!(view));
    obj.insert("componentType".to_string(), serde_json::json!(component_type));
    obj.insert("count".to_string(), serde_json::json!(count));
    obj.insert("type".to_string(), serde_json::json!(ty));
    if let Some(min) = min {
        obj.insert("min".to_string(), serde_json::json!(min));
    }
    if let Some(max) = max {
        obj.insert("max".to_string(), serde_json::json!(max));
    }
    accessors.push(serde_json::Value::Object(obj));
    accessors.len() - 1
}

fn encode_indices(indices: &[u32], vertex_count: usize) -> Result<(Vec<u8>, u32), String> {
    if vertex_count <= u16::MAX as usize {
        let mut bytes = Vec::with_capacity(indices.len() * 2);
        for &idx in indices {
            let idx = idx as u16;
            bytes.extend_from_slice(&idx.to_le_bytes());
        }
        Ok((bytes, 5123))
    } else {
        let mut bytes = Vec::with_capacity(indices.len() * 4);
        for &idx in indices {
            bytes.extend_from_slice(&idx.to_le_bytes());
        }
        Ok((bytes, 5125))
    }
}

fn min_max_vec3(data: &[[f32; 3]]) -> (Vec<f32>, Vec<f32>) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for value in data {
        for i in 0..3 {
            min[i] = min[i].min(value[i]);
            max[i] = max[i].max(value[i]);
        }
    }
    (min.to_vec(), max.to_vec())
}

fn align_to_four(buffer: &mut Vec<u8>) {
    let padding = (4 - (buffer.len() % 4)) % 4;
    buffer.extend(std::iter::repeat_n(0u8, padding));
}

