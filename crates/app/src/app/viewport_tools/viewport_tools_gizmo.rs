use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use glam::{EulerRot, Mat3, Mat4, Quat, Vec3};

use lobedo_core::{NodeId, ParamValue};

use super::{BoxDrag, BoxHandle, GizmoAxis, GizmoHit, TransformDrag, TransformMode};
use super::LobedoApp;
use super::viewport_tools_math::{project_world_to_screen, raycast_plane, viewport_view_proj};

pub(super) struct TransformParams {
    pub(super) translate: [f32; 3],
    pub(super) rotate: [f32; 3],
    pub(super) scale: [f32; 3],
    pub(super) pivot: [f32; 3],
}

pub(super) struct BoxParams {
    pub(super) center: Vec3,
    pub(super) size: Vec3,
}

pub(super) fn transform_params(graph: &lobedo_core::Graph, node_id: NodeId) -> TransformParams {
    let fallback = TransformParams {
        translate: [0.0, 0.0, 0.0],
        rotate: [0.0, 0.0, 0.0],
        scale: [1.0, 1.0, 1.0],
        pivot: [0.0, 0.0, 0.0],
    };
    let Some(node) = graph.node(node_id) else {
        return fallback;
    };
    TransformParams {
        translate: node.params.get_vec3("translate", fallback.translate),
        rotate: node.params.get_vec3("rotate_deg", fallback.rotate),
        scale: node.params.get_vec3("scale", fallback.scale),
        pivot: node.params.get_vec3("pivot", fallback.pivot),
    }
}

pub(super) fn transform_origin(graph: &lobedo_core::Graph, node_id: NodeId) -> Option<Vec3> {
    let params = transform_params(graph, node_id);
    let translate = Vec3::from(params.translate);
    let pivot = Vec3::from(params.pivot);
    Some(translate + pivot)
}

pub(super) fn transform_quat(rotate_deg: [f32; 3]) -> Quat {
    let rot = Vec3::from(rotate_deg) * std::f32::consts::PI / 180.0;
    Quat::from_euler(EulerRot::XYZ, rot.x, rot.y, rot.z)
}

pub(super) fn transform_basis(rotate_deg: [f32; 3]) -> Mat3 {
    Mat3::from_quat(transform_quat(rotate_deg))
}

fn quat_to_euler_deg(quat: Quat) -> [f32; 3] {
    let (x, y, z) = quat.to_euler(EulerRot::XYZ);
    [x.to_degrees(), y.to_degrees(), z.to_degrees()]
}

pub(super) fn box_params(graph: &lobedo_core::Graph, node_id: NodeId) -> Option<BoxParams> {
    let node = graph.node(node_id)?;
    let (center, size) = if node.name == "Splat Heal" {
        (
            Vec3::from(node.params.get_vec3("heal_center", [0.0, 0.0, 0.0])),
            Vec3::from(node.params.get_vec3("heal_size", [1.0, 1.0, 1.0])),
        )
    } else {
        (
            Vec3::from(node.params.get_vec3("center", [0.0, 0.0, 0.0])),
            Vec3::from(node.params.get_vec3("size", [1.0, 1.0, 1.0])),
        )
    };
    Some(BoxParams {
        center,
        size: size.abs(),
    })
}

fn set_box_params(app: &mut LobedoApp, node_id: NodeId, center: Vec3, size: Vec3) {
    let mut size = size.abs();
    let is_splat_heal = app
        .project
        .graph
        .node(node_id)
        .is_some_and(|node| node.name == "Splat Heal");
    if is_splat_heal {
        let shape = app
            .project
            .graph
            .node(node_id)
            .map(|node| node.params.get_string("heal_shape", "all").to_lowercase())
            .unwrap_or_else(|| "all".to_string());
        if shape == "sphere" {
            size = Vec3::splat(size.max_element());
        }
        let _ = app.project.graph.set_param(
            node_id,
            "heal_center".to_string(),
            ParamValue::Vec3(center.to_array()),
        );
        let _ = app.project.graph.set_param(
            node_id,
            "heal_size".to_string(),
            ParamValue::Vec3(size.to_array()),
        );
    } else {
        let _ = app.project.graph.set_param(
            node_id,
            "center".to_string(),
            ParamValue::Vec3(center.to_array()),
        );
        let _ = app.project.graph.set_param(
            node_id,
            "size".to_string(),
            ParamValue::Vec3(size.to_array()),
        );
    }
    app.mark_eval_dirty();
}

pub(super) fn axis_dir(axis: GizmoAxis) -> Vec3 {
    match axis {
        GizmoAxis::X => Vec3::X,
        GizmoAxis::Y => Vec3::Y,
        GizmoAxis::Z => Vec3::Z,
    }
}

