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
        BuiltinNodeKind::Image => Some(NodeHelpPage {
            name: "Image",
            description: &[
                "Loads an image from disk or URL and outputs it as an image stream.",
                "The image is decoded to RGB float data (0-1) for downstream ML nodes.",
                "Use this as the entry point for DepthPro and segmentation pipelines.",
            ],
            inputs: &[],
            outputs: &["image: Decoded RGB image."],
            parameters: &[("path", "Path or URL to a PNG/JPEG image.")],
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
        BuiltinNodeKind::WorldLabsGenerate => Some(NodeHelpPage {
            name: "WorldLabs Generate",
            description: &[
                "Generates a Gaussian splat model using the WorldLabs API.",
                "Mode selects between Generate (API) and Load (marble/ directory).",
                "Text/Image prompt settings apply when Generate is selected.",
                "Generation runs when the Generate button is pressed and saves to marble/.",
                "When SPZ variants are available, all resolutions (100k/500k/full) are cached.",
                "Set environment variable WLT_API_KEY (or WORLDLABS_API_KEY) with your API key.",
                "Outputs splats from PLY or SPZ assets when available.",
            ],
            inputs: &[],
            outputs: &["out: Generated splat geometry."],
            parameters: &[
                ("io_mode", "Mode: Generate or Load."),
                ("mode", "Prompt mode (Text or Image)."),
                ("text_prompt", "Text prompt (required for text mode)."),
                ("image_path", "Image file path or URL (image mode)."),
                ("auto_enhance", "Allow recaptioning to improve prompts."),
                ("is_pano", "Treat the image as a panorama."),
                ("model", "WorldLabs model selection."),
                ("seed", "Seed for deterministic output (-1 for random)."),
                ("tags", "Optional tags (comma or space separated)."),
                ("display_name", "Optional display name."),
                ("flip_y_load", "Flip Y after loading from marble/ (legacy files)."),
                ("load_model", "Model filename in marble/ when in Load mode."),
            ],
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
