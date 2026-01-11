use std::borrow::Cow;

use egui::{
    Color32, Context, FontId, Frame, Id, Margin, Order, Pos2, Rect, RichText,
    ScrollArea, Vec2,
};

#[path = "help_pages.rs"]
mod help_pages;
pub use help_pages::node_help_page;

pub fn node_help(node_name: &str) -> Option<&'static str> {
    match node_name {
        "Box" => Some("Creates a box mesh."),
        "Grid" => Some("Creates a planar grid mesh."),
        "Sphere" => Some("Creates a UV sphere mesh."),
        "Tube" => Some("Creates a tube/cylinder mesh."),
        "Circle" => Some("Creates a circle as a curve or mesh."),
        "Curve" => Some("Creates a polyline curve from the supplied points."),
        "File" => Some("Reads a mesh from an OBJ or glTF file."),
        "Splat Read" | "Read Splats" => Some("Reads Gaussian splats from a PLY file."),
        "Splat Write" => Some("Writes splats to a PLY file using the Write button."),
        "Delete" => Some("Deletes geometry inside a selection shape."),
        "Splat Prune" | "Prune" => Some("Filters splats by opacity and scale ranges."),
        "Splat Regularize" | "Regularize" => Some("Clamps and normalizes splat parameters."),
        "Splat LOD" => Some("Reduces splat count via voxel clustering."),
        "Splat to Mesh" => Some("Converts splats to a triangle mesh or SDF volume."),
        "Splat Deform" => Some("Deforms splats from an edited point cloud."),
        "Splat Delight" => Some("Neutralizes baked lighting in splat SH coefficients."),
        "Splat Integrate" => Some("Matches splat lighting to a target environment."),
        "Splat Merge" => Some("Geometrically joins two splat models with feathering or skirts."),
        "Volume from Geometry" => Some("Converts geometry into a sparse volume."),
        "Volume Combine" => Some("Combines two volumes into one."),
        "Volume Blur" => Some("Blurs volume values in a voxel neighborhood."),
        "Volume to Mesh" => Some("Extracts a mesh surface from a volume."),
        "Group" => Some("Creates a named group by shape or viewport selection."),
        "Transform" => Some("Transforms geometry with translate/rotate/scale."),
        "FFD" => Some("Deforms geometry using a lattice of control points."),
        "Copy/Transform" => Some("Creates multiple copies with incremental transforms."),
        "Merge" => Some("Merges all input geometry."),
        "Copy to Points" => Some("Copies source geometry onto template points."),
        "Scatter" => Some("Scatters random points over surfaces, curves, or volumes."),
        "Sweep" => Some("Sweeps a profile curve or polygon along a path curve to form a surface."),
        "Normal" => Some("Computes normals."),
        "Color" => Some("Sets a color attribute."),
        "Noise/Mountain" => Some("Displaces points along normals with noise."),
        "Erosion Noise" => Some("Applies erosion-style noise to point height."),
        "Smooth" => Some("Smooths attributes."),
        "UV Texture" => Some("Generates UVs using basic projections."),
        "UV Unwrap" => Some("Generates UVs by clustering faces and projecting."),
        "UV View" => Some("Displays UV wireframe for the incoming mesh."),
        "Material" => Some("Assigns a material with PBR parameters."),
        "Ray" => Some("Projects points onto geometry using raycasts."),
        "Attribute Noise" => Some("Writes noise into an attribute."),
        "Attribute from Feature" => Some("Computes area/gradient features into an attribute."),
        "Attribute from Volume" => Some("Samples a volume into an attribute."),
        "Attribute Transfer" => Some("Transfers attributes between inputs."),
        "Attribute Math" => Some("Applies math operations to attributes."),
        "Wrangle" => Some("Runs a small script to edit attributes, with query helpers and volume sampling."),
        "OBJ Output" => Some("Writes mesh geometry to an OBJ file using the Write button."),
        "GLTF Output" => Some("Writes mesh geometry to a glTF/GLB file using the Write button."),
        "Output" => Some("Final output node; passes geometry through."),
        _ => None,
    }
}

