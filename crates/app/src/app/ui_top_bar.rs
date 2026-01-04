use eframe::egui;

use super::LobedoApp;

impl LobedoApp {
    pub(super) fn show_top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.new_project();
                        ui.close();
                    }

                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if ui.button("Open...").clicked() {
                            self.open_project_dialog();
                            ui.close();
                        }

                        if ui.button("Save").clicked() {
                            if let Some(path) = self.project_path.clone() {
                                if let Err(err) = self.save_project_to(&path) {
                                    tracing::error!("failed to save project: {}", err);
                                } else {
                                    tracing::info!("project saved");
                                }
                            } else {
                                tracing::warn!("no project path set; use Save As");
                            }
                            ui.close();
                        }

                        if ui.button("Save As...").clicked() {
                            self.save_project_dialog();
                            ui.close();
                        }
                    }

                    #[cfg(target_arch = "wasm32")]
                    {
                        ui.add_enabled(false, egui::Button::new("Open..."));
                        ui.add_enabled(false, egui::Button::new("Save"));
                        ui.add_enabled(false, egui::Button::new("Save As..."));
                        ui.label("File I/O is not available in web builds.");
                    }
                });

                ui.separator();
                ui.label("Lobedo");
                ui.separator();
                ui.checkbox(
                    &mut self.project.settings.panels.show_inspector,
                    "Parameters",
                );
                ui.checkbox(
                    &mut self.project.settings.panels.show_spreadsheet,
                    "Spreadsheet",
                );
                ui.checkbox(&mut self.project.settings.panels.show_debug, "Debug");
                ui.checkbox(&mut self.project.settings.panels.show_console, "Console");
            });
        });
    }
}
