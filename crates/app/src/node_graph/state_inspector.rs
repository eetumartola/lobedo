use egui::Ui;

use lobedo_core::{Graph, ParamValue};

use super::params::edit_param;
use super::state::NodeGraphState;

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

        ui.label(format!("{} ({})", node.name, node.category));
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

        if params.is_empty() {
            ui.label("No parameters.");
            return false;
        }

        let mut changed = false;
        for (key, value) in params {
            if matches!(node_name.as_str(), "Group" | "Delete") {
                if let Some(shape) = shape.as_deref() {
                    match key.as_str() {
                        "size" if shape != "box" && shape != "sphere" => continue,
                        "radius" => continue,
                        "center" if shape == "plane" || shape == "group" => continue,
                        "plane_origin" | "plane_normal" if shape != "plane" => continue,
                        _ => {}
                    }
                }
            }
            let (next_value, did_change) = edit_param(ui, &node_name, &key, value);
            if did_change && graph.set_param(node_id, key, next_value).is_ok() {
                changed = true;
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