pub fn param_help(node_name: &str, param: &str) -> Option<Cow<'static, str>> {  
    let help = match (node_name, param) {
        ("Box", "size") => Some("Box dimensions in X/Y/Z."),
        ("Box", "center") => Some("Box center in world space."),
        ("Grid", "size") => Some("Grid size in X/Z."),
        ("Grid", "rows") => Some("Rows (subdivisions) along Z."),
        ("Grid", "cols") => Some("Columns (subdivisions) along X."),
        ("Grid", "center") => Some("Grid center in world space."),
        ("Circle", "output") => Some("Output as a curve or a mesh."),
        ("Circle", "radius") => Some("Circle radius."),
        ("Circle", "segments") => Some("Number of segments."),
        ("Circle", "center") => Some("Circle center in world space."),
        ("Sphere", "radius") => Some("Sphere radius."),
        ("Sphere", "rows") => Some("Latitude segments."),
        ("Sphere", "cols") => Some("Longitude segments."),
        ("Sphere", "center") => Some("Sphere center in world space."),
        ("Tube", "radius") => Some("Tube radius."),
        ("Tube", "height") => Some("Tube height."),
        ("Tube", "rows") => Some("Height segments."),
        ("Tube", "cols") => Some("Side segments."),
        ("Tube", "capped") => Some("Add caps at the ends."),
        ("Tube", "center") => Some("Tube center in world space."),
        ("Curve", "points") => Some("Curve points encoded as x y z; x y z; ..."),
        ("Curve", "subdivs") => Some("Subdivisions per segment for smoothing."),
        ("Curve", "closed") => Some("Close the curve loop."),
        ("File", "path") => Some("Path to an OBJ or glTF file."),
        ("Splat Read", "path") => Some("Path to a splat PLY file."),
        ("Splat Read", "read_mode") => Some("Read full SH data or base color only."),
        ("Read Splats", "path") => Some("Path to a splat PLY file."),
        ("Read Splats", "read_mode") => Some("Read full SH data or base color only."),
        ("Splat Write", "path") => Some("Output PLY file path."),
        ("Splat Write", "format") => Some("PLY file format (binary is faster)."),
        ("Delete", "shape") => Some("Selection shape used for culling."),
        ("Delete", "invert") => Some("Invert selection (keep inside)."),
        ("Delete", "center") => Some("Shape center."),
        ("Delete", "size") => Some("Box size in X/Y/Z."),
        ("Delete", "radius") => Some("Sphere radius."),
        ("Delete", "plane_origin") => Some("Plane origin for half-space delete."),
        ("Delete", "plane_normal") => Some("Plane normal for half-space delete."),
        ("Delete", "group") => Some("Optional group to restrict deletion."),
        ("Delete", "group_type") => Some("Group domain to use."),
        ("Splat Prune", "min_opacity") => Some("Minimum log-opacity to keep."),
        ("Splat Prune", "max_opacity") => Some("Maximum log-opacity to keep."),
        ("Splat Prune", "min_scale") => Some("Minimum log-scale to keep."),
        ("Splat Prune", "max_scale") => Some("Maximum log-scale to keep."),
        ("Splat Prune", "remove_invalid") => Some("Drop splats with NaN/Inf."),
        ("Splat Regularize", "min_scale") => Some("Minimum log-scale to keep."),
        ("Splat Regularize", "max_scale") => Some("Maximum log-scale to keep."),
        ("Splat Regularize", "normalize_opacity") => {
            Some("Renormalize opacity to a stable range.")
        }
        ("Splat Regularize", "normalize_rotation") => Some("Normalize/repair rotations."),
        ("Splat Regularize", "remove_invalid") => Some("Drop splats with NaN/Inf."),
        ("Splat LOD", "voxel_size") => Some("Voxel size for clustering."),
        ("Splat LOD", "target_count") => Some("Optional target count (0 = disabled)."),
        ("Splat to Mesh", "output") => Some("Output type (mesh or SDF volume)."),
        ("Splat to Mesh", "algorithm") => Some("Conversion method."),
        ("Splat to Mesh", "voxel_size") => Some("Voxel size for density grid."),
        ("Splat to Mesh", "voxel_size_max") => Some("Max voxel dimension (safety clamp)."),
        ("Splat to Mesh", "n_sigma") => Some("Gaussian support radius in sigmas."),
        ("Splat to Mesh", "density_iso") => Some("Density threshold for marching cubes."),
        ("Splat to Mesh", "surface_iso") => Some("Surface threshold for ellipsoid method."),
        ("Splat to Mesh", "bounds_padding") => Some("Padding around bounds in sigmas."),
        ("Splat to Mesh", "transfer_color") => Some("Transfer splat color to mesh Cd."),
        ("Splat to Mesh", "max_m2") => Some("Exponent clamp for ellipsoid blend."),
        ("Splat to Mesh", "smooth_k") => Some("Smooth-min blend sharpness."),
        ("Splat to Mesh", "shell_radius") => Some("Shell thickness for ellipsoid."),
        ("Splat to Mesh", "blur_iters") => Some("Density blur iterations."),
        ("Splat Deform", "allow_new") => Some("Allow creation of new splats."),
        ("Splat Deform", "derive_rot_scale") => Some("Infer rotation/scale from deformation."),
        ("Splat Delight", "delight_mode") => Some("Delighting method to apply."),
        ("Splat Delight", "source_env") => Some("Source lighting estimate used for ratios or irradiance."),
        ("Splat Delight", "neutral_env") => Some("Target neutral lighting for ratio-based delighting."),
        ("Splat Delight", "source_color") => Some("Custom source lighting color (DC)."),
        ("Splat Delight", "neutral_color") => Some("Custom neutral lighting color (DC)."),
        ("Splat Delight", "eps") => Some("Stability epsilon scale for ratios/division."),
        ("Splat Delight", "ratio_min") => Some("Minimum ratio clamp for SH transfer."),
        ("Splat Delight", "ratio_max") => Some("Maximum ratio clamp for SH transfer."),
        ("Splat Delight", "high_band_gain") => Some("Gain applied to higher SH bands."),
        ("Splat Delight", "output_sh_order") => Some("Zero SH bands above this order."),
        ("Splat Delight", "albedo_max") => Some("Maximum albedo clamp for irradiance divide."),
        ("Splat Delight", "group") => Some("Optional group to restrict delighting."),
        ("Splat Delight", "group_type") => Some("Group domain to use."),
        ("Splat Integrate", "relight_mode") => Some("Relighting method to apply."),
        ("Splat Integrate", "source_env") => Some("Source lighting estimate for SH ratios."),
        ("Splat Integrate", "target_env") => Some("Target lighting estimate to match."),
        ("Splat Integrate", "source_color") => Some("Custom source lighting color (DC)."),
        ("Splat Integrate", "target_color") => Some("Custom target lighting color (DC)."),
        ("Splat Integrate", "eps") => Some("Stability epsilon scale for ratios."),
        ("Splat Integrate", "ratio_min") => Some("Minimum ratio clamp for SH transfer."),
        ("Splat Integrate", "ratio_max") => Some("Maximum ratio clamp for SH transfer."),
        ("Splat Integrate", "high_band_gain") => Some("Gain applied to higher SH bands."),
        ("Splat Integrate", "high_band_mode") => Some("Apply ratios to higher bands in hybrid mode."),
        ("Splat Integrate", "output_sh_order") => Some("Zero SH bands above this order."),
        ("Splat Integrate", "albedo_max") => Some("Maximum albedo clamp for diffuse relighting."),
        ("Splat Integrate", "group") => Some("Optional group to restrict integration."),
        ("Splat Integrate", "group_type") => Some("Group domain to use."),
        ("Splat Merge", "method") => Some("Join method."),
        ("Splat Merge", "blend_radius") => Some("Blend radius for feathering/fade."),
        ("Splat Merge", "fade_originals") => Some("Fade original splats near the seam."),
        ("Splat Merge", "skirt_max_dist") => Some("Maximum distance to bridge with skirt splats."),
        ("Splat Merge", "skirt_step") => Some("Spacing between skirt splats."),
        ("Splat Merge", "skirt_max_new") => Some("Maximum skirt splats per pair."),
        ("Splat Merge", "seam_alpha") => Some("Opacity for seam splats."),
        ("Splat Merge", "seam_scale") => Some("Scale multiplier for seam splats."),
        ("Splat Merge", "seam_dc_only") => Some("Use DC-only SH for seam splats."),
        ("Volume from Geometry", "mode") => Some("Volume type to generate (density or SDF)."),
        ("Volume from Geometry", "max_dim") => Some("Largest voxel dimension (grid resolution)."),
        ("Volume from Geometry", "padding") => Some("Padding around the bounds."),
        ("Volume from Geometry", "density_scale") => Some("Density value inside the volume."),
        ("Volume from Geometry", "sdf_band") => Some("SDF band width for rendering."),
        ("Volume Combine", "op") => Some("How to combine the two volumes."),
        ("Volume Combine", "resolution") => Some("Output resolution (lower/higher/average)."),
        ("Volume Blur", "radius") => Some("Blur radius in world units."),
        ("Volume Blur", "iterations") => Some("Number of blur passes."),
        ("Volume to Mesh", "mode") => Some("Treat input volume as density or SDF."),
        ("Volume to Mesh", "density_iso") => Some("Isovalue for density surfaces."),
        ("Volume to Mesh", "surface_iso") => Some("Isovalue for SDF surfaces."),
        ("Group", "group") => Some("Name of the group to create."),
        ("Group", "base_group") => Some("Optional source group to filter first."),
        ("Group", "domain") => Some("Group domain (vertex/point/primitive)."),
        ("Group", "shape") => Some("Selection shape (box/sphere/plane/selection/attribute)."),
        ("Group", "select_backface") => Some("Allow selecting back-facing elements."),
        ("Group", "invert") => Some("Invert selection (keep outside)."),
        ("Group", "center") => Some("Shape center."),
        ("Group", "size") => Some("Box size in X/Y/Z."),
        ("Group", "radius") => Some("Sphere radius."),
        ("Group", "plane_origin") => Some("Plane origin for half-space selection."),
        ("Group", "plane_normal") => Some("Plane normal for half-space selection."),
        ("Group", "attr") => Some("Attribute name for attribute-range selection."),
        ("Group", "attr_min") => Some("Minimum attribute value to include."),
        ("Group", "attr_max") => Some("Maximum attribute value to include."),
        ("Transform", "translate") => Some("Translation in X/Y/Z."),
        ("Transform", "rotate_deg") => Some("Rotation in degrees (XYZ)."),
        ("Transform", "scale") => Some("Scale factors (XYZ)."),
        ("Transform", "pivot") => Some("Pivot point."),
        ("Transform", "group") => Some("Optional group to restrict transform."),
        ("Transform", "group_type") => Some("Group domain to use."),
        ("FFD", "res_x") => Some("Control points along X (lattice resolution)."),
        ("FFD", "res_y") => Some("Control points along Y (lattice resolution)."),
        ("FFD", "res_z") => Some("Control points along Z (lattice resolution)."),
        ("FFD", "use_input_bounds") => Some("Use input geometry bounds for the lattice."),
        ("FFD", "center") => Some("Lattice center when not using input bounds."),
        ("FFD", "size") => Some("Lattice size when not using input bounds."),
        ("FFD", "padding") => Some("Expand lattice bounds by this amount."),
        ("FFD", "extrapolate") => Some("Allow extrapolation outside the lattice."),
        ("Copy/Transform", "count") => Some("Number of copies."),
        ("Copy/Transform", "translate") => Some("Base translation for the first copy."),
        ("Copy/Transform", "rotate_deg") => Some("Base rotation for the first copy (degrees)."),
        ("Copy/Transform", "scale") => Some("Base scale for the first copy."),
        ("Copy/Transform", "pivot") => Some("Pivot point for the base transform."),
        ("Copy/Transform", "translate_step") => Some("Per-copy translation step."),
        ("Copy/Transform", "rotate_step_deg") => Some("Per-copy rotation step (degrees)."),
        ("Copy/Transform", "scale_step") => Some("Per-copy scale step."),
        ("Copy to Points", "align_to_normals") => Some("Align copies to template normals."),
        ("Copy to Points", "translate") => Some("Translation applied to each copy."),
        ("Copy to Points", "rotate_deg") => Some("Rotation applied to each copy (degrees)."),
        ("Copy to Points", "scale") => Some("Scale applied to each copy."),
        ("Copy to Points", "inherit") => Some("Template point attributes to copy onto each instance."),
        ("Copy to Points", "copy_attr") => Some("Name of the per-copy index attribute."),
        ("Copy to Points", "copy_attr_class") => Some("Attribute class for the per-copy index."),
        ("Copy to Points", "group") => Some("Restrict to a template point group."),
        ("Copy to Points", "group_type") => Some("Group domain to use."),
        ("Scatter", "count") => Some("Number of points to scatter."),
        ("Scatter", "seed") => Some("Random seed."),
        ("Scatter", "density_attr") => Some("Optional density attribute for weighting."),
        ("Scatter", "density_min") => Some("Minimum mapped density value (0->min)."),
        ("Scatter", "density_max") => Some("Maximum mapped density value (1->max)."),
        ("Scatter", "inherit") => Some("Attributes to inherit from the source."),
        ("Scatter", "group") => Some("Restrict scattering to a group."),
        ("Scatter", "group_type") => Some("Group domain to use."),
        ("Sweep", "profile_closed") => Some("Close the profile when no curve primitive is supplied."),
        ("Sweep", "path_closed") => Some("Close the path when no curve primitive is supplied."),
        ("Sweep", "up") => Some("Up vector used to orient the swept profile."),
        ("Normal", "threshold_deg") => Some("Angle threshold for smoothing."),
        ("Normal", "group") => Some("Restrict normal recompute to a group."),
        ("Normal", "group_type") => Some("Group domain to use."),
        ("Color", "color_mode") => Some("Constant color or color from attribute."),
        ("Color", "color") => Some("Color value (RGB)."),
        ("Color", "attr") => Some("Attribute to map into the gradient."),
        ("Color", "gradient") => Some("Gradient stops like 0:#000000;1:#ffffff."),
        ("Color", "domain") => Some("Attribute domain to write."),
        ("Color", "group") => Some("Restrict to a group."),
        ("Color", "group_type") => Some("Group domain to use."),
        ("Noise/Mountain", "amplitude") => Some("Displacement strength."),
        ("Noise/Mountain", "frequency") => Some("Noise frequency."),
        ("Noise/Mountain", "seed") => Some("Noise seed."),
        ("Noise/Mountain", "offset") => Some("Noise space offset."),
        ("Noise/Mountain", "group") => Some("Restrict to a group."),
        ("Noise/Mountain", "group_type") => Some("Group domain to use."),
        ("Erosion Noise", "erosion_strength") => Some("Height offset strength."),
        ("Erosion Noise", "erosion_freq") => Some("Erosion pattern frequency."),
        ("Erosion Noise", "erosion_octaves") => Some("Number of erosion octaves."),
        ("Erosion Noise", "erosion_roughness") => Some("Amplitude falloff per octave."),
        ("Erosion Noise", "erosion_lacunarity") => Some("Frequency growth per octave."),
        ("Erosion Noise", "erosion_slope_strength") => Some("Slope influence on flow."),
        ("Erosion Noise", "erosion_branch_strength") => Some("Branching influence on flow."),
        ("Erosion Noise", "do_mask") => Some("Write erosion mask to @mask."),
        ("Erosion Noise", "group") => Some("Restrict to a group."),
        ("Erosion Noise", "group_type") => Some("Group domain to use."),
        ("Smooth", "attr") => Some("Attribute name(s) to smooth."),
        ("Smooth", "domain") => Some("Attribute domain to smooth."),
        ("Smooth", "smooth_space") => Some("World-space or surface-distance smoothing."),
        ("Smooth", "radius") => Some("Neighbor radius (0 = auto/1-ring)."),
        ("Smooth", "iterations") => Some("Number of smoothing passes."),
        ("Smooth", "strength") => Some("Blend strength per pass."),
        ("Smooth", "group") => Some("Restrict to a group."),
        ("Smooth", "group_type") => Some("Group domain to use."),
        ("UV Texture", "projection") => Some("Projection type."),
        ("UV Texture", "axis") => Some("Primary projection axis."),
        ("UV Texture", "scale") => Some("UV scale."),
        ("UV Texture", "offset") => Some("UV offset."),
        ("UV Unwrap", "padding") => Some("Island padding in UV space."),
        ("UV Unwrap", "normal_threshold") => Some("Angle threshold for island splits."),
        ("Material", "name") => Some("Material name."),
        ("Material", "base_color") => Some("Base color (albedo)."),
        ("Material", "base_color_tex") => Some("Texture path for base color."),
        ("Material", "metallic") => Some("Metallic factor."),
        ("Material", "roughness") => Some("Roughness factor."),
        ("Ray", "method") => Some("Ray direction mode."),
        ("Ray", "direction") => Some("Ray direction (Direction mode)."),
        ("Ray", "max_distance") => Some("Max ray distance."),
        ("Ray", "apply_transform") => Some("Move points to hit location."),
        ("Ray", "attr") => Some("Attribute(s) to import from the hit."),
        ("Ray", "hit_group") => Some("Group name to mark hits."),
        ("Ray", "group") => Some("Restrict source points to a group."),
        ("Ray", "group_type") => Some("Group domain to use."),
        ("Attribute Noise", "attr") => Some("Attribute name to write."),
        ("Attribute Noise", "domain") => Some("Attribute domain to write."),
        ("Attribute Noise", "data_type") => Some("Attribute data type."),
        ("Attribute Noise", "noise_type") => Some("Noise basis."),
        ("Attribute Noise", "amplitude") => Some("Noise amplitude."),
        ("Attribute Noise", "frequency") => Some("Noise frequency."),
        ("Attribute Noise", "offset") => Some("Noise space offset."),
        ("Attribute Noise", "seed") => Some("Noise seed."),
        ("Attribute Noise", "group") => Some("Restrict to a group."),
        ("Attribute Noise", "group_type") => Some("Group domain to use."),
        ("Attribute from Feature", "feature") => Some("Feature to compute."),
        ("Attribute from Feature", "attr") => Some("Destination attribute (empty = default)."),
        ("Attribute from Feature", "domain") => Some("Attribute domain to write."),
        ("Attribute from Feature", "group") => Some("Restrict to a group."),
        ("Attribute from Feature", "group_type") => Some("Group domain to use."),
        ("Attribute from Volume", "attr") => Some("Attribute name (empty = volume)."),
        ("Attribute from Volume", "domain") => Some("Attribute domain to write."),
        ("Attribute from Volume", "group") => Some("Restrict to a group."),
        ("Attribute from Volume", "group_type") => Some("Group domain to use."),
        ("Attribute Transfer", "attr") => Some("Space-delimited list of attributes."),
        ("Attribute Transfer", "domain") => Some("Attribute domain to transfer."),
        ("Attribute Transfer", "group") => Some("Restrict to a group."),
        ("Attribute Transfer", "group_type") => Some("Group domain to use."),
        ("Attribute Math", "attr") => Some("Source attribute."),
        ("Attribute Math", "result") => Some("Destination attribute."),
        ("Attribute Math", "domain") => Some("Attribute domain to operate on."),
        ("Attribute Math", "op") => Some("Math operation."),
        ("Attribute Math", "value_f") => Some("Scalar operand."),
        ("Attribute Math", "value_v3") => Some("Vector operand."),
        ("Attribute Math", "group") => Some("Restrict to a group."),
        ("Attribute Math", "group_type") => Some("Group domain to use."),
        ("Wrangle", "mode") => Some("Domain to iterate over."),
        ("Wrangle", "code") => Some("Wrangle code snippet."),
        ("Wrangle", "group") => Some("Restrict to a group."),
        ("Wrangle", "group_type") => Some("Group domain to use."),
        ("OBJ Output", "path") => Some("Output OBJ file path."),
        ("GLTF Output", "path") => Some("Output glTF/GLB file path."),
        _ => common_param_help(param),
    };
    help.map(Cow::Borrowed)
        .or_else(|| Some(Cow::Owned(format!("{} parameter.", param))))
}

