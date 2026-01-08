use std::collections::BTreeMap;

use eframe::egui;

use lobedo_core::{AttributeDomain, AttributeInfo, AttributeType, Geometry};

use crate::app::LobedoApp;

pub(super) struct NodeInfoPanel {
    pub(super) node_id: lobedo_core::NodeId,
    pub(super) screen_pos: egui::Pos2,
    pub(super) open: bool,
}

impl LobedoApp {
    pub(super) fn show_node_info_panel(
        &mut self,
        ctx: &egui::Context,
        panel_slot: &mut Option<NodeInfoPanel>,
    ) {
        let Some(mut panel) = panel_slot.take() else {
            return;
        };
        if !panel.open {
            return;
        }

        let Some(node) = self.project.graph.node(panel.node_id) else {
            return;
        };

        let title = format!("Node Info - {}", node.name);
        let mut open = panel.open;
        let node_id = panel.node_id;
        let window = egui::Window::new(title)
            .collapsible(true)
            .resizable(true)
            .default_pos(panel.screen_pos)
            .open(&mut open);

        window.show(ctx, |ui| {
            if let Some(geometry) = self.eval_state.geometry_for_node(node_id) {
                self.show_geometry_info(ui, geometry, node);
            } else {
                ui.label("No geometry available for this node.");
            }
        });

        panel.open = open;
        if panel.open {
            *panel_slot = Some(panel);
        }
    }

