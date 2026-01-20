use eframe::egui;
use lobedo_core::SplatShadingMode;

use super::LobedoApp;

impl LobedoApp {
    pub(super) fn show_preferences_window(&mut self, ctx: &egui::Context) {
        if !self.show_preferences {
            return;
        }

        let mut open = self.show_preferences;
        egui::Window::new("Preferences")
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Rendering");
                ui.horizontal(|ui| {
                    ui.label("Splat shading");
                    let shading = &mut self.project.settings.render_debug.splat_shading_mode;
                    egui::ComboBox::from_id_salt("pref_splat_shading")
                        .selected_text(match shading {
                            SplatShadingMode::ColorOnly => "Color Only",
                            SplatShadingMode::FullSh => "Full SH",
                        })
                        .show_ui(ui, |ui| {
                            for (mode, label) in [
                                (SplatShadingMode::ColorOnly, "Color Only"),
                                (SplatShadingMode::FullSh, "Full SH"),
                            ] {
                                if ui.selectable_label(*shading == mode, label).clicked() {
                                    *shading = mode;
                                }
                            }
                    });
                });
                ui.checkbox(
                    &mut self.project.settings.render_debug.splat_depth_prepass,
                    "Depth prepass for splats",
                );
                ui.separator();
                ui.label("Splat tiling");
                let tile_ui_enabled = !cfg!(target_arch = "wasm32");
                if !tile_ui_enabled {
                    ui.label("Tile binning is disabled in web builds.");
                }
                ui.add_enabled(
                    tile_ui_enabled,
                    egui::Checkbox::new(
                        &mut self.project.settings.render_debug.splat_tile_binning,
                        "Enable tile binning",
                    ),
                );
                let tile_settings_enabled = tile_ui_enabled
                    && self.project.settings.render_debug.splat_tile_binning;
                ui.horizontal(|ui| {
                    ui.label("Tile size (px)");
                    ui.add_enabled(
                        tile_settings_enabled,
                        egui::DragValue::new(
                            &mut self.project.settings.render_debug.splat_tile_size,
                        )
                        .speed(1.0)
                        .update_while_editing(false),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Tile threshold");
                    ui.add_enabled(
                        tile_settings_enabled,
                        egui::DragValue::new(
                            &mut self.project.settings.render_debug.splat_tile_threshold,
                        )
                        .speed(1000.0)
                        .update_while_editing(false),
                    );
                });
            });
        self.show_preferences = open;
    }
}
