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

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let input = require_mesh_input(inputs, 0, "OBJ Output requires a mesh input")?;
    let path = params.get_string("path", "output.obj");
    if path.trim().is_empty() {
        return Err("OBJ Output requires a path".to_string());
    }
    write_obj(path, &input)?;
    Ok(input)
}

#[cfg(target_arch = "wasm32")]
fn write_obj(_path: &str, _mesh: &Mesh) -> Result<(), String> {
    Err("OBJ Output is not supported in web builds".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn write_obj(path: &str, mesh: &Mesh) -> Result<(), String> {
    use std::io::Write;

    let mut file = std::fs::File::create(path).map_err(|err| err.to_string())?;
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
        if let Some(attr) = mesh.attribute(AttributeDomain::Point, "uv") {
            if let crate::attributes::AttributeRef::Vec2(values) = attr {
                if values.len() == mesh.positions.len() {
                    uv_mode = UvMode::PerVertex(values.to_vec());
                }
            }
        }
    }
    if matches!(uv_mode, UvMode::None) {
        if let Some(attr) = mesh.attribute(AttributeDomain::Vertex, "uv") {
            if let crate::attributes::AttributeRef::Vec2(values) = attr {
                if values.len() == mesh.indices.len() {
                    uv_mode = UvMode::PerCorner(values.to_vec());
                }
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
        let mut corner_uv_index = 1u32;
        for tri in mesh.indices.chunks_exact(3) {
            let a = tri[0] + 1;
            let b = tri[1] + 1;
            let c = tri[2] + 1;
            match (&uv_mode, has_normals) {
                (UvMode::PerCorner(_), true) => {
                    let ta = corner_uv_index;
                    let tb = corner_uv_index + 1;
                    let tc = corner_uv_index + 2;
                    writeln!(file, "f {a}/{ta}/{a} {b}/{tb}/{b} {c}/{tc}/{c}")
                        .map_err(|err| err.to_string())?;
                    corner_uv_index += 3;
                }
                (UvMode::PerCorner(_), false) => {
                    let ta = corner_uv_index;
                    let tb = corner_uv_index + 1;
                    let tc = corner_uv_index + 2;
                    writeln!(file, "f {a}/{ta} {b}/{tb} {c}/{tc}")
                        .map_err(|err| err.to_string())?;
                    corner_uv_index += 3;
                }
                (UvMode::PerVertex(_), true) => {
                    writeln!(file, "f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c}")
                        .map_err(|err| err.to_string())?;
                }
                (UvMode::PerVertex(_), false) => {
                    writeln!(file, "f {a}/{a} {b}/{b} {c}/{c}")
                        .map_err(|err| err.to_string())?;
                }
                (UvMode::None, true) => {
                    writeln!(file, "f {a}//{a} {b}//{b} {c}//{c}")
                        .map_err(|err| err.to_string())?;
                }
                (UvMode::None, false) => {
                    writeln!(file, "f {a} {b} {c}").map_err(|err| err.to_string())?;
                }
            }
        }
    }
    Ok(())
}