pub fn show_help_page_window(ctx: &Context, node_name: &str, open: &mut bool) {
    let Some(page) = node_help_page(node_name) else {
        return;
    };

    egui::Window::new(format!("Help - {}", page.name))
        .open(open)
        .collapsible(false)
        .resizable(true)
        .min_width(420.0)
        .show(ctx, |ui| {
            ui.heading(page.name);
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(6.0);

            show_text_section(ui, "Description", page.description);
            show_list_section(ui, "Inputs", page.inputs);
            show_list_section(ui, "Outputs", page.outputs);
            show_param_section(ui, "Parameters", page.parameters);
        });
}

fn show_section_title(ui: &mut egui::Ui, title: &str) {
    ui.label(RichText::new(title).strong());
}

fn show_text_section(ui: &mut egui::Ui, title: &str, lines: &[&str]) {
    show_section_title(ui, title);
    ui.add_space(4.0);
    let mut has_any = false;
    for line in lines {
        has_any = true;
        ui.label(*line);
        ui.add_space(4.0);
    }
    if !has_any {
        ui.label("None.");
        ui.add_space(4.0);
    }
    ui.add_space(6.0);
}

fn show_list_section(ui: &mut egui::Ui, title: &str, items: &[&str]) {
    show_section_title(ui, title);
    ui.add_space(4.0);
    if items.is_empty() {
        ui.label("None.");
        ui.add_space(4.0);
    } else {
        for item in items {
            ui.label(format!("• {}", item));
        }
        ui.add_space(4.0);
    }
    ui.add_space(6.0);
}

