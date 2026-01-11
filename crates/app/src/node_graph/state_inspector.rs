use egui::Ui;

use lobedo_core::{Graph, ParamValue};

use super::help::{node_help, show_help_page_window, show_help_tooltip};
use super::params::edit_param;
use super::state::{NodeGraphState, WriteRequest, WriteRequestKind};

impl NodeGraphState {
    pub fn show_inspector(&mut self, ui: &mut Ui, graph: &mut Graph) -> bool {
        if let Some(help_name) = self.help_page_node.clone() {
            let mut open = true;
            show_help_page_window(ui.ctx(), &help_name, &mut open);
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
        let node_category = node.category.clone();
        let param_values = node.params.values.clone();
        let title = format!("{} ({})", node_name, node_category);
        let mut help_requested = false;
        ui.horizontal(|ui| {
            let response =
                ui.add(egui::Label::new(title).sense(egui::Sense::hover()));
            if response.hovered() {
                if let Some(help) = node_help(&node.name) {
                    show_help_tooltip(ui.ctx(), response.rect, help);
                }
            }
            let available = ui.available_width();
            ui.allocate_ui_with_layout(
                egui::vec2(available, 0.0),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    if ui.button("Help").clicked() {
                        help_requested = true;
                    }
                },
            );
        });
        if help_requested {
            self.help_page_node = Some(node.name.clone());
        }
        ui.separator();

        let mut param_keys: Vec<String> = param_values.keys().cloned().collect();
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
        let shape = param_values.get("shape").and_then(|value| match value {
            ParamValue::String(value) => Some(value.to_lowercase()),
            _ => None,
        });
        let color_mode = if node_name == "Color" {
            Some(
                param_values
                    .get("color_mode")
                    .and_then(|value| match value {
                        ParamValue::Int(value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or(0)
                    .clamp(0, 1),
            )
        } else {
            None
        };
        let ray_method = if node_name == "Ray" {
            Some(
                param_values
                    .get("method")
                    .and_then(|value| match value {
                        ParamValue::Int(value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or(0)
                    .clamp(0, 2),
            )
        } else {
            None
        };
        let volume_from_mode = if node_name == "Volume from Geometry" {
            Some(
                param_values
                    .get("mode")
                    .and_then(|value| match value {
                        ParamValue::String(value) => Some(value.to_lowercase()),
                        _ => None,
                    })
                    .unwrap_or_else(|| "density".to_string()),
            )
        } else {
            None
        };
        let splat_to_mesh_method = if node_name == "Splat to Mesh" {
            Some(
                param_values
                    .get("algorithm")
                    .and_then(|value| match value {
                        ParamValue::Int(value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or(0)
                    .clamp(0, 1),
            )
        } else {
            None
        };
        let splat_to_mesh_output = if node_name == "Splat to Mesh" {
            Some(
                param_values
                    .get("output")
                    .and_then(|value| match value {
                        ParamValue::Int(value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or(0)
                    .clamp(0, 1),
            )
        } else {
            None
        };
        let splat_delight_mode = if node_name == "Splat Delight" {
            Some(
                param_values
                    .get("delight_mode")
                    .and_then(|value| match value {
                        ParamValue::Int(value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or(1)
                    .clamp(0, 2),
            )
        } else {
            None
        };
        let splat_delight_source_env = if node_name == "Splat Delight" {
            Some(
                param_values
                    .get("source_env")
                    .and_then(|value| match value {
                        ParamValue::Int(value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or(0)
                    .clamp(0, 2),
            )
        } else {
            None
        };
        let splat_delight_neutral_env = if node_name == "Splat Delight" {       
            Some(
                param_values
                    .get("neutral_env")
                    .and_then(|value| match value {
                        ParamValue::Int(value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or(0)
                    .clamp(0, 1),
            )
        } else {
            None
        };
        let splat_integrate_mode = if node_name == "Splat Integrate" {
            Some(
                param_values
                    .get("relight_mode")
                    .and_then(|value| match value {
                        ParamValue::Int(value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or(2)
                    .clamp(0, 2),
            )
        } else {
            None
        };
        let splat_integrate_source_env = if node_name == "Splat Integrate" {
            Some(
                param_values
                    .get("source_env")
                    .and_then(|value| match value {
                        ParamValue::Int(value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or(0)
                    .clamp(0, 2),
            )
        } else {
            None
        };
        let splat_integrate_target_env = if node_name == "Splat Integrate" {
            Some(
                param_values
                    .get("target_env")
                    .and_then(|value| match value {
                        ParamValue::Int(value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or(0)
                    .clamp(0, 2),
            )
        } else {
            None
        };
        let splat_merge_method = if node_name == "Splat Merge" {
            Some(
                param_values
                    .get("method")
                    .and_then(|value| match value {
                        ParamValue::Int(value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or(0)
                    .clamp(0, 1),
            )
        } else {
            None
        };
        let splat_heal_method = if node_name == "Splat Heal" {
            Some(
                param_values
                    .get("method")
                    .and_then(|value| match value {
                        ParamValue::Int(value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or(0)
                    .clamp(0, 1),
            )
        } else {
            None
        };
        let splat_cluster_method = if node_name == "Splat Cluster" {
            Some(
                param_values
                    .get("method")
                    .and_then(|value| match value {
                        ParamValue::Int(value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or(0)
                    .clamp(0, 1),
            )
        } else {
            None
        };
        let volume_to_mesh_mode = if node_name == "Volume to Mesh" {
            Some(
                param_values
                    .get("mode")
                    .and_then(|value| match value {
                        ParamValue::String(value) => Some(value.to_lowercase()),
                        _ => None,
                    })
                    .unwrap_or_else(|| "density".to_string()),
            )
        } else {
            None
        };

        if param_keys.is_empty() {
            ui.label("No parameters.");
            return false;
        }

        let mut changed = false;
        for key in param_keys {
            let Some(value) = param_values.get(&key).cloned() else {
                continue;
            };
            if node_name == "FFD" {
                if key == "lattice_points" {
                    continue;
                }
                if matches!(key.as_str(), "center" | "size") {
                    let use_input_bounds = param_values
                        .get("use_input_bounds")
                        .and_then(|value| match value {
                            ParamValue::Bool(value) => Some(*value),
                            _ => None,
                        })
                        .unwrap_or(true);
                    if use_input_bounds {
                        continue;
                    }
                }
            }
            if matches!(node_name.as_str(), "Group" | "Delete") {
                if key == "selection" {
                    continue;
                }
                if let Some(shape) = shape.as_deref() {
                    match key.as_str() {
                        "size" if shape != "box" && shape != "sphere" => continue,
                        "radius" => continue,
                        "center" if shape == "plane" || shape == "selection" || shape == "attribute" => continue,
                        "plane_origin" | "plane_normal" if shape != "plane" => continue,
                        "select_backface" if shape != "selection" => continue,
                        "attr" | "attr_min" | "attr_max" if shape != "attribute" => continue,
                        _ => {}
                    }
                }
            }
            if node_name == "Volume from Geometry" && key == "voxel_size" {
                continue;
            }
            if node_name == "Volume from Geometry" {
                if let Some(mode) = volume_from_mode.as_deref() {
                    let is_density = !mode.contains("sdf");
                    match (is_density, key.as_str()) {
                        (true, "sdf_band") => continue,
                        (false, "density_scale") => continue,
                        _ => {}
                    }
                }
            }
            if node_name == "Splat to Mesh" {
                if let Some(output) = splat_to_mesh_output {
                    if output == 1 {
                        match key.as_str() {
                            "algorithm"
                            | "density_iso"
                            | "surface_iso"
                            | "transfer_color"
                            | "blur_iters" => continue,
                            _ => {}
                        }
                    }
                }
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
            if node_name == "Color" {
                if let Some(mode) = color_mode {
                    match (mode, key.as_str()) {
                        (0, "attr") | (0, "gradient") => continue,
                        (1, "color") => continue,
                        _ => {}
                    }
                }
            }
            if node_name == "Ray" {
                if let Some(method) = ray_method {
                    if method != 1 && key == "direction" {
                        continue;
                    }
                }
            }
            if node_name == "Splat Delight" {
                if let Some(mode) = splat_delight_mode {
                    match (mode, key.as_str()) {
                        (0, "source_env")
                        | (0, "neutral_env")
                        | (0, "source_color")
                        | (0, "neutral_color")
                        | (0, "eps")
                        | (0, "ratio_min")
                        | (0, "ratio_max")
                        | (0, "high_band_gain")
                        | (0, "albedo_max") => continue,
                        (1, "albedo_max") => continue,
                        (2, "neutral_env")
                        | (2, "neutral_color")
                        | (2, "ratio_min")
                        | (2, "ratio_max") => continue,
                        _ => {}
                    }
                }
                if let Some(source_env) = splat_delight_source_env {
                    if source_env != 2 && key == "source_color" {
                        continue;
                    }
                }
                if let Some(neutral_env) = splat_delight_neutral_env {
                    if neutral_env != 1 && key == "neutral_color" {
                        continue;
                    }
                }
            }
            if node_name == "Splat Integrate" {
                if let Some(mode) = splat_integrate_mode {
                    match (mode, key.as_str()) {
                        (0, "albedo_max") | (0, "high_band_mode") => continue,
                        (1, "source_env")
                        | (1, "source_color")
                        | (1, "eps")
                        | (1, "ratio_min")
                        | (1, "ratio_max")
                        | (1, "high_band_gain")
                        | (1, "high_band_mode") => continue,
                        _ => {}
                    }
                }
                if let Some(source_env) = splat_integrate_source_env {
                    if source_env != 2 && key == "source_color" {
                        continue;
                    }
                }
                if let Some(target_env) = splat_integrate_target_env {
                    if target_env != 2 && key == "target_color" {
                        continue;
                    }
                }
            }
            if node_name == "Splat Merge" {
                if let Some(method) = splat_merge_method {
                    match (method, key.as_str()) {
                        (0, "skirt_max_dist")
                        | (0, "skirt_step")
                        | (0, "skirt_max_new")
                        | (0, "seam_alpha")
                        | (0, "seam_scale")
                        | (0, "seam_dc_only") => continue,
                        _ => {}
                    }
                }
            }
            if node_name == "Splat Heal" {
                if let Some(method) = splat_heal_method {
                    match (method, key.as_str()) {
                        (0, "sdf_band")
                        | (0, "sdf_close")
                        | (0, "smooth_k")
                        | (0, "shell_radius") => continue,
                        (1, "close_radius") => continue,
                        _ => {}
                    }
                }
            }
            if node_name == "Splat Cluster" {
                if let Some(method) = splat_cluster_method {
                    match (method, key.as_str()) {
                        (0, "eps") | (0, "min_pts") => continue,
                        (1, "cell_size") => continue,
                        _ => {}
                    }
                }
            }
            let (next_value, did_change) = edit_param(ui, &node_name, &key, value);
            if did_change && graph.set_param(node_id, key, next_value).is_ok() {
                changed = true;
            }
        }

        if matches!(
            node_name.as_str(),
            "OBJ Output" | "GLTF Output" | "Splat Write" | "Write Splats"
        ) {
            ui.separator();
            let label = if node_name == "OBJ Output" {
                "Write OBJ"
            } else if node_name == "GLTF Output" {
                "Write GLTF"
            } else {
                "Write PLY"
            };
            let can_write = !cfg!(target_arch = "wasm32");
            if ui.add_enabled(can_write, egui::Button::new(label)).clicked() {
                let kind = if node_name == "OBJ Output" {
                    WriteRequestKind::Obj
                } else if node_name == "GLTF Output" {
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
