use egui::{Frame, Pos2, Ui};

use egui::{Color32, TextStyle};
use lobedo_core::{BuiltinNodeKind, Graph, NodeId, ProjectSettings};

use super::menu::{builtin_menu_items, menu_layout, render_menu_layout};
use super::state::{NodeGraphState, NodeInfoRequest};
use super::utils::{add_builtin_node, add_builtin_node_checked};

impl NodeGraphState {
    pub(super) fn show_node_menu(&mut self, ui: &mut Ui, graph: &mut Graph) -> bool {
        let mut close_menu = ui.input(|i| i.key_pressed(egui::Key::Escape));
        let mut menu_rect = None;
        let mut changed = false;
        let node_id = self.node_menu_node;

        let response = egui::Window::new("node_context_menu")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::LEFT_TOP, self.node_menu_screen_pos.to_vec2())
            .frame(Frame::popup(ui.style()))
            .show(ui.ctx(), |ui| {
                let font_id = TextStyle::Button.resolve(ui.style());
                let node_info_width = ui
                    .painter()
                    .layout_no_wrap("Node info".to_string(), font_id.clone(), Color32::WHITE)
                    .size()
                    .x;
                let delete_node_width = ui
                    .painter()
                    .layout_no_wrap("Delete node".to_string(), font_id, Color32::WHITE)
                    .size()
                    .x;
                let min_label_width = node_info_width.max(delete_node_width).max(96.0);
                let min_width = min_label_width + ui.spacing().button_padding.x * 2.0 + 8.0;
                ui.set_min_width(min_width);
                if ui.button("Node info").clicked() {
                    if let Some(node_id) = node_id {
                        self.info_request = Some(NodeInfoRequest {
                            node_id,
                            screen_pos: self.node_menu_screen_pos,
                        });
                    }
                    close_menu = true;
                }
                if ui.button("Delete node").clicked() {
                    if let Some(node_id) = node_id {
                        if self.delete_node(graph, node_id) {
                            changed = true;
                        }
                    }
                    close_menu = true;
                }
            });

        if let Some(inner) = response {
            menu_rect = Some(inner.response.rect);
        }

        if !close_menu {
            if let Some(rect) = menu_rect {
                if ui.input(|i| i.pointer.any_pressed()) {
                    let hover = ui.input(|i| i.pointer.hover_pos());
                    if hover.is_none_or(|pos| !rect.contains(pos)) {
                        close_menu = true;
                    }
                }
            }
        }

        if close_menu {
            self.node_menu_open = false;
            self.node_menu_node = None;
        }

