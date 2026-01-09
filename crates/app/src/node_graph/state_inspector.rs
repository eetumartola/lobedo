use egui::Ui;

use lobedo_core::{Graph, ParamValue};

use super::help::{node_help, show_help_tooltip};
use super::params::edit_param;
use super::state::{NodeGraphState, WriteRequest, WriteRequestKind};

impl NodeGraphState {
    pub fn show_inspector(&mut self, ui: &mut Ui, graph: &mut Graph) -> bool {
        let Some(node_id) = self.selected_node else {
            ui.label("No selection.");
            return false;
        };

        let Some(node) = graph.node(node_id) else {
            self.selected_node = None;
            ui.label("No selection.");
            return false;
        };

        let title = format!("{} ({})", node.name, node.category);
        let response = ui.add(egui::Label::new(title).sense(egui::Sense::hover()));
        if response.hovered() {
            if let Some(help) = node_help(&node.name) {
                show_help_tooltip(ui.ctx(), response.rect, help);
            }
        }
        ui.separator();

        let params: Vec<(String, ParamValue)> = node
            .params
            .values
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        let node_name = node.name.clone();
        let shape = node
            .params
            .values
            .get("shape")
            .and_then(|value| match value {
                ParamValue::String(value) => Some(value.to_lowercase()),
                _ => None,
            });
        let splat_to_mesh_method = if node_name == "Splat to Mesh" {
            Some(node.params.get_int("algorithm", 0).clamp(0, 1))
        } else {
            None
        };
        let volume_to_mesh_mode = if node_name == "Volume to Mesh" {
            Some(node.params.get_string("mode", "density").to_lowercase())
        } else {
            None
        };

        if params.is_empty() {
            ui.label("No parameters.");
            return false;
        }

        let mut changed = false;
        for (key, value) in params {
            if matches!(node_name.as_str(), "Group" | "Delete") {
                if key == "selection" {
                    continue;
                }
                if let Some(shape) = shape.as_deref() {
                    match key.as_str() {
                        "size" if shape != "box" && shape != "sphere" => continue,
                        "radius" => continue,
                        "center" if shape == "plane" || shape == "selection" => continue,
                        "plane_origin" | "plane_normal" if shape != "plane" => continue,
                        "select_backface" if shape != "selection" => continue,
                        _ => {}
                    }
                }
            }
            if node_name == "Volume from Geometry" && key == "voxel_size" {
                continue;
            }
            if node_name == "Splat to Mesh" {
                if let Some(method) = splat_to_mesh_method {
                    match (method, key.as_str()) {
                        (0, "surface_iso") | (0, "smooth_k") | (0, "shell_radius") => continue,
                        (1, "density_iso") | (1, "blur_iters") => continue,
                        _ => {}
                    }
                }
            }
            if node_name == "Volume to Mesh" {
                if let Some(mode) = volume_to_mesh_mode.as_deref() {
                    let is_density = !mode.contains("sdf");
                    match (is_density, key.as_str()) {
                        (true, "surface_iso") => continue,
                        (false, "density_iso") => continue,
                        _ => {}
                    }
                }
            }
            let (next_value, did_change) = edit_param(ui, &node_name, &key, value);
            if did_change && graph.set_param(node_id, key, next_value).is_ok() {
                changed = true;
            }
        }

        if matches!(node_name.as_str(), "OBJ Output" | "Splat Write" | "Write Splats") {
            ui.separator();
            let label = if node_name == "OBJ Output" {
                "Write OBJ"
            } else {
                "Write PLY"
            };
            let can_write = !cfg!(target_arch = "wasm32");
            if ui.add_enabled(can_write, egui::Button::new(label)).clicked() {
                let kind = if node_name == "OBJ Output" {
                    WriteRequestKind::Obj
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

    pub fn inspector_row_count(&self, graph: &Graph) -> usize {
        let Some(node_id) = self.selected_node else {
            return 1;
        };
        let Some(node) = graph.node(node_id) else {
            return 1;
        };
        let count = node.params.values.len();
        if count == 0 {
            1
        } else {
            count
        }
    }
}
