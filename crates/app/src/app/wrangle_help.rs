use eframe::egui;

use crate::app::LobedoApp;

pub(super) struct WrangleHelpPanel {
    pub(super) screen_pos: egui::Pos2,
    pub(super) open: bool,
}

impl LobedoApp {
    pub(super) fn show_wrangle_help_panel(
        &mut self,
        ctx: &egui::Context,
        panel_slot: &mut Option<WrangleHelpPanel>,
    ) {
        let Some(mut panel) = panel_slot.take() else {
            return;
        };
        if !panel.open {
            return;
        }

        let mut open = panel.open;
        let window = egui::Window::new("Wrangle Help")
            .collapsible(true)
            .resizable(true)
            .default_pos(panel.screen_pos)
            .open(&mut open);

        window.show(ctx, |ui| {
            ui.heading("Wrangle Cheat Sheet");
            ui.separator();
            ui.label("Assignments (use @ for attributes):");
            ui.monospace("@Cd = vec3(1.0, 0.0, 0.0);");
            ui.monospace("@P = @P + vec3(0.0, 1.0, 0.0);");
            ui.separator();
            ui.label("Implicit attributes:");
            ui.monospace("@ptnum @vtxnum @primnum @numpt @numvtx @numprim");
            ui.separator();
            ui.label("Swizzles:");
            ui.monospace("@P.x");
            ui.monospace("@N.xy");
            ui.monospace("@Cd.rgb");
            ui.separator();
            ui.label("Functions:");
            ui.monospace(
                "sin cos tan abs floor ceil pow min max clamp lerp len dot normalize",
            );
            ui.monospace("point vertex prim splat sample");
            ui.monospace("point(0, P, @ptnum)");
            ui.monospace("splat(1, P, @ptnum)");
            ui.monospace("sample(1, @P)");
            ui.separator();
            ui.label("Inputs:");
            ui.label("point/vertex/prim/splat take (input_index, attr_name, element_index).");
            ui.label("Input 0 is the main input; input 1 is the secondary input.");
            ui.label("splat() always queries splat centers.");
            ui.separator();
            ui.label("Constructors:");
            ui.monospace("vec2(x, y) vec3(x, y, z) vec4(x, y, z, w)");
            ui.separator();
            ui.label("Operators:");
            ui.monospace("+ - * / ( )");
            ui.separator();
            ui.label("Notes:");
            ui.label("Mode selects vertex/point/prim/detail evaluation.");
            ui.label("@P writes only in Point mode; @N writes in Point/Vertex.");
            ui.label("Missing attributes read as 0.");
        });

        panel.open = open;
        if panel.open {
            *panel_slot = Some(panel);
        }
    }
}