        changed
    }

    pub fn open_add_menu(&mut self, pos: Pos2) {
        self.add_menu_open = true;
        self.add_menu_screen_pos = pos;
        self.add_menu_filter.clear();
        self.add_menu_focus = true;
        if self.graph_transform.valid {
            self.add_menu_graph_pos = self.graph_transform.to_global.inverse() * pos;
        } else {
            self.add_menu_graph_pos = self.next_pos;
        }
    }

    pub fn add_demo_graph(&mut self, graph: &mut Graph) {
        let origin = self.next_pos;
        let box_id = add_builtin_node(
            graph,
            &mut self.snarl,
            &mut self.core_to_snarl,
            &mut self.snarl_to_core,
            BuiltinNodeKind::Box,
            origin,
        );
        let transform_id = add_builtin_node(
            graph,
            &mut self.snarl,
            &mut self.core_to_snarl,
            &mut self.snarl_to_core,
            BuiltinNodeKind::Transform,
            Pos2::new(origin.x + 240.0, origin.y),
        );
        let output_id = add_builtin_node_checked(
            graph,
            &mut self.snarl,
            &mut self.core_to_snarl,
            &mut self.snarl_to_core,
            BuiltinNodeKind::Output,
            Pos2::new(origin.x + 480.0, origin.y),
        );

        let box_out = graph
            .node(box_id)
            .and_then(|node| node.outputs.first().copied());
        let transform_in = graph
            .node(transform_id)
            .and_then(|node| node.inputs.first().copied());
        let transform_out = graph
            .node(transform_id)
            .and_then(|node| node.outputs.first().copied());
        let output_in = output_id
            .and_then(|id| graph.node(id))
            .and_then(|node| node.inputs.first().copied());

        if let (Some(box_out), Some(transform_in), Some(transform_out), Some(output_in)) =
            (box_out, transform_in, transform_out, output_in)
        {
            let _ = graph.add_link(box_out, transform_in);
            let _ = graph.add_link(transform_out, output_in);
        }

        self.needs_wire_sync = true;
    }

    pub(super) fn show_add_menu(
        &mut self,
        ui: &mut Ui,
        graph: &mut Graph,
        settings: &mut ProjectSettings,
    ) -> bool {
        let mut close_menu = ui.input(|i| i.key_pressed(egui::Key::Escape));
        let mut menu_rect = None;
        let activate_first = ui.input(|i| i.key_pressed(egui::Key::Enter));
        let mut changed = false;

        let response = egui::Window::new("add_node_menu")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::LEFT_TOP, self.add_menu_screen_pos.to_vec2())
            .frame(Frame::popup(ui.style()))
            .show(ui.ctx(), |ui| {
                ui.label("Add node");
                ui.separator();
                if ui.button("Note").clicked() {
                    let graph_pos = self.add_menu_graph_pos;
                    if self.add_note(settings, graph_pos) {
                        changed = true;
                    }
                    close_menu = true;
                    return;
                }
                ui.separator();
                let search_id = ui.make_persistent_id("add_node_search");
                let search = egui::TextEdit::singleline(&mut self.add_menu_filter)
                    .id(search_id)
                    .hint_text("Search...");
                let search_response = ui.add(search);
                if self.add_menu_focus {
                    ui.memory_mut(|mem| mem.request_focus(search_id));
                    self.add_menu_focus = false;
                }
                if search_response.has_focus() && activate_first {
                    ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                }

        let items = builtin_menu_items();
        let filter = self.add_menu_filter.to_lowercase();
        if filter.is_empty() {
            let layout = menu_layout(&items);
            if let Some(kind) = render_menu_layout(ui, layout) {
                if let Some(core_id) = self.try_add_node(graph, kind, self.add_menu_graph_pos) {
                    changed = true;
                    if let Some(pending) = self.pending_wire.take() {
                        if self.connect_pending_wire(graph, core_id, pending) {
                            changed = true;
                        }
                    }
                }
                close_menu = true;
            }
        } else {
                    let mut matched = false;
                    let mut first_match: Option<BuiltinNodeKind> = None;
                    for item in items {
                        let submenu = item.submenu.unwrap_or("");
                        if !item.name.to_lowercase().contains(&filter)
                            && !item.category.to_lowercase().contains(&filter)
                            && !submenu.to_lowercase().contains(&filter)
                        {
                            continue;
                        }
                        matched = true;
                        if first_match.is_none() {
                            first_match = Some(item.kind);
                        }
                        let label = if submenu.is_empty() {
                            item.name.to_string()
                        } else {
                            format!("{} / {}", submenu, item.name)
                        };
                        if ui.button(label).clicked() {
                            if let Some(core_id) =
                                self.try_add_node(graph, item.kind, self.add_menu_graph_pos)
                            {
                                changed = true;
                                if let Some(pending) = self.pending_wire.take() {
                                    if self.connect_pending_wire(graph, core_id, pending) {
                                        changed = true;
                                    }
                                }
                            }
                            close_menu = true;
                        }
                    }
                    if !matched {
                        ui.label("No matches.");
                    } else if activate_first {
                        if let Some(kind) = first_match {
                            if let Some(core_id) =
                                self.try_add_node(graph, kind, self.add_menu_graph_pos)
                            {
                                changed = true;
                                if let Some(pending) = self.pending_wire.take() {
                                    if self.connect_pending_wire(graph, core_id, pending) {
                                        changed = true;
                                    }
                                }
                            }
                            close_menu = true;
                        }
                    }
                }
            });

        if let Some(inner) = response {
            menu_rect = Some(inner.response.rect);
        }

        if !close_menu {
            if let Some(rect) = menu_rect {
                if ui.input(|i| i.pointer.any_pressed()) {
                    let hover = ui.input(|i| i.pointer.hover_pos());
                    if hover.is_none_or(|pos| !rect.contains(pos)) {
                        close_menu = true;
                    }
                }
            }
        }

        if close_menu {
            self.add_menu_open = false;
            self.pending_wire = None;
        }

        changed
    }

    fn try_add_node(
        &mut self,
        graph: &mut Graph,
        kind: BuiltinNodeKind,
        pos: Pos2,
    ) -> Option<NodeId> {
        let core_id = add_builtin_node_checked(
            graph,
            &mut self.snarl,
            &mut self.core_to_snarl,
            &mut self.snarl_to_core,
            kind,
            pos,
        )?;
        self.needs_wire_sync = true;
        Some(core_id)
    }
}
