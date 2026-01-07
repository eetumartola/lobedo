use eframe::egui;
use lobedo_core::ShadingMode;

use super::LobedoApp;

impl LobedoApp {
    pub(super) fn show_side_panels(
        &mut self,
        ctx: &egui::Context,
        pointer_down: bool,
        undo_pushed: &mut bool,
    ) {
        if !self.project.settings.panels.show_debug && !self.project.settings.panels.show_console {
            return;
        }

        egui::SidePanel::right("side_panels")
            .resizable(true)
            .default_width(280.0)
            .show(ctx, |ui| {
                if self.project.settings.panels.show_debug {
                    egui::CollapsingHeader::new("Debug")
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.label("Viewport overlays");
                            ui.checkbox(
                                &mut self.project.settings.render_debug.show_grid,
                                "Grid",
                            );
                            ui.checkbox(
                                &mut self.project.settings.render_debug.show_axes,
                                "Axes",
                            );
                            ui.checkbox(
                                &mut self.project.settings.render_debug.show_normals,
                                "Normals",
                            );
                            if self.project.settings.render_debug.show_normals {
                                ui.horizontal(|ui| {
                                    ui.label("Normal length");
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.project.settings.render_debug.normal_length,
                                        )
                                        .speed(0.02)
                                        .range(0.01..=10.0),
                                    );
                                });
                            }
                            ui.checkbox(
                                &mut self.project.settings.render_debug.show_bounds,
                                "Bounds",
                            );
                            ui.checkbox(
                                &mut self.project.settings.render_debug.show_points,
                                "Points",
                            );
                            ui.checkbox(
                                &mut self.project.settings.render_debug.show_splats,
                                "Splats",
                            );
                            if self.project.settings.render_debug.show_points
                                || self.project.settings.render_debug.show_splats
                            {
                                ui.horizontal(|ui| {
                                    ui.label("Point size");
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.project.settings.render_debug.point_size,
                                        )
                                        .speed(0.5)
                                        .range(1.0..=24.0),
                                    );
                                });
                            }
                            ui.checkbox(
                                &mut self.project.settings.render_debug.key_shadows,
                                "Key shadows",
                            );
                            ui.checkbox(
                                &mut self.project.settings.render_debug.show_stats,
                                "Stats overlay",
                            );

                            ui.separator();
                            ui.label("Shading");
                            let shading = &mut self.project.settings.render_debug.shading_mode;
                            egui::ComboBox::from_label("Mode")
                                .selected_text(match shading {
                                    ShadingMode::Lit => "Lit",
                                    ShadingMode::Normals => "Normals",
                                    ShadingMode::Depth => "Depth",
                                    ShadingMode::SplatOpacity => "Splat Opacity",
                                    ShadingMode::SplatScale => "Splat Scale",
                                    ShadingMode::SplatOverdraw => "Splat Overdraw",
                                })
                                .show_ui(ui, |ui| {
                                    for (mode, label) in [
                                        (ShadingMode::Lit, "Lit"),
                                        (ShadingMode::Normals, "Normals"),
                                        (ShadingMode::Depth, "Depth"),
                                        (ShadingMode::SplatOpacity, "Splat Opacity"),
                                        (ShadingMode::SplatScale, "Splat Scale"),
                                        (ShadingMode::SplatOverdraw, "Splat Overdraw"),
                                    ] {
                                        if ui
                                            .selectable_label(*shading == mode, label)
                                            .clicked()
                                        {
                                            *shading = mode;
                                        }
                                    }
                                });

