use std::collections::BTreeMap;

use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{mesh_in, mesh_out, require_mesh_input};

pub const NAME: &str = "OBJ Output";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Outputs".to_string(),
        inputs: vec![mesh_in("in")],
        outputs: vec![mesh_out("out")],
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

    let has_uv = mesh
        .uvs
        .as_ref()
        .is_some_and(|uvs| uvs.len() == mesh.positions.len());
    if let Some(uvs) = &mesh.uvs {
        if has_uv {
            for uv in uvs {
                writeln!(file, "vt {} {}", uv[0], uv[1]).map_err(|err| err.to_string())?;
            }
        }
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
        for tri in mesh.indices.chunks_exact(3) {
            let a = tri[0] + 1;
            let b = tri[1] + 1;
            let c = tri[2] + 1;
            if has_uv && has_normals {
                writeln!(file, "f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c}")
                    .map_err(|err| err.to_string())?;
            } else if has_uv {
                writeln!(file, "f {a}/{a} {b}/{b} {c}/{c}").map_err(|err| err.to_string())?;
            } else if has_normals {
                writeln!(file, "f {a}//{a} {b}//{b} {c}//{c}")
                    .map_err(|err| err.to_string())?;
            } else {
                writeln!(file, "f {a} {b} {c}").map_err(|err| err.to_string())?;
            }
        }
    }
    Ok(())
}
