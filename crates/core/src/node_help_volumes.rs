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
        BuiltinNodeKind::VolumeFromSplats => Some(NodeHelpPage {
            name: "Volume from Splats",
            description: &[
                "Builds a volume from splat primitives using splat radii.",
                "SDF mode outputs signed distance; Density mode outputs a smoothed field.",
                "Fill mode flood-fills closed shells to avoid hollow volumes.",
            ],
            inputs: &["in: Geometry containing splats."],
            outputs: &["out: Volume (density or SDF)."],
            parameters: &[
                ("mode", "Volume type: Density or SDF."),
                ("fill_mode", "Shell keeps only the surface; Fill tags the interior."),
                ("shape", "Distance shape per splat: Ellipsoid or Sphere."),
                ("max_dim", "Largest voxel dimension (grid resolution)."),
                ("padding", "Padding around the bounds."),
                ("radius_mode", "How to derive radius from splat scales (Avg/Min/Max)."),
                ("radius_scale", "Scale factor applied to the splat radius."),
                ("min_radius", "Minimum radius clamp."),
                ("fill_shell", "Shell thickness in voxels for fill detection."),
                ("fill_normal_bias", "Expand fill shell where gradients are weak."),
                ("density_scale", "Density value inside the volume."),
                ("sdf_band", "SDF band width for rendering."),
                ("refine_steps", "Normal-based ellipsoid distance refinement steps."),
                ("support_sigma", "Scale splat radii to control support size."),
                ("ellipsoid_blend", "Blend between sphere and ellipsoid distance."),
                ("outlier_filter", "Enable neighbor-based outlier filtering."),
                ("outlier_radius", "Neighborhood radius for outlier detection."),
                ("outlier_min_neighbors", "Minimum neighbors required to keep a splat."),
                ("outlier_min_opacity", "Minimum log-opacity to include in filtering."),
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

