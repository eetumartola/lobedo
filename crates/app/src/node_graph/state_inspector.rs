use egui::Ui;
use serde_json::json;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::TryRecvError;

use lobedo_core::{
    param_specs_for_kind_id, param_specs_for_name, BuiltinNodeKind, Graph, NodeParams, ParamValue,
    ParamWidget,
};

use super::help::{node_help, show_help_page_window, show_help_tooltip};
use super::params::{edit_group_row, edit_param, edit_param_with_spec};
use super::state::{ModelDownloadResult, NodeGraphState, WriteRequest, WriteRequestKind};

const DEPTHPRO_MODEL_URL: &str = "https://huggingface.co/Jens-Duttke/DepthPro-ONNX-HighPerf/blob/main/depthpro_1536x1536_bs1_fp16_opset21_optimized.onnx";
const DEPTHPRO_MODEL_FILENAME: &str = "depthpro_1536x1536_bs1_fp16_opset21_optimized.onnx";
const DEPTHPRO_MODEL_DIR: &str = "models/depthpro";
const SAM_ENCODER_URL: &str =
    "https://huggingface.co/visheratin/segment-anything-vit-b/blob/main/encoder.onnx";
const SAM_DECODER_URL: &str =
    "https://huggingface.co/visheratin/segment-anything-vit-b/blob/main/decoder.onnx";
const SAM_MODEL_DIR: &str = "models/sam";
const SAM_ENCODER_FILENAME: &str = "encoder.onnx";
const SAM_DECODER_FILENAME: &str = "decoder.onnx";
const ONNX_RUNTIME_URL: &str = "https://files.pythonhosted.org/packages/c0/b4/569d298f9fc4d286c11c45e85d9ffa9e877af12ace98af8cab52396e8f46/onnxruntime-1.23.2-cp312-cp312-win_amd64.whl";
const ONNX_DIRECTML_PYPI: &str = "https://pypi.org/pypi/onnxruntime-directml/json";
const ONNX_RUNTIME_DIR: &str = "models/onnxruntime";
const ONNX_DIRECTML_DIR: &str = "models/onnxruntime-directml";
const ONNX_RUNTIME_DLL: &str = "onnxruntime.dll";

