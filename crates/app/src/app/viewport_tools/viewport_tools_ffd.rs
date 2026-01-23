use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use glam::Vec3;

use lobedo_core::{encode_curve_points, parse_curve_points, BuiltinNodeKind, NodeId, ParamValue};

use super::{GizmoAxis, LobedoApp};
use super::viewport_tools_gizmo::{axis_color, axis_dir, gizmo_scale};
use super::viewport_tools_math::{project_world_to_screen, viewport_view_proj};

const FFD_MIN_AXIS_SIZE: f32 = 1.0e-6;

impl LobedoApp {
    pub(crate) fn ensure_ffd_lattice_points(&mut self, node_id: NodeId) {
        let params = {
            let Some(node) = self.project.graph.node(node_id) else {
                return;
            };
            if node.builtin_kind() != Some(BuiltinNodeKind::Ffd) {
                return;
            }
            node.params.clone()
        };
        let (res_x, res_y, res_z) = ffd_resolution(&params);
        let total = res_x * res_y * res_z;
        let points = parse_curve_points(params.get_string("lattice_points", ""));
        if points.len() == total && !points.is_empty() {
            return;
        }
        let use_input_bounds = params.get_bool("use_input_bounds", true);
        let padding = params.get_float("padding", 0.0).max(0.0);
        let mut bounds = if use_input_bounds {
            self.ffd_input_bounds(node_id)
        } else {
            None
        };
        if bounds.is_none() {
            bounds = Some(ffd_bounds_from_params(&params));
        }
        let (mut min, mut max) = bounds.unwrap_or((Vec3::ZERO, Vec3::ONE));
        if padding > 0.0 {
            let pad = Vec3::splat(padding);
            min -= pad;
            max += pad;
        }
        let points = default_ffd_points(res_x, res_y, res_z, min, max);
        let _ = self.set_ffd_points(node_id, &points);
    }

    pub(super) fn update_ffd_point(&mut self, node_id: NodeId, index: usize, point: Vec3) -> bool {
        let raw = {
            let Some(node) = self.project.graph.node(node_id) else {
                return false;
            };
            if node.builtin_kind() != Some(BuiltinNodeKind::Ffd) {
                return false;
            }
            node.params.get_string("lattice_points", "")
        };
        let mut points = parse_curve_points(raw);
        if index >= points.len() {
            return false;
        }
        points[index] = point.to_array();
        self.set_ffd_points(node_id, &points)
    }

    pub(super) fn set_ffd_points(&mut self, node_id: NodeId, points: &[[f32; 3]]) -> bool {
        let encoded = encode_curve_points(points);
        if self
            .project
            .graph
            .set_param(node_id, "lattice_points".to_string(), ParamValue::String(encoded))
            .is_ok()
        {
            self.mark_eval_dirty();
            return true;
        }
        false
    }

    pub(super) fn ffd_input_bounds(&self, node_id: NodeId) -> Option<(Vec3, Vec3)> {
        if let Some(input_id) = super::input_node_for(&self.project.graph, node_id, 0) {
            if let Some(geometry) = self.eval_state.geometry_for_node(input_id) {
                if let Some(bounds) = geometry_bounds(geometry) {
                    return Some(bounds);
                }
            }
        }
        let geometry = self.eval_state.geometry_for_node(node_id)?;
        geometry_bounds(geometry)
    }
}

fn geometry_bounds(geometry: &lobedo_core::Geometry) -> Option<(Vec3, Vec3)> {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    let mut any = false;
    for mesh in &geometry.meshes {
        for pos in &mesh.positions {
            let v = Vec3::from(*pos);
            if !v.is_finite() {
                continue;
            }
            min = min.min(v);
            max = max.max(v);
            any = true;
        }
    }
    for splat in &geometry.splats {
        for pos in &splat.positions {
            let v = Vec3::from(*pos);
            if !v.is_finite() {
                continue;
            }
            min = min.min(v);
            max = max.max(v);
            any = true;
        }
    }
    if any {
        Some((min, max))
    } else {
        None
    }
}

fn ffd_resolution(params: &lobedo_core::NodeParams) -> (usize, usize, usize) {
    let res_x = params.get_int("res_x", 2).max(2) as usize;
    let res_y = params.get_int("res_y", 2).max(2) as usize;
    let res_z = params.get_int("res_z", 2).max(2) as usize;
    (res_x, res_y, res_z)
}

fn ffd_bounds_from_params(params: &lobedo_core::NodeParams) -> (Vec3, Vec3) {
    let center = Vec3::from(params.get_vec3("center", [0.0, 0.0, 0.0]));
    let size = Vec3::from(params.get_vec3("size", [1.0, 1.0, 1.0])).abs();
    let half = size * 0.5;
    (center - half, center + half)
}

