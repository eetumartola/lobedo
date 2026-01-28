use egui::Ui;
use std::collections::HashSet;

use lobedo_core::{
    param_specs_for_kind_id, param_specs_for_name, BuiltinNodeKind, Graph, NodeParams, ParamWidget,
};

use super::help::{node_help, show_help_page_window, show_help_tooltip};
use super::params::{edit_group_row, edit_param, edit_param_with_spec};
use super::state::{NodeGraphState, WriteRequest, WriteRequestKind};

impl NodeGraphState {
    pub fn show_inspector(&mut self, ui: &mut Ui, graph: &mut Graph) -> bool {
        if let Some(help_key) = self.help_page_node.clone() {
            let mut open = true;
            show_help_page_window(ui.ctx(), &help_key, &mut open);
            if !open {
                self.help_page_node = None;
            }
        }

        let Some(node_id) = self.selected_node else {
            ui.label("No selection.");
            return false;
        };

        let Some(node) = graph.node(node_id) else {
            self.selected_node = None;
            ui.label("No selection.");
            return false;
        };

        let node_name = node.name.clone();
        let node_kind = node.builtin_kind();
        let node_category = node.category.clone();
        let param_values = node.params.values.clone();
        let visible_params = NodeParams {
            values: param_values.clone(),
        };
        let title = format!("{} ({})", node_name, node_category);
        let mut help_requested = false;
        let header_height = 32.0;
        let help_width = 64.0;
        let total_width = ui.available_width();
        let (row_rect, _) =
            ui.allocate_exact_size(egui::vec2(total_width, header_height), egui::Sense::hover());
        let label_rect = egui::Rect::from_min_size(
            row_rect.min,
            egui::vec2((total_width - help_width).max(0.0), header_height),
        );
        let help_rect = egui::Rect::from_min_size(
            egui::pos2(row_rect.max.x - help_width, row_rect.min.y),
            egui::vec2(help_width, header_height),
        );
        let mut label_response = None;
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(label_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                ui.set_min_height(header_height);
                let response = ui.add(egui::Label::new(title).sense(egui::Sense::hover()));
                label_response = Some(response);
            },
        );
        if let Some(response) = label_response {
            if response.hovered() {
                let help_key = if node.kind_id.is_empty() {
                    node.name.as_str()
                } else {
                    node.kind_id.as_str()
                };
                if let Some(help) = node_help(help_key) {
                    show_help_tooltip(ui.ctx(), response.rect, help);
                }
            }
        }
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(help_rect)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
            |ui| {
                ui.set_min_height(header_height);
                if ui.add_sized([help_width - 4.0, header_height], egui::Button::new("Help")).clicked() {
                    help_requested = true;
                }
            },
        );
        if help_requested {
            self.help_page_node = Some(if node.kind_id.is_empty() {
                node.name.clone()
            } else {
                node.kind_id.clone()
            });
        }

        let mut changed = false;
        let param_specs = if !node.kind_id.is_empty() {
            param_specs_for_kind_id(&node.kind_id)
        } else {
            param_specs_for_name(&node_name)
        };
        let mut spec_keys = HashSet::new();
        let should_skip = |key: &str| -> bool {
            if matches!(node_kind, Some(BuiltinNodeKind::Group | BuiltinNodeKind::Delete))
                && key == "selection"
            {
                return true;
            }
            if node_kind == Some(BuiltinNodeKind::VolumeFromGeometry) && key == "voxel_size" {
                return true;
            }
            false
        };

        let mut rendered_any = false;
        let group_value = param_values.get("group").cloned();
        let group_type_value = param_values.get("group_type").cloned();
        if let Some(group_value) = group_value {
            let group_spec = param_specs.iter().find(|spec| spec.key == "group");
            let group_type_spec = param_specs.iter().find(|spec| spec.key == "group_type");
            let (next_group, next_group_type, did_change) = edit_group_row(
                ui,
                &node_name,
                node_kind,
                group_spec,
                group_type_spec,
                group_value.clone(),
                group_type_value.clone(),
            );
            if did_change {
                if next_group != group_value {
                    let _ = graph.set_param(node_id, "group".to_string(), next_group);
                    changed = true;
                }
                if let Some(next_group_type) = next_group_type.clone() {
                    if group_type_value.as_ref() != Some(&next_group_type) {
                        let _ = graph.set_param(node_id, "group_type".to_string(), next_group_type);
                        changed = true;
                    }
                }
            }
            rendered_any = true;
            spec_keys.insert("group".to_string());
            if group_type_value.is_some() || group_type_spec.is_some() {
                spec_keys.insert("group_type".to_string());
            }
        }
        if !param_specs.is_empty() {
            for spec in &param_specs {
                let Some(value) = param_values.get(spec.key).cloned() else {
                    continue;
                };
                if spec_keys.contains(spec.key) {
                    continue;
                }
                spec_keys.insert(spec.key.to_string());
                if !spec.is_visible(&visible_params) {
                    continue;
                }
                let (next_value, did_change) =
                    edit_param_with_spec(ui, &node_name, node_kind, spec, value);
                if did_change
                    && graph
                        .set_param(node_id, spec.key.to_string(), next_value)
                        .is_ok()
                {
                    changed = true;
                }
                rendered_any = true;
            }
            if param_values.len() > spec_keys.len() {
                ui.separator();
            }
        }

        let mut param_keys: Vec<String> = param_values
            .keys()
            .filter(|key| !spec_keys.contains(*key))
            .cloned()
            .collect();
        param_keys.sort_by(|a, b| {
            let priority = |key: &str| match key {
                "group" => 0,
                "group_type" => 1,
                _ => 2,
            };
            let pa = priority(a);
            let pb = priority(b);
            pa.cmp(&pb).then_with(|| a.cmp(b))
        });

        if param_keys.is_empty() && !rendered_any {
            ui.label("No parameters.");
            return false;
        }

        for key in param_keys {
            let Some(value) = param_values.get(&key).cloned() else {
                continue;
            };
            if should_skip(&key) {
                continue;
            }
            let (next_value, did_change) = edit_param(ui, &node_name, node_kind, &key, value);
            if did_change && graph.set_param(node_id, key, next_value).is_ok() {
                changed = true;
            }
        }

        if matches!(
            node_kind,
            Some(BuiltinNodeKind::ObjOutput | BuiltinNodeKind::GltfOutput | BuiltinNodeKind::WriteSplats)
        ) {
            ui.separator();
            let label = if node_kind == Some(BuiltinNodeKind::ObjOutput) {
                "Write OBJ"
            } else if node_kind == Some(BuiltinNodeKind::GltfOutput) {
                "Write GLTF"
            } else {
                "Write PLY"
            };
            let can_write = !cfg!(target_arch = "wasm32");
            if ui.add_enabled(can_write, egui::Button::new(label)).clicked() {
                let kind = if node_kind == Some(BuiltinNodeKind::ObjOutput) {
                    WriteRequestKind::Obj
                } else if node_kind == Some(BuiltinNodeKind::GltfOutput) {
                    WriteRequestKind::Gltf
                } else {
                    WriteRequestKind::Splat
                };
                self.pending_write_request = Some(WriteRequest { node_id, kind });
            }
            if !can_write {
                ui.label("Writing is not available in web builds.");
            }
        }

        changed
    }

    pub fn inspector_desired_height(&self, graph: &Graph) -> f32 {
        let row_height = 36.0;
        let separator_height = 8.0;
        let item_spacing = 6.0;
        let Some(node_id) = self.selected_node else {
            return row_height;
        };
        let Some(node) = graph.node(node_id) else {
            return row_height;
        };

        let node_name = node.name.clone();
        let node_kind = node.builtin_kind();
        let param_values = node.params.values.clone();
        let visible_params = NodeParams {
            values: param_values.clone(),
        };
        let param_specs = if !node.kind_id.is_empty() {
            param_specs_for_kind_id(&node.kind_id)
        } else {
            param_specs_for_name(&node_name)
        };

        let mut heights = Vec::new();
        let mut spec_keys = HashSet::new();
        let should_skip = |key: &str| -> bool {
            if matches!(node_kind, Some(BuiltinNodeKind::Group | BuiltinNodeKind::Delete))
                && key == "selection"
            {
                return true;
            }
            if node_kind == Some(BuiltinNodeKind::VolumeFromGeometry) && key == "voxel_size" {
                return true;
            }
            false
        };

        let group_value = param_values.get("group").cloned();
        let group_type_value = param_values.get("group_type").cloned();
        if group_value.is_some() {
            heights.push(row_height);
            spec_keys.insert("group".to_string());
            if group_type_value.is_some()
                || param_specs.iter().any(|spec| spec.key == "group_type")
            {
                spec_keys.insert("group_type".to_string());
            }
        }

        let row_height_for_spec = |spec: &lobedo_core::ParamSpec| match spec.widget {
            ParamWidget::Gradient => 112.0,
            ParamWidget::Code => 120.0,
            _ => row_height,
        };

        if !param_specs.is_empty() {
            for spec in &param_specs {
                let Some(_value) = param_values.get(spec.key) else {
                    continue;
                };
                if spec_keys.contains(spec.key) {
                    continue;
                }
                spec_keys.insert(spec.key.to_string());
                if !spec.is_visible(&visible_params) {
                    continue;
                }
                heights.push(row_height_for_spec(spec));
            }
            if param_values.len() > spec_keys.len() {
                heights.push(separator_height);
            }
        }

        let mut param_keys: Vec<String> = param_values
            .keys()
            .filter(|key| !spec_keys.contains(*key))
            .cloned()
            .collect();
        param_keys.sort_by(|a, b| {
            let priority = |key: &str| match key {
                "group" => 0,
                "group_type" => 1,
                _ => 2,
            };
            let pa = priority(a);
            let pb = priority(b);
            pa.cmp(&pb).then_with(|| a.cmp(b))
        });
        for key in param_keys {
            if should_skip(&key) {
                continue;
            }
            heights.push(row_height);
        }

        if matches!(
            node_kind,
            Some(
                BuiltinNodeKind::ObjOutput
                    | BuiltinNodeKind::GltfOutput
                    | BuiltinNodeKind::WriteSplats
            )
        ) {
            heights.push(separator_height);
            heights.push(row_height);
            if cfg!(target_arch = "wasm32") {
                heights.push(row_height);
            }
        }

        if heights.is_empty() {
            return row_height;
        }

        let rows = heights.len() as f32;
        heights.iter().sum::<f32>() + item_spacing * (rows - 1.0).max(0.0)
    }
}

