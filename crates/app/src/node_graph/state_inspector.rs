use egui::Ui;
use std::collections::HashSet;

use lobedo_core::{
    param_specs_for_kind_id, param_specs_for_name, BuiltinNodeKind, Graph, ParamValue,
};

use super::help::{node_help, show_help_page_window, show_help_tooltip};
use super::params::{edit_param, edit_param_with_spec};
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
        let node_kind = node.builtin_kind();
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

        let mut changed = false;
        let param_specs = if !node.kind_id.is_empty() {
            param_specs_for_kind_id(&node.kind_id)
        } else {
            param_specs_for_name(&node_name)
        };
        let mut spec_keys = HashSet::new();
        let shape = param_values.get("shape").and_then(|value| match value {
            ParamValue::String(value) => Some(value.to_lowercase()),
            _ => None,
        });
        let color_mode = if node_kind == Some(BuiltinNodeKind::Color) {
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
        let ray_method = if node_kind == Some(BuiltinNodeKind::Ray) {
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
        let volume_from_mode = if node_kind == Some(BuiltinNodeKind::VolumeFromGeometry) {
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
        let splat_to_mesh_method = if node_kind == Some(BuiltinNodeKind::SplatToMesh) {
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
        let splat_to_mesh_output = if node_kind == Some(BuiltinNodeKind::SplatToMesh) {
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
        let splat_delight_mode = if node_kind == Some(BuiltinNodeKind::SplatDelight) {
            Some(
                param_values
                    .get("delight_mode")
                    .and_then(|value| match value {
                        ParamValue::Int(value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or(1)
                    .clamp(0, 3),
            )
        } else {
            None
        };
        let splat_delight_source_env = if node_kind == Some(BuiltinNodeKind::SplatDelight) {
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
        let splat_delight_neutral_env = if node_kind == Some(BuiltinNodeKind::SplatDelight) {       
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
        let splat_integrate_mode = if node_kind == Some(BuiltinNodeKind::SplatIntegrate) {
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
        let splat_integrate_source_env = if node_kind == Some(BuiltinNodeKind::SplatIntegrate) {
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
        let splat_integrate_target_env = if node_kind == Some(BuiltinNodeKind::SplatIntegrate) {
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
        let splat_merge_method = if node_kind == Some(BuiltinNodeKind::SplatMerge) {
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
        let splat_heal_method = if node_kind == Some(BuiltinNodeKind::SplatHeal) {
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
        let splat_cluster_method = if node_kind == Some(BuiltinNodeKind::SplatCluster) {
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
        let volume_to_mesh_mode = if node_kind == Some(BuiltinNodeKind::VolumeToMesh) {
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
        let attr_promote_rename = if node_kind == Some(BuiltinNodeKind::AttributePromote) {
            Some(
                param_values
                    .get("rename")
                    .and_then(|value| match value {
                        ParamValue::Bool(value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or(false),
            )
        } else {
            None
        };
        let attr_noise_type = if node_kind == Some(BuiltinNodeKind::AttributeNoise) {
            Some(
                param_values
                    .get("noise_type")
                    .and_then(|value| match value {
                        ParamValue::Int(value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or(0),
            )
        } else {
            None
        };
        let attr_noise_fractal = if node_kind == Some(BuiltinNodeKind::AttributeNoise) {
            Some(
                param_values
                    .get("fractal_type")
                    .and_then(|value| match value {
                        ParamValue::Int(value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or(1),
            )
        } else {
            None
        };

        let should_skip = |key: &str| -> bool {
            if node_kind == Some(BuiltinNodeKind::Ffd) {
                if key == "lattice_points" {
                    return true;
                }
                if matches!(key, "center" | "size") {
                    let use_input_bounds = param_values
                        .get("use_input_bounds")
                        .and_then(|value| match value {
                            ParamValue::Bool(value) => Some(*value),
                            _ => None,
                        })
                        .unwrap_or(true);
                    if use_input_bounds {
                        return true;
                    }
                }
            }
            if matches!(node_kind, Some(BuiltinNodeKind::Group | BuiltinNodeKind::Delete)) {
                if key == "selection" {
                    return true;
                }
                if let Some(shape) = shape.as_deref() {
                    match key {
                        "size" if shape != "box" && shape != "sphere" => return true,
                        "radius" => return true,
                        "center"
                            if shape == "plane"
                                || shape == "selection"
                                || shape == "attribute" =>
                        {
                            return true
                        }
                        "plane_origin" | "plane_normal" if shape != "plane" => return true,
                        "select_backface" if shape != "selection" => return true,
                        "attr" | "attr_min" | "attr_max" if shape != "attribute" => return true,
                        _ => {}
                    }
                }
            }
            if node_kind == Some(BuiltinNodeKind::VolumeFromGeometry) && key == "voxel_size" {
                return true;
            }
            if node_kind == Some(BuiltinNodeKind::VolumeFromGeometry) {
                if let Some(mode) = volume_from_mode.as_deref() {
                    let is_density = !mode.contains("sdf");
                    match (is_density, key) {
                        (true, "sdf_band") => return true,
                        (false, "density_scale") => return true,
                        _ => {}
                    }
                }
            }
            if node_kind == Some(BuiltinNodeKind::SplatToMesh) {
                if let Some(output) = splat_to_mesh_output {
                    if output == 1 {
                        match key {
                            "algorithm"
                            | "density_iso"
                            | "surface_iso"
                            | "transfer_color"
                            | "blur_iters" => return true,
                            _ => {}
                        }
                    }
                }
                if let Some(method) = splat_to_mesh_method {
                    match (method, key) {
                        (0, "surface_iso") | (0, "smooth_k") | (0, "shell_radius") => {
                            return true
                        }
                        (1, "density_iso") | (1, "blur_iters") => return true,
                        _ => {}
                    }
                }
            }
            if node_kind == Some(BuiltinNodeKind::VolumeToMesh) {
                if let Some(mode) = volume_to_mesh_mode.as_deref() {
                    let is_density = !mode.contains("sdf");
                    match (is_density, key) {
                        (true, "surface_iso") => return true,
                        (false, "density_iso") => return true,
                        _ => {}
                    }
                }
            }
            if node_kind == Some(BuiltinNodeKind::Color) {
                if let Some(mode) = color_mode {
                    match (mode, key) {
                        (0, "attr") | (0, "gradient") => return true,
                        (1, "color") => return true,
                        _ => {}
                    }
                }
            }
            if node_kind == Some(BuiltinNodeKind::Ray) {
                if let Some(method) = ray_method {
                    if method != 1 && key == "direction" {
                        return true;
                    }
                }
            }
            if node_kind == Some(BuiltinNodeKind::SplatDelight) {
                if let Some(mode) = splat_delight_mode {
                    match (mode, key) {
                        (0, "source_env")
                        | (0, "neutral_env")
                        | (0, "source_color")
                        | (0, "neutral_color")
                        | (0, "eps")
                        | (0, "ratio_min")
                        | (0, "ratio_max")
                        | (0, "high_band_gain")
                        | (0, "albedo_max") => return true,
                        (1, "albedo_max") => return true,
                        (2, "neutral_env")
                        | (2, "neutral_color")
                        | (2, "ratio_min")
                        | (2, "ratio_max") => return true,
                        (3, "source_env")
                        | (3, "source_color")
                        | (3, "albedo_max") => return true,
                        _ => {}
                    }
                }
                if let Some(source_env) = splat_delight_source_env {
                    if source_env != 2 && key == "source_color" {
                        return true;
                    }
                }
                if let Some(neutral_env) = splat_delight_neutral_env {
                    if neutral_env != 1 && key == "neutral_color" {
                        return true;
                    }
                }
            }
            if node_kind == Some(BuiltinNodeKind::SplatIntegrate) {
                if let Some(mode) = splat_integrate_mode {
                    match (mode, key) {
                        (0, "albedo_max") | (0, "high_band_mode") => return true,
                        (1, "source_env")
                        | (1, "source_color")
                        | (1, "eps")
                        | (1, "ratio_min")
                        | (1, "ratio_max")
                        | (1, "high_band_gain")
                        | (1, "high_band_mode") => return true,
                        _ => {}
                    }
                }
                if let Some(source_env) = splat_integrate_source_env {
                    if source_env != 2 && key == "source_color" {
                        return true;
                    }
                }
                if let Some(target_env) = splat_integrate_target_env {
                    if target_env != 2 && key == "target_color" {
                        return true;
                    }
                }
            }
            if node_kind == Some(BuiltinNodeKind::SplatMerge) {
                if let Some(method) = splat_merge_method {
                    match (method, key) {
                        (0, "skirt_max_dist")
                        | (0, "skirt_step")
                        | (0, "skirt_max_new")
                        | (0, "seam_alpha")
                        | (0, "seam_scale")
                        | (0, "seam_dc_only")
                        | (0, "preview_skirt") => return true,
                        _ => {}
                    }
                }
            }
            if node_kind == Some(BuiltinNodeKind::SplatHeal) {
                if let Some(method) = splat_heal_method {
                    match (method, key) {
                        (0, "sdf_band")
                        | (0, "sdf_close")
                        | (0, "smooth_k")
                        | (0, "shell_radius") => return true,
                        (1, "close_radius") => return true,
                        _ => {}
                    }
                }
                if matches!(key, "heal_center" | "heal_size") {
                    return true;
                }
            }
            if node_kind == Some(BuiltinNodeKind::SplatCluster) {
                if let Some(method) = splat_cluster_method {
                    match (method, key) {
                        (0, "eps") | (0, "min_pts") => return true,
                        (1, "cell_size") => return true,
                        _ => {}
                    }
                }
            }
            if node_kind == Some(BuiltinNodeKind::AttributePromote) {
                if let Some(rename) = attr_promote_rename {
                    if !rename && key == "new_name" {
                        return true;
                    }
                }
            }
            if node_kind == Some(BuiltinNodeKind::AttributeNoise) {
                if let Some(fractal) = attr_noise_fractal {
                    if fractal == 0
                        && matches!(key, "octaves" | "lacunarity" | "roughness")
                    {
                        return true;
                    }
                }
                if let Some(noise_type) = attr_noise_type {
                    if noise_type != 4 && key == "flow_rotation" {
                        return true;
                    }
                    if !matches!(noise_type, 12 | 13) && key == "distortion" {
                        return true;
                    }
                }
            }
            false
        };

        let mut rendered_any = false;
        if !param_specs.is_empty() {
            for spec in &param_specs {
                let Some(value) = param_values.get(spec.key).cloned() else {
                    continue;
                };
                spec_keys.insert(spec.key.to_string());
                if should_skip(spec.key) {
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