fn default_ffd_points(
    res_x: usize,
    res_y: usize,
    res_z: usize,
    min: Vec3,
    max: Vec3,
) -> Vec<[f32; 3]> {
    let size = (max - min).max(Vec3::splat(FFD_MIN_AXIS_SIZE));
    let mut points = Vec::with_capacity(res_x * res_y * res_z);
    for z in 0..res_z {
        let tz = if res_z > 1 { z as f32 / (res_z - 1) as f32 } else { 0.0 };
        let pz = min.z + size.z * tz;
        for y in 0..res_y {
            let ty = if res_y > 1 { y as f32 / (res_y - 1) as f32 } else { 0.0 };
            let py = min.y + size.y * ty;
            for x in 0..res_x {
                let tx = if res_x > 1 { x as f32 / (res_x - 1) as f32 } else { 0.0 };
                let px = min.x + size.x * tx;
                points.push([px, py, pz]);
            }
        }
    }
    points
}

fn ffd_point_index(res_x: usize, res_y: usize, x: usize, y: usize, z: usize) -> usize {
    x + res_x * (y + res_y * z)
}

pub(super) fn draw_ffd_lattice_overlay(app: &LobedoApp, ui: &egui::Ui, rect: Rect, node_id: NodeId) {
    let Some(node) = app.project.graph.node(node_id) else {
        return;
    };
    if node.builtin_kind() != Some(BuiltinNodeKind::Ffd) {
        return;
    }
    let (res_x, res_y, res_z) = ffd_resolution(&node.params);
    let total = res_x * res_y * res_z;
    let points = parse_curve_points(node.params.get_string("lattice_points", ""));
    if points.len() != total || points.is_empty() {
        return;
    }
    let points: Vec<Vec3> = points.into_iter().map(Vec3::from).collect();
    let view_proj = viewport_view_proj(app.camera_state(), rect, ui.ctx().pixels_per_point());
    let painter = ui.painter();
    let stroke = Stroke::new(1.5, Color32::from_rgb(240, 200, 90));

    for z in 0..res_z {
        for y in 0..res_y {
            for x in 0..res_x.saturating_sub(1) {
                let a = points[ffd_point_index(res_x, res_y, x, y, z)];
                let b = points[ffd_point_index(res_x, res_y, x + 1, y, z)];
                if let (Some(a), Some(b)) = (
                    project_world_to_screen(view_proj, rect, a),
                    project_world_to_screen(view_proj, rect, b),
                ) {
                    painter.line_segment([a, b], stroke);
                }
            }
        }
    }

    for z in 0..res_z {
        for x in 0..res_x {
            for y in 0..res_y.saturating_sub(1) {
                let a = points[ffd_point_index(res_x, res_y, x, y, z)];
                let b = points[ffd_point_index(res_x, res_y, x, y + 1, z)];
                if let (Some(a), Some(b)) = (
                    project_world_to_screen(view_proj, rect, a),
                    project_world_to_screen(view_proj, rect, b),
                ) {
                    painter.line_segment([a, b], stroke);
                }
            }
        }
    }

    for y in 0..res_y {
        for x in 0..res_x {
            for z in 0..res_z.saturating_sub(1) {
                let a = points[ffd_point_index(res_x, res_y, x, y, z)];
                let b = points[ffd_point_index(res_x, res_y, x, y, z + 1)];
                if let (Some(a), Some(b)) = (
                    project_world_to_screen(view_proj, rect, a),
                    project_world_to_screen(view_proj, rect, b),
                ) {
                    painter.line_segment([a, b], stroke);
                }
            }
        }
    }
}

pub(super) fn draw_ffd_lattice_handles(app: &LobedoApp, ui: &egui::Ui, rect: Rect, node_id: NodeId) {
    let Some(node) = app.project.graph.node(node_id) else {
        return;
    };
    if node.builtin_kind() != Some(BuiltinNodeKind::Ffd) {
        return;
    }
    let (res_x, res_y, res_z) = ffd_resolution(&node.params);
    let total = res_x * res_y * res_z;
    let points = parse_curve_points(node.params.get_string("lattice_points", ""));
    if points.len() != total || points.is_empty() {
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
pub(super) struct FfdHandlePick {
    pub index: usize,
    pub axis: Option<GizmoAxis>,
    pub point: Vec3,
}

pub(super) fn pick_ffd_handle(
    camera: render::CameraState,
    rect: Rect,
    pixels_per_point: f32,
    mouse: Pos2,
    node_id: NodeId,
    graph: &lobedo_core::Graph,
) -> Option<FfdHandlePick> {
    let node = graph.node(node_id)?;
    if node.builtin_kind() != Some(BuiltinNodeKind::Ffd) {
        return None;
    }
    let (res_x, res_y, res_z) = ffd_resolution(&node.params);
    let total = res_x * res_y * res_z;
    let points = parse_curve_points(node.params.get_string("lattice_points", ""));
    if points.len() != total || points.is_empty() {
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
            best = Some(FfdHandlePick {
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
                best = Some(FfdHandlePick {
                    index: idx,
                    axis: Some(axis),
                    point: world,
                });
            }
        }
    }
    best
}
