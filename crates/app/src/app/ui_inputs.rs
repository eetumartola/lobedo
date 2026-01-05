use eframe::egui;

use super::LobedoApp;

impl LobedoApp {
    pub(super) fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input() {
            return;
        }
        let undo_pressed = ctx.input(|i| {
            i.key_pressed(egui::Key::Z) && i.modifiers.command && !i.modifiers.shift
        });
        let redo_pressed = ctx.input(|i| {
            (i.key_pressed(egui::Key::Z) && i.modifiers.command && i.modifiers.shift)
                || (i.key_pressed(egui::Key::Y) && i.modifiers.command)
        });
        let fit_pressed = ctx.input(|i| i.key_pressed(egui::Key::A));
        let delete_pressed = ctx.input(|i| {
            i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)
        });
        if undo_pressed {
            self.try_undo();
        } else if redo_pressed {
            self.try_redo();
        } else if delete_pressed {
            if self.node_graph.selected_node_id().is_some() {
                ctx.input_mut(|i| {
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Delete);
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace);
                });
                let snapshot = self.snapshot_undo();
                if self
                    .node_graph
                    .delete_selected_node(&mut self.project.graph)
                {
                    self.mark_eval_dirty();
                    self.queue_undo_snapshot(snapshot, false);
                }
            }
        } else if fit_pressed {
            if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
                if let Some(rect) = self.last_viewport_rect {
                    if rect.contains(pos) {
                        ctx.input_mut(|i| {
                            i.consume_key(egui::Modifiers::NONE, egui::Key::A);
                        });
                        self.fit_viewport_to_scene();
                        return;
                    }
                }
                if let Some(rect) = self.last_node_graph_rect {
                    if rect.contains(pos) {
                        ctx.input_mut(|i| {
                            i.consume_key(egui::Modifiers::NONE, egui::Key::A);
                        });
                        self.node_graph.fit_to_rect(rect);
                    }
                }
            }
        }
    }

    pub(super) fn handle_tab_add_menu(&mut self, ctx: &egui::Context) {
        let tab_pressed = ctx.input(|i| i.key_pressed(egui::Key::Tab));
        if !tab_pressed {
            return;
        }

        let hover_pos = ctx.input(|i| i.pointer.hover_pos());
        if let (Some(rect), Some(pos)) = (self.last_node_graph_rect, hover_pos) {
            if rect.contains(pos) && !ctx.wants_keyboard_input() {
                ctx.input_mut(|i| {
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
                });
                self.node_graph.open_add_menu(pos);
            }
        }
    }
}
