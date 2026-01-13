use egui::Ui;

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use rfd::FileDialog;
#[cfg(target_arch = "wasm32")]
use rfd::AsyncFileDialog;
#[cfg(target_arch = "wasm32")]
use std::sync::{Mutex, OnceLock};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

use lobedo_core::{parse_color_gradient, ColorGradient, ParamValue};

use super::help::{param_help, show_help_tooltip};

pub(super) fn edit_param(
    ui: &mut Ui,
    node_name: &str,
    label: &str,
    value: ParamValue,
) -> (ParamValue, bool) {
    let display_label = display_label(node_name, label);
    let help = param_help(node_name, label);
    let help = help.as_deref();
    match value {
        ParamValue::Float(mut v) => {
            let changed = param_row_with_label(ui, label, &display_label, help, |ui| {
                let mut changed = false;
                let spacing = 8.0;
                let height = ui.spacing().interact_size.y;
                let controls_width = ui.max_rect().width();
                let prev_spacing = ui.spacing().item_spacing;
                ui.spacing_mut().item_spacing = egui::vec2(0.0, prev_spacing.y);
                let value_width = ((controls_width - spacing * 2.0) / 3.0).clamp(52.0, 110.0);
                if ui
                    .add_sized(
                        [value_width, height],
                        egui::DragValue::new(&mut v)
                            .speed(0.1)
                            .update_while_editing(false),
                    )
                    .changed()
                {
                    changed = true;
                }
                ui.add_space(spacing);
                let range = float_slider_range(node_name, label, v);
                let slider_width = (controls_width - value_width - spacing).max(120.0);
                let min = *range.start();
                let max = *range.end();
                let mut slider_value = v.clamp(min, max);
                if ui
                    .add_sized(
                        [slider_width, height],
                        egui::Slider::new(&mut slider_value, range).show_value(false),
                    )
                    .changed()
                {
                    v = slider_value;
                    changed = true;
                }
                ui.spacing_mut().item_spacing = prev_spacing;
                changed
            });
            (ParamValue::Float(v), changed)
        }
        ParamValue::Int(mut v) => {
            let changed = if label == "domain" || label == "mode" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[(1, "Vertex"), (0, "Point"), (2, "Primitive"), (3, "Detail")],
                    "Point",
                )
            } else if label == "group_type" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[
                        (0, "Auto"),
                        (1, "Vertex"),
                        (2, "Point"),
                        (3, "Primitive"),
                    ],
                    "Auto",
                )
            } else if label == "copy_attr_class" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[(0, "Point"), (1, "Vertex"), (2, "Primitive")],
                    "Point",
                )
            } else if label == "read_mode" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[(0, "Full (SH)"), (1, "Base Color")],
                    "Full (SH)",
                )
            } else if label == "data_type" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[(0, "Float"), (1, "Vec2"), (2, "Vec3")],
                    "Vec3",
                )
            } else if label == "noise_type" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[(0, "Value"), (1, "Perlin")],
                    "Value",
                )
            } else if label == "color_mode" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[(0, "Constant"), (1, "From Attribute")],
                    "Constant",
                )
            } else if label == "feature" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[(0, "Area"), (1, "Gradient")],
                    "Area",
                )
            } else if label == "method" && node_name == "Splat Merge" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[(0, "Feather"), (1, "Skirt")],
                    "Feather",
                )
            } else if label == "method" && node_name == "Splat Heal" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[(0, "Voxel Close"), (1, "SDF Patch")],
                    "Voxel Close",
                )
            } else if label == "method" && node_name == "Splat Cluster" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[(0, "Grid"), (1, "DBSCAN")],
                    "Grid",
                )
            } else if label == "method" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[(0, "Normal"), (1, "Direction"), (2, "Closest")],
                    "Normal",
                )
            } else if label == "op" && node_name == "Volume Combine" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[
                        (0, "Add"),
                        (1, "Subtract"),
                        (2, "Multiply"),
                        (3, "Min"),
                        (4, "Max"),
                        (5, "Average"),
                    ],
                    "Add",
                )
            } else if label == "resolution" && node_name == "Volume Combine" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[(0, "Lower"), (1, "Higher"), (2, "Average")],
                    "Lower",
                )
            } else if label == "op" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[(0, "Add"), (1, "Subtract"), (2, "Multiply"), (3, "Divide")],
                    "Add",
                )
            } else if label == "algorithm" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[(0, "Density (Iso)"), (1, "Ellipsoid (Smooth Min)")],
                    "Density (Iso)",
                )
            } else if label == "output" && node_name == "Splat to Mesh" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[(0, "Mesh"), (1, "SDF Volume")],
                    "Mesh",
                )
            } else if label == "projection" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[
                        (0, "Planar"),
                        (1, "Box"),
                        (2, "Cylindrical"),
                        (3, "Spherical"),
                    ],
                    "Planar",
                )
            } else if label == "axis" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[(0, "X"), (1, "Y"), (2, "Z")],
                    "Y",
                )
            } else if label == "smooth_space" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[(0, "World"), (1, "Surface")],
                    "World",
                )
            } else if label == "format" && node_name == "Splat Write" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[(0, "Binary"), (1, "ASCII")],
                    "Binary",
                )
            } else if label == "delight_mode" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[
                        (0, "Band 0 Only"),
                        (1, "SH Ratio"),
                        (2, "Irradiance Divide"),
                        (3, "Env Splat"),
                    ],
                    "SH Ratio",
                )
            } else if label == "relight_mode" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[(0, "SH Ratio"), (1, "Diffuse"), (2, "Hybrid")],
                    "Hybrid",
                )
            } else if label == "source_env" || label == "target_env" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[(0, "From Splats"), (1, "Uniform White"), (2, "Custom")],
                    "From Splats",
                )
            } else if label == "neutral_env" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[(0, "Uniform White"), (1, "Custom")],
                    "Uniform White",
                )
            } else if label == "high_band_mode" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[(0, "Scale Only"), (1, "Scale + Ratio")],
                    "Scale Only",
                )
            } else if label == "output_sh_order" {
                combo_row_i32(
                    ui,
                    label,
                    &display_label,
                    help,
                    &mut v,
                    &[(0, "L0"), (1, "L1"), (2, "L2"), (3, "L3")],
                    "L3",
                )
            } else {
                param_row_with_label(ui, label, &display_label, help, |ui| {
                    let mut changed = false;
                    let spacing = 8.0;
                    let height = ui.spacing().interact_size.y;
                    let controls_width = ui.max_rect().width();
                    let prev_spacing = ui.spacing().item_spacing;
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, prev_spacing.y);
                    let value_width = ((controls_width - spacing * 2.0) / 3.0).clamp(52.0, 110.0);
                    if ui
                        .add_sized(
                            [value_width, height],
                            egui::DragValue::new(&mut v)
                                .speed(1.0)
                                .update_while_editing(false),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    ui.add_space(spacing);
                    let range = int_slider_range(node_name, label, v);
                    let slider_width = (controls_width - value_width - spacing).max(120.0);
                    let min = *range.start();
                    let max = *range.end();
                    let mut slider_value = v.clamp(min, max);
                    if ui
                        .add_sized(
                            [slider_width, height],
                            egui::Slider::new(&mut slider_value, range).show_value(false),
                        )
                        .changed()
                    {
                        v = slider_value;
                        changed = true;
                    }
                    ui.spacing_mut().item_spacing = prev_spacing;
                    changed
                })
            };
            (ParamValue::Int(v), changed)
        }
        ParamValue::Bool(mut v) => {
            let changed = param_row_with_label(ui, label, &display_label, help, |ui| {
                let checkbox = egui::Checkbox::without_text(&mut v);
                ui.add(checkbox).changed()
            });
            (ParamValue::Bool(v), changed)
        }
        ParamValue::Vec2(mut v) => {
            let changed = param_row_with_label(ui, label, &display_label, help, |ui| {
                let mut changed = false;
                let spacing = 8.0;
                let available = ui.available_width();
                let prev_spacing = ui.spacing().item_spacing;
                ui.spacing_mut().item_spacing = egui::vec2(0.0, prev_spacing.y);
                let value_width = ((available - spacing) / 2.0).clamp(56.0, 120.0);
                let height = ui.spacing().interact_size.y;
                let len = v.len();
                for (idx, item) in v.iter_mut().enumerate() {
                    if ui
                        .add_sized(
                            [value_width, height],
                            egui::DragValue::new(item)
                                .speed(0.1)
                                .update_while_editing(false),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    if idx + 1 < len {
                        ui.add_space(spacing);
                    }
                }
                ui.spacing_mut().item_spacing = prev_spacing;
                changed
            });
            (ParamValue::Vec2(v), changed)
        }
        ParamValue::Vec3(mut v) => {
            let changed = param_row_with_label(ui, label, &display_label, help, |ui| {
                let mut changed = false;
                let spacing = 8.0;
                let available = ui.available_width();
                let prev_spacing = ui.spacing().item_spacing;
                ui.spacing_mut().item_spacing = egui::vec2(0.0, prev_spacing.y);
                let value_width = ((available - spacing * 2.0) / 3.0).clamp(52.0, 110.0);
                let height = ui.spacing().interact_size.y;
                let len = v.len();
                for (idx, item) in v.iter_mut().enumerate() {
                    if ui
                        .add_sized(
                            [value_width, height],
                            egui::DragValue::new(item)
                                .speed(0.1)
                                .update_while_editing(false),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    if idx + 1 < len {
                        ui.add_space(spacing);
                    }
                }
                ui.spacing_mut().item_spacing = prev_spacing;
                changed
            });
            (ParamValue::Vec3(v), changed)
        }
        ParamValue::String(mut v) => {
            let changed = if label == "gradient" {
                param_row_with_height_label(ui, label, &display_label, help, 112.0, |ui| {
                    edit_gradient_field(ui, node_name, label, &mut v)
                })
            } else if label == "mode"
                && matches!(node_name, "Volume to Mesh" | "Volume from Geometry")
            {
                let options = &[("density", "Density"), ("sdf", "SDF")];
                combo_row_string(ui, label, &display_label, help, &mut v, options, "Density")
            } else if label == "shape" {
                let options: &[(&str, &str)] = if node_name == "Group" {
                    &[
                        ("box", "Box"),
                        ("sphere", "Sphere"),
                        ("plane", "Plane"),
                        ("selection", "Selection"),
                        ("attribute", "Attribute"),
                    ]
                } else {
                    &[("box", "Box"), ("sphere", "Sphere"), ("plane", "Plane")]
                };
                combo_row_string(ui, label, &display_label, help, &mut v, options, "Box")
            } else if label == "heal_shape" {
                let options = &[("all", "All"), ("box", "Box"), ("sphere", "Sphere")];
                combo_row_string(ui, label, &display_label, help, &mut v, options, "All")
            } else if label == "output" && node_name == "Circle" {
                let options = &[("curve", "Curve"), ("mesh", "Mesh")];
                combo_row_string(ui, label, &display_label, help, &mut v, options, "Curve")
            } else if label == "code" {
                param_row_with_height_label(ui, label, &display_label, help, 120.0, |ui| {
                    ui.add_sized(
                        [ui.available_width().max(160.0), 100.0],
                        egui::TextEdit::multiline(&mut v)
                            .code_editor()
                            .desired_rows(4),
                    )
                    .changed()
                })
            } else {
                let use_picker = path_picker_kind(node_name, label).is_some();
                if use_picker {
                    param_row_with_label(ui, label, &display_label, help, |ui| {
                        edit_path_field(ui, node_name, label, &mut v)
                    })
                } else {
                    param_row_with_label(ui, label, &display_label, help, |ui| {
                        let height = ui.spacing().interact_size.y;
                        ui.add_sized(
                            [ui.available_width().max(160.0), height],
                            egui::TextEdit::singleline(&mut v),
                        )
                        .changed()
                    })
                }
            };
            (ParamValue::String(v), changed)
        }
    }
}