pub(super) fn axis_color(axis: GizmoAxis) -> Color32 {
    match axis {
        GizmoAxis::X => Color32::from_rgb(220, 80, 80),
        GizmoAxis::Y => Color32::from_rgb(80, 200, 120),
        GizmoAxis::Z => Color32::from_rgb(80, 120, 220),
    }
}

pub(super) fn gizmo_scale(view_proj: Mat4, rect: Rect, origin: Vec3, target_px: f32) -> f32 {
    let axes = [Vec3::X, Vec3::Y, Vec3::Z];
    for axis in axes {
        if let (Some(o), Some(a)) = (
            project_world_to_screen(view_proj, rect, origin),
            project_world_to_screen(view_proj, rect, origin + axis),
        ) {
            let len = (a - o).length();
            if len > 1.0 {
                return target_px / len;
            }
        }
    }
    1.0
}

pub(super) fn pick_gizmo_hit(
    origin: Vec3,
    view_proj: Mat4,
    rect: Rect,
    mouse: Pos2,
    allow_rotate: bool,
    basis: Mat3,
) -> Option<GizmoHit> {
    let origin_screen = project_world_to_screen(view_proj, rect, origin)?;
    let scale = gizmo_scale(view_proj, rect, origin, 90.0);
    let threshold = 10.0;

    if allow_rotate {
        let mut best = None;
        let mut best_dist = f32::INFINITY;
        for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
            let axis_world = basis * axis_dir(axis);
            let points = rotation_ring_points(view_proj, rect, origin, axis_world, scale);
            let dist = super::viewport_tools_math::distance_to_polyline(mouse, &points);
            if dist < best_dist {
                best_dist = dist;
                best = Some(GizmoHit::Ring(axis));
            }
        }
        if best_dist <= threshold {
            return best;
        }
    }

    let mut best = None;
    let mut best_dist = f32::INFINITY;
    for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
        let dir = basis * axis_dir(axis);
        let end_world = origin + dir * scale;
        let end_screen = project_world_to_screen(view_proj, rect, end_world)?;
        let dist = super::viewport_tools_math::distance_to_segment(mouse, origin_screen, end_screen);
        if dist < best_dist {
            best_dist = dist;
            best = Some(GizmoHit::Axis(axis));
        }
    }
    if best_dist <= threshold { best } else { None }
}

pub(super) fn apply_transform_drag(
    app: &mut LobedoApp,
    drag: TransformDrag,
    rect: Rect,
    pixels_per_point: f32,
    mouse: Pos2,
) {
    let view_proj = viewport_view_proj(app.camera_state(), rect, pixels_per_point);
    let origin_screen = if let Some(screen) = project_world_to_screen(view_proj, rect, drag.origin)
    {
        screen
    } else {
        return;
    };
    let axis = drag.axis;
    let axis_world = drag.axis_world.normalize_or_zero();
    let scale_world = gizmo_scale(view_proj, rect, drag.origin, 90.0);
    let axis_end_world = drag.origin + axis_world * scale_world;
    let axis_end_screen = match project_world_to_screen(view_proj, rect, axis_end_world) {
        Some(pos) => pos,
        None => return,
    };
    let axis_screen = axis_end_screen - origin_screen;
    let axis_screen_len = axis_screen.length();
    if axis_screen_len <= 1.0e-5 {
        return;
    }
    let axis_screen_dir = axis_screen / axis_screen_len;
    let delta_screen = mouse - drag.start_mouse;
    let delta_along = delta_screen.dot(axis_screen_dir);
    let world_per_pixel = scale_world / axis_screen_len;
    let delta_world = delta_along * world_per_pixel;

    match drag.mode {
        TransformMode::Translate => {
            let mut translate = Vec3::from(drag.start_translate);
            translate += axis_world * delta_world;
            let _ = app.project.graph.set_param(
                drag.node_id,
                "translate".to_string(),
                ParamValue::Vec3(translate.to_array()),
            );
            app.mark_eval_dirty();
        }
        TransformMode::Scale => {
            let mut scale = Vec3::from(drag.start_scale);
            let axis_idx = axis_index(axis);
            scale[axis_idx] = (scale[axis_idx] + delta_world).max(0.001);
            scale = Vec3::new(scale.x.max(0.001), scale.y.max(0.001), scale.z.max(0.001));
            let _ = app.project.graph.set_param(
                drag.node_id,
                "scale".to_string(),
                ParamValue::Vec3(scale.to_array()),
            );
            app.mark_eval_dirty();
        }
        TransformMode::Rotate => {
            let plane_normal = axis_world;
            let hit = raycast_plane(
                app.camera_state(),
                rect,
                pixels_per_point,
                mouse,
                drag.origin,
                plane_normal,
            );
            let Some(hit) = hit else {
                return;
            };
            let current_vec = (hit - drag.origin).normalize_or_zero();
            let Some(start_vec) = drag.start_vec else {
                return;
            };
            let cross = start_vec.cross(current_vec);
            let dot = start_vec.dot(current_vec).clamp(-1.0, 1.0);
            let angle = cross.dot(plane_normal).atan2(dot);
            let delta_quat = Quat::from_axis_angle(axis_world, angle);
            let new_quat = (delta_quat * drag.start_quat).normalize();
            let rotate = Vec3::from(quat_to_euler_deg(new_quat));
            let _ = app.project.graph.set_param(
                drag.node_id,
                "rotate_deg".to_string(),
                ParamValue::Vec3(rotate.to_array()),
            );
            app.mark_eval_dirty();
        }
    }
}