fn show_param_section(ui: &mut egui::Ui, title: &str, params: &[(&str, &str)]) {
    show_section_title(ui, title);
    ui.add_space(4.0);
    if params.is_empty() {
        ui.label("None.");
        ui.add_space(4.0);
    } else {
        ScrollArea::vertical()
            .max_height(240.0)
            .show(ui, |ui| {
                for (name, desc) in params {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(*name).strong());
                        ui.label(*desc);
                    });
                    ui.add_space(2.0);
                }
            });
    }
}

fn common_param_help(param: &str) -> Option<&'static str> {
    match param {
        "group" => Some("Group name to restrict this operation (empty = all)."),
        "group_type" => Some("Group domain to use (Auto/Vertex/Point/Primitive)."),
        "domain" => Some("Attribute domain to read/write."),
        "attr" => Some("Attribute name(s) to operate on."),
        "result" => Some("Destination attribute name."),
        "data_type" => Some("Attribute type (float/vec2/vec3)."),
        "noise_type" => Some("Noise basis."),
        "feature" => Some("Feature to compute."),
        "method" => Some("Ray direction mode."),
        "projection" => Some("Projection type."),
        "axis" => Some("Primary axis."),
        "shape" => Some("Selection shape."),
        "invert" => Some("Invert selection."),
        "center" => Some("Shape center."),
        "size" => Some("Shape size in X/Y/Z."),
        "radius" => Some("Sphere radius."),
        "plane_origin" => Some("Plane origin."),
        "plane_normal" => Some("Plane normal."),
        "rows" => Some("Row count."),
        "cols" => Some("Column count."),
        "count" => Some("Number of items."),
        "seed" => Some("Random seed."),
        "translate" => Some("Translation in X/Y/Z."),
        "rotate_deg" => Some("Rotation in degrees (XYZ)."),
        "scale" => Some("Scale factors (XYZ)."),
        "pivot" => Some("Pivot point for transforms."),
        "offset" => Some("Offset value."),
        "amplitude" => Some("Amplitude."),
        "frequency" => Some("Frequency."),
        "threshold_deg" => Some("Angle threshold in degrees."),
        "iterations" => Some("Number of iterations."),
        "strength" => Some("Blend strength."),
        "max_distance" => Some("Maximum distance."),
        "apply_transform" => Some("Apply the transform to points."),
        "direction" => Some("Direction vector."),
        "hit_group" => Some("Group name to mark hits."),
        "path" => Some("File path."),
        _ => None,
    }
}

