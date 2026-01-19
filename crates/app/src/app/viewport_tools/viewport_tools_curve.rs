use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use glam::Vec3;

use lobedo_core::{encode_curve_points, parse_curve_points, NodeId, ParamValue};

use super::{GizmoAxis, LobedoApp};
use super::viewport_tools_gizmo::{axis_color, axis_dir, gizmo_scale};
use super::viewport_tools_math::{project_world_to_screen, viewport_view_proj};

impl LobedoApp {
    pub(super) fn append_curve_point(&mut self, node_id: NodeId, point: Vec3) -> bool {
        let Some(node) = self.project.graph.node(node_id) else {
            return false;
        };
        let mut points = parse_curve_points(node.params.get_string("points", ""));
        points.push(point.to_array());
        self.set_curve_points(node_id, &points)
    }

    pub(super) fn update_curve_point(&mut self, node_id: NodeId, index: usize, point: Vec3) -> bool {
        let Some(node) = self.project.graph.node(node_id) else {
            return false;
        };
        let mut points = parse_curve_points(node.params.get_string("points", ""));
        if index >= points.len() {
            return false;
        }
        points[index] = point.to_array();
        self.set_curve_points(node_id, &points)
    }

    pub(super) fn set_curve_points(&mut self, node_id: NodeId, points: &[[f32; 3]]) -> bool {
        let encoded = encode_curve_points(points);
        if self
            .project
            .graph
            .set_param(node_id, "points".to_string(), ParamValue::String(encoded))
            .is_ok()
        {
            self.mark_eval_dirty();
            return true;
        }
        false
    }
}

pub(super) fn draw_curve_overlay(
    app: &LobedoApp,
    ui: &egui::Ui,
    rect: Rect,
    node_id: NodeId,
    draw_points: bool,
) {
    let Some(node) = app.project.graph.node(node_id) else {
        return;
    };
    let points = parse_curve_points(node.params.get_string("points", ""));
    let closed = node.params.get_bool("closed", false);
    if points.is_empty() {
        return;
    }
    let view_proj = viewport_view_proj(app.camera_state(), rect, ui.ctx().pixels_per_point());
    let painter = ui.painter();
    let mut first = None;
    let mut prev = None;
    for p in points {
        let world = Vec3::from(p);
        if let Some(screen) = project_world_to_screen(view_proj, rect, world) {
            if draw_points {
                painter.circle_filled(screen, 3.0, Color32::from_rgb(240, 200, 90));
            }
            if let Some(prev) = prev {
                painter.line_segment(
                    [prev, screen],
                    Stroke::new(2.0, Color32::from_rgb(240, 200, 90)),
                );
            }
            if first.is_none() {
                first = Some(screen);
            }
            prev = Some(screen);
        }
    }
    if closed {
        if let (Some(first), Some(last)) = (first, prev) {
            painter.line_segment(
                [last, first],
                Stroke::new(2.0, Color32::from_rgb(240, 200, 90)),
            );
        }
    }
}

pub(super) fn draw_curve_handles(app: &LobedoApp, ui: &egui::Ui, rect: Rect, node_id: NodeId) {
    let Some(node) = app.project.graph.node(node_id) else {
        return;
    };
    let points = parse_curve_points(node.params.get_string("points", ""));
    if points.is_empty() {
        return;
    }
    let view_proj = viewport_view_proj(app.camera_state(), rect, ui.ctx().pixels_per_point());
    let painter = ui.painter();
    let fill = Color32::from_rgb(240, 200, 90);
    let stroke = Stroke::new(1.0, Color32::from_rgb(255, 235, 170));
    for point in points {
        let world = Vec3::from(point);
        let Some(screen) = project_world_to_screen(view_proj, rect, world) else {
            continue;
        };
        let axis_len = gizmo_scale(view_proj, rect, world, 28.0);
        for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
            let dir = axis_dir(axis);
            let end_world = world + dir * axis_len;
            let Some(end_screen) = project_world_to_screen(view_proj, rect, end_world) else {
                continue;
            };
            let color = axis_color(axis);
            painter.line_segment([screen, end_screen], Stroke::new(2.0, color));
            painter.circle_filled(end_screen, 3.0, color);
        }
        painter.circle_filled(screen, 3.5, fill);
        painter.circle_stroke(screen, 5.5, stroke);
    }
}

#[derive(Clone, Copy)]
pub(super) struct CurveHandlePick {
    pub index: usize,
    pub axis: Option<GizmoAxis>,
    pub point: Vec3,
}

pub(super) fn pick_curve_handle(
    camera: render::CameraState,
    rect: Rect,
    pixels_per_point: f32,
    mouse: Pos2,
    node_id: NodeId,
    graph: &lobedo_core::Graph,
) -> Option<CurveHandlePick> {
    let node = graph.node(node_id)?;
    let points = parse_curve_points(node.params.get_string("points", ""));
    if points.is_empty() {
        return None;
    }
    let view_proj = viewport_view_proj(camera, rect, pixels_per_point);
    let mut best = None;
    let mut best_dist = f32::INFINITY;
    let point_threshold = 10.0;
    let axis_threshold = 6.0;
    for (idx, point) in points.iter().enumerate() {
        let world = Vec3::from(*point);
        let Some(screen) = project_world_to_screen(view_proj, rect, world) else {
            continue;
        };
        let dist = (screen - mouse).length();
        if dist <= point_threshold && dist < best_dist {
            best_dist = dist;
            best = Some(CurveHandlePick {
                index: idx,
                axis: None,
                point: world,
            });
        }

        let axis_len = gizmo_scale(view_proj, rect, world, 28.0);
        for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
            let end_world = world + axis_dir(axis) * axis_len;
            let Some(end_screen) = project_world_to_screen(view_proj, rect, end_world) else {
                continue;
            };
            let dist = super::viewport_tools_math::distance_to_segment(mouse, screen, end_screen);
            if dist <= axis_threshold && dist < best_dist {
                best_dist = dist;
                best = Some(CurveHandlePick {
                    index: idx,
                    axis: Some(axis),
                    point: world,
                });
            }
        }
    }
    best
}