pub(super) fn apply_box_drag(
    app: &mut LobedoApp,
    drag: BoxDrag,
    rect: Rect,
    pixels_per_point: f32,
    mouse: Pos2,
) {
    match drag.handle {
        BoxHandle::Center => {
            let Some(start_hit) = drag.start_hit else {
                return;
            };
            let forward = super::viewport_tools_math::camera_forward(app.camera_state());
            let Some(hit) = raycast_plane(
                app.camera_state(),
                rect,
                pixels_per_point,
                mouse,
                drag.start_center,
                forward,
            ) else {
                return;
            };
            let delta = hit - start_hit;
            let size = drag.start_size.abs().max(Vec3::splat(0.001));
            let center = drag.start_center + delta;
            set_box_params(app, drag.node_id, center, size);
        }
        BoxHandle::Face { axis, sign } => {
            let view_proj = viewport_view_proj(app.camera_state(), rect, pixels_per_point);
            let Some(delta_world) = axis_drag_delta(
                view_proj,
                rect,
                drag.start_center,
                drag.start_mouse,
                mouse,
                axis,
            ) else {
                return;
            };
            let mut size = drag.start_size.abs();
            let axis_idx = axis_index(axis);
            let start_axis = size[axis_idx].max(0.001);
            let new_axis = (start_axis + sign * delta_world).max(0.001);
            let delta_used = (new_axis - start_axis) * sign;
            size[axis_idx] = new_axis;
            let center = drag.start_center + axis_dir(axis) * (delta_used * 0.5);
            set_box_params(app, drag.node_id, center, size);
        }
    }
}

pub(super) fn axis_drag_delta(
    view_proj: Mat4,
    rect: Rect,
    origin: Vec3,
    start_mouse: Pos2,
    mouse: Pos2,
    axis: GizmoAxis,
) -> Option<f32> {
    let origin_screen = project_world_to_screen(view_proj, rect, origin)?;
    let scale_world = gizmo_scale(view_proj, rect, origin, 90.0);
    let axis_world = axis_dir(axis);
    let axis_end_world = origin + axis_world * scale_world;
    let axis_end_screen = project_world_to_screen(view_proj, rect, axis_end_world)?;
    let axis_screen = axis_end_screen - origin_screen;
    let axis_screen_len = axis_screen.length();
    if axis_screen_len <= 1.0e-5 {
        return None;
    }
    let axis_screen_dir = axis_screen / axis_screen_len;
    let delta_screen = mouse - start_mouse;
    let delta_along = delta_screen.dot(axis_screen_dir);
    let world_per_pixel = scale_world / axis_screen_len;
    Some(delta_along * world_per_pixel)
}

pub(super) fn axis_index(axis: GizmoAxis) -> usize {
    match axis {
        GizmoAxis::X => 0,
        GizmoAxis::Y => 1,
        GizmoAxis::Z => 2,
    }
}

pub(super) fn draw_transform_gizmo(
    app: &LobedoApp,
    ui: &egui::Ui,
    rect: Rect,
    node_id: NodeId,
    mode: TransformMode,
) {
    let origin = transform_origin(&app.project.graph, node_id);
    let Some(origin) = origin else {
        return;
    };
    let params = transform_params(&app.project.graph, node_id);
    let basis = transform_basis(params.rotate);
    let view_proj = viewport_view_proj(app.camera_state(), rect, ui.ctx().pixels_per_point());
    let origin_screen = match project_world_to_screen(view_proj, rect, origin) {
        Some(pos) => pos,
        None => return,
    };
    let scale = gizmo_scale(view_proj, rect, origin, 90.0);
    let painter = ui.painter();
    for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
        let dir = basis * axis_dir(axis);
        let end_world = origin + dir * scale;
        let end_screen = match project_world_to_screen(view_proj, rect, end_world) {
            Some(pos) => pos,
            None => continue,
        };
        let color = axis_color(axis);
        let stroke = Stroke::new(3.0, color);
        painter.line_segment([origin_screen, end_screen], stroke);
        painter.circle_filled(end_screen, 5.0, color);
    }

    if mode != TransformMode::Scale {
        for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
            let axis_world = basis * axis_dir(axis);
            draw_rotation_ring(painter, view_proj, rect, origin, axis_world, scale, axis);
        }
    }
}

