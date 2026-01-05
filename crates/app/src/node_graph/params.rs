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

use lobedo_core::ParamValue;

pub(super) fn edit_param(
    ui: &mut Ui,
    node_name: &str,
    label: &str,
    value: ParamValue,
) -> (ParamValue, bool) {
    match value {
        ParamValue::Float(mut v) => {
            let changed = param_row(ui, label, |ui| {
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
                        egui::DragValue::new(&mut v).speed(0.1),
                    )
                    .changed()
                {
                    changed = true;
                }
                ui.add_space(spacing);
                let range = float_slider_range(node_name, label, v);
                let slider_width = (controls_width - value_width - spacing).max(120.0);
                if ui
                    .add_sized(
                        [slider_width, height],
                        egui::Slider::new(&mut v, range).show_value(false),
                    )
                    .changed()
                {
                    changed = true;
                }
                ui.spacing_mut().item_spacing = prev_spacing;
                changed
            });
            (ParamValue::Float(v), changed)
        }
        ParamValue::Int(mut v) => {
            let changed = if label == "domain" || label == "mode" {
                param_row(ui, label, |ui| {
                    let mut changed = false;
                    let options = [(1, "Vertex"), (0, "Point"), (2, "Primitive"), (3, "Detail")];
                    let selected = options
                        .iter()
                        .find(|(value, _)| *value == v)
                        .map(|(_, name)| *name)
                        .unwrap_or("Point");
                    egui::ComboBox::from_id_salt(label)
                        .selected_text(selected)
                        .show_ui(ui, |ui| {
                            for (value, name) in options {
                                if ui.selectable_value(&mut v, value, name).changed() {
                                    changed = true;
                                }
                            }
                        });
                    changed
                })
            } else if label == "group_type" {
                param_row(ui, label, |ui| {
                    let mut changed = false;
                    let options = [
                        (0, "Auto"),
                        (1, "Vertex"),
                        (2, "Point"),
                        (3, "Primitive"),
                    ];
                    let selected = options
                        .iter()
                        .find(|(value, _)| *value == v)
                        .map(|(_, name)| *name)
                        .unwrap_or("Auto");
                    egui::ComboBox::from_id_salt(label)
                        .selected_text(selected)
                        .show_ui(ui, |ui| {
                            for (value, name) in options {
                                if ui.selectable_value(&mut v, value, name).changed() {
                                    changed = true;
                                }
                            }
                        });
                    changed
                })
            } else if label == "read_mode" {
                param_row(ui, label, |ui| {
                    let mut changed = false;
                    let options = [(0, "Full (SH)"), (1, "Base Color")];
                    let selected = options
                        .iter()
                        .find(|(value, _)| *value == v)
                        .map(|(_, name)| *name)
                        .unwrap_or("Full (SH)");
                    egui::ComboBox::from_id_salt(label)
                        .selected_text(selected)
                        .show_ui(ui, |ui| {
                            for (value, name) in options {
                                if ui.selectable_value(&mut v, value, name).changed() {
                                    changed = true;
                                }
                            }
                        });
                    changed
                })
            } else if label == "data_type" {
                param_row(ui, label, |ui| {
                    let mut changed = false;
                    let options = [(0, "Float"), (1, "Vec2"), (2, "Vec3")];
                    let selected = options
                        .iter()
                        .find(|(value, _)| *value == v)
                        .map(|(_, name)| *name)
                        .unwrap_or("Vec3");
                    egui::ComboBox::from_id_salt(label)
                        .selected_text(selected)
                        .show_ui(ui, |ui| {
                            for (value, name) in options {
                                if ui.selectable_value(&mut v, value, name).changed() {
                                    changed = true;
                                }
                            }
                        });
                    changed
                })
            } else if label == "noise_type" {
                param_row(ui, label, |ui| {
                    let mut changed = false;
                    let options = [(0, "Value"), (1, "Perlin")];
                    let selected = options
                        .iter()
                        .find(|(value, _)| *value == v)
                        .map(|(_, name)| *name)
                        .unwrap_or("Value");
                    egui::ComboBox::from_id_salt(label)
                        .selected_text(selected)
                        .show_ui(ui, |ui| {
                            for (value, name) in options {
                                if ui.selectable_value(&mut v, value, name).changed() {
                                    changed = true;
                                }
                            }
                        });
                    changed
                })
            } else if label == "feature" {
                param_row(ui, label, |ui| {
                    let mut changed = false;
                    let options = [(0, "Area"), (1, "Gradient")];
                    let selected = options
                        .iter()
                        .find(|(value, _)| *value == v)
                        .map(|(_, name)| *name)
                        .unwrap_or("Area");
                    egui::ComboBox::from_id_salt(label)
                        .selected_text(selected)
                        .show_ui(ui, |ui| {
                            for (value, name) in options {
                                if ui.selectable_value(&mut v, value, name).changed() {
                                    changed = true;
                                }
                            }
                        });
                    changed
                })
            } else if label == "method" {
                param_row(ui, label, |ui| {
                    let mut changed = false;
                    let options = [(0, "Normal"), (1, "Direction"), (2, "Closest")];
                    let selected = options
                        .iter()
                        .find(|(value, _)| *value == v)
                        .map(|(_, name)| *name)
                        .unwrap_or("Normal");
                    egui::ComboBox::from_id_salt(label)
                        .selected_text(selected)
                        .show_ui(ui, |ui| {
                            for (value, name) in options {
                                if ui.selectable_value(&mut v, value, name).changed() {
                                    changed = true;
                                }
                            }
                        });
                    changed
                })
            } else if label == "op" {
                param_row(ui, label, |ui| {
                    let mut changed = false;
                    let options = [(0, "Add"), (1, "Subtract"), (2, "Multiply"), (3, "Divide")];
                    let selected = options
                        .iter()
                        .find(|(value, _)| *value == v)
                        .map(|(_, name)| *name)
                        .unwrap_or("Add");
                    egui::ComboBox::from_id_salt(label)
                        .selected_text(selected)
                        .show_ui(ui, |ui| {
                            for (value, name) in options {
                                if ui.selectable_value(&mut v, value, name).changed() {
                                    changed = true;
                                }
                            }
                        });
                    changed
                })
            } else {
                param_row(ui, label, |ui| {
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
                            egui::DragValue::new(&mut v).speed(1.0),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    ui.add_space(spacing);
                    let range = int_slider_range(node_name, label, v);
                    let slider_width = (controls_width - value_width - spacing).max(120.0);
                    if ui
                        .add_sized(
                            [slider_width, height],
                            egui::Slider::new(&mut v, range).show_value(false),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                    ui.spacing_mut().item_spacing = prev_spacing;
                    changed
                })
            };
            (ParamValue::Int(v), changed)
        }
        ParamValue::Bool(mut v) => {
            let changed = param_row(ui, label, |ui| {
                let checkbox = egui::Checkbox::without_text(&mut v);
                ui.add(checkbox).changed()
            });
            (ParamValue::Bool(v), changed)
        }
        ParamValue::Vec2(mut v) => {
            let changed = param_row(ui, label, |ui| {
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
                        .add_sized([value_width, height], egui::DragValue::new(item).speed(0.1))
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
            let changed = param_row(ui, label, |ui| {
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
                        .add_sized([value_width, height], egui::DragValue::new(item).speed(0.1))
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
            let changed = if label == "shape" {
                param_row(ui, label, |ui| {
                    let mut changed = false;
                    let options = [("box", "Box"), ("sphere", "Sphere"), ("plane", "Plane"), ("group", "Group")];
                    let selected = options
                        .iter()
                        .find(|(value, _)| *value == v)
                        .map(|(_, name)| *name)
                        .unwrap_or("Box");
                    egui::ComboBox::from_id_salt(label)
                        .selected_text(selected)
                        .show_ui(ui, |ui| {
                            for (value, name) in options {
                                if ui.selectable_value(&mut v, value.to_string(), name).changed() {
                                    changed = true;
                                }
                            }
                        });
                    changed
                })
            } else if label == "code" {
                param_row_with_height(ui, label, 120.0, |ui| {
                    ui.add_sized(
                        [ui.available_width().max(160.0), 100.0],
                        egui::TextEdit::multiline(&mut v)
                            .code_editor()
                            .desired_rows(4),
                    )
                    .changed()
                })
            } else {
                let use_picker = label == "path" && path_picker_kind(node_name).is_some();
                if use_picker {
                    param_row(ui, label, |ui| edit_path_field(ui, node_name, &mut v))
                } else {
                    param_row(ui, label, |ui| {
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

fn edit_path_field(ui: &mut Ui, node_name: &str, value: &mut String) -> bool {
    let height = ui.spacing().interact_size.y;
    let spacing = 6.0;
    let button_width = height;
    let total_width = ui.available_width();
    let text_width = (total_width - button_width - spacing)
        .max(80.0)
        .min(total_width);
    let mut changed = false;
    #[cfg(target_arch = "wasm32")]
    if let Some(kind) = path_picker_kind(node_name) {
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
    if let Some(kind) = path_picker_kind(node_name) {
        if open_path_picker_button(ui, kind, value, button_width, height) {
            changed = true;
        }
    }
    changed
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PathPickerKind {
    ReadObj,
    WriteObj,
    ReadSplat,
    WriteSplat,
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

fn path_picker_kind(node_name: &str) -> Option<PathPickerKind> {
    match node_name {
        "File" => Some(PathPickerKind::ReadObj),
        "OBJ Output" => Some(PathPickerKind::WriteObj),
        "Splat Read" | "Read Splats" => Some(PathPickerKind::ReadSplat),
        "Splat Write" => Some(PathPickerKind::WriteSplat),
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
        if matches!(kind, PathPickerKind::WriteObj | PathPickerKind::WriteSplat) {
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
                    PathPickerKind::ReadObj | PathPickerKind::WriteObj => ("OBJ", &["obj"][..]),
                    PathPickerKind::ReadSplat | PathPickerKind::WriteSplat => ("PLY", &["ply"][..]),
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
        PathPickerKind::ReadObj => ("OBJ", &["obj"][..], false, "model.obj"),
        PathPickerKind::WriteObj => ("OBJ", &["obj"][..], true, "output.obj"),
        PathPickerKind::ReadSplat => ("PLY", &["ply"][..], false, "splats.ply"),
        PathPickerKind::WriteSplat => ("PLY", &["ply"][..], true, "output.ply"),
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

fn param_row(ui: &mut Ui, label: &str, add_controls: impl FnOnce(&mut Ui) -> bool) -> bool {
    param_row_with_height(ui, label, 36.0, add_controls)
}

fn param_row_with_height(
    ui: &mut Ui,
    label: &str,
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
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(label_rect)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
        |ui| {
            ui.set_min_height(row_height);
            ui.label(label);
        },
    );
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

fn float_slider_range(
    _node_name: &str,
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
        "value_f" => -10.0..=10.0,
        "max_distance" => 0.0..=1000.0,
        "voxel_size" => 0.0..=10.0,
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
        "iterations" => 0..=20,
        "seed" => 0..=100,
        "count" if node_name == "Scatter" => 0..=1000,
        "count" if node_name == "Copy/Transform" => 1..=100,
        "target_count" => 0..=1_000_000,
        _ => -1000..=1000,
    }
}
