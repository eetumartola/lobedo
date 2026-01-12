use eframe::egui;
use lobedo_core::NodeId;

use super::spreadsheet::show_spreadsheet;
use super::viewport_tools::input_node_for;
use super::LobedoApp;

type ViewportAction = (&'static str, bool, fn(&mut LobedoApp, NodeId));

impl LobedoApp {
    pub(super) fn show_central_panel(
        &mut self,
        ctx: &egui::Context,
        pointer_down: bool,
        undo_pushed: &mut bool,
    ) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let (left_rect, right_rect, separator_rect) = self.split_central_rect(ui);
            self.show_left_panel(ui, left_rect);
            self.show_right_panel(ui, right_rect, pointer_down, undo_pushed);

            let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(55, 55, 55));
            ui.painter().line_segment(
                [
                    egui::pos2(separator_rect.center().x, separator_rect.top()),
                    egui::pos2(separator_rect.center().x, separator_rect.bottom()),
                ],
                stroke,
            );
        });
    }

    fn split_central_rect(&mut self, ui: &egui::Ui) -> (egui::Rect, egui::Rect, egui::Rect) {
        let full = ui.available_rect_before_wrap();
        let ratio = self.project.settings.viewport_split.clamp(0.2, 0.8);
        let separator_width = 3.0;
        let separator_x = full.min.x + full.width() * ratio;
        let separator_rect = egui::Rect::from_min_size(
            egui::pos2(separator_x - separator_width * 0.5, full.min.y),
            egui::vec2(separator_width, full.height()),
        );
        let response = ui.interact(
            separator_rect,
            ui.make_persistent_id("viewport_split_drag"),
            egui::Sense::drag(),
        );
        if response.dragged() {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                let next = ((pos.x - full.min.x) / full.width()).clamp(0.2, 0.8);
                self.project.settings.viewport_split = next;
            }
        }

        let left = egui::Rect::from_min_max(
            full.min,
            egui::pos2(separator_rect.min.x, full.max.y),
        );
        let right = egui::Rect::from_min_max(
            egui::pos2(separator_rect.max.x, full.min.y),
            full.max,
        );
        (left, right, separator_rect)
    }

    fn show_left_panel(&mut self, ui: &mut egui::Ui, left_rect: egui::Rect) {
        ui.scope_builder(egui::UiBuilder::new().max_rect(left_rect), |ui| {
            let full = ui.available_rect_before_wrap();
            let show_sheet = self.project.settings.panels.show_spreadsheet;
            let separator_height = if show_sheet { 1.0 } else { 0.0 };
            let min_sheet = 140.0;
            let min_viewport = 220.0;
            let total_height = full.height();
            let (viewport_height, sheet_height) = if show_sheet {
                let mut viewport_height = (total_height
                    * self.project.settings.viewport_sheet_split.clamp(0.3, 0.9))
                .clamp(min_viewport, (total_height - min_sheet).max(min_viewport));
                if total_height <= min_viewport + min_sheet + separator_height {
                    viewport_height = total_height.max(min_viewport);
                }
                let sheet_height = (total_height - viewport_height - separator_height).max(0.0);
                (viewport_height, sheet_height)
            } else {
                (total_height.max(min_viewport), 0.0)
            };
            let viewport_rect =
                egui::Rect::from_min_size(full.min, egui::vec2(full.width(), viewport_height));
            let separator_rect = egui::Rect::from_min_size(
                egui::pos2(full.min.x, viewport_rect.max.y),
                egui::vec2(full.width(), separator_height),
            );
            let sheet_rect = egui::Rect::from_min_size(
                egui::pos2(full.min.x, separator_rect.max.y),
                egui::vec2(full.width(), sheet_height),
            );

            if sheet_height > 0.0 {
                let sep_response = ui.interact(
                    separator_rect,
                    ui.make_persistent_id("viewport_sheet_split"),
                    egui::Sense::drag(),
                );
                if sep_response.dragged() {
                    if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                        let local = (pos.y - full.min.y)
                            .clamp(min_viewport, (total_height - min_sheet).max(min_viewport));
                        self.project.settings.viewport_sheet_split =
                            (local / total_height).clamp(0.3, 0.9);
                    }
                }
                let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 70, 70));
                ui.painter().line_segment(
                    [
                        egui::pos2(separator_rect.left(), separator_rect.center().y),
                        egui::pos2(separator_rect.right(), separator_rect.center().y),
                    ],
                    stroke,
                );
            }

            self.show_viewport_panel(ui, viewport_rect);

            if sheet_height > 0.0 {
                self.show_spreadsheet_panel(ui, sheet_rect);
            }
        });
    }

    fn show_viewport_panel(&mut self, ui: &mut egui::Ui, viewport_rect: egui::Rect) {
        ui.scope_builder(egui::UiBuilder::new().max_rect(viewport_rect), |ui| {
            self.last_viewport_rect = Some(viewport_rect);
            let available = ui.available_size();
            let (rect, response) =
                ui.allocate_exact_size(available, egui::Sense::click_and_drag());
            self.handle_viewport_input(&response, rect);
            ui.painter()
                .rect_filled(rect, 0.0, egui::Color32::from_rgb(28, 28, 28));
            if let Some(renderer) = &self.viewport_renderer {
                let camera = self.camera_state();
                let debug = self.viewport_debug();
                let callback = renderer.paint_callback(rect, camera, debug);
                ui.painter().add(egui::Shape::Callback(callback));

                if self.project.settings.render_debug.show_stats {
                    let stats = renderer.stats_snapshot();
                    let text = format!(
                        "FPS: {:.1}\nFrame: {:.2} ms\nVerts: {}\nTris: {}\nMeshes: {}\nCache: {} hits / {} misses / {} uploads",
                        stats.fps,
                        stats.frame_time_ms,
                        stats.vertex_count,
                        stats.triangle_count,
                        stats.mesh_count,
                        stats.cache_hits,
                        stats.cache_misses,
                        stats.cache_uploads
                    );
                    let font_id = egui::FontId::monospace(12.0);
                    let galley = ui.fonts_mut(|f| {
                        f.layout_no_wrap(text.clone(), font_id.clone(), egui::Color32::WHITE)
                    });
                    let padding = egui::vec2(6.0, 4.0);
                    let bg_rect = egui::Rect::from_min_size(
                        rect.min + egui::vec2(8.0, 8.0),
                        galley.size() + padding * 2.0,
                    );
                    let painter = ui.painter();
                    painter.rect_filled(
                        bg_rect,
                        4.0,
                        egui::Color32::from_black_alpha(160),
                    );
                    painter.galley(bg_rect.min + padding, galley, egui::Color32::WHITE);
                }
            } else {
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "WGPU not ready",
                    egui::FontId::proportional(14.0),
                    egui::Color32::GRAY,
                );
            }

            self.draw_viewport_tools(ui, rect);
            self.show_viewport_node_actions(ui, rect);
            self.show_viewport_toolbar(ui, rect);
        });
    }

    fn show_viewport_toolbar(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let toolbar_rect = egui::Rect::from_min_size(
            egui::pos2(rect.max.x - 36.0, rect.min.y + 8.0),
            egui::vec2(28.0, 220.0),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(toolbar_rect), |ui| {
            ui.set_min_width(toolbar_rect.width());
            ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_rgb(45, 45, 45);
            ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::from_rgb(70, 70, 70);
            ui.visuals_mut().widgets.active.bg_fill = egui::Color32::from_rgb(90, 90, 90);
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);

            let debug = &mut self.project.settings.render_debug;
            if ui
                .add(egui::Button::new("N").selected(debug.show_normals))
                .on_hover_text("Normals")
                .clicked()
            {
                debug.show_normals = !debug.show_normals;
            }
            if ui
                .add(egui::Button::new("ST").selected(debug.show_stats))
                .on_hover_text("Stats overlay")
                .clicked()
            {
                debug.show_stats = !debug.show_stats;
            }
            if ui
                .add(egui::Button::new("G").selected(debug.show_grid))
                .on_hover_text("Grid")
                .clicked()
            {
                debug.show_grid = !debug.show_grid;
            }
            if ui
                .add(egui::Button::new("A").selected(debug.show_axes))
                .on_hover_text("Axes")
                .clicked()
            {
                debug.show_axes = !debug.show_axes;
            }
            if ui
                .add(egui::Button::new("P").selected(debug.show_points))
                .on_hover_text("Points")
                .clicked()
            {
                debug.show_points = !debug.show_points;
            }
            if ui
                .add(egui::Button::new("SP").selected(debug.show_splats))
                .on_hover_text("Splats")
                .clicked()
            {
                debug.show_splats = !debug.show_splats;
            }
            if ui
                .add(egui::Button::new("S").selected(debug.key_shadows))
                .on_hover_text("Key shadows")
                .clicked()
            {
                debug.key_shadows = !debug.key_shadows;
            }
        });
    }

    fn show_viewport_node_actions(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        if let Some(active_id) = self.group_select_node_id() {
            let selected = self.selected_group_select_node();
            if selected != Some(active_id) {
                self.deactivate_group_select();
            }
        }
        let Some(node_id) = self.node_graph.selected_node_id() else {
            return;
        };
        let (node_name, group_shape, group_selection) = {
            let Some(node) = self.project.graph.node(node_id) else {
                return;
            };
            let name = node.name.clone();
            let shape = if name == "Group" {
                Some(node.params.get_string("shape", "box").to_lowercase())
            } else {
                None
            };
            let selection = if name == "Group" {
                Some(node.params.get_string("selection", "").to_string())
            } else {
                None
            };
            (name, shape, selection)
        };
        let mut actions: Vec<ViewportAction> = Vec::new();
        if node_name == "Curve" {
            actions.push((
                "Add Curve",
                self.curve_draw_active(node_id),
                toggle_curve_draw,
            ));
            actions.push((
                "Edit Curve",
                self.curve_edit_active(node_id),
                toggle_curve_edit,
            ));
        }
        if node_name == "FFD" {
            if input_node_for(&self.project.graph, node_id, 1).is_none() {
                self.ensure_ffd_lattice_points(node_id);
            }
            actions.push((
                "Edit Lattice",
                self.ffd_edit_active(node_id),
                toggle_ffd_edit,
            ));
        }
        let mut footer = None;
        if node_name == "Group" {
            if group_shape.as_deref() == Some("selection") {
                actions.push(("Select", self.group_select_active(node_id), toggle_group_select));
                let count = selection_count(group_selection.as_deref().unwrap_or(""));
                footer = Some(format!("Selected: {}", count));
            }
        }
        if actions.is_empty() {
            return;
        }

        let button_size = egui::vec2(130.0, 32.0);
        let spacing = 10.0;
        let action_count = actions.len() as f32;
        let total_width = button_size.x * action_count + spacing * (action_count - 1.0);
        let pos = egui::pos2(rect.center().x - total_width * 0.5, rect.min.y + 8.0);
        let ctx = ui.ctx();
        egui::Area::new(egui::Id::new("viewport_node_actions"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                let frame = egui::Frame::NONE
                    .fill(egui::Color32::from_black_alpha(160))
                    .corner_radius(egui::CornerRadius::same(4))
                    .inner_margin(egui::Margin::symmetric(8, 6));
                frame.show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(spacing, 0.0);
                    ui.style_mut().text_styles.insert(
                        egui::TextStyle::Button,
                        egui::FontId::proportional(16.0),
                    );
                    ui.horizontal(|ui| {
                        for (label, active, action) in &actions {
                            if ui
                                .add_sized(
                                    button_size,
                                    egui::Button::new(*label).selected(*active),
                                )
                                .clicked()
                            {
                                action(self, node_id);
                            }
                        }
                    });
                    if let Some(footer) = &footer {
                        ui.add_space(4.0);
                        ui.label(footer);
                    }
                });
            });
    }

    fn show_spreadsheet_panel(&mut self, ui: &mut egui::Ui, sheet_rect: egui::Rect) {
        ui.scope_builder(egui::UiBuilder::new().max_rect(sheet_rect), |ui| {
            ui.painter().rect_filled(
                sheet_rect,
                0.0,
                egui::Color32::from_rgb(38, 38, 38),
            );
            let frame = egui::Frame::NONE
                .fill(egui::Color32::from_rgb(38, 38, 38))
                .inner_margin(egui::Margin::symmetric(12, 10));
            frame.show(ui, |ui| {
                let style = ui.style_mut();
                style.visuals = egui::Visuals::dark();
                style.visuals.override_text_color = Some(egui::Color32::from_rgb(220, 220, 220));
                let stroke_width = 0.25;
                style.visuals.window_stroke.width = stroke_width;
                style.visuals.widgets.inactive.bg_stroke.width = stroke_width;
                style.visuals.widgets.hovered.bg_stroke.width = stroke_width;
                style.visuals.widgets.active.bg_stroke.width = stroke_width;
                style.spacing.item_spacing = egui::vec2(10.0, 6.0);
                let selected = self.node_graph.selected_node_id();
                let geometry = selected.and_then(|id| self.eval_state.geometry_for_node(id));
                let mesh = geometry.and_then(|geo| geo.merged_mesh());
                let splats = geometry.and_then(|geo| geo.merged_splats());
                show_spreadsheet(
                    ui,
                    mesh.as_ref(),
                    splats.as_ref(),
                    &mut self.spreadsheet_mode,
                    &mut self.spreadsheet_domain,
                );
            });
        });
    }

    fn show_right_panel(
        &mut self,
        ui: &mut egui::Ui,
        right_rect: egui::Rect,
        pointer_down: bool,
        undo_pushed: &mut bool,
    ) {
        let mut params_height = 0.0;
        let separator_height = 1.0;
        let min_params = 140.0;
        if self.project.settings.panels.show_inspector {
            let selected = self.node_graph.selected_node_id();
            let rows = self.node_graph.inspector_row_count(&self.project.graph);
            let row_height = 36.0;
            let header_height = 46.0;
            let padding = 40.0;
            let desired_height = header_height + rows as f32 * row_height + padding;
            let max = right_rect.height() * 0.5;
            let target = desired_height.clamp(min_params, max.max(min_params));
            if selected != self.last_selected_node
                || (self.project.settings.node_params_split * right_rect.height()) < target
            {
                let clamped = desired_height.clamp(min_params, max.max(min_params));
                self.project.settings.node_params_split =
                    (clamped / right_rect.height()).clamp(0.1, 0.5);
                self.last_selected_node = selected;
            }
            params_height = (right_rect.height()
                * self.project.settings.node_params_split.clamp(0.1, 0.5))
            .clamp(min_params, right_rect.height() * 0.5);
        }
        let params_ratio = if params_height > 0.0 { 1.0 } else { 0.0 };
        let params_rect = if params_ratio > 0.0 {
            egui::Rect::from_min_size(
                right_rect.min,
                egui::vec2(right_rect.width(), params_height),
            )
        } else {
            egui::Rect::from_min_size(right_rect.min, egui::vec2(right_rect.width(), 0.0))
        };
        let separator_rect = if params_ratio > 0.0 {
            egui::Rect::from_min_size(
                egui::pos2(right_rect.min.x, params_rect.max.y),
                egui::vec2(right_rect.width(), separator_height),
            )
        } else {
            egui::Rect::from_min_size(right_rect.min, egui::vec2(0.0, 0.0))
        };
        let graph_rect = if params_ratio > 0.0 {
            egui::Rect::from_min_max(
                egui::pos2(right_rect.min.x, separator_rect.max.y),
                right_rect.max,
            )
        } else {
            right_rect
        };

        if params_ratio > 0.0 {
            let sep_response = ui.interact(
                separator_rect,
                ui.make_persistent_id("node_split"),
                egui::Sense::drag(),
            );
            if sep_response.dragged() {
                if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                    let local =
                        (pos.y - right_rect.min.y).clamp(min_params, right_rect.height() * 0.5);
                    self.project.settings.node_params_split =
                        (local / right_rect.height()).clamp(0.1, 0.5);
                }
            }
            let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 70, 70));
            ui.painter().line_segment(
                [
                    egui::pos2(separator_rect.left(), separator_rect.center().y),
                    egui::pos2(separator_rect.right(), separator_rect.center().y),
                ],
                stroke,
            );
        }

        if params_ratio > 0.0 {
            self.show_node_params_panel(ui, params_rect, pointer_down, undo_pushed);
        }

        self.show_node_graph_panel(ui, graph_rect, pointer_down, undo_pushed);
    }

    fn show_node_params_panel(
        &mut self,
        ui: &mut egui::Ui,
        params_rect: egui::Rect,
        pointer_down: bool,
        undo_pushed: &mut bool,
    ) {
        ui.painter().rect_filled(
            params_rect,
            0.0,
            egui::Color32::from_rgb(55, 55, 55),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(params_rect), |ui| {
            let frame = egui::Frame::NONE
                .fill(egui::Color32::from_rgb(55, 55, 55))
                .inner_margin(egui::Margin::symmetric(16, 12));
            frame.show(ui, |ui| {
                let style = ui.style_mut();
                style.visuals = egui::Visuals::dark();
                let text_color = egui::Color32::from_rgb(230, 230, 230);
                style.visuals.override_text_color = Some(text_color);
                style.visuals.widgets.inactive.fg_stroke.color = text_color;
                style.visuals.widgets.hovered.fg_stroke.color = text_color;
                style.visuals.widgets.active.fg_stroke.color = text_color;
                style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(60, 60, 60);
                style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(75, 75, 75);
                style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(90, 90, 90);
                style.visuals.widgets.inactive.bg_stroke.color = egui::Color32::from_rgb(85, 85, 85);
                style.visuals.widgets.hovered.bg_stroke.color =
                    egui::Color32::from_rgb(105, 105, 105);
                style.visuals.widgets.active.bg_stroke.color =
                    egui::Color32::from_rgb(125, 125, 125);
                let stroke_width = 0.25;
                style.visuals.window_stroke.width = stroke_width;
                style.visuals.widgets.inactive.bg_stroke.width = stroke_width;
                style.visuals.widgets.hovered.bg_stroke.width = stroke_width;
                style.visuals.widgets.active.bg_stroke.width = stroke_width;
                style.visuals.extreme_bg_color = egui::Color32::from_rgb(45, 45, 45);
                style.visuals.faint_bg_color = egui::Color32::from_rgb(55, 55, 55);
                style.text_styles.insert(
                    egui::TextStyle::Body,
                    egui::FontId::proportional(16.0),
                );
                style.text_styles.insert(
                    egui::TextStyle::Button,
                    egui::FontId::proportional(16.0),
                );
                style.text_styles.insert(
                    egui::TextStyle::Heading,
                    egui::FontId::proportional(18.0),
                );
                style.spacing.item_spacing = egui::vec2(10.0, 8.0);
                style.spacing.interact_size = egui::vec2(44.0, 26.0);

                let max_height = ui.available_height();
                egui::ScrollArea::vertical().max_height(max_height).show(ui, |ui| {
                    let snapshot = self.snapshot_undo();
                    if self.node_graph.show_inspector(ui, &mut self.project.graph) {
                        self.mark_eval_dirty();
                        if !*undo_pushed {
                            self.queue_undo_snapshot(snapshot, pointer_down);
                            *undo_pushed = true;
                        }
                    }
                    if let Some(request) = self.node_graph.take_write_request() {
                        self.handle_write_request(request);
                    }
                    self.show_splat_read_params(ui);
                    self.show_uv_view_params(ui);
                });
            });
        });
    }

    fn show_node_graph_panel(
        &mut self,
        ui: &mut egui::Ui,
        graph_rect: egui::Rect,
        pointer_down: bool,
        undo_pushed: &mut bool,
    ) {
        ui.scope_builder(egui::UiBuilder::new().max_rect(graph_rect), |ui| {
            self.last_node_graph_rect = Some(graph_rect);
            let hover_pos = ui.input(|i| i.pointer.hover_pos());
            let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
            if scroll_delta.abs() > 0.0 {
                if let Some(pos) = hover_pos {
                    if graph_rect.contains(pos) {
                        self.node_graph.zoom_at(pos, scroll_delta);
                        ui.input_mut(|i| i.raw_scroll_delta = egui::Vec2::ZERO);
                    }
                }
            }
            let rmb_down = ui.input(|i| i.pointer.button_down(egui::PointerButton::Secondary));
            if rmb_down {
                if let Some(pos) = ui.input(|i| i.pointer.latest_pos()) {
                    if graph_rect.contains(pos) {
                        let delta = ui.input(|i| i.pointer.delta());
                        if delta.y.abs() > 0.0 {
                            let zoom_delta = -delta.y * 3.0;
                            self.node_graph.zoom_at(pos, zoom_delta);
                        }
                    }
                }
            }
            let snapshot = self.snapshot_undo();
            self.node_graph
                .show(ui, &mut self.project.graph, &mut self.eval_dirty);
            if self.fit_nodes_on_load {
                if let Some(rect) = self.last_node_graph_rect {
                    self.node_graph.fit_to_rect(rect);
                    self.fit_nodes_on_load = false;
                }
            }
            let layout_moved = self.node_graph.take_layout_changed();
            if (self.node_graph.take_changed() || layout_moved) && !*undo_pushed {
                self.queue_undo_snapshot(snapshot, pointer_down);
                *undo_pushed = true;
            }
        });
    }

    fn show_splat_read_params(&self, ui: &mut egui::Ui) {
        let Some(node_id) = self.node_graph.selected_node_id() else {
            return;
        };
        let Some(node) = self.project.graph.node(node_id) else {
            return;
        };
        if !matches!(node.name.as_str(), "Splat Read" | "Read Splats") {
            return;
        }

        ui.separator();
        ui.label("Splat info");
        let Some(geometry) = self.eval_state.geometry_for_node(node_id) else {
            ui.label("No splat data available yet.");
            return;
        };
        if geometry.splats.is_empty() {
            ui.label("No splats loaded.");
            return;
        }

        let splat_geo = &geometry.splats[0];
        if geometry.splats.len() > 1 {
            ui.label("Multiple splat primitives; showing the first.");
        }
        ui.label(format!("Path: {}", node.params.get_string("path", "<unset>")));
        ui.label(format!("Splats: {}", splat_geo.len()));
        ui.label(format!("SH coeffs/channel: {}", splat_geo.sh_coeffs));
        ui.label(format!("SH order: {}", sh_order_label(splat_geo.sh_coeffs)));
    }

    fn show_uv_view_params(&self, ui: &mut egui::Ui) {
        let Some(node_id) = self.node_graph.selected_node_id() else {
            return;
        };
        let Some(node) = self.project.graph.node(node_id) else {
            return;
        };
        if node.name != "UV View" {
            return;
        }

        ui.separator();
        ui.label("UV View");
        let Some(geometry) = self.eval_state.geometry_for_node(node_id) else {
            ui.label("No geometry available for this node.");
            return;
        };
        let Some(mesh) = geometry.merged_mesh() else {
            ui.label("No mesh output available for this node.");
            return;
        };
        let Some(corner_uvs) = mesh_corner_uvs(&mesh) else {
            ui.label("No UVs found on this mesh.");
            return;
        };
        if mesh.indices.len() < 3 {
            ui.label("Mesh has no triangles.");
            return;
        }

        ui.label(format!("Triangles: {}", mesh.indices.len() / 3));
        ui.add_space(6.0);
        let height = 240.0;
        let width = ui.available_width().max(160.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(32, 32, 32));
        painter.rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 60)),
            egui::StrokeKind::Outside,
        );

        let (min_uv, max_uv) = uv_bounds(&corner_uvs);
        let span = (max_uv - min_uv).max(egui::vec2(1.0e-6, 1.0e-6));
        let padding = 8.0;
        let inner = rect.shrink(padding);
        let scale = (inner.width() / span.x).min(inner.height() / span.y);
        let uv_size = egui::vec2(span.x * scale, span.y * scale);
        let offset = egui::vec2(
            inner.min.x + (inner.width() - uv_size.x) * 0.5,
            inner.min.y + (inner.height() - uv_size.y) * 0.5,
        );

        let to_screen = |uv: [f32; 2]| -> egui::Pos2 {
            let x = offset.x + (uv[0] - min_uv.x) * scale;
            let y = offset.y + (max_uv.y - uv[1]) * scale;
            egui::pos2(x, y)
        };

        let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 200, 255));
        for tri in corner_uvs.chunks_exact(3) {
            let a = to_screen(tri[0]);
            let b = to_screen(tri[1]);
            let c = to_screen(tri[2]);
            painter.line_segment([a, b], stroke);
            painter.line_segment([b, c], stroke);
            painter.line_segment([c, a], stroke);
        }
    }
}