fn edit_gradient_field(ui: &mut Ui, node_name: &str, label: &str, value: &mut String) -> bool {
    let mut gradient = parse_color_gradient(value);
    if gradient.stops.is_empty() {
        gradient = ColorGradient::default();
    }
    let mut stops = gradient.stops.clone();
    if stops.is_empty() {
        stops = ColorGradient::default().stops;
    }

    let selected_id = ui.make_persistent_id((node_name, label, "gradient_selected"));
    let drag_id = ui.make_persistent_id((node_name, label, "gradient_drag"));
    let mut selected =
        ui.data_mut(|d| d.get_temp::<usize>(selected_id).unwrap_or(0));
    if selected >= stops.len() {
        selected = stops.len().saturating_sub(1);
    }
    let mut dragging = ui.data_mut(|d| d.get_temp::<bool>(drag_id).unwrap_or(false));

    let mut changed = false;
    let width = ui.available_width().max(160.0);
    let bar_total_height = 28.0;
    let bar_height = 16.0;
    let (bar_area, response) =
        ui.allocate_exact_size(egui::vec2(width, bar_total_height), egui::Sense::click_and_drag());
    let bar_rect = egui::Rect::from_min_size(bar_area.min, egui::vec2(width, bar_height));
    let handle_y = bar_rect.max.y + 6.0;
    let handle_radius = 6.0;

    let painter = ui.painter();
    let mut mesh = egui::Mesh::default();
    if stops.len() >= 2 {
        for stop in &stops {
            let x = bar_rect.min.x + stop.pos.clamp(0.0, 1.0) * bar_rect.width();
            let color = color32_from_rgb(stop.color);
            mesh.vertices.push(egui::epaint::Vertex {
                pos: egui::pos2(x, bar_rect.min.y),
                uv: egui::Pos2::ZERO,
                color,
            });
            mesh.vertices.push(egui::epaint::Vertex {
                pos: egui::pos2(x, bar_rect.max.y),
                uv: egui::Pos2::ZERO,
                color,
            });
        }
        for i in 0..stops.len() - 1 {
            let i0 = (i * 2) as u32;
            let i1 = i0 + 1;
            let i2 = i0 + 2;
            let i3 = i0 + 3;
            mesh.indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
        }
        painter.add(egui::Shape::mesh(mesh));
    } else if let Some(stop) = stops.first() {
        painter.rect_filled(bar_rect, 2.0, color32_from_rgb(stop.color));
    }
    painter.rect_stroke(
        bar_rect,
        2.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 60)),
        egui::StrokeKind::Inside,
    );

    let mut handle_hit = None;
    for (idx, stop) in stops.iter().enumerate() {
        let x = bar_rect.min.x + stop.pos.clamp(0.0, 1.0) * bar_rect.width();
        let center = egui::pos2(x, handle_y);
        let dist = response
            .interact_pointer_pos()
            .map(|pos| (pos - center).length())
            .unwrap_or(f32::INFINITY);
        if dist <= handle_radius {
            handle_hit = Some(idx);
        }
        let fill = color32_from_rgb(stop.color);
        let outline = if idx == selected {
            egui::Color32::from_rgb(255, 235, 170)
        } else {
            egui::Color32::from_rgb(20, 20, 20)
        };
        painter.circle_filled(center, handle_radius - 1.0, fill);
        painter.circle_stroke(center, handle_radius, egui::Stroke::new(1.0, outline));
    }

    let pointer_pos = response.interact_pointer_pos();
    if response.drag_started() {
        if let Some(hit) = handle_hit {
            selected = hit;
            dragging = true;
            changed = true;
        }
    }
    if response.clicked() {
        if let Some(hit) = handle_hit {
            selected = hit;
        } else if let Some(pos) = pointer_pos {
            if bar_rect.contains(pos) {
                let t = ((pos.x - bar_rect.min.x) / bar_rect.width()).clamp(0.0, 1.0);
                let color = gradient.sample(t);
                stops.push(lobedo_core::ColorStop { pos: t, color });
                stops.sort_by(|a, b| a.pos.partial_cmp(&b.pos).unwrap_or(std::cmp::Ordering::Equal));
                selected = find_stop_index(&stops, t);
                changed = true;
            }
        }
    }
    if response.dragged() && dragging {
        if let Some(pos) = pointer_pos {
            if let Some(stop) = stops.get_mut(selected) {
                stop.pos = ((pos.x - bar_rect.min.x) / bar_rect.width()).clamp(0.0, 1.0);
                let pos_marker = stop.pos;
                stops.sort_by(|a, b| a.pos.partial_cmp(&b.pos).unwrap_or(std::cmp::Ordering::Equal));
                selected = find_stop_index(&stops, pos_marker);
                changed = true;
            }
        }
    }
    if response.drag_stopped() {
        dragging = false;
    }

    ui.add_space(4.0);
    if let Some(stop) = stops.get(selected).copied() {
        let mut stop_color = stop.color;
        let mut stop_pos = stop.pos;
        let mut delete_stop = false;
        ui.horizontal(|ui| {
            if ui.color_edit_button_rgb(&mut stop_color).changed() {
                changed = true;
            }
            ui.add_space(6.0);
            ui.label("Pos");
            if ui
                .add(
                    egui::DragValue::new(&mut stop_pos)
                        .speed(0.01)
                        .range(0.0..=1.0)
                        .update_while_editing(false),
                )
                .changed()
            {
                stop_pos = stop_pos.clamp(0.0, 1.0);
                changed = true;
            }
            if ui.button("Delete").clicked() {
                delete_stop = true;
            }
        });
        if delete_stop {
            let (min_idx, max_idx) = endpoints_for(&stops);
            if stops.len() > 2 && selected != min_idx && selected != max_idx {
                stops.remove(selected);
                selected = selected.saturating_sub(1).min(stops.len().saturating_sub(1));
                changed = true;
            }
        } else if let Some(stop) = stops.get_mut(selected) {
            stop.color = stop_color;
            stop.pos = stop_pos.clamp(0.0, 1.0);
            let pos_marker = stop.pos;
            stops.sort_by(|a, b| a.pos.partial_cmp(&b.pos).unwrap_or(std::cmp::Ordering::Equal));
            selected = find_stop_index(&stops, pos_marker);
        }
    }

    ui.add_space(4.0);
    let height = ui.spacing().interact_size.y;
    let text_width = ui.available_width().max(140.0);
    let text_changed = ui
        .add_sized([text_width, height], egui::TextEdit::singleline(value))
        .changed();

    if changed {
        let gradient = ColorGradient { stops };
        *value = gradient.to_string();
    }

    ui.data_mut(|d| d.insert_temp(selected_id, selected));
    ui.data_mut(|d| d.insert_temp(drag_id, dragging));

    changed || text_changed
}