impl NodeGraphState {
    pub fn show_inspector(
        &mut self,
        ui: &mut Ui,
        graph: &mut Graph,
        eval_state: Option<&lobedo_core::GeometryEvalState>,
    ) -> bool {
        self.poll_model_download();
        self.poll_sam_download();
        self.poll_runtime_download();
        self.poll_directml_download();
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

        let Some(node) = graph.node(node_id).cloned() else {
            self.selected_node = None;
            ui.label("No selection.");
            return false;
        };

        let node_name = node.name.clone();
        let node_kind = node.builtin_kind();
        let node_category = node.category.clone();
        let mut param_values = node.params.values.clone();
        let mut auto_changed = false;
        if node_kind == Some(BuiltinNodeKind::WorldLabsGenerate) {
            if !param_values.contains_key("io_mode") {
                let _ = graph.set_param(
                    node_id,
                    "io_mode".to_string(),
                    ParamValue::Int(0),
                );
                param_values.insert("io_mode".to_string(), ParamValue::Int(0));
                auto_changed = true;
            }
            if !param_values.contains_key("load_model") {
                let _ = graph.set_param(
                    node_id,
                    "load_model".to_string(),
                    ParamValue::String(String::new()),
                );
                param_values.insert(
                    "load_model".to_string(),
                    ParamValue::String(String::new()),
                );
                auto_changed = true;
            }
        }
        let visible_params = NodeParams {
            values: param_values.clone(),
        };
        let title = format!("{node_name} ({node_category})");
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

        let mut changed = auto_changed;
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

        if node_kind == Some(BuiltinNodeKind::DepthImage) {
            ui.separator();
            let runtime_dir = onnxruntime_dir_path();
            let runtime_path = onnxruntime_dylib_path();
            let directml_dir = onnxruntime_directml_dir_path();
            let directml_path = onnxruntime_directml_dylib_path();
            let runtime_exists = {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    runtime_path.exists()
                }
                #[cfg(target_arch = "wasm32")]
                {
                    false
                }
            };
            let directml_exists = {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    directml_path.exists()
                }
                #[cfg(target_arch = "wasm32")]
                {
                    false
                }
            };
            let can_runtime_download =
                !cfg!(target_arch = "wasm32") && cfg!(target_os = "windows");
            let runtime_label = if runtime_exists {
                "Re-download ONNX Runtime"
            } else {
                "Download ONNX Runtime"
            };
            if ui
                .add_enabled(
                    can_runtime_download && !self.runtime_download.active,
                    egui::Button::new(runtime_label),
                )
                .clicked()
            {
                self.start_onnxruntime_download(runtime_dir);
            }
            let directml_label = if directml_exists {
                "Re-download DirectML Runtime"
            } else {
                "Download DirectML Runtime"
            };
            if ui
                .add_enabled(
                    can_runtime_download && !self.directml_download.active,
                    egui::Button::new(directml_label),
                )
                .clicked()
            {
                self.start_directml_download(directml_dir);
            }
            let runtime_status = if self.runtime_download.active {
                self.runtime_download
                    .message
                    .clone()
                    .unwrap_or_else(|| "Downloading ONNX Runtime...".to_string())
            } else if let Some(message) = self.runtime_download.message.as_ref() {
                message.clone()
            } else if let Some(message) = self.directml_download.message.as_ref() {
                message.clone()
            } else if !can_runtime_download {
                "Runtime downloads are only available on Windows desktop builds.".to_string()
            } else {
                String::new()
            };
            if !runtime_status.is_empty() {
                ui.label(runtime_status);
            }

