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

use lobedo_core::{
    parse_color_gradient, BuiltinNodeKind, ColorGradient, ParamKind, ParamOption, ParamRange,
    ParamSpec, ParamValue, ParamWidget, ParamPathKind,
};

use super::help::{param_help, show_help_tooltip};

pub(super) fn edit_param(
    ui: &mut Ui,
    node_name: &str,
    node_kind: Option<BuiltinNodeKind>,
    label: &str,
    value: ParamValue,
) -> (ParamValue, bool) {
    let display_label = display_label(node_name, node_kind, label);
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
                let range = -1000.0..=1000.0;
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
                            .speed(1.0)
                            .update_while_editing(false),
                    )
                    .changed()
                {
                    changed = true;
                }
                ui.add_space(spacing);
                let range = -1000..=1000;
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
            let changed = param_row_with_label(ui, label, &display_label, help, |ui| {
                let height = ui.spacing().interact_size.y;
                ui.add_sized(
                    [ui.available_width().max(160.0), height],
                    egui::TextEdit::singleline(&mut v),
                )
                .changed()
            });
            (ParamValue::String(v), changed)
        }
    }
}

pub(super) fn edit_param_with_spec(
    ui: &mut Ui,
    node_name: &str,
    node_kind: Option<BuiltinNodeKind>,
    spec: &ParamSpec,
    value: ParamValue,
) -> (ParamValue, bool) {
    let display_label = if spec.label.is_empty() {
        display_label(node_name, node_kind, spec.key)
    } else {
        spec.label.to_string()
    };
    let fallback_help = if spec.help.is_some() {
        None
    } else {
        param_help(node_name, spec.key)
    };
    let help = spec.help.or(fallback_help.as_deref());

    let fallback_value = value.clone();
    match (spec.kind, value) {
        (ParamKind::Float, ParamValue::Float(mut v)) => {
            let changed = param_row_with_label(ui, spec.key, &display_label, help, |ui| {
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
                let range = match spec.range {
                    Some(ParamRange::Float { min, max }) => min..=max,
                    _ => -1000.0..=1000.0,
                };
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
        (ParamKind::Int, ParamValue::Int(mut v)) => {
            let mut options_i32 = Vec::new();
            for option in &spec.options {
                if let ParamOption::Int { value, label } = option {
                    options_i32.push((*value, *label));
                }
            }
            let changed = if spec.widget == ParamWidget::Combo && !options_i32.is_empty() {
                combo_row_i32(
                    ui,
                    spec.key,
                    &display_label,
                    help,
                    &mut v,
                    &options_i32,
                    options_i32
                        .first()
                        .map(|(_, label)| *label)
                        .unwrap_or("Option"),
                )
            } else {
                param_row_with_label(ui, spec.key, &display_label, help, |ui| {
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
                    let range = match spec.range {
                        Some(ParamRange::Int { min, max }) => min..=max,
                        _ => -1000..=1000,
                    };
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
        (ParamKind::Bool, ParamValue::Bool(mut v)) => {
            let changed = param_row_with_label(ui, spec.key, &display_label, help, |ui| {
                let checkbox = egui::Checkbox::without_text(&mut v);
                ui.add(checkbox).changed()
            });
            (ParamValue::Bool(v), changed)
        }
        (ParamKind::Vec2, ParamValue::Vec2(mut v)) => {
            let changed = param_row_with_label(ui, spec.key, &display_label, help, |ui| {
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
        (ParamKind::Vec3, ParamValue::Vec3(mut v)) => {
            let changed = param_row_with_label(ui, spec.key, &display_label, help, |ui| {
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
        (ParamKind::String, ParamValue::String(mut v)) => {
            let mut options_str = Vec::new();
            for option in &spec.options {
                if let ParamOption::String { value, label } = option {
                    options_str.push((*value, *label));
                }
            }
            let changed = if spec.widget == ParamWidget::Combo && !options_str.is_empty() {
                combo_row_string(
                    ui,
                    spec.key,
                    &display_label,
                    help,
                    &mut v,
                    &options_str,
                    options_str
                        .first()
                        .map(|(_, label)| *label)
                        .unwrap_or("Option"),
                )
            } else if spec.widget == ParamWidget::Gradient {
                param_row_with_height_label(ui, spec.key, &display_label, help, 112.0, |ui| {
                    edit_gradient_field(ui, node_name, spec.key, &mut v)
                })
            } else if spec.widget == ParamWidget::Code {
                param_row_with_height_label(ui, spec.key, &display_label, help, 120.0, |ui| {
                    ui.add_sized(
                        [ui.available_width().max(160.0), 100.0],
                        egui::TextEdit::multiline(&mut v)
                            .code_editor()
                            .desired_rows(4),
                    )
                    .changed()
                })
            } else {
                let picker_kind = path_picker_kind_from_spec(spec);
                if spec.widget == ParamWidget::Path {
                    param_row_with_label(ui, spec.key, &display_label, help, |ui| {
                        edit_path_field(ui, picker_kind, &mut v)
                    })
                } else {
                    param_row_with_label(ui, spec.key, &display_label, help, |ui| {
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
        _ => edit_param(ui, node_name, node_kind, spec.key, fallback_value),
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

fn edit_path_field(
    ui: &mut Ui,
    kind: Option<PathPickerKind>,
    value: &mut String,
) -> bool {
    let height = ui.spacing().interact_size.y;
    let spacing = 6.0;
    let button_width = height;
    let total_width = ui.available_width();
    let text_width = (total_width - button_width - spacing)
        .max(80.0)
        .min(total_width);
    let mut changed = false;
    #[cfg(target_arch = "wasm32")]
    if let Some(kind) = kind {
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
    if let Some(kind) = kind {
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

fn path_picker_kind_from_spec(spec: &ParamSpec) -> Option<PathPickerKind> {
    spec.path_kind.map(|kind| match kind {
        ParamPathKind::ReadMesh => PathPickerKind::ReadMesh,
        ParamPathKind::WriteObj => PathPickerKind::WriteObj,
        ParamPathKind::WriteGltf => PathPickerKind::WriteGltf,
        ParamPathKind::ReadSplat => PathPickerKind::ReadSplat,
        ParamPathKind::WriteSplat => PathPickerKind::WriteSplat,
        ParamPathKind::ReadTexture => PathPickerKind::ReadTexture,
    })
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

fn display_label(_node_name: &str, _node_kind: Option<BuiltinNodeKind>, key: &str) -> String {
    key.to_string()
}



