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
        if undo_pressed {
            self.try_undo();
        } else if redo_pressed {
            self.try_redo();
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