fn mesh_corner_uvs(mesh: &lobedo_core::Mesh) -> Option<Vec<[f32; 2]>> {
    if mesh.indices.is_empty() {
        return None;
    }
    if let Some(lobedo_core::AttributeRef::Vec2(values)) =
        mesh.attribute(lobedo_core::AttributeDomain::Vertex, "uv")
    {
        if values.len() == mesh.indices.len() {
            return Some(values.to_vec());
        }
    }

    let mut point_uvs = None;
    if let Some(lobedo_core::AttributeRef::Vec2(values)) =
        mesh.attribute(lobedo_core::AttributeDomain::Point, "uv")
    {
        if values.len() == mesh.positions.len() {
            point_uvs = Some(values.to_vec());
        }
    }
    if point_uvs.is_none() {
        if let Some(uvs) = mesh.uvs.as_ref() {
            if uvs.len() == mesh.positions.len() {
                point_uvs = Some(uvs.clone());
            }
        }
    }
    let uvs = point_uvs?;
    let mut corner_uvs = Vec::with_capacity(mesh.indices.len());
    for &idx in &mesh.indices {
        corner_uvs.push(*uvs.get(idx as usize).unwrap_or(&[0.0, 0.0]));
    }
    Some(corner_uvs)
}

fn uv_bounds(uvs: &[[f32; 2]]) -> (egui::Vec2, egui::Vec2) {
    let mut min = egui::Vec2::new(f32::INFINITY, f32::INFINITY);
    let mut max = egui::Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
    for uv in uvs {
        min.x = min.x.min(uv[0]);
        min.y = min.y.min(uv[1]);
        max.x = max.x.max(uv[0]);
        max.y = max.y.max(uv[1]);
    }
    (min, max)
}