            let model_path = depthpro_model_path();
            let model_exists = {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    model_path.exists()
                }
                #[cfg(target_arch = "wasm32")]
                {
                    false
                }
            };
            let can_download = !cfg!(target_arch = "wasm32");
            let label = if model_exists {
                "Re-download Model"
            } else {
                "Download Model"
            };
            if ui
                .add_enabled(
                    can_download && !self.model_download.active,
                    egui::Button::new(label),
                )
                .clicked()
            {
                self.start_depthpro_download(model_path);
            }
            let status = if self.model_download.active {
                self.model_download
                    .message
                    .clone()
                    .unwrap_or_else(|| "Downloading DepthPro model...".to_string())
            } else if let Some(message) = self.model_download.message.as_ref() {
                message.clone()
            } else if !can_download {
                "Model downloads are not available in web builds.".to_string()
            } else {
                String::new()
            };
            if !status.is_empty() {
                ui.label(status);
            }

            let sam_encoder = sam_encoder_path();
            let sam_decoder = sam_decoder_path();
            let sam_exists = {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    sam_encoder.exists() && sam_decoder.exists()
                }
                #[cfg(target_arch = "wasm32")]
                {
                    false
                }
            };
            let sam_label = if sam_exists {
                "Re-download SAM Model"
            } else {
                "Download SAM Model"
            };
            if ui
                .add_enabled(
                    can_download && !self.sam_download.active,
                    egui::Button::new(sam_label),
                )
                .clicked()
            {
                self.start_sam_download(sam_encoder, sam_decoder);
            }
            let sam_status = if self.sam_download.active {
                self.sam_download
                    .message
                    .clone()
                    .unwrap_or_else(|| "Downloading SAM model...".to_string())
            } else if let Some(message) = self.sam_download.message.as_ref() {
                message.clone()
            } else if !can_download {
                "Model downloads are not available in web builds.".to_string()
            } else {
                String::new()
            };
            if !sam_status.is_empty() {
                ui.label(sam_status);
            }
        }

        if node_kind == Some(BuiltinNodeKind::ImagePreview) {
            if let Some(range_label) = image_preview_range_label(graph, node_id, eval_state) {
                ui.separator();
                ui.label(range_label);
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

        if node_kind == Some(BuiltinNodeKind::WorldLabsGenerate) {
            ui.separator();
            let io_mode = visible_params.get_int("io_mode", 0).clamp(0, 1);
            if io_mode == 1 {
                #[cfg(target_arch = "wasm32")]
                {
                    ui.label("Loading marble models is not available in web builds.");
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let mut current = visible_params.get_string("load_model", "").to_string();
                    let catalog = marble_model_catalog();
                    if catalog.is_empty() {
                        ui.label("No marble models found in marble/.");
                    } else {
                        let mut selected_base = None;
                        let mut selected_variant = None;
                        if !current.trim().is_empty() {
                            for (base, variants) in &catalog {
                                if let Some(found) =
                                    variants.iter().find(|variant| variant.filename == current)
                                {
                                    selected_base = Some(base.clone());
                                    selected_variant = Some(found.variant.clone());
                                    break;
                                }
                            }
                        }
                        if selected_base.is_none() {
                            if let Some((base, variants)) = catalog.first() {
                                if let Some(first_variant) = variants.first() {
                                    selected_base = Some(base.clone());
                                    selected_variant = Some(first_variant.variant.clone());
                                    current = first_variant.filename.clone();
                                }
                            }
                        }
                        let mut base_names: Vec<String> =
                            catalog.iter().map(|(base, _)| base.clone()).collect();
                        base_names.sort();
                        let mut base_selection = selected_base.clone().unwrap_or_else(|| {
                            base_names.first().cloned().unwrap_or_default()
                        });
                        let mut base_changed = false;
                        egui::ComboBox::from_id_salt("worldlabs_load_base")
                            .selected_text(base_selection.clone())
                            .show_ui(ui, |ui| {
                                for base in &base_names {
                                    if ui
                                        .selectable_value(
                                            &mut base_selection,
                                            base.clone(),
                                            base.as_str(),
                                        )
                                        .changed()
                                    {
                                        base_changed = true;
                                    }
                                }
                            });
                        if base_changed {
                            selected_variant = None;
                        }
                        let variants = catalog
                            .iter()
                            .find(|(base, _)| base == &base_selection)
                            .map(|(_, variants)| variants.clone())
                            .unwrap_or_default();
                        if variants.is_empty() {
                            ui.label("No variants found for selected model.");
                        } else if variants.len() > 1 {
                            let mut variant_selection = selected_variant
                                .clone()
                                .unwrap_or_else(|| variants[0].variant.clone());
                            let mut variant_label = variants
                                .iter()
                                .find(|variant| variant.variant == variant_selection)
                                .map(|variant| variant.label.clone())
                                .unwrap_or_else(|| variant_selection.clone());
                            let mut variant_changed = false;
                            egui::ComboBox::from_id_salt("worldlabs_load_variant")
                                .selected_text(variant_label.clone())
                                .show_ui(ui, |ui| {
                                    for variant in &variants {
                                        if ui
                                            .selectable_value(
                                                &mut variant_selection,
                                                variant.variant.clone(),
                                                variant.label.as_str(),
                                            )
                                            .changed()
                                        {
                                            variant_label = variant.label.clone();
                                            variant_changed = true;
                                        }
                                    }
                                });
                            if base_changed || variant_changed {
                                if let Some(found) = variants
                                    .iter()
                                    .find(|variant| variant.variant == variant_selection)
                                {
                                    current = found.filename.clone();
                                } else if let Some(first) = variants.first() {
                                    current = first.filename.clone();
                                }
                            }
                        } else if let Some(first) = variants.first() {
                            if base_changed || current.trim().is_empty() {
                                current = first.filename.clone();
                            }
                        }
                        if current != visible_params.get_string("load_model", "") {
                            if graph
                                .set_param(
                                    node_id,
                                    "load_model".to_string(),
                                    ParamValue::String(current),
                                )
                                .is_ok()
                            {
                                changed = true;
                            }
                        }
                    }
                }
            } else {
                let can_generate = !cfg!(target_arch = "wasm32");
                if ui
                    .add_enabled(can_generate, egui::Button::new("Generate"))
                    .clicked()
                {
                    let snapshot = json!({
                        "io_mode": visible_params.get_int("io_mode", 0),
                        "mode": visible_params.get_int("mode", 0),
                        "text_prompt": visible_params.get_string("text_prompt", ""),
                        "image_path": visible_params.get_string("image_path", ""),
                        "auto_enhance": visible_params.get_bool("auto_enhance", true),
                        "is_pano": visible_params.get_bool("is_pano", false),
                        "model": visible_params.get_int("model", 0),
                        "seed": visible_params.get_int("seed", -1),
                        "tags": visible_params.get_string("tags", ""),
                        "display_name": visible_params.get_string("display_name", ""),
                    });
                    let snapshot_text = snapshot.to_string();
                    if graph
                        .set_param(
                            node_id,
                            "request_snapshot".to_string(),
                            ParamValue::String(snapshot_text),
                        )
                        .is_ok()
                    {
                        changed = true;
                    }
                    let next_token = visible_params.get_int("request_token", 0).saturating_add(1);
                    if graph
                        .set_param(
                            node_id,
                            "request_token".to_string(),
                            ParamValue::Int(next_token),
                        )
                        .is_ok()
                    {
                        changed = true;
                    }
                }
                if !can_generate {
                    ui.label("WorldLabs Generate is not available in web builds.");
                }
            }
        }

        changed
    }

    fn poll_model_download(&mut self) {
        let Some(receiver) = &self.model_download.receiver else {
            return;
        };
        match receiver.try_recv() {
            Ok(result) => {
                self.model_download.active = false;
                self.model_download.receiver = None;
                self.model_download.message = Some(result.message);
            }
            Err(TryRecvError::Disconnected) => {
                self.model_download.active = false;
                self.model_download.receiver = None;
                self.model_download.message =
                    Some("Model download failed: connection lost.".to_string());
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    fn poll_sam_download(&mut self) {
        let Some(receiver) = &self.sam_download.receiver else {
            return;
        };
        match receiver.try_recv() {
            Ok(result) => {
                self.sam_download.active = false;
                self.sam_download.receiver = None;
                self.sam_download.message = Some(result.message);
            }
            Err(TryRecvError::Disconnected) => {
                self.sam_download.active = false;
                self.sam_download.receiver = None;
                self.sam_download.message =
                    Some("SAM model download failed: connection lost.".to_string());
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    fn poll_runtime_download(&mut self) {
        let Some(receiver) = &self.runtime_download.receiver else {
            return;
        };
        match receiver.try_recv() {
            Ok(result) => {
                self.runtime_download.active = false;
                self.runtime_download.receiver = None;
                self.runtime_download.message = Some(result.message);
            }
            Err(TryRecvError::Disconnected) => {
                self.runtime_download.active = false;
                self.runtime_download.receiver = None;
                self.runtime_download.message =
                    Some("Runtime download failed: connection lost.".to_string());
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    fn start_depthpro_download(&mut self, path: PathBuf) {
        if self.model_download.active {
            return;
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.model_download.active = false;
            self.model_download.message =
                Some("Model downloads are not available in web builds.".to_string());
            let _ = path;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::sync::mpsc;
            self.model_download.active = true;
            self.model_download.message = Some("Starting download...".to_string());
            let (tx, rx) = mpsc::channel();
            self.model_download.receiver = Some(rx);
            std::thread::spawn(move || {
                let result = download_depthpro_model(DEPTHPRO_MODEL_URL, &path);
                let _ = tx.send(result);
            });
        }
    }

    fn start_sam_download(&mut self, encoder_path: PathBuf, decoder_path: PathBuf) {
        if self.sam_download.active {
            return;
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.sam_download.active = false;
            self.sam_download.message =
                Some("Model downloads are not available in web builds.".to_string());
            let _ = encoder_path;
            let _ = decoder_path;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::sync::mpsc;
            self.sam_download.active = true;
            self.sam_download.message = Some("Starting download...".to_string());
            let (tx, rx) = mpsc::channel();
            self.sam_download.receiver = Some(rx);
            std::thread::spawn(move || {
                let result = download_sam_model(
                    SAM_ENCODER_URL,
                    SAM_DECODER_URL,
                    &encoder_path,
                    &decoder_path,
                );
                let _ = tx.send(result);
            });
        }
    }

    fn start_onnxruntime_download(&mut self, dir: PathBuf) {
        if self.runtime_download.active {
            return;
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.runtime_download.active = false;
            self.runtime_download.message =
                Some("Runtime downloads are not available in web builds.".to_string());
            let _ = dir;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::sync::mpsc;
            self.runtime_download.active = true;
            self.runtime_download.message = Some("Starting download...".to_string());
            let (tx, rx) = mpsc::channel();
            self.runtime_download.receiver = Some(rx);
            std::thread::spawn(move || {
                let result = download_onnxruntime_runtime(ONNX_RUNTIME_URL, &dir);
                let _ = tx.send(result);
            });
        }
    }

    fn poll_directml_download(&mut self) {
        let Some(receiver) = &self.directml_download.receiver else {
            return;
        };
        match receiver.try_recv() {
            Ok(result) => {
                self.directml_download.active = false;
                self.directml_download.receiver = None;
                self.directml_download.message = Some(result.message);
            }
            Err(TryRecvError::Disconnected) => {
                self.directml_download.active = false;
                self.directml_download.receiver = None;
                self.directml_download.message =
                    Some("DirectML download failed: connection lost.".to_string());
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    fn start_directml_download(&mut self, dir: PathBuf) {
        if self.directml_download.active {
            return;
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.directml_download.active = false;
            self.directml_download.message =
                Some("Runtime downloads are not available in web builds.".to_string());
            let _ = dir;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::sync::mpsc;
            self.directml_download.active = true;
            self.directml_download.message = Some("Starting download...".to_string());
            let (tx, rx) = mpsc::channel();
            self.directml_download.receiver = Some(rx);
            std::thread::spawn(move || {
                let result = download_directml_runtime(ONNX_DIRECTML_PYPI, &dir);
                let _ = tx.send(result);
            });
        }
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

        if node_kind == Some(BuiltinNodeKind::DepthImage) {
            heights.push(separator_height);
            heights.push(row_height);
            let show_status = self.model_download.active
                || self.model_download.message.is_some()
                || cfg!(target_arch = "wasm32");
            if show_status {
                heights.push(row_height);
            }
            heights.push(row_height);
            let show_runtime_status = self.runtime_download.active
                || self.runtime_download.message.is_some()
                || self.directml_download.active
                || self.directml_download.message.is_some()
                || cfg!(target_arch = "wasm32");
            if show_runtime_status {
                heights.push(row_height);
            }
        }

        if node_kind == Some(BuiltinNodeKind::ImagePreview) {
            heights.push(separator_height);
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

        if node_kind == Some(BuiltinNodeKind::WorldLabsGenerate) {
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

fn depthpro_model_path() -> PathBuf {
    PathBuf::from(DEPTHPRO_MODEL_DIR).join(DEPTHPRO_MODEL_FILENAME)
}

fn sam_encoder_path() -> PathBuf {
    PathBuf::from(SAM_MODEL_DIR).join(SAM_ENCODER_FILENAME)
}

fn sam_decoder_path() -> PathBuf {
    PathBuf::from(SAM_MODEL_DIR).join(SAM_DECODER_FILENAME)
}

fn onnxruntime_dir_path() -> PathBuf {
    PathBuf::from(ONNX_RUNTIME_DIR)
}

fn onnxruntime_dylib_path() -> PathBuf {
    onnxruntime_dir_path().join(ONNX_RUNTIME_DLL)
}

fn onnxruntime_directml_dir_path() -> PathBuf {
    PathBuf::from(ONNX_DIRECTML_DIR)
}

fn onnxruntime_directml_dylib_path() -> PathBuf {
    onnxruntime_directml_dir_path().join(ONNX_RUNTIME_DLL)
}

#[cfg(not(target_arch = "wasm32"))]
fn normalize_download_url(url: &str) -> String {
    if url.contains("huggingface.co") {
        let mut out = url.replace("/blob/", "/resolve/");
        if !out.contains("download=") {
            let sep = if out.contains('?') { "&" } else { "?" };
            out = format!("{out}{sep}download=1");
        }
        return out;
    }
    url.to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn download_model_file(url: &str, path: &Path) -> Result<(), String> {
    use std::io::Write;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let temp_path = path.with_extension("download");
    if temp_path.exists() {
        let _ = std::fs::remove_file(&temp_path);
    }
    let download_url = normalize_download_url(url);
    let response = ureq::get(&download_url)
        .call()
        .map_err(|err| format!("Request failed: {err}"))?;
    let mut reader = response.into_reader();
    let mut file = std::fs::File::create(&temp_path).map_err(|err| err.to_string())?;
    std::io::copy(&mut reader, &mut file).map_err(|err| err.to_string())?;
    file.flush().map_err(|err| err.to_string())?;
    if path.exists() {
        std::fs::remove_file(path).map_err(|err| err.to_string())?;
    }
    std::fs::rename(&temp_path, path).map_err(|err| err.to_string())?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn download_depthpro_model(url: &str, path: &Path) -> ModelDownloadResult {
    let download_result = download_model_file(url, path);

    match download_result {
        Ok(()) => ModelDownloadResult {
            message: format!("Downloaded model to {}", path.display()),
        },
        Err(err) => ModelDownloadResult {
            message: format!("Model download failed: {err}"),
        },
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn download_onnxruntime_runtime(url: &str, dir: &Path) -> ModelDownloadResult {
    match download_runtime_zip(url, dir) {
        Ok(()) => ModelDownloadResult {
            message: format!("Downloaded ONNX Runtime to {}", dir.display()),
        },
        Err(err) => ModelDownloadResult {
            message: format!("Runtime download failed: {err}"),
        },
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn download_sam_model(
    encoder_url: &str,
    decoder_url: &str,
    encoder_path: &Path,
    decoder_path: &Path,
) -> ModelDownloadResult {
    let download_result = (|| -> Result<(), String> {
        download_model_file(encoder_url, encoder_path)?;
        download_model_file(decoder_url, decoder_path)?;
        Ok(())
    })();

    match download_result {
        Ok(()) => ModelDownloadResult {
            message: format!(
                "Downloaded SAM model to {}, {}",
                encoder_path.display(),
                decoder_path.display()
            ),
        },
        Err(err) => ModelDownloadResult {
            message: format!("SAM model download failed: {err}"),
        },
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn download_directml_runtime(url: &str, dir: &Path) -> ModelDownloadResult {
    let download_result = (|| -> Result<(), String> {
        let response = ureq::get(url)
            .call()
            .map_err(|err| format!("Request failed: {err}"))?;
        let body = response
            .into_string()
            .map_err(|err| format!("Failed to read response: {err}"))?;
        let value: serde_json::Value =
            serde_json::from_str(&body).map_err(|err| format!("JSON parse failed: {err}"))?;
        let releases = value
            .get("releases")
            .and_then(|v| v.as_object())
            .ok_or_else(|| "PyPI JSON missing releases".to_string())?;
        let mut versions: Vec<String> = releases.keys().cloned().collect();
        versions.sort_by(|a, b| compare_versions(a, b).reverse());
        let preferred_tags = ["cp312", "cp311", "cp310", "cp39", "cp38"];
        let mut selected_url = None;
        for version in versions {
            let files = match releases.get(&version).and_then(|v| v.as_array()) {
                Some(files) => files,
                None => continue,
            };
            for tag in preferred_tags {
                if let Some(file) = files.iter().find(|entry| {
                    entry
                        .get("filename")
                        .and_then(|v| v.as_str())
                        .map(|name| name.contains("win_amd64") && name.contains(tag))
                        .unwrap_or(false)
                }) {
                    if let Some(url) = file.get("url").and_then(|v| v.as_str()) {
                        selected_url = Some(url.to_string());
                        break;
                    }
                }
            }
            if selected_url.is_some() {
                break;
            }
        }
        let Some(selected_url) = selected_url else {
            return Err("No suitable DirectML wheel found in PyPI metadata.".to_string());
        };
        download_runtime_zip(&selected_url, dir)
    })();

    match download_result {
        Ok(()) => ModelDownloadResult {
            message: format!("Downloaded DirectML runtime to {}", dir.display()),
        },
        Err(err) => ModelDownloadResult {
            message: format!("DirectML download failed: {err}"),
        },
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn download_runtime_zip(url: &str, dir: &Path) -> Result<(), String> {
    use std::io::{Read, Write};
    use zip::ZipArchive;

    let temp_dir = dir.with_extension("download");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).map_err(|err| err.to_string())?;
    }
    std::fs::create_dir_all(&temp_dir).map_err(|err| err.to_string())?;

    let response = ureq::get(url)
        .call()
        .map_err(|err| format!("Request failed: {err}"))?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut bytes)
        .map_err(|err| err.to_string())?;
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|err| err.to_string())?;

    let mut extracted_any = false;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|err| err.to_string())?;
        let name = file.name();
        if !name.ends_with(".dll") {
            continue;
        }
        let Some(filename) = Path::new(name).file_name() else {
            continue;
        };
        let out_path = temp_dir.join(filename);
        let mut out_file = std::fs::File::create(&out_path).map_err(|err| err.to_string())?;
        std::io::copy(&mut file, &mut out_file).map_err(|err| err.to_string())?;
        out_file.flush().map_err(|err| err.to_string())?;
        extracted_any = true;
    }

    if !extracted_any {
        return Err("No DLLs found in ONNX Runtime package.".to_string());
    }

    if dir.exists() {
        std::fs::remove_dir_all(dir).map_err(|err| err.to_string())?;
    }
    std::fs::rename(&temp_dir, dir).map_err(|err| err.to_string())?;
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn download_depthpro_model(_url: &str, _path: &Path) -> ModelDownloadResult {
    ModelDownloadResult {
        message: "Model downloads are not available in web builds.".to_string(),
    }
}

#[cfg(target_arch = "wasm32")]
fn download_sam_model(
    _encoder_url: &str,
    _decoder_url: &str,
    _encoder_path: &Path,
    _decoder_path: &Path,
) -> ModelDownloadResult {
    ModelDownloadResult {
        message: "Model downloads are not available in web builds.".to_string(),
    }
}

#[cfg(target_arch = "wasm32")]
fn download_onnxruntime_runtime(_url: &str, _dir: &Path) -> ModelDownloadResult {
    ModelDownloadResult {
        message: "Runtime downloads are not available in web builds.".to_string(),
    }
}

#[cfg(target_arch = "wasm32")]
fn download_directml_runtime(_url: &str, _dir: &Path) -> ModelDownloadResult {
    ModelDownloadResult {
        message: "Runtime downloads are not available in web builds.".to_string(),
    }
}

fn image_preview_range_label(
    graph: &Graph,
    node_id: lobedo_core::NodeId,
    eval_state: Option<&lobedo_core::GeometryEvalState>,
) -> Option<String> {
    let eval_state = eval_state?;
    let node = graph.node(node_id)?;
    let input_pin = node.inputs.first().copied()?;
    let link = graph.input_link(input_pin)?;
    let image = eval_state.image_for_pin(link.from)?;
    match image {
        lobedo_core::ImageData::RgbF32 { data, .. } => {
            let (min, max) = finite_min_max_f32(data)?;
            Some(format!("Input range (RGB): [{min:.6}, {max:.6}]"))
        }
        lobedo_core::ImageData::R32F { data, .. } => {
            let (min, max) = finite_min_max_f32(data)?;
            Some(format!("Input range (Depth): [{min:.6}, {max:.6}]"))
        }
        lobedo_core::ImageData::R32U { data, .. } => {
            let (min, max) = finite_min_max_u32(data)?;
            Some(format!("Input range (Seg): [{min:.0}, {max:.0}]"))
        }
    }
}

fn finite_min_max_f32(values: &[f32]) -> Option<(f32, f32)> {
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for &value in values {
        if value.is_finite() {
            if value < min {
                min = value;
            }
            if value > max {
                max = value;
            }
        }
    }
    if min.is_finite() && max.is_finite() {
        Some((min, max))
    } else {
        None
    }
}

fn finite_min_max_u32(values: &[u32]) -> Option<(f32, f32)> {
    if values.is_empty() {
        return None;
    }
    let mut min = u32::MAX;
    let mut max = u32::MIN;
    for &value in values {
        min = min.min(value);
        max = max.max(value);
    }
    Some((min as f32, max as f32))
}

fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| {
        s.split('.')
            .map(|part| part.parse::<i32>().unwrap_or(0))
            .collect::<Vec<_>>()
    };
    let aa = parse(a);
    let bb = parse(b);
    let len = aa.len().max(bb.len());
    for i in 0..len {
        let av = *aa.get(i).unwrap_or(&0);
        let bv = *bb.get(i).unwrap_or(&0);
        match av.cmp(&bv) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

#[derive(Clone)]
struct MarbleModelVariant {
    base: String,
    variant: String,
    label: String,
    filename: String,
}

#[cfg(not(target_arch = "wasm32"))]
fn marble_model_catalog() -> Vec<(String, Vec<MarbleModelVariant>)> {
    let dir = PathBuf::from("marble");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut variants = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext != "ply" && ext != "spz" {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("")
            .to_string();
        let (base, variant) = split_variant(&stem);
        let label = variant_label(&variant, &ext);
        variants.push(MarbleModelVariant {
            base,
            variant,
            label,
            filename: name.to_string(),
        });
    }
    let mut grouped: std::collections::BTreeMap<String, Vec<MarbleModelVariant>> =
        std::collections::BTreeMap::new();
    for variant in variants {
        grouped
            .entry(variant.base.clone())
            .or_default()
            .push(variant);
    }
    let mut out = Vec::new();
    for (base, mut variants) in grouped {
        variants.sort_by(|a, b| {
            let ra = variant_rank(&a.variant);
            let rb = variant_rank(&b.variant);
            ra.cmp(&rb).then_with(|| a.variant.cmp(&b.variant))
        });
        out.push((base, variants));
    }
    out
}

#[cfg(not(target_arch = "wasm32"))]
fn split_variant(stem: &str) -> (String, String) {
    for suffix in ["_full", "_500k", "_100k"] {
        if let Some(base) = stem.strip_suffix(suffix) {
            let variant = suffix.trim_start_matches('_').to_string();
            return (base.to_string(), variant);
        }
    }
    (stem.to_string(), "default".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn variant_rank(variant: &str) -> i32 {
    match variant {
        "full" => 0,
        "500k" => 1,
        "100k" => 2,
        "default" => 3,
        _ => 4,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn variant_label(variant: &str, ext: &str) -> String {
    match variant {
        "full" => "Full".to_string(),
        "500k" => "500k".to_string(),
        "100k" => "100k".to_string(),
        "default" => match ext {
            "ply" => "PLY".to_string(),
            "spz" => "SPZ".to_string(),
            _ => variant.to_string(),
        },
        _ => variant.to_string(),
    }
}

#[cfg(target_arch = "wasm32")]
fn marble_model_catalog() -> Vec<(String, Vec<MarbleModelVariant>)> {
    Vec::new()
}

