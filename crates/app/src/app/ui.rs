use eframe::egui;

use super::LobedoApp;

impl eframe::App for LobedoApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.sync_wgpu_renderer(frame);
        self.update_window_title(ctx);
        let pointer_down = ctx.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
        self.pause_viewport = ctx
            .input(|i| i.pointer.interact_pos().or_else(|| i.pointer.hover_pos()))
            .zip(self.last_node_graph_rect)
            .is_some_and(|(pos, rect)| pointer_down && rect.contains(pos));
        if ctx.input(|i| i.pointer.any_released()) {
            self.flush_pending_undo();
        }
        self.handle_keyboard_shortcuts(ctx);
        let mut undo_pushed = false;
        self.handle_tab_add_menu(ctx);
        self.show_top_bar(ctx);
        self.show_preferences_window(ctx);
        self.show_side_panels(ctx, pointer_down, &mut undo_pushed);
        self.show_central_panel(ctx, pointer_down, &mut undo_pushed);
        self.sync_selection_overlay();
        self.handle_info_panels(ctx);
        self.evaluate_if_needed();
    }
}
