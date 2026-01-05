use eframe::egui;

use super::spreadsheet::show_spreadsheet;
use super::LobedoApp;

impl LobedoApp {
    pub(super) fn show_central_panel(
        &mut self,
        ctx: &egui::Context,
        pointer_down: bool,
        undo_pushed: &mut bool,
    ) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let (left_rect, right_rect) = self.split_central_rect(ui);
            self.show_left_panel(ui, left_rect);
            self.show_right_panel(ui, right_rect, pointer_down, undo_pushed);
            self.last_node_graph_rect = Some(right_rect);
        });
    }

    fn split_central_rect(&self, ui: &egui::Ui) -> (egui::Rect, egui::Rect) {
        let full = ui.available_rect_before_wrap();
        let ratio = self.project.settings.viewport_split.clamp(0.2, 0.8);
        let left_width = full.width() * ratio;
        let left = egui::Rect::from_min_size(full.min, egui::vec2(left_width, full.height()));
        let right = egui::Rect::from_min_max(egui::pos2(left.max.x, full.min.y), full.max);
        (left, right)
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
            let available = ui.available_size();
            let (rect, response) =
                ui.allocate_exact_size(available, egui::Sense::click_and_drag());
            self.handle_viewport_input(&response);
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

            self.show_viewport_toolbar(ui, rect);
        });
    }

    fn show_viewport_toolbar(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let toolbar_rect = egui::Rect::from_min_size(
            egui::pos2(rect.max.x - 36.0, rect.min.y + 8.0),
            egui::vec2(28.0, 160.0),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(toolbar_rect), |ui| {
            ui.set_min_width(toolbar_rect.width());
            ui.visuals_mut().widgets.inactive.bg_fill = egui::Color32::from_rgb(45, 45, 45);
            ui.visuals_mut().widgets.hovered.bg_fill = egui::Color32::from_rgb(70, 70, 70);
            ui.visuals_mut().widgets.active.bg_fill = egui::Color32::from_rgb(90, 90, 90);
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);

            let debug = &mut self.project.settings.render_debug;
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
            let snapshot = self.snapshot_undo();
            self.node_graph
                .show(ui, &mut self.project.graph, &mut self.eval_dirty);
            let layout_moved = self.node_graph.take_layout_changed();
            if (self.node_graph.take_changed() || layout_moved) && !*undo_pushed {
                self.queue_undo_snapshot(snapshot, pointer_down);
                *undo_pushed = true;
            }
        });
    }
}
