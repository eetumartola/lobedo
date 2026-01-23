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
        "Boolean SDF" => Some("Combines meshes using SDF-based boolean operations."),
        "Boolean Geo" => Some("Combines meshes using polygon booleans or SDF clipping."),
        "File" => Some("Reads a mesh from an OBJ or glTF file or URL."),
        "Splat Read" | "Read Splats" => Some("Reads Gaussian splats from a PLY file or URL."),
        "Splat Write" => Some("Writes splats to a PLY file using the Write button."),
        "Delete" => Some("Deletes geometry inside a selection shape."),
        "Splat Prune" | "Prune" => Some("Filters splats by opacity and scale ranges."),
        "Splat Regularize" | "Regularize" => Some("Clamps and normalizes splat parameters."),
        "Splat LOD" => Some("Reduces splat count via voxel clustering."),
        "Splat to Mesh" => Some("Converts splats to a triangle mesh or SDF volume."),
        "Splat Deform" => Some("Deforms splats from an edited point cloud."),
        "Splat Delight" => Some("Neutralizes baked lighting in splat SH coefficients."),
        "Splat Integrate" => Some("Matches splat lighting to a target environment."),
        "Splat Heal" => Some("Fills small holes in splat surfaces by adding new splats."),
        "Splat Outlier" => Some("Removes sparse splats using density clustering."),
        "Splat Cluster" => Some("Labels splats with a cluster id attribute."),
        "Splat Merge" => Some("Geometrically joins two splat models with feathering or skirts."),
        "Volume from Geometry" => Some("Converts geometry into a sparse volume."),
        "Volume Combine" => Some("Combines two volumes into one."),
        "Volume Blur" => Some("Blurs volume values in a voxel neighborhood."),
        "Volume to Mesh" => Some("Extracts a mesh surface from a volume."),
        "Group" => Some("Creates a named group by shape or viewport selection."),
        "Group Expand" => Some("Expands or contracts a group by topology."),
        "Transform" => Some("Transforms geometry with translate/rotate/scale."),
        "FFD" => Some("Deforms geometry using a lattice of control points."),
        "Copy/Transform" => Some("Creates multiple copies with incremental transforms."),
        "Merge" => Some("Merges all input geometry."),
        "Copy to Points" => Some("Copies source meshes or splats onto template points."),
        "Scatter" => Some("Scatters random points over surfaces, curves, or volumes."),
        "Sweep" => Some("Sweeps a profile curve or polygon along a path curve to form a surface."),
        "Normal" => Some("Computes normals."),
        "Color" => Some("Sets a color attribute."),
        "Noise/Mountain" => Some("Displaces points along normals with noise."),
        "Erosion Noise" => Some("Applies erosion-style noise to point height."),
        "Smooth" => Some("Smooths attributes."),
        "Resample" => Some("Resamples meshes, curves, and volumes."),
        "UV Texture" => Some("Generates UVs using basic projections."),
        "UV Unwrap" => Some("Generates UVs by clustering faces and projecting."),
        "UV View" => Some("Displays UV wireframe for the incoming mesh."),
        "Material" => Some("Assigns a material with PBR parameters."),
        "Ray" => Some("Projects points onto geometry using raycasts."),
        "Attribute Noise" => Some("Writes noise into an attribute."),
        "Attribute Expand" => Some("Expands or contracts attribute values across neighbors."),
        "Attribute Promote" => Some("Promotes attributes between classes with merge rules."),
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

pub fn param_help(_node_name: &str, param: &str) -> Option<Cow<'static, str>> {
    common_param_help(param)
        .map(Cow::Borrowed)
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
        "out_group" => Some("Output group name for group expansion."),
        "domain" => Some("Attribute domain to read/write."),
        "attr" => Some("Attribute name(s) to operate on."),
        "result" => Some("Destination attribute name."),
        "data_type" => Some("Attribute type (float/vec2/vec3)."),
        "noise_type" => Some("Noise basis (Fast/Perlin/Simplex/Worley/etc)."),
        "fractal_type" => Some("Fractal mode (None/Standard/Terrain/Hybrid)."),
        "octaves" => Some("Number of fractal octaves."),
        "lacunarity" => Some("Frequency multiplier per octave."),
        "roughness" => Some("Amplitude multiplier per octave."),
        "flow_rotation" => Some("Perlin Flow rotation in degrees."),
        "distortion" => Some("Cloud noise distortion amount."),
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
        "expand_mode" => Some("Expand or Contract mode."),
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
