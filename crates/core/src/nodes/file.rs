use std::collections::BTreeMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::geometry_out;

pub const NAME: &str = "File";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Sources".to_string(),
        inputs: Vec::new(),
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([(
            "path".to_string(),
            ParamValue::String(r"C:\code\lobedo\geo\pig.obj".to_string()),
        )]),
    }
}

pub fn compute(params: &NodeParams, _inputs: &[Mesh]) -> Result<Mesh, String> {
    let path = params.get_string("path", "");
    if path.trim().is_empty() {
        return Err("File node requires a path".to_string());
    }
    load_obj_mesh(path)
}

#[cfg(target_arch = "wasm32")]
fn load_obj_mesh(_path: &str) -> Result<Mesh, String> {
    Err("File node is not supported in web builds".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn load_obj_mesh(path: &str) -> Result<Mesh, String> {
    let path = Path::new(path);
    if !path.exists() {
        return Err(format!("File not found: {}", path.display()));
    }

    let (models, _) = {
        let options = tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        };
        tobj::load_obj(path, &options).map_err(|err| format!("OBJ load failed: {err}"))?
    };

    if models.is_empty() {
        return Err("OBJ has no geometry".to_string());
    }

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut include_normals = true;
    let mut include_uvs = true;
    let mut vertex_offset = 0u32;

    for model in models {
        let mesh = &model.mesh;
        if mesh.positions.len() % 3 != 0 {
            return Err("OBJ has malformed positions".to_string());
        }
        let vertex_count = mesh.positions.len() / 3;

        positions.extend(mesh.positions.chunks_exact(3).map(|v| [v[0], v[1], v[2]]));
        indices.extend(mesh.indices.iter().map(|i| i + vertex_offset));
        vertex_offset += vertex_count as u32;

        if mesh.normals.len() == mesh.positions.len() {
            normals.extend(mesh.normals.chunks_exact(3).map(|n| [n[0], n[1], n[2]]));
        } else {
            include_normals = false;
        }

        if mesh.texcoords.len() / 2 == vertex_count {
            uvs.extend(mesh.texcoords.chunks_exact(2).map(|t| [t[0], t[1]]));
        } else {
            include_uvs = false;
        }
    }

    let mut mesh = Mesh::with_positions_indices(positions, indices);
    if include_normals && !normals.is_empty() {
        mesh.normals = Some(normals);
    }
    if include_uvs && !uvs.is_empty() {
        let corner_uvs: Vec<[f32; 2]> = mesh
            .indices
            .iter()
            .filter_map(|idx| uvs.get(*idx as usize).copied())
            .collect();
        if corner_uvs.len() == mesh.indices.len() {
            let _ = mesh.set_attribute(
                AttributeDomain::Vertex,
                "uv",
                AttributeStorage::Vec2(corner_uvs),
            );
        }
        mesh.uvs = Some(uvs);
    }

    if mesh.normals.is_none() && mesh.corner_normals.is_none() {
        mesh.compute_normals();
    }

    Ok(mesh)
}

