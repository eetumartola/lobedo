use std::collections::BTreeMap;

#[cfg(not(target_arch = "wasm32"))]
use crate::attributes::AttributeDomain;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{geometry_in, geometry_out, require_mesh_input};

pub const NAME: &str = "OBJ Output";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Outputs".to_string(),
        inputs: vec![geometry_in("in")],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([(
            "path".to_string(),
            ParamValue::String("output.obj".to_string()),
        )]),
    }
}

pub fn compute(_params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let input = require_mesh_input(inputs, 0, "OBJ Output requires a mesh input")?;
    Ok(input)
}

#[cfg(target_arch = "wasm32")]
pub fn write_obj(_path: &str, _mesh: &Mesh) -> Result<(), String> {
    Err("OBJ Output is not supported in web builds".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write_obj(path: &str, mesh: &Mesh) -> Result<(), String> {
    use std::io::{BufWriter, Write};

    let file = std::fs::File::create(path).map_err(|err| err.to_string())?;
    let mut file = BufWriter::new(file);
    for p in &mesh.positions {
        writeln!(file, "v {} {} {}", p[0], p[1], p[2]).map_err(|err| err.to_string())?;
    }

    enum UvMode {
        None,
        PerVertex(Vec<[f32; 2]>),
        PerCorner(Vec<[f32; 2]>),
    }

    let mut uv_mode = UvMode::None;
    if let Some(uvs) = &mesh.uvs {
        if uvs.len() == mesh.positions.len() {
            uv_mode = UvMode::PerVertex(uvs.clone());
        }
    }
    if matches!(uv_mode, UvMode::None) {
        if let Some(crate::attributes::AttributeRef::Vec2(values)) =
            mesh.attribute(AttributeDomain::Point, "uv")
        {
            if values.len() == mesh.positions.len() {
                uv_mode = UvMode::PerVertex(values.to_vec());
            }
        }
    }
    if matches!(uv_mode, UvMode::None) {
        if let Some(crate::attributes::AttributeRef::Vec2(values)) =
            mesh.attribute(AttributeDomain::Vertex, "uv")
        {
            if values.len() == mesh.indices.len() {
                uv_mode = UvMode::PerCorner(values.to_vec());
            }
        }
    }

    match &uv_mode {
        UvMode::PerVertex(uvs) => {
            for uv in uvs {
                writeln!(file, "vt {} {}", uv[0], uv[1]).map_err(|err| err.to_string())?;
            }
        }
        UvMode::PerCorner(uvs) => {
            for uv in uvs {
                writeln!(file, "vt {} {}", uv[0], uv[1]).map_err(|err| err.to_string())?;
            }
        }
        UvMode::None => {}
    }

    let has_normals = mesh
        .normals
        .as_ref()
        .is_some_and(|normals| normals.len() == mesh.positions.len());
    if let Some(normals) = &mesh.normals {
        if has_normals {
            for n in normals {
                writeln!(file, "vn {} {} {}", n[0], n[1], n[2]).map_err(|err| err.to_string())?;
            }
        }
    }

    if !mesh.indices.is_empty() {
        let face_counts = if mesh.face_counts.is_empty() {
            if mesh.indices.len().is_multiple_of(3) {
                vec![3u32; mesh.indices.len() / 3]
            } else {
                vec![mesh.indices.len() as u32]
            }
        } else {
            mesh.face_counts.clone()
        };
        let mut corner_uv_index = 1u32;
        let mut cursor = 0usize;
        for count in face_counts {
            let count = count as usize;
            if count == 0 || cursor + count > mesh.indices.len() {
                cursor = cursor.saturating_add(count);
                continue;
            }
            let face = &mesh.indices[cursor..cursor + count];
            match (&uv_mode, has_normals) {
                (UvMode::PerCorner(_), true) => {
                    let mut parts = Vec::with_capacity(count);
                    for (offset, idx) in face.iter().enumerate() {
                        let v = idx + 1;
                        let t = corner_uv_index + offset as u32;
                        parts.push(format!("{v}/{t}/{v}"));
                    }
                    writeln!(file, "f {}", parts.join(" ")).map_err(|err| err.to_string())?;
                    corner_uv_index += count as u32;
                }
                (UvMode::PerCorner(_), false) => {
                    let mut parts = Vec::with_capacity(count);
                    for (offset, idx) in face.iter().enumerate() {
                        let v = idx + 1;
                        let t = corner_uv_index + offset as u32;
                        parts.push(format!("{v}/{t}"));
                    }
                    writeln!(file, "f {}", parts.join(" ")).map_err(|err| err.to_string())?;
                    corner_uv_index += count as u32;
                }
                (UvMode::PerVertex(_), true) => {
                    let parts: Vec<String> = face
                        .iter()
                        .map(|idx| {
                            let v = idx + 1;
                            format!("{v}/{v}/{v}")
                        })
                        .collect();
                    writeln!(file, "f {}", parts.join(" ")).map_err(|err| err.to_string())?;
                }
                (UvMode::PerVertex(_), false) => {
                    let parts: Vec<String> = face
                        .iter()
                        .map(|idx| {
                            let v = idx + 1;
                            format!("{v}/{v}")
                        })
                        .collect();
                    writeln!(file, "f {}", parts.join(" ")).map_err(|err| err.to_string())?;
                }
                (UvMode::None, true) => {
                    let parts: Vec<String> = face
                        .iter()
                        .map(|idx| {
                            let v = idx + 1;
                            format!("{v}//{v}")
                        })
                        .collect();
                    writeln!(file, "f {}", parts.join(" ")).map_err(|err| err.to_string())?;
                }
                (UvMode::None, false) => {
                    let parts: Vec<String> = face
                        .iter()
                        .map(|idx| format!("{}", idx + 1))
                        .collect();
                    writeln!(file, "f {}", parts.join(" ")).map_err(|err| err.to_string())?;
                }
            }
            cursor += count;
        }
    }
    Ok(())
}

