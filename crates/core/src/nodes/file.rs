use std::collections::BTreeMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::assets;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::geometry_out;
use crate::param_spec::ParamSpec;
use crate::gltf_io;

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

pub fn param_specs() -> Vec<ParamSpec> {
    vec![ParamSpec::string("path", "Path").with_help("Path or URL to an OBJ or glTF file.")]
}

pub fn compute(params: &NodeParams, _inputs: &[Mesh]) -> Result<Mesh, String> {
    let path = params.get_string("path", "");
    if path.trim().is_empty() {
        return Err("File node requires a path".to_string());
    }
    load_mesh(path)
}

fn load_mesh(path: &str) -> Result<Mesh, String> {
    if is_gltf_path(path) {
        return gltf_io::load_gltf_mesh(path);
    }
    if let Some(data) = assets::load_bytes(path) {
        if is_glb_bytes(&data) {
            return gltf_io::load_gltf_mesh_bytes(&data);
        }
        return load_obj_mesh_bytes(&data);
    }
    load_obj_mesh(path)
}

fn load_obj_mesh(path: &str) -> Result<Mesh, String> {
    if let Some(data) = assets::load_bytes(path) {
        return load_obj_mesh_bytes(&data);
    }
    #[cfg(target_arch = "wasm32")]
    {
        if assets::is_url(path) {
            return Err("File URL is downloading; retrying shortly.".to_string());
        }
        return Err("File node is not supported in web builds without a picked file".to_string());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
    if assets::is_url(path) {
        return Err(format!("Failed to download URL: {path}"));
    }
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

    build_mesh_from_models(models)
    }
}

fn load_obj_mesh_bytes(data: &[u8]) -> Result<Mesh, String> {
    use std::io::{BufReader, Cursor};

    let options = tobj::LoadOptions {
        triangulate: true,
        single_index: true,
        ..Default::default()
    };
    let mut reader = BufReader::new(Cursor::new(data));
    let (models, _) = tobj::load_obj_buf(&mut reader, &options, |_path| {
        Ok((Vec::new(), Default::default()))
    })
    .map_err(|err| format!("OBJ load failed: {err}"))?;
    build_mesh_from_models(models)
}

fn is_gltf_path(path: &str) -> bool {
    let name = if let Some(idx) = path.rfind("::") {
        &path[idx + 2..]
    } else {
        path
    };
    let ext = std::path::Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(ext.as_str(), "gltf" | "glb")
}

fn is_glb_bytes(data: &[u8]) -> bool {
    data.len() >= 4 && &data[0..4] == b"glTF"
}

fn build_mesh_from_models(models: Vec<tobj::Model>) -> Result<Mesh, String> {
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