pub fn show_help_tooltip(ctx: &Context, anchor: Rect, text: &str) {
    let mut pos = ctx
        .pointer_hover_pos()
        .unwrap_or_else(|| Pos2::new(anchor.max.x, anchor.max.y));
    pos += Vec2::new(12.0, 16.0);

    let font = FontId::proportional(13.0);
    let galley =
        ctx.fonts_mut(|f| f.layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE));
    let padding = Vec2::new(10.0, 6.0);
    let size = galley.size() + padding * 2.0;
    let screen = ctx.content_rect();
    if pos.x + size.x > screen.max.x {
        pos.x = (screen.max.x - size.x - 6.0).max(screen.min.x + 6.0);
    }
    if pos.y + size.y > screen.max.y {
        pos.y = (screen.max.y - size.y - 6.0).max(screen.min.y + 6.0);
    }

    egui::Area::new(Id::new("node_help_tooltip"))
        .order(Order::Foreground)
        .fixed_pos(pos)
        .interactable(false)
        .show(ctx, |ui| {
            let frame = Frame::NONE
                .fill(Color32::from_rgb(30, 30, 30))
                .inner_margin(Margin::symmetric(8, 6));
            frame.show(ui, |ui| {
                ui.set_min_width(size.x);
                ui.label(egui::RichText::new(text).font(font).color(Color32::WHITE));
            });
        });

}
