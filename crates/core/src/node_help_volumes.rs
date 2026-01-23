use crate::nodes_builtin::BuiltinNodeKind;

use crate::node_help::NodeHelpPage;

pub fn node_help_page(kind: BuiltinNodeKind) -> Option<NodeHelpPage> {
    match kind {
        BuiltinNodeKind::VolumeFromGeometry => Some(NodeHelpPage {
            name: "Volume from Geometry",
            description: &[
                "Voxelizes geometry into a regular grid volume.",
                "Density mode fills the interior with a constant density and smooth antialiasing at the boundary.",
                "SDF mode computes signed distance; grid resolution is derived from the max dimension parameter.",
            ],
            inputs: &["in: Geometry to voxelize."],
            outputs: &["out: Volume (density or SDF)."],
            parameters: &[
                ("mode", "Volume type: Density or SDF."),
                ("max_dim", "Largest voxel dimension (grid resolution)."),
                ("padding", "Padding around the bounds."),
                ("density_scale", "Density value inside the volume."),
                ("sdf_band", "SDF band width for rendering."),
            ],
        }),
        BuiltinNodeKind::VolumeCombine => Some(NodeHelpPage {
            name: "Volume Combine",
            description: &[
                "Combines two volumes into a single volume by resampling them onto a shared grid.",
                "Operators include add, subtract, multiply, min, max, and average for CSG-like workflows.",
                "Resolution mode picks whether the output grid matches the lower, higher, or average input resolution.",
            ],
            inputs: &["a: First volume.", "b: Second volume."],
            outputs: &["out: Combined volume."],
            parameters: &[
                ("op", "Operator: Add, Subtract, Multiply, Min, Max, or Average."),
                ("resolution", "Resolution: Lower, Higher, or Average."),
            ],
        }),
        BuiltinNodeKind::VolumeBlur => Some(NodeHelpPage {
            name: "Volume Blur",
            description: &[
                "Blurs volume values over a radius in world space.",
                "Multiple iterations approximate a smoother kernel for softer results.",
                "Use it to smooth density fields before meshing or compositing.",
            ],
            inputs: &["in: Volume to blur."],
            outputs: &["out: Blurred volume."],
            parameters: &[
                ("radius", "Blur radius in world units."),
                ("iterations", "Number of blur passes."),
            ],
        }),
        BuiltinNodeKind::VolumeToMesh => Some(NodeHelpPage {
            name: "Volume to Mesh",
            description: &[
                "Extracts a surface from a volume using marching cubes.",
                "Density mode uses a density isovalue, while SDF mode uses an isosurface at 0.0.",
                "Use it to convert volume results back into polygon meshes.",
            ],
            inputs: &["in: Volume to convert."],
            outputs: &["out: Polygon mesh."],
            parameters: &[
                ("mode", "Mode: Density or SDF."),
                ("density_iso", "Isovalue for density surfaces."),
                ("surface_iso", "Isovalue for SDF surfaces."),
            ],
        }),
        _ => None,
    }
}

