use std::borrow::Cow;

use egui::{
    Color32, Context, FontId, Frame, Id, Margin, Order, Pos2, Rect, RichText,
    ScrollArea, Vec2,
};
use lobedo_core::{builtin_kind_from_id, builtin_kind_from_name, help_summary, node_help_page};

pub fn node_help(help_key: &str) -> Option<&'static str> {
    let kind = builtin_kind_from_id(help_key).or_else(|| builtin_kind_from_name(help_key))?;
    help_summary(kind)
}

pub fn param_help(_node_name: &str, param: &str) -> Option<Cow<'static, str>> {
    common_param_help(param)
        .map(Cow::Borrowed)
        .or_else(|| Some(Cow::Owned(format!("{} parameter.", param))))
}

pub fn show_help_page_window(ctx: &Context, help_key: &str, open: &mut bool) {
    let Some(page) = node_help_page(help_key) else {
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
