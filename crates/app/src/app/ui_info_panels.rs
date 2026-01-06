use eframe::egui;

use super::node_info::NodeInfoPanel;
use super::wrangle_help::WrangleHelpPanel;
use super::LobedoApp;

impl LobedoApp {
    pub(super) fn handle_info_panels(&mut self, ctx: &egui::Context) {
        if let Some(pos) = self.node_graph.take_wrangle_help_request() {
            self.wrangle_help_panel = Some(WrangleHelpPanel {
                screen_pos: pos,
                open: true,
            });
        }

        let middle_down = ctx.input(|i| i.pointer.button_down(egui::PointerButton::Middle));
        if middle_down {
            let hover_pos = ctx.input(|i| i.pointer.hover_pos().or_else(|| i.pointer.latest_pos()));
            if let Some(pos) = hover_pos {
                if let Some(node_id) = self.node_graph.node_at_screen_pos(pos) {
                    self.held_info_panel = Some(NodeInfoPanel {
                        node_id,
                        screen_pos: pos,
                        open: true,
                    });
                } else {
                    self.held_info_panel = None;
                }
            }
        } else {
            self.held_info_panel = None;
        }

        if let Some(request) = self.node_graph.take_info_request() {
            self.info_panel = Some(NodeInfoPanel {
                node_id: request.node_id,
                screen_pos: request.screen_pos,
                open: true,
            });
        }

        let mut info_panel = self.info_panel.take();
        self.show_node_info_panel(ctx, &mut info_panel);
        self.info_panel = info_panel;

        let mut held_info_panel = self.held_info_panel.take();
        self.show_node_info_panel(ctx, &mut held_info_panel);
        self.held_info_panel = held_info_panel;

        let mut wrangle_help_panel = self.wrangle_help_panel.take();
        self.show_wrangle_help_panel(ctx, &mut wrangle_help_panel);
        self.wrangle_help_panel = wrangle_help_panel;

    }
}