                            if *shading == ShadingMode::Depth {
                                ui.horizontal(|ui| {
                                    ui.label("Near");
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.project.settings.render_debug.depth_near,
                                        )
                                        .speed(0.1)
                                        .range(0.01..=1000.0),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Far");
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.project.settings.render_debug.depth_far,
                                        )
                                        .speed(0.1)
                                        .range(0.01..=5000.0),
                                    );
                                });
                                let near = self.project.settings.render_debug.depth_near;
                                let far = self.project.settings.render_debug.depth_far;
                                if far <= near + 0.01 {
                                    self.project.settings.render_debug.depth_far = near + 0.01;
                                }
                            }
                            if matches!(
                                *shading,
                                ShadingMode::SplatOpacity
                                    | ShadingMode::SplatScale
                                    | ShadingMode::SplatOverdraw
                            ) {
                                ui.horizontal(|ui| {
                                    ui.label("Min");
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.project.settings.render_debug.splat_debug_min,
                                        )
                                        .speed(0.05),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Max");
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut self.project.settings.render_debug.splat_debug_max,
                                        )
                                        .speed(0.05),
                                    );
                                });
                                let min = self.project.settings.render_debug.splat_debug_min;
                                let max = self.project.settings.render_debug.splat_debug_max;
                                if max <= min + 0.0001 {
                                    self.project.settings.render_debug.splat_debug_max =
                                        min + 0.0001;
                                }
                            }

                            ui.separator();
                            ui.label("Evaluation");
                            if ui.button("Create demo graph").clicked() {
                                let snapshot = self.snapshot_undo();
                                self.node_graph.add_demo_graph(&mut self.project.graph);
                                self.mark_eval_dirty();
                                if !*undo_pushed {
                                    self.queue_undo_snapshot(snapshot, pointer_down);
                                    *undo_pushed = true;
                                }
                            }
                            if ui.button("Recompute now").clicked() {
                                self.eval_dirty = false;
                                self.last_param_change = None;
                                self.evaluate_graph();
                            }

                            if let Some(report) = &self.last_eval_report {
                                let computed = report.computed.len();
                                ui.label(format!(
                                    "Computed: {}  Cache hits: {}  Misses: {}",
                                    computed, report.cache_hits, report.cache_misses
                                ));
                                if let Some(ms) = self.last_eval_ms {
                                    ui.label(format!("Last eval: {:.2} ms", ms));
                                }
                                if !report.output_valid {
                                    ui.colored_label(egui::Color32::RED, "Output invalid");
                                }
                                let mut nodes: Vec<_> = report.node_reports.values().collect();
                                nodes.sort_by(|a, b| {
                                    b.duration_ms
                                        .partial_cmp(&a.duration_ms)
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                });
                                for entry in nodes.into_iter().take(5) {
                                    ui.label(format!(
                                        "{:?}: {:.2} ms{}",
                                        entry.node,
                                        entry.duration_ms,
                                        if entry.cache_hit { " (cache)" } else { "" }
                                    ));
                                }
                                if !report.dirty.is_empty() {
                                    ui.separator();
                                    ui.label(format!("Dirty nodes: {}", report.dirty.len()));
                                    for entry in &report.dirty {
                                        let reason = match entry.reason {
                                            lobedo_core::DirtyReason::NewNode => "new",
                                            lobedo_core::DirtyReason::ParamChanged => "param",
                                            lobedo_core::DirtyReason::UpstreamChanged => "upstream",
                                            lobedo_core::DirtyReason::ParamAndUpstreamChanged => {
                                                "param+upstream"
                                            }
                                        };
                                        ui.label(format!("{:?}: {}", entry.node, reason));
                                    }
                                }
                            }

                            egui::ComboBox::from_label("Log level")
                                .selected_text(format!("{:?}", self.log_level))
                                .show_ui(ui, |ui| {
                                    for level in [
                                        tracing_subscriber::filter::LevelFilter::ERROR,
                                        tracing_subscriber::filter::LevelFilter::WARN,
                                        tracing_subscriber::filter::LevelFilter::INFO,
                                        tracing_subscriber::filter::LevelFilter::DEBUG,
                                        tracing_subscriber::filter::LevelFilter::TRACE,
                                    ] {
                                        if ui
                                            .selectable_label(
                                                self.log_level == level,
                                                format!("{:?}", level),
                                            )
                                            .clicked()
                                        {
                                            self.set_log_level(level);
                                        }
                                    }
                                });
                        });
                }

                if self.project.settings.panels.show_console {
                    egui::CollapsingHeader::new("Console")
                        .default_open(true)
                        .show(ui, |ui| {
                            let console_lines = self.console.snapshot();
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                for line in console_lines {
                                    ui.label(line);
                                }
                            });
                        });
                }
            });
    }
}
