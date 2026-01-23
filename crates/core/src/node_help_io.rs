use crate::nodes_builtin::BuiltinNodeKind;

use crate::node_help::NodeHelpPage;

pub fn node_help_page(kind: BuiltinNodeKind) -> Option<NodeHelpPage> {
    match kind {
        BuiltinNodeKind::File => Some(NodeHelpPage {
            name: "File",
            description: &[
                "Reads mesh geometry from OBJ or glTF/GLB files on disk or via URL.",
                "Positions, indices, normals, and UVs are imported when present.",
                "Materials are mapped into a primitive material attribute when available.",
            ],
            inputs: &[],
            outputs: &["out: Mesh geometry from file."],
            parameters: &[("path", "Path or URL to an OBJ or glTF/GLB file.")],
        }),
        BuiltinNodeKind::ObjOutput => Some(NodeHelpPage {
            name: "OBJ Output",
            description: &[
                "Exports mesh geometry to OBJ.",
                "Positions, normals, and vertex UVs are written when available.",
                "Writing is performed when the Write button is pressed.",
            ],
            inputs: &["in: Mesh geometry to write."],
            outputs: &["out: Pass-through geometry."],
            parameters: &[("path", "Output OBJ file path.")],
        }),
        BuiltinNodeKind::GltfOutput => Some(NodeHelpPage {
            name: "GLTF Output",
            description: &[
                "Exports mesh geometry to glTF/GLB.",
                "The exporter includes material parameters and UVs when present.",
                "Writing is performed when the Write button is pressed.",
            ],
            inputs: &["in: Mesh geometry to write."],
            outputs: &["out: Pass-through geometry."],
            parameters: &[("path", "Output glTF/GLB file path.")],
        }),
        BuiltinNodeKind::Output => Some(NodeHelpPage {
            name: "Output",
            description: &[
                "Marks the final output of a graph branch.",
                "The node simply passes geometry through, but it is used by the UI to choose display and export.",
                "Use it to make pipelines explicit and easier to read.",
            ],
            inputs: &["in: Geometry to output."],
            outputs: &["out: Pass-through geometry."],
            parameters: &[],
        }),
        _ => None,
    }
}

