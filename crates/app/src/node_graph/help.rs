use std::borrow::Cow;

use egui::{Color32, Context, FontId, Frame, Id, Margin, Order, Pos2, Rect, Vec2};

pub fn node_help(node_name: &str) -> Option<&'static str> {
    match node_name {
        "Box" => Some("Creates a box mesh."),
        "Grid" => Some("Creates a planar grid mesh."),
        "Sphere" => Some("Creates a UV sphere mesh."),
        "Tube" => Some("Creates a tube/cylinder mesh."),
        "File" => Some("Reads a mesh from an OBJ file."),
        "Splat Read" | "Read Splats" => Some("Reads Gaussian splats from a PLY file."),
        "Splat Write" => Some("Writes splats to a PLY file."),
        "Delete" => Some("Deletes geometry inside a selection shape."),
        "Splat Prune" | "Prune" => Some("Filters splats by opacity and scale ranges."),
        "Splat Regularize" | "Regularize" => Some("Clamps and normalizes splat parameters."),
        "Splat LOD" => Some("Reduces splat count via voxel clustering."),
        "Splat to Mesh" => Some("Converts splats to a triangle mesh."),
        "Splat Deform" => Some("Deforms splats from an edited point cloud."),
        "Group" => Some("Creates a named group by shape or existing group."),
        "Transform" => Some("Transforms geometry with translate/rotate/scale."),
        "Copy/Transform" => Some("Creates multiple copies with incremental transforms."),
        "Merge" => Some("Merges all input geometry."),
        "Copy to Points" => Some("Copies source geometry onto template points."),
        "Scatter" => Some("Scatters random points over a surface."),
        "Normal" => Some("Computes normals."),
        "Color" => Some("Sets a color attribute."),
        "Noise/Mountain" => Some("Displaces points along normals with noise."),
        "Smooth" => Some("Smooths attributes."),
        "UV Texture" => Some("Generates UVs using basic projections."),
        "UV Unwrap" => Some("Generates UVs by clustering faces and projecting."),
        "UV View" => Some("Displays UV wireframe for the incoming mesh."),
        "Material" => Some("Assigns a material with PBR parameters."),
        "Ray" => Some("Projects points onto geometry using raycasts."),
        "Attribute Noise" => Some("Writes noise into an attribute."),
        "Attribute from Feature" => Some("Computes area/gradient features into an attribute."),
        "Attribute Transfer" => Some("Transfers attributes between inputs."),
        "Attribute Math" => Some("Applies math operations to attributes."),
        "Wrangle" => Some("Runs a small script to edit attributes."),
        "OBJ Output" => Some("Writes mesh geometry to an OBJ file."),
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
        ("File", "path") => Some("Path to an OBJ file."),
        ("Splat Read", "path") => Some("Path to a splat PLY file."),
        ("Splat Read", "read_mode") => Some("Read full SH data or base color only."),
        ("Read Splats", "path") => Some("Path to a splat PLY file."),
        ("Read Splats", "read_mode") => Some("Read full SH data or base color only."),
        ("Splat Write", "path") => Some("Output PLY file path."),
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
        ("Group", "group") => Some("Name of the group to create."),
        ("Group", "base_group") => Some("Optional source group to filter first."),
        ("Group", "domain") => Some("Group domain (vertex/point/primitive)."),
        ("Group", "shape") => Some("Selection shape used to populate the group."),
        ("Group", "invert") => Some("Invert selection (keep outside)."),
        ("Group", "center") => Some("Shape center."),
        ("Group", "size") => Some("Box size in X/Y/Z."),
        ("Group", "radius") => Some("Sphere radius."),
        ("Group", "plane_origin") => Some("Plane origin for half-space selection."),
        ("Group", "plane_normal") => Some("Plane normal for half-space selection."),
        ("Transform", "translate") => Some("Translation in X/Y/Z."),
        ("Transform", "rotate_deg") => Some("Rotation in degrees (XYZ)."),
        ("Transform", "scale") => Some("Scale factors (XYZ)."),
        ("Transform", "pivot") => Some("Pivot point."),
        ("Transform", "group") => Some("Optional group to restrict transform."),
        ("Transform", "group_type") => Some("Group domain to use."),
        ("Copy/Transform", "count") => Some("Number of copies."),
        ("Copy/Transform", "translate_step") => Some("Per-copy translation step."),
        ("Copy/Transform", "rotate_step_deg") => Some("Per-copy rotation step (degrees)."),
        ("Copy/Transform", "scale_step") => Some("Per-copy scale step."),
        ("Copy to Points", "align_to_normals") => Some("Align copies to template normals."),
        ("Copy to Points", "translate") => Some("Translation applied to each copy."),
        ("Copy to Points", "rotate_deg") => Some("Rotation applied to each copy (degrees)."),
        ("Copy to Points", "scale") => Some("Scale applied to each copy."),
        ("Copy to Points", "group") => Some("Restrict to a template point group."),
        ("Copy to Points", "group_type") => Some("Group domain to use."),
        ("Scatter", "count") => Some("Number of points to scatter."),
        ("Scatter", "seed") => Some("Random seed."),
        ("Scatter", "group") => Some("Restrict scattering to a group."),
        ("Scatter", "group_type") => Some("Group domain to use."),
        ("Normal", "threshold_deg") => Some("Angle threshold for smoothing."),
        ("Normal", "group") => Some("Restrict normal recompute to a group."),
        ("Normal", "group_type") => Some("Group domain to use."),
        ("Color", "color") => Some("Color value (RGB)."),
        ("Color", "domain") => Some("Attribute domain to write."),
        ("Color", "group") => Some("Restrict to a group."),
        ("Color", "group_type") => Some("Group domain to use."),
        ("Noise/Mountain", "amplitude") => Some("Displacement strength."),
        ("Noise/Mountain", "frequency") => Some("Noise frequency."),
        ("Noise/Mountain", "seed") => Some("Noise seed."),
        ("Noise/Mountain", "offset") => Some("Noise space offset."),
        ("Noise/Mountain", "group") => Some("Restrict to a group."),
        ("Noise/Mountain", "group_type") => Some("Group domain to use."),
        ("Smooth", "attr") => Some("Attribute name(s) to smooth."),
        ("Smooth", "domain") => Some("Attribute domain to smooth."),
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
        _ => common_param_help(param),
    };
    help.map(Cow::Borrowed)
        .or_else(|| Some(Cow::Owned(format!("{} parameter.", param))))
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