pub(super) fn draw_box_handles(app: &LobedoApp, ui: &egui::Ui, rect: Rect, node_id: NodeId) {
    let Some(params) = box_params(&app.project.graph, node_id) else {
        return;
    };
    let view_proj = viewport_view_proj(app.camera_state(), rect, ui.ctx().pixels_per_point());
    let painter = ui.painter();
    let fill = Color32::from_rgb(240, 200, 90);
    let stroke = Stroke::new(1.0, Color32::from_rgb(255, 235, 170));
    for (handle, world) in box_handle_positions(params.center, params.size) {
        if let Some(screen) = project_world_to_screen(view_proj, rect, world) {
            let radius = match handle {
                BoxHandle::Center => 5.5,
                BoxHandle::Face { .. } => 4.5,
            };
            painter.circle_filled(screen, radius, fill);
            painter.circle_stroke(screen, radius + 2.0, stroke);
        }
    }
}

fn draw_rotation_ring(
    painter: &egui::Painter,
    view_proj: Mat4,
    rect: Rect,
    origin: Vec3,
    axis_world: Vec3,
    radius: f32,
    axis_color_key: GizmoAxis,
) {
    let axis_dir = axis_world.normalize_or_zero();
    let helper = if axis_dir.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    let u = axis_dir.cross(helper).normalize_or_zero();
    let v = axis_dir.cross(u).normalize_or_zero();
    let steps = 32;
    let mut points = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let angle = t * std::f32::consts::TAU;
        let world = origin + (u * angle.cos() + v * angle.sin()) * radius;
        if let Some(screen) = project_world_to_screen(view_proj, rect, world) {
            points.push(screen);
        }
    }
    if points.len() >= 2 {
        let color = axis_color(axis_color_key);
        painter.add(egui::Shape::line(points, Stroke::new(1.0, color)));
    }
}

fn rotation_ring_points(
    view_proj: Mat4,
    rect: Rect,
    origin: Vec3,
    axis_world: Vec3,
    radius: f32,
) -> Vec<Pos2> {
    let axis_dir = axis_world.normalize_or_zero();
    let helper = if axis_dir.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    let u = axis_dir.cross(helper).normalize_or_zero();
    let v = axis_dir.cross(u).normalize_or_zero();
    let steps = 32;
    let mut points = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let angle = t * std::f32::consts::TAU;
        let world = origin + (u * angle.cos() + v * angle.sin()) * radius;
        if let Some(screen) = project_world_to_screen(view_proj, rect, world) {
            points.push(screen);
        }
    }
    points
}

fn box_handle_positions(center: Vec3, size: Vec3) -> Vec<(BoxHandle, Vec3)> {
    let mut handles = Vec::with_capacity(7);
    let mut half = size.abs() * 0.5;
    half = Vec3::new(half.x.max(0.001), half.y.max(0.001), half.z.max(0.001));
    handles.push((BoxHandle::Center, center));
    handles.push((BoxHandle::Face { axis: GizmoAxis::X, sign: 1.0 }, center + Vec3::X * half.x));
    handles.push((BoxHandle::Face { axis: GizmoAxis::X, sign: -1.0 }, center - Vec3::X * half.x));
    handles.push((BoxHandle::Face { axis: GizmoAxis::Y, sign: 1.0 }, center + Vec3::Y * half.y));
    handles.push((BoxHandle::Face { axis: GizmoAxis::Y, sign: -1.0 }, center - Vec3::Y * half.y));
    handles.push((BoxHandle::Face { axis: GizmoAxis::Z, sign: 1.0 }, center + Vec3::Z * half.z));
    handles.push((BoxHandle::Face { axis: GizmoAxis::Z, sign: -1.0 }, center - Vec3::Z * half.z));
    handles
}

pub(super) fn pick_box_handle(
    view_proj: Mat4,
    rect: Rect,
    mouse: Pos2,
    center: Vec3,
    size: Vec3,
) -> Option<BoxHandle> {
    let mut best = None;
    let mut best_dist = f32::INFINITY;
    for (handle, world) in box_handle_positions(center, size) {
        let Some(screen) = project_world_to_screen(view_proj, rect, world) else {
            continue;
        };
        let dist = (screen - mouse).length();
        let threshold = match handle {
            BoxHandle::Center => 14.0,
            BoxHandle::Face { .. } => 12.0,
        };
        if dist < threshold && dist < best_dist {
            best_dist = dist;
            best = Some(handle);
        }
    }
    best
}