    fn show_geometry_info(
        &self,
        ui: &mut egui::Ui,
        geometry: &Geometry,
        node: &lobedo_core::Node,
    ) {
        let splat_count: usize = geometry.splats.iter().map(|s| s.len()).sum();
        let volume_voxels: u64 = geometry
            .volumes
            .iter()
            .map(|v| v.dims[0] as u64 * v.dims[1] as u64 * v.dims[2] as u64)
            .sum();
        ui.heading("Geometry");
        egui::Grid::new("node_info_geo_counts")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Mesh Primitives");
                ui.label(geometry.meshes.len().to_string());
                ui.end_row();

                ui.label("Splat Primitives");
                ui.label(geometry.splats.len().to_string());
                ui.end_row();

                ui.label("Splat Count");
                ui.label(splat_count.to_string());
                ui.end_row();

                ui.label("Curve Primitives");
                ui.label(geometry.curves.len().to_string());
                ui.end_row();

                ui.label("Volume Primitives");
                ui.label(geometry.volumes.len().to_string());
                ui.end_row();

                ui.label("Volume Voxels");
                ui.label(volume_voxels.to_string());
                ui.end_row();
            });

        if (node.name == "Splat Read" || node.name == "Read Splats") && !geometry.splats.is_empty()
        {
            ui.separator();
            ui.heading("Splats");
            let splat_geo = &geometry.splats[0];
            if geometry.splats.len() > 1 {
                ui.label("Multiple splat primitives; showing the first.");
            }
            egui::Grid::new("node_info_splat_file")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Path");
                    ui.label(node.params.get_string("path", "<unset>"));
                    ui.end_row();

                    ui.label("Splat Count");
                    ui.label(splat_geo.len().to_string());
                    ui.end_row();

                    ui.label("SH Coeffs / Channel");
                    ui.label(splat_geo.sh_coeffs.to_string());
                    ui.end_row();

                    ui.label("SH Order");
                    ui.label(sh_order_label(splat_geo.sh_coeffs));
                    ui.end_row();
                });
        }

        if !geometry.volumes.is_empty() {
            ui.separator();
            ui.heading("Volumes");
            if geometry.volumes.len() > 1 {
                ui.label("Multiple volumes; showing the first.");
            }
            let volume = &geometry.volumes[0];
            let kind = match volume.kind {
                lobedo_core::VolumeKind::Density => "Density",
                lobedo_core::VolumeKind::Sdf => "SDF",
            };
            let (min, max) = volume.world_bounds();
            let center = (min + max) * 0.5;
            let size = max - min;
            egui::Grid::new("node_info_volume")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Kind");
                    ui.label(kind);
                    ui.end_row();

                    ui.label("Dims");
                    ui.label(format!(
                        "{} {} {}",
                        volume.dims[0], volume.dims[1], volume.dims[2]
                    ));
                    ui.end_row();

                    ui.label("Voxel Size");
                    ui.label(format!("{:.4}", volume.voxel_size));
                    ui.end_row();

                    ui.label("Bounds Center");
                    ui.label(format!("{:.3} {:.3} {:.3}", center.x, center.y, center.z));
                    ui.end_row();

                    ui.label("Bounds Size");
                    ui.label(format!("{:.3} {:.3} {:.3}", size.x, size.y, size.z));
                    ui.end_row();
                });
        }

        self.show_groups_section(ui, geometry);

        if !geometry.meshes.is_empty() || !geometry.curves.is_empty() {
            ui.separator();
            self.show_mesh_info(ui, geometry);
        }
    }

    fn show_groups_section(&self, ui: &mut egui::Ui, geometry: &Geometry) {
        let merged_mesh = geometry.merged_mesh();
        let merged_splats = geometry.merged_splats();
        if merged_mesh.is_none() && merged_splats.is_none() {
            return;
        }

        ui.separator();
        ui.heading("Groups");

        if let Some(mesh) = merged_mesh {
            self.show_group_table(
                ui,
                "Point",
                mesh.groups.map(AttributeDomain::Point),
                mesh.positions.len(),
            );
            self.show_group_table(
                ui,
                "Vertex",
                mesh.groups.map(AttributeDomain::Vertex),
                mesh.indices.len(),
            );
            self.show_group_table(
                ui,
                "Primitive",
                mesh.groups.map(AttributeDomain::Primitive),
                mesh.indices.len() / 3,
            );
        }

        if let Some(splats) = merged_splats {
            self.show_group_table(
                ui,
                "Splat Point",
                splats.groups.map(AttributeDomain::Point),
                splats.len(),
            );
            self.show_group_table(
                ui,
                "Splat Primitive",
                splats.groups.map(AttributeDomain::Primitive),
                splats.len(),
            );
        }
    }

    fn show_group_table(
        &self,
        ui: &mut egui::Ui,
        label: &str,
        groups: &BTreeMap<String, Vec<bool>>,
        total: usize,
    ) {
        if groups.is_empty() {
            return;
        }
        ui.label(format!("{label} Groups"));
        egui::Grid::new(format!("node_info_groups_{label}"))
            .num_columns(2)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                for (name, values) in groups {
                    let count = values.iter().filter(|v| **v).count();
                    ui.label(name);
                    ui.label(format!("{count}/{total}"));
                    ui.end_row();
                }
            });
        ui.add_space(6.0);
    }

    fn show_mesh_info(&self, ui: &mut egui::Ui, geometry: &Geometry) {
        let mesh = geometry.merged_mesh();
        let mesh_point_count = mesh.as_ref().map(|m| m.positions.len()).unwrap_or(0);
        let mesh_vertex_count = mesh.as_ref().map(|m| m.indices.len()).unwrap_or(0);
        let mesh_prim_count = mesh
            .as_ref()
            .map(|m| m.indices.len() / 3)
            .unwrap_or(0);
        let curve_vertex_count: usize = geometry.curves.iter().map(|c| c.indices.len()).sum();
        let curve_prim_count = geometry.curves.len();
        let point_count = mesh_point_count;
        let vertex_count = mesh_vertex_count + curve_vertex_count;
        let prim_count = mesh_prim_count + curve_prim_count;
        let detail_count = if point_count == 0 && vertex_count == 0 {
            0
        } else {
            1
        };

        ui.heading("Counts");
        egui::Grid::new("node_info_counts")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label("Points");
                ui.label(point_count.to_string());
                ui.end_row();

                ui.label("Vertices");
                ui.label(vertex_count.to_string());
                ui.end_row();

                ui.label("Primitives");
                ui.label(prim_count.to_string());
                ui.end_row();

                ui.label("Detail");
                ui.label(detail_count.to_string());
                ui.end_row();
            });

        ui.separator();
        ui.heading("Bounds");
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        let mut has_bounds = false;
        if let Some(mesh) = mesh.as_ref() {
            if let Some(bounds) = mesh.bounds() {
                min = bounds.min;
                max = bounds.max;
                has_bounds = true;
            }
        }
        if has_bounds {
            let center = [
                (min[0] + max[0]) * 0.5,
                (min[1] + max[1]) * 0.5,
                (min[2] + max[2]) * 0.5,
            ];
            let size = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
            egui::Grid::new("node_info_bounds")
                .num_columns(4)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Center");
                    ui.label(format!("{:.3}", center[0]));
                    ui.label(format!("{:.3}", center[1]));
                    ui.label(format!("{:.3}", center[2]));
                    ui.end_row();

                    ui.label("Size");
                    ui.label(format!("{:.3}", size[0]));
                    ui.label(format!("{:.3}", size[1]));
                    ui.label(format!("{:.3}", size[2]));
                    ui.end_row();

                    ui.label("Min");
                    ui.label(format!("{:.3}", min[0]));
                    ui.label(format!("{:.3}", min[1]));
                    ui.label(format!("{:.3}", min[2]));
                    ui.end_row();

                    ui.label("Max");
                    ui.label(format!("{:.3}", max[0]));
                    ui.label(format!("{:.3}", max[1]));
                    ui.label(format!("{:.3}", max[2]));
                    ui.end_row();
                });
        } else {
            ui.label("No bounds available.");
        }

        let Some(mesh) = mesh.as_ref() else {
            return;
        };

        ui.separator();
        ui.heading("Attributes");
        let mut attrs = mesh.list_attributes();
        attrs.sort_by(|a, b| {
            let domain_order = |domain| match domain {
                AttributeDomain::Vertex => 0,
                AttributeDomain::Point => 1,
                AttributeDomain::Primitive => 2,
                AttributeDomain::Detail => 3,
            };
            domain_order(a.domain)
                .cmp(&domain_order(b.domain))
                .then_with(|| a.name.cmp(&b.name))
        });
        for domain in [
            AttributeDomain::Point,
            AttributeDomain::Vertex,
            AttributeDomain::Primitive,
            AttributeDomain::Detail,
        ] {
            let group: Vec<&AttributeInfo> = attrs.iter().filter(|a| a.domain == domain).collect();
            if group.is_empty() {
                continue;
            }
            ui.label(format!(
                "{} Attributes ({})",
                attribute_domain_label(domain),
                group.len()
            ));
            egui::Grid::new(format!("node_info_attr_{:?}", domain))
                .num_columns(3)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    for attr in group {
                        let mut name = attr.name.clone();
                        if attr.implicit {
                            name.push_str(" (implicit)");
                        }
                        ui.label(name);
                        ui.label(attribute_type_label(attr.data_type));
                        ui.label(attr.len.to_string());
                        ui.end_row();
                    }
                });
            ui.add_space(8.0);
        }
    }
}

fn attribute_type_label(ty: AttributeType) -> &'static str {
    match ty {
        AttributeType::Float => "Flt",
        AttributeType::Int => "Int",
        AttributeType::Vec2 => "2-Flt",
        AttributeType::Vec3 => "3-Flt",
        AttributeType::Vec4 => "4-Flt",
        AttributeType::String => "Str",
    }
}

fn attribute_domain_label(domain: AttributeDomain) -> &'static str {
    match domain {
        AttributeDomain::Point => "Point",
        AttributeDomain::Vertex => "Vertex",
        AttributeDomain::Primitive => "Primitive",
        AttributeDomain::Detail => "Detail",
    }
}

fn sh_order_label(sh_coeffs: usize) -> String {
    let total = 1 + sh_coeffs;
    let order = (total as f32).sqrt().round() as usize;
    if order * order == total && order > 0 {
        let max_l = order - 1;
        format!("L{} ({} bands)", max_l, max_l + 1)
    } else {
        format!("Partial ({} coeffs)", total)
    }
}