fn endpoints_for(stops: &[lobedo_core::ColorStop]) -> (usize, usize) {
    if stops.is_empty() {
        return (0, 0);
    }
    let mut min_idx = 0usize;
    let mut max_idx = 0usize;
    let mut min_pos = stops[0].pos;
    let mut max_pos = stops[0].pos;
    for (idx, stop) in stops.iter().enumerate().skip(1) {
        if stop.pos < min_pos {
            min_pos = stop.pos;
            min_idx = idx;
        }
        if stop.pos > max_pos {
            max_pos = stop.pos;
            max_idx = idx;
        }
    }
    (min_idx, max_idx)
}

fn find_stop_index(stops: &[lobedo_core::ColorStop], pos: f32) -> usize {
    let mut best_idx = 0usize;
    let mut best_dist = f32::INFINITY;
    for (idx, stop) in stops.iter().enumerate() {
        let dist = (stop.pos - pos).abs();
        if dist < best_dist {
            best_dist = dist;
            best_idx = idx;
        }
    }
    best_idx
}

fn color32_from_rgb(color: [f32; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(
        (color[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (color[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (color[2].clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

fn edit_path_field(ui: &mut Ui, node_name: &str, label: &str, value: &mut String) -> bool {
    let height = ui.spacing().interact_size.y;
    let spacing = 6.0;
    let button_width = height;
    let total_width = ui.available_width();
    let text_width = (total_width - button_width - spacing)
        .max(80.0)
        .min(total_width);
    let mut changed = false;
    #[cfg(target_arch = "wasm32")]
    if let Some(kind) = path_picker_kind(node_name, label) {
        if let Some(result) = take_file_pick(kind) {
            let key = lobedo_core::store_bytes(result.name, result.bytes);
            *value = key;
            changed = true;
        }
    }
    if ui
        .add_sized([text_width, height], egui::TextEdit::singleline(value))
        .changed()
    {
        changed = true;
    }
    ui.add_space(spacing);
    if let Some(kind) = path_picker_kind(node_name, label) {
        if open_path_picker_button(ui, kind, value, button_width, height) {
            changed = true;
        }
    }
    changed
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PathPickerKind {
    ReadMesh,
    WriteObj,
    WriteGltf,
    ReadSplat,
    WriteSplat,
    ReadTexture,
}

#[cfg(target_arch = "wasm32")]
struct FilePickResult {
    kind: PathPickerKind,
    name: String,
    bytes: Vec<u8>,
}

#[cfg(target_arch = "wasm32")]
static FILE_PICK_RESULT: OnceLock<Mutex<Option<FilePickResult>>> = OnceLock::new();

#[cfg(target_arch = "wasm32")]
fn file_pick_result() -> &'static Mutex<Option<FilePickResult>> {
    FILE_PICK_RESULT.get_or_init(|| Mutex::new(None))
}

#[cfg(target_arch = "wasm32")]
fn queue_file_pick(kind: PathPickerKind, name: String, bytes: Vec<u8>) {
    let store = file_pick_result();
    *store.lock().expect("file pick lock") = Some(FilePickResult { kind, name, bytes });
}

#[cfg(target_arch = "wasm32")]
fn take_file_pick(kind: PathPickerKind) -> Option<FilePickResult> {
    let store = file_pick_result();
    let mut guard = store.lock().expect("file pick lock");
    match guard.as_ref().map(|res| res.kind) {
        Some(found) if found == kind => guard.take(),
        _ => None,
    }
}

fn path_picker_kind(node_name: &str, label: &str) -> Option<PathPickerKind> {
    match (node_name, label) {
        ("File", "path") => Some(PathPickerKind::ReadMesh),
        ("OBJ Output", "path") => Some(PathPickerKind::WriteObj),
        ("GLTF Output", "path") => Some(PathPickerKind::WriteGltf),
        ("Splat Read", "path") | ("Read Splats", "path") => Some(PathPickerKind::ReadSplat),
        ("Splat Write", "path") => Some(PathPickerKind::WriteSplat),
        ("Material", "base_color_tex") => Some(PathPickerKind::ReadTexture),
        _ => None,
    }
}

fn open_path_picker_button(
    ui: &mut Ui,
    kind: PathPickerKind,
    value: &mut String,
    button_width: f32,
    height: f32,
) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        if matches!(
            kind,
            PathPickerKind::WriteObj | PathPickerKind::WriteGltf | PathPickerKind::WriteSplat
        ) {
            ui.add_enabled(false, egui::Button::new("..."))
                .on_hover_text("Save dialogs are not available in web builds yet");
            let _ = (value, button_width, height);
            return false;
        }
        let clicked = ui
            .add_sized([button_width, height], egui::Button::new("..."))
            .on_hover_text("Browse")
            .clicked();
        if clicked {
            let kind_copy = kind;
            spawn_local(async move {
                let (label, extensions) = match kind_copy {
                    PathPickerKind::ReadMesh => ("Mesh", &["obj", "gltf", "glb"][..]),
                    PathPickerKind::WriteObj => ("OBJ", &["obj"][..]),
                    PathPickerKind::WriteGltf => ("glTF", &["glb", "gltf"][..]),
                    PathPickerKind::ReadSplat | PathPickerKind::WriteSplat => ("PLY", &["ply"][..]),
                    PathPickerKind::ReadTexture => ("Image", &["png", "jpg", "jpeg"][..]),
                };
                let dialog = AsyncFileDialog::new().add_filter(label, extensions);
                if let Some(file) = dialog.pick_file().await {
                    let name = file.file_name();
                    let bytes = file.read().await;
                    queue_file_pick(kind_copy, name, bytes);
                }
            });
        }
        let _ = value;
        return false;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let clicked = ui
            .add_sized([button_width, height], egui::Button::new("..."))
            .on_hover_text("Browse")
            .clicked();
        if clicked {
            if let Some(path) = open_path_picker(kind, value) {
                *value = path;
                return true;
            }
        }
        false
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn open_path_picker(kind: PathPickerKind, current: &str) -> Option<String> {
        let (label, extensions, is_save, default_name) = match kind {
        PathPickerKind::ReadMesh => ("Mesh", &["obj", "gltf", "glb"][..], false, "model.obj"),
        PathPickerKind::WriteObj => ("OBJ", &["obj"][..], true, "output.obj"),
        PathPickerKind::WriteGltf => ("glTF", &["glb", "gltf"][..], true, "output.glb"),
        PathPickerKind::ReadSplat => ("PLY", &["ply"][..], false, "splats.ply"),
        PathPickerKind::WriteSplat => ("PLY", &["ply"][..], true, "output.ply"),
        PathPickerKind::ReadTexture => ("Image", &["png", "jpg", "jpeg"][..], false, "texture.png"),
        };
    let mut dialog = FileDialog::new().add_filter(label, extensions);
    if !current.trim().is_empty() {
        let path = Path::new(current);
        if let Some(parent) = path.parent() {
            dialog = dialog.set_directory(parent);
        }
        if is_save {
            if let Some(name) = path.file_name() {
                dialog = dialog.set_file_name(name.to_string_lossy().into_owned());
            }
        }
    } else if is_save {
        dialog = dialog.set_file_name(default_name);
    }
    let picked = if is_save {
        dialog.save_file()
    } else {
        dialog.pick_file()
    };
    picked.map(|path| path.display().to_string())
}

fn param_row_with_label(
    ui: &mut Ui,
    id: &str,
    label: &str,
    help: Option<&str>,
    add_controls: impl FnOnce(&mut Ui) -> bool,
) -> bool {
    param_row_with_height_label(ui, id, label, help, 36.0, add_controls)
}

fn param_row_with_height_label(
    ui: &mut Ui,
    _id: &str,
    label: &str,
    help: Option<&str>,
    row_height: f32,
    add_controls: impl FnOnce(&mut Ui) -> bool,
) -> bool {
    let total_width = ui.available_width();
    let label_width = (total_width * 0.2).clamp(80.0, 160.0);
    let controls_width = (total_width - label_width).max(120.0);
    let mut changed = false;
    let (row_rect, _) = ui.allocate_exact_size(
        egui::vec2(total_width, row_height),
        egui::Sense::hover(),
    );
    let label_rect = egui::Rect::from_min_size(row_rect.min, egui::vec2(label_width, row_height));
    let controls_rect = egui::Rect::from_min_size(
        egui::pos2(row_rect.min.x + label_width, row_rect.min.y),
        egui::vec2(controls_width, row_height),
    );
    let mut label_response = None;
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(label_rect)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
        |ui| {
            ui.set_min_height(row_height);
            let response = ui.add(egui::Label::new(label).sense(egui::Sense::hover()));
            label_response = Some(response);
        },
    );
    if let (Some(help), Some(response)) = (help, label_response) {
        if response.hovered() {
            show_help_tooltip(ui.ctx(), response.rect, help);
        }
    }
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(controls_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.set_min_height(row_height);
            if add_controls(ui) {
                changed = true;
            }
        },
    );
    changed
}

fn combo_row_i32(
    ui: &mut Ui,
    id: &str,
    label: &str,
    help: Option<&str>,
    value: &mut i32,
    options: &[(i32, &str)],
    fallback: &str,
) -> bool {
    param_row_with_label(ui, id, label, help, |ui| {
        let mut changed = false;
        let selected = options
            .iter()
            .find(|(opt_value, _)| *opt_value == *value)
            .map(|(_, name)| *name)
            .unwrap_or(fallback);
        egui::ComboBox::from_id_salt(id)
            .selected_text(selected)
            .show_ui(ui, |ui| {
                for (opt_value, name) in options.iter().copied() {
                    if ui.selectable_value(value, opt_value, name).changed() {
                        changed = true;
                    }
                }
            });
        changed
    })
}

fn combo_row_string(
    ui: &mut Ui,
    id: &str,
    label: &str,
    help: Option<&str>,
    value: &mut String,
    options: &[(&str, &str)],
    fallback: &str,
) -> bool {
    param_row_with_label(ui, id, label, help, |ui| {
        let mut changed = false;
        let selected = options
            .iter()
            .find(|(opt_value, _)| *opt_value == value.as_str())
            .map(|(_, name)| *name)
            .unwrap_or(fallback);
        egui::ComboBox::from_id_salt(id)
            .selected_text(selected)
            .show_ui(ui, |ui| {
                for (opt_value, name) in options.iter().copied() {
                    if ui
                        .selectable_value(value, opt_value.to_string(), name)
                        .changed()
                    {
                        changed = true;
                    }
                }
            });
        changed
    })
}

fn display_label(node_name: &str, key: &str) -> String {
    if node_name == "Splat Cluster" {
        return match key {
            "cell_size" => "Cell Size",
            "eps" => "Radius",
            "min_pts" => "Min Points",
            "attr" => "Attribute",
            _ => key,
        }
        .to_string();
    }
    if node_name == "Splat Outlier" {
        return match key {
            "eps" => "Radius",
            "min_pts" => "Min Points",
            "min_cluster_size" => "Min Cluster Size",
            _ => key,
        }
        .to_string();
    }
    if node_name == "Color" {
        return match key {
            "color_mode" => "Mode",
            "attr" => "Attribute",
            "gradient" => "Gradient",
            _ => key,
        }
        .to_string();
    }
    if node_name == "Curve" {
        return match key {
            "points" => "Points",
            "subdivs" => "Subdivs",
            "closed" => "Closed",
            _ => key,
        }
        .to_string();
    }
    if node_name == "Circle" {
        return match key {
            "output" => "Output",
            "radius" => "Radius",
            "segments" => "Segments",
            "center" => "Center",
            _ => key,
        }
        .to_string();
    }
    if node_name == "Group" {
        return match key {
            "select_backface" => "Select Backfaces",
            "attr_min" => "Range Min",
            "attr_max" => "Range Max",
            "attr" => "Attribute",
            _ => key,
        }
        .to_string();
    }
    if node_name == "Scatter" {
        return match key {
            "density_attr" => "Density Attribute",
            "density_min" => "Density Min",
            "density_max" => "Density Max",
            "inherit" => "Inherit Attributes",
            _ => key,
        }
        .to_string();
    }
    if node_name == "Smooth" {
        return match key {
            "smooth_space" => "Space",
            "radius" => "Radius",
            _ => key,
        }
        .to_string();
    }
    if node_name == "Erosion Noise" {
        return match key {
            "erosion_strength" => "Erosion Strength",
            "erosion_freq" => "Erosion Freq",
            "erosion_octaves" => "Erosion Octaves",
            "erosion_roughness" => "Erosion Roughness",
            "erosion_lacunarity" => "Erosion Lacunarity",
            "erosion_slope_strength" => "Erosion Slope Strength",
            "erosion_branch_strength" => "Erosion Branch Strength",
            "do_mask" => "Output Mask",
            _ => key,
        }
        .to_string();
    }
    if node_name == "Copy/Transform" {
        return match key {
            "translate" => "Translate",
            "rotate_deg" => "Rotate",
            "scale" => "Scale",
            "pivot" => "Pivot",
            "translate_step" => "Translate Step",
            "rotate_step_deg" => "Rotate Step",
            "scale_step" => "Scale Step",
            "count" => "Count",
            _ => key,
        }
        .to_string();
    }
    if node_name == "Copy to Points" {
        return match key {
            "inherit" => "Inherit Attributes",
            "copy_attr" => "Copy Attribute",
            "copy_attr_class" => "Copy Attribute Class",
            _ => key,
        }
        .to_string();
    }
    if node_name == "FFD" {
        return match key {
            "res_x" => "Res X",
            "res_y" => "Res Y",
            "res_z" => "Res Z",
            "use_input_bounds" => "Use Input Bounds",
            "padding" => "Padding",
            "extrapolate" => "Extrapolate",
            _ => key,
        }
        .to_string();
    }
    if node_name == "Splat to Mesh" {
        return match key {
            "output" => "Output",
            "algorithm" => "Method",
            "voxel_size" => "Voxel Size",
            "n_sigma" => "Support Sigma",
            "density_iso" => "Density Threshold",
            "surface_iso" => "Surface Threshold",
            "bounds_padding" => "Bounds Padding (sigma)",
            "transfer_color" => "Transfer Color",
            "max_m2" => "Exponent Clamp",
            "smooth_k" => "Blend Sharpness",
            "shell_radius" => "Shell Radius",
            "blur_iters" => "Density Blur",
            "voxel_size_max" => "Max Voxel Dimension",
            _ => key,
        }
        .to_string();
    }
    if node_name == "Splat Merge" {
        return match key {
            "method" => "Method",
            "blend_radius" => "Blend Radius",
            "fade_originals" => "Fade Originals",
            "skirt_max_dist" => "Skirt Max Dist",
            "skirt_step" => "Skirt Step",
            "skirt_max_new" => "Skirt Max New",
            "seam_alpha" => "Seam Alpha",
            "seam_scale" => "Seam Scale",
            "seam_dc_only" => "Seam DC Only",
            "preview_skirt" => "Preview Skirt",
            _ => key,
        }
        .to_string();
    }
    if node_name == "Splat Heal" {
        return match key {
            "heal_shape" => "Heal Bounds",
            "method" => "Method",
            "preview_surface" => "Preview Surface",
            "voxel_size" => "Voxel Size",
            "voxel_size_max" => "Max Voxel Dim",
            "n_sigma" => "Support Sigma",
            "density_iso" => "Density Threshold",
            "bounds_padding" => "Bounds Padding",
            "close_radius" => "Close Radius",
            "fill_stride" => "Fill Stride",
            "max_new" => "Max New",
            "sdf_band" => "SDF Band",
            "sdf_close" => "SDF Close",
            "search_radius" => "Search Radius",
            "min_distance" => "Min Distance",
            "scale_mul" => "Scale Mult",
            "opacity_mul" => "Opacity Mult",
            "copy_sh" => "Copy SH",
            "max_m2" => "Exponent Clamp",
            "smooth_k" => "Blend Sharpness",
            "shell_radius" => "Shell Radius",
            "blur_iters" => "Density Blur",
            _ => key,
        }
        .to_string();
    }
    if node_name == "Volume from Geometry" {
        return match key {
            "mode" => "Mode",
            "max_dim" => "Max Dimension",
            "padding" => "Padding",
            "density_scale" => "Density Scale",
            "sdf_band" => "SDF Band",
            _ => key,
        }
        .to_string();
    }
    if node_name == "Volume Combine" {
        return match key {
            "op" => "Operator",
            "resolution" => "Resolution",
            _ => key,
        }
        .to_string();
    }
    if node_name == "Volume to Mesh" {
        return match key {
            "mode" => "Mode",
            "density_iso" => "Density Iso",
            "surface_iso" => "Surface Iso",
            _ => key,
        }
        .to_string();
    }
    if node_name == "UV Texture" {
        return match key {
            "projection" => "Projection",
            "axis" => "Axis",
            "scale" => "Scale",
            "offset" => "Offset",
            _ => key,
        }
        .to_string();
    }
    if node_name == "UV Unwrap" {
        return match key {
            "padding" => "Padding",
            "normal_threshold" => "Normal Threshold",
            _ => key,
        }
        .to_string();
    }
    if node_name == "Material" {
        return match key {
            "name" => "Name",
            "base_color" => "Base Color",
            "base_color_tex" => "Base Color Texture",
            "metallic" => "Metallic",
            "roughness" => "Roughness",
            _ => key,
        }
        .to_string();
    }
    if node_name == "Attribute from Volume" {
        return match key {
            "attr" => "Attribute",
            "domain" => "Domain",
            _ => key,
        }
        .to_string();
    }
    if node_name == "Splat Delight" {
        return match key {
            "delight_mode" => "Mode",
            "source_env" => "Source Env",
            "neutral_env" => "Neutral Env",
            "source_color" => "Source Color",
            "neutral_color" => "Neutral Color",
            "eps" => "Epsilon",
            "ratio_min" => "Ratio Min",
            "ratio_max" => "Ratio Max",
            "high_band_gain" => "High Band Gain",
            "output_sh_order" => "Output SH Order",
            "albedo_max" => "Albedo Max",
            _ => key,
        }
        .to_string();
    }
    if node_name == "Splat Integrate" {
        return match key {
            "relight_mode" => "Mode",
            "source_env" => "Source Env",
            "target_env" => "Target Env",
            "source_color" => "Source Color",
            "target_color" => "Target Color",
            "eps" => "Epsilon",
            "ratio_min" => "Ratio Min",
            "ratio_max" => "Ratio Max",
            "high_band_gain" => "High Band Gain",
            "high_band_mode" => "High Band Mode",
            "output_sh_order" => "Output SH Order",
            "albedo_max" => "Albedo Max",
            _ => key,
        }
        .to_string();
    }
    if node_name == "Splat Write" {
        return match key {
            "path" => "Path",
            "format" => "Format",
            _ => key,
        }
        .to_string();
    }
    if node_name == "GLTF Output" {
        return match key {
            "path" => "Path",
            _ => key,
        }
        .to_string();
    }
    key.to_string()
}

fn float_slider_range(
    node_name: &str,
    label: &str,
    _value: f32,
) -> std::ops::RangeInclusive<f32> {
    match label {
        "min_scale" | "max_scale" => -10.0..=10.0,
        "min_opacity" | "max_opacity" => -2.0..=2.0,
        "threshold_deg" => 0.0..=180.0,
        "amplitude" => -10.0..=10.0,
        "frequency" => 0.0..=10.0,
        "strength" => 0.0..=1.0,
        "density_min" | "density_max" => 0.0..=1.0,
        "value_f" => -10.0..=10.0,
        "radius" => 0.0..=1000.0,
        "max_distance" => 0.0..=1000.0,
        "voxel_size" => 0.0..=10.0,
        "n_sigma" => 0.0..=6.0,
        "density_iso" if node_name == "Volume to Mesh" => 0.0..=1.0,
        "surface_iso" if node_name == "Volume to Mesh" => -1.0..=1.0,
        "density_iso" => 0.0..=10.0,
        "surface_iso" => -5.0..=5.0,
        "bounds_padding" => 0.0..=10.0,
        "normal_threshold" => 0.0..=180.0,
        "max_m2" => 0.0..=10.0,
        "smooth_k" => 0.001..=2.0,
        "shell_radius" => 0.1..=4.0,
        "sdf_band" if node_name == "Splat Heal" => 0.0..=5.0,
        "sdf_close" => -2.0..=2.0,
        "search_radius" => 0.0..=10.0,
        "min_distance" => 0.0..=10.0,
        "scale_mul" => 0.1..=10.0,
        "opacity_mul" => 0.0..=2.0,
        "cell_size" => 0.0..=10.0,
        "eps" if node_name == "Splat Cluster" || node_name == "Splat Outlier" => 0.0..=10.0,
        "padding" => 0.0..=10.0,
        "density_scale" => 0.0..=10.0,
        "sdf_band" => 0.0..=10.0,
        "metallic" | "roughness" => 0.0..=1.0,
        "erosion_strength" => 0.0..=1.0,
        "erosion_freq" => 0.0..=30.0,
        "erosion_roughness" => 0.0..=1.0,
        "erosion_lacunarity" => 1.0..=4.0,
        "erosion_slope_strength" => 0.0..=5.0,
        "erosion_branch_strength" => 0.0..=5.0,
        "ratio_min" | "ratio_max" => 0.0..=10.0,
        "high_band_gain" => 0.0..=1.0,
        "eps" => 0.0..=0.1,
        "albedo_max" => 0.0..=4.0,
        _ => -1000.0..=1000.0,
    }
}

fn int_slider_range(
    node_name: &str,
    label: &str,
    _value: i32,
) -> std::ops::RangeInclusive<i32> {
    match label {
        "domain" => 0..=3,
        "op" => 0..=3,
        "rows" | "cols" => 2..=64,
        "res_x" | "res_y" | "res_z" => 2..=8,
        "subdivs" => 1..=64,
        "segments" => 3..=256,
        "iterations" => 0..=20,
        "seed" => 0..=100,
        "blur_iters" => 0..=6,
        "voxel_size_max" => 8..=2048,
        "max_dim" => 8..=512,
        "close_radius" => 0..=6,
        "fill_stride" => 1..=8,
        "max_new" => 0..=100_000,
        "min_pts" => 1..=128,
        "min_cluster_size" => 0..=100_000,
        "erosion_octaves" => 1..=8,
        "count" if node_name == "Scatter" => 0..=1000,
        "count" if node_name == "Copy/Transform" => 1..=100,
        "target_count" => 0..=1_000_000,
        _ => -1000..=1000,
    }
}