fn sh_order_label(sh_coeffs: usize) -> String {
    let total = 1 + sh_coeffs;
    let order = (total as f32).sqrt().round() as usize;
    if order * order == total && order > 0 {
        let max_l = order - 1;
        format!("L{} ({} bands)", max_l, max_l + 1)
    } else {
        format!("Partial ({} coeffs)", total)
    }
}

fn toggle_curve_draw(app: &mut LobedoApp, node_id: NodeId) {
    if app.curve_draw_active(node_id) {
        app.deactivate_curve_draw();
    } else {
        app.activate_curve_draw(node_id);
    }
}

fn toggle_curve_edit(app: &mut LobedoApp, node_id: NodeId) {
    if app.curve_edit_active(node_id) {
        app.deactivate_curve_edit();
    } else {
        app.activate_curve_edit(node_id);
    }
}

fn toggle_ffd_edit(app: &mut LobedoApp, node_id: NodeId) {
    if app.ffd_edit_active(node_id) {
        app.deactivate_ffd_edit();
    } else {
        app.activate_ffd_edit(node_id);
    }
}

fn toggle_group_select(app: &mut LobedoApp, node_id: NodeId) {
    if app.group_select_active(node_id) {
        app.deactivate_group_select();
    } else {
        app.activate_group_select(node_id);
    }
}

fn selection_count(value: &str) -> usize {
    let mut count = 0usize;
    for token in value.split(|c: char| c.is_whitespace() || c == ',' || c == ';') {
        if token.trim().parse::<usize>().is_ok() {
            count += 1;
        }
    }
    count
}
