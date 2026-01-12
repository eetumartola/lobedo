use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use std::collections::BTreeSet;
use glam::{EulerRot, Mat3, Mat4, Quat, Vec3};

use lobedo_core::{encode_curve_points, parse_curve_points, NodeId, ParamValue};
use lobedo_core::AttributeDomain;

use super::LobedoApp;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum TransformMode {
    #[default]
    Translate,
    Rotate,
    Scale,
}

#[derive(Clone, Copy)]
enum GizmoAxis {
    X,
    Y,
    Z,
}

enum GizmoHit {
    Axis(GizmoAxis),
    Ring(GizmoAxis),
}

#[derive(Clone, Copy)]
enum BoxHandle {
    Center,
    Face { axis: GizmoAxis, sign: f32 },
}

#[derive(Clone, Copy)]
enum SelectionAction {
    Replace,
    Add,
    Remove,
}

#[derive(Clone, Copy)]
struct TransformDrag {
    node_id: NodeId,
    axis: GizmoAxis,
    axis_world: Vec3,
    mode: TransformMode,
    start_mouse: Pos2,
    start_translate: [f32; 3],
    start_scale: [f32; 3],
    origin: Vec3,
    start_vec: Option<Vec3>,
    start_quat: Quat,
}

#[derive(Clone, Copy)]
struct BoxDrag {
    node_id: NodeId,
    handle: BoxHandle,
    start_mouse: Pos2,
    start_center: Vec3,
    start_size: Vec3,
    start_hit: Option<Vec3>,
}

#[derive(Clone, Copy)]
struct CurveDrawState {
    node_id: NodeId,
}

#[derive(Clone, Copy)]
struct CurvePointDrag {
    index: usize,
    axis: Option<GizmoAxis>,
    start_mouse: Pos2,
    start_point: Vec3,
    start_hit: Option<Vec3>,
}

#[derive(Clone, Copy)]
struct CurveEditState {
    node_id: NodeId,
    drag: Option<CurvePointDrag>,
}

#[derive(Clone, Copy)]
struct FfdPointDrag {
    index: usize,
    axis: Option<GizmoAxis>,
    start_mouse: Pos2,
    start_point: Vec3,
    start_hit: Option<Vec3>,
}

#[derive(Clone, Copy)]
struct FfdEditState {
    node_id: NodeId,
    drag: Option<FfdPointDrag>,
}

#[derive(Clone, Copy)]
struct GroupSelectState {
    node_id: NodeId,
    drag_start: Option<Pos2>,
    drag_rect: Option<Rect>,
}

#[derive(Default)]
pub struct ViewportToolState {
    pub transform_mode: TransformMode,
    transform_drag: Option<TransformDrag>,
    box_drag: Option<BoxDrag>,
    curve_draw: Option<CurveDrawState>,
    curve_edit: Option<CurveEditState>,
    ffd_edit: Option<FfdEditState>,
    group_select: Option<GroupSelectState>,
}

impl ViewportToolState {
    pub(super) fn is_dragging(&self) -> bool {
        self.transform_drag.is_some()
            || self.box_drag.is_some()
            || self.curve_edit.and_then(|edit| edit.drag).is_some()
            || self.ffd_edit.and_then(|edit| edit.drag).is_some()
            || self
                .group_select
                .and_then(|select| select.drag_start)
                .is_some()
    }
}

impl LobedoApp {
    pub(super) fn activate_curve_draw(&mut self, node_id: NodeId) {
        self.viewport_tools.curve_draw = Some(CurveDrawState { node_id });
        self.viewport_tools.curve_edit = None;
    }

    pub(super) fn activate_curve_edit(&mut self, node_id: NodeId) {
        self.viewport_tools.curve_edit = Some(CurveEditState {
            node_id,
            drag: None,
        });
        self.viewport_tools.curve_draw = None;
    }

    pub(super) fn deactivate_curve_draw(&mut self) {
        self.viewport_tools.curve_draw = None;
    }

    pub(super) fn deactivate_curve_edit(&mut self) {
        self.viewport_tools.curve_edit = None;
    }

    pub(super) fn curve_draw_active(&self, node_id: NodeId) -> bool {
        self.viewport_tools
            .curve_draw
            .is_some_and(|state| state.node_id == node_id)
    }

    pub(super) fn curve_edit_active(&self, node_id: NodeId) -> bool {
        self.viewport_tools
            .curve_edit
            .is_some_and(|state| state.node_id == node_id)
    }

    pub(super) fn activate_ffd_edit(&mut self, node_id: NodeId) {
        self.ensure_ffd_lattice_points(node_id);
        self.viewport_tools.ffd_edit = Some(FfdEditState {
            node_id,
            drag: None,
        });
        self.viewport_tools.curve_draw = None;
        self.viewport_tools.curve_edit = None;
        self.viewport_tools.group_select = None;
    }

    pub(super) fn deactivate_ffd_edit(&mut self) {
        self.viewport_tools.ffd_edit = None;
    }

    pub(super) fn ffd_edit_active(&self, node_id: NodeId) -> bool {
        self.viewport_tools
            .ffd_edit
            .is_some_and(|state| state.node_id == node_id)
    }

    pub(super) fn activate_group_select(&mut self, node_id: NodeId) {
        self.viewport_tools.group_select = Some(GroupSelectState {
            node_id,
            drag_start: None,
            drag_rect: None,
        });
        self.viewport_tools.curve_draw = None;
        self.viewport_tools.curve_edit = None;
    }

    pub(super) fn deactivate_group_select(&mut self) {
        self.viewport_tools.group_select = None;
    }

    pub(super) fn group_select_active(&self, node_id: NodeId) -> bool {
        self.viewport_tools
            .group_select
            .is_some_and(|state| state.node_id == node_id)
    }

    pub(super) fn group_select_node_id(&self) -> Option<NodeId> {
        self.viewport_tools.group_select.map(|state| state.node_id)
    }

    pub(super) fn selected_group_select_node(&self) -> Option<NodeId> {
        let node_id = self.node_graph.selected_node_id()?;
        let node = self.project.graph.node(node_id)?;
        if node.name != "Group" {
            return None;
        }
        let shape = node.params.get_string("shape", "box").to_lowercase();
        if shape == "selection" {
            Some(node_id)
        } else {
            None
        }
    }

    pub(super) fn handle_viewport_tools_input(
        &mut self,
        response: &egui::Response,
        rect: Rect,
    ) -> bool {
        if !response.hovered() {
            return false;
        }

        let ctx = response.ctx.clone();
        let pointer_pos = ctx.input(|i| i.pointer.interact_pos());
        let primary_down = ctx.input(|i| i.pointer.primary_down());
        let primary_pressed = ctx.input(|i| i.pointer.primary_pressed());
        let primary_released = ctx.input(|i| i.pointer.primary_released());
        let modifiers = ctx.input(|i| i.modifiers);
        let secondary_clicked = response.clicked_by(egui::PointerButton::Secondary);
        let primary_clicked = response.clicked_by(egui::PointerButton::Primary);

        if let Some(mut select) = self.viewport_tools.group_select {
            if self.selected_group_select_node() != Some(select.node_id) {
                self.viewport_tools.group_select = None;
            } else {
                if modifiers.alt {
                    select.drag_start = None;
                    select.drag_rect = None;
                    self.viewport_tools.group_select = Some(select);
                    return false;
                }
                let mut capture = false;
                if primary_pressed {
                    select.drag_start = pointer_pos;
                    select.drag_rect = None;
                    let snapshot = self.snapshot_undo();
                    self.queue_undo_snapshot(snapshot, true);
                    capture = true;
                }
                if primary_down {
                    if let (Some(start), Some(pos)) = (select.drag_start, pointer_pos) {
                        if (pos - start).length() > 4.0 {
                            select.drag_rect = Some(Rect::from_two_pos(start, pos));
                        }
                    }
                    capture = true;
                }
                if primary_released {
                    if let Some((domain, allow_backface)) =
                        group_selection_settings(&self.project.graph, select.node_id)
                    {
                        let action = selection_action(modifiers);
                        let indices = if let Some(scene) = self.last_scene.as_ref() {
                            let view_proj = viewport_view_proj(
                                self.camera_state(),
                                rect,
                                ctx.pixels_per_point(),
                            );
                            if let Some(rect_sel) = select.drag_rect {
                                selection_indices_in_rect(
                                    scene,
                                    domain,
                                    view_proj,
                                    rect,
                                    rect_sel,
                                    self.camera_state(),
                                    allow_backface,
                                )
                            } else if let Some(pos) = pointer_pos.or(select.drag_start) {
                                let mut picked = BTreeSet::new();
                                if let Some(index) =
                                    pick_selection_index(
                                        scene,
                                        domain,
                                        view_proj,
                                        rect,
                                        pos,
                                        self.camera_state(),
                                        allow_backface,
                                    )
                                {
                                    picked.insert(index);
                                }
                                picked
                            } else {
                                BTreeSet::new()
                            }
                        } else {
                            BTreeSet::new()
                        };
                        self.apply_group_selection(select.node_id, action, indices);
                    }
                    select.drag_start = None;
                    select.drag_rect = None;
                    capture = true;
                }
                self.viewport_tools.group_select = Some(select);
                if capture {
                    return true;
                }
            }
        }

        if let Some(curve) = self.viewport_tools.curve_draw {
            if secondary_clicked {
                self.viewport_tools.curve_draw = None;
                return true;
            }
            if primary_clicked {
                if let Some(pos) = pointer_pos {
                    if let Some(hit) = raycast_plane_y(
                        self.camera_state(),
                        rect,
                        ctx.pixels_per_point(),
                        pos,
                        0.0,
                    ) {
                        let snapshot = self.snapshot_undo();
                        if self.append_curve_point(curve.node_id, hit) {
                            self.queue_undo_snapshot(snapshot, false);
                        }
                    }
                }
            }
            return true;
        }

        if let Some(mut edit) = self.viewport_tools.curve_edit {
            if secondary_clicked {
                self.viewport_tools.curve_edit = None;
                return true;
            }
            if !primary_down {
                if edit.drag.is_some() {
                    edit.drag = None;
                    self.viewport_tools.curve_edit = Some(edit);
                    return true;
                }
                return false;
            }
            if let Some(pos) = pointer_pos {
                if let Some(drag) = edit.drag {
                    match drag.axis {
                        Some(axis) => {
                            let view_proj = viewport_view_proj(
                                self.camera_state(),
                                rect,
                                ctx.pixels_per_point(),
                            );
                            if let Some(delta_world) = axis_drag_delta(
                                view_proj,
                                rect,
                                drag.start_point,
                                drag.start_mouse,
                                pos,
                                axis,
                            ) {
                                let new_point =
                                    drag.start_point + axis_dir(axis) * delta_world;
                                self.update_curve_point(edit.node_id, drag.index, new_point);
                            }
                        }
                        None => {
                            let Some(start_hit) = drag.start_hit else {
                                return true;
                            };
                            if let Some(hit) = raycast_plane_y(
                                self.camera_state(),
                                rect,
                                ctx.pixels_per_point(),
                                pos,
                                0.0,
                            ) {
                                let delta = hit - start_hit;
                                self.update_curve_point(
                                    edit.node_id,
                                    drag.index,
                                    drag.start_point + delta,
                                );
                            }
                        }
                    }
                    self.viewport_tools.curve_edit = Some(edit);
                    return true;
                } else if primary_pressed {
                    if let Some(pick) = pick_curve_handle(
                        self.camera_state(),
                        rect,
                        ctx.pixels_per_point(),
                        pos,
                        edit.node_id,
                        &self.project.graph,
                    ) {
                        let snapshot = self.snapshot_undo();
                        self.queue_undo_snapshot(snapshot, true);
                        let start_hit = if pick.axis.is_none() {
                            raycast_plane_y(
                                self.camera_state(),
                                rect,
                                ctx.pixels_per_point(),
                                pos,
                                0.0,
                            )
                        } else {
                            None
                        };
                        if pick.axis.is_some() || start_hit.is_some() {
                            edit.drag = Some(CurvePointDrag {
                                index: pick.index,
                                axis: pick.axis,
                                start_mouse: pos,
                                start_point: pick.point,
                                start_hit,
                            });
                            self.viewport_tools.curve_edit = Some(edit);
                            return true;
                        }
                    }
                }
            }
            self.viewport_tools.curve_edit = Some(edit);
            return false;
        }

        if let Some(mut edit) = self.viewport_tools.ffd_edit {
            self.ensure_ffd_lattice_points(edit.node_id);
            if secondary_clicked {
                self.viewport_tools.ffd_edit = None;
                return true;
            }
            if modifiers.alt {
                if edit.drag.is_some() {
                    edit.drag = None;
                    self.viewport_tools.ffd_edit = Some(edit);
                    return true;
                }
                return false;
            }
            if !primary_down {
                if edit.drag.is_some() {
                    edit.drag = None;
                    self.viewport_tools.ffd_edit = Some(edit);
                    return true;
                }
                return false;
            }
            if let Some(pos) = pointer_pos {
                if let Some(drag) = edit.drag {
                    match drag.axis {
                        Some(axis) => {
                            let view_proj = viewport_view_proj(
                                self.camera_state(),
                                rect,
                                ctx.pixels_per_point(),
                            );
                            if let Some(delta_world) = axis_drag_delta(
                                view_proj,
                                rect,
                                drag.start_point,
                                drag.start_mouse,
                                pos,
                                axis,
                            ) {
                                let new_point =
                                    drag.start_point + axis_dir(axis) * delta_world;
                                self.update_ffd_point(edit.node_id, drag.index, new_point);
                            }
                        }
                        None => {
                            let Some(start_hit) = drag.start_hit else {
                                return true;
                            };
                            let forward = camera_forward(self.camera_state());
                            if let Some(hit) = raycast_plane(
                                self.camera_state(),
                                rect,
                                ctx.pixels_per_point(),
                                pos,
                                drag.start_point,
                                forward,
                            ) {
                                let delta = hit - start_hit;
                                self.update_ffd_point(
                                    edit.node_id,
                                    drag.index,
                                    drag.start_point + delta,
                                );
                            }
                        }
                    }
                    self.viewport_tools.ffd_edit = Some(edit);
                    return true;
                } else if primary_pressed {
                    if let Some(pick) = pick_ffd_handle(
                        self.camera_state(),
                        rect,
                        ctx.pixels_per_point(),
                        pos,
                        edit.node_id,
                        &self.project.graph,
                    ) {
                        let snapshot = self.snapshot_undo();
                        self.queue_undo_snapshot(snapshot, true);
                        let start_hit = if pick.axis.is_none() {
                            let forward = camera_forward(self.camera_state());
                            raycast_plane(
                                self.camera_state(),
                                rect,
                                ctx.pixels_per_point(),
                                pos,
                                pick.point,
                                forward,
                            )
                        } else {
                            None
                        };
                        if pick.axis.is_some() || start_hit.is_some() {
                            edit.drag = Some(FfdPointDrag {
                                index: pick.index,
                                axis: pick.axis,
                                start_mouse: pos,
                                start_point: pick.point,
                                start_hit,
                            });
                            self.viewport_tools.ffd_edit = Some(edit);
                            return true;
                        }
                    }
                }
            }
            self.viewport_tools.ffd_edit = Some(edit);
            return false;
        }

        if let Some(node_id) = self.selected_box_node() {
            if let Some(pos) = pointer_pos {
                if primary_down && self.viewport_tools.box_drag.is_none() {
                    if let Some(params) = box_params(&self.project.graph, node_id) {
                        let view_proj = viewport_view_proj(
                            self.camera_state(),
                            rect,
                            ctx.pixels_per_point(),
                        );
                        if let Some(handle) = pick_box_handle(
                            view_proj,
                            rect,
                            pos,
                            params.center,
                            params.size,
                        ) {
                            let snapshot = self.snapshot_undo();
                            self.queue_undo_snapshot(snapshot, true);
                            let start_hit = match handle {
                                BoxHandle::Center => {
                                    let forward = camera_forward(self.camera_state());
                                    raycast_plane(
                                        self.camera_state(),
                                        rect,
                                        ctx.pixels_per_point(),
                                        pos,
                                        params.center,
                                        forward,
                                    )
                                }
                                _ => None,
                            };
                            self.viewport_tools.box_drag = Some(BoxDrag {
                                node_id,
                                handle,
                                start_mouse: pos,
                                start_center: params.center,
                                start_size: params.size,
                                start_hit,
                            });
                            return true;
                        }
                    }
                }
            }

            if let Some(drag) = self.viewport_tools.box_drag {
                if !primary_down {
                    self.viewport_tools.box_drag = None;
                    return true;
                }
                if let Some(pos) = pointer_pos {
                    apply_box_drag(self, drag, rect, ctx.pixels_per_point(), pos);
                }
                return true;
            }
        }
        if self.viewport_tools.box_drag.is_some() {
            self.viewport_tools.box_drag = None;
        }

        if let Some(node_id) = self.selected_transform_node() {
            if let Some(pos) = pointer_pos {
                let view_proj = viewport_view_proj(
                    self.camera_state(),
                    rect,
                    ctx.pixels_per_point(),
                );
                let origin = transform_origin(&self.project.graph, node_id);
                if let Some(origin) = origin {
                    if primary_down && self.viewport_tools.transform_drag.is_none() {
                        let params = transform_params(&self.project.graph, node_id);
                        let basis = transform_basis(params.rotate);
                        let allow_rotate =
                            self.viewport_tools.transform_mode != TransformMode::Scale;
                        if let Some(hit) =
                            pick_gizmo_hit(origin, view_proj, rect, pos, allow_rotate, basis)
                        {
                            let (axis, mode) = match hit {
                                GizmoHit::Axis(axis) => {
                                    let mode = if self.viewport_tools.transform_mode
                                        == TransformMode::Scale
                                    {
                                        TransformMode::Scale
                                    } else {
                                        TransformMode::Translate
                                    };
                                    (axis, mode)
                                }
                                GizmoHit::Ring(axis) => (axis, TransformMode::Rotate),
                            };
                            let axis_world = basis * axis_dir(axis);
                            let snapshot = self.snapshot_undo();
                            self.queue_undo_snapshot(snapshot, true);
                            let start_vec = if mode == TransformMode::Rotate {
                                let plane_normal = axis_world.normalize_or_zero();
                                let hit = raycast_plane(
                                    self.camera_state(),
                                    rect,
                                    ctx.pixels_per_point(),
                                    pos,
                                    origin,
                                    plane_normal,
                                );
                                hit.map(|p| (p - origin).normalize_or_zero())
                            } else {
                                None
                            };
                            if mode == TransformMode::Rotate && start_vec.is_none() {
                                return false;
                            }
                            let start_quat = transform_quat(params.rotate);
                            self.viewport_tools.transform_drag = Some(TransformDrag {
                                node_id,
                                axis,
                                axis_world,
                                mode,
                                start_mouse: pos,
                                start_translate: params.translate,
                                start_scale: params.scale,
                                origin,
                                start_vec,
                                start_quat,
                            });
                        }
                    }
                }
            }

            if let Some(drag) = self.viewport_tools.transform_drag {
                if !primary_down {
                    self.viewport_tools.transform_drag = None;
                    return false;
                }
                if let Some(pos) = pointer_pos {
                    apply_transform_drag(self, drag, rect, ctx.pixels_per_point(), pos);
                }
                return true;
            }
        }

        false
    }

    pub(super) fn draw_viewport_tools(&self, ui: &egui::Ui, rect: Rect) {
        if let Some(curve) = self.viewport_tools.curve_draw {
            draw_curve_overlay(self, ui, rect, curve.node_id, true);
        }
        if let Some(curve) = self.viewport_tools.curve_edit {
            draw_curve_overlay(self, ui, rect, curve.node_id, true);
            draw_curve_handles(self, ui, rect, curve.node_id);
        }
        if let Some(ffd) = self.viewport_tools.ffd_edit {
            draw_ffd_lattice_overlay(self, ui, rect, ffd.node_id);
            draw_ffd_lattice_handles(self, ui, rect, ffd.node_id);
        }
        if let Some(node_id) = self.selected_group_select_node() {
            draw_group_selection_overlay(self, ui, rect, node_id);
        }
        if let Some(select) = self.viewport_tools.group_select {
            if let Some(selection_rect) = select.drag_rect {
                let painter = ui.painter();
                let fill = Color32::from_rgba_unmultiplied(255, 235, 170, 40);
                let stroke = Stroke::new(1.0, Color32::from_rgb(255, 235, 170));
                painter.rect(selection_rect, 0.0, fill, stroke, egui::StrokeKind::Inside);
            }
        }
        if let Some(node_id) = self.selected_box_node() {
            draw_box_handles(self, ui, rect, node_id);
        }
        if let Some(node_id) = self.selected_transform_node() {
            draw_transform_gizmo(self, ui, rect, node_id, self.viewport_tools.transform_mode);
        }
    }

    fn selected_transform_node(&self) -> Option<NodeId> {
        let node_id = self.node_graph.selected_node_id()?;
        let node = self.project.graph.node(node_id)?;
        if matches!(node.name.as_str(), "Transform" | "Copy/Transform") {
            Some(node_id)
        } else {
            None
        }
    }

    fn selected_box_node(&self) -> Option<NodeId> {
        let node_id = self.node_graph.selected_node_id()?;
        let node = self.project.graph.node(node_id)?;
        match node.name.as_str() {
            "Box" => Some(node_id),
            "Group" | "Delete" => {
                let shape = node.params.get_string("shape", "box").to_lowercase();
                if shape == "box" {
                    Some(node_id)
                } else {
                    None
                }
            }
            "Splat Heal" => {
                let shape = node.params.get_string("heal_shape", "all").to_lowercase();
                if matches!(shape.as_str(), "box" | "sphere") {
                    Some(node_id)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn append_curve_point(&mut self, node_id: NodeId, point: Vec3) -> bool {
        let Some(node) = self.project.graph.node(node_id) else {
            return false;
        };
        let mut points = parse_curve_points(node.params.get_string("points", ""));
        points.push(point.to_array());
        self.set_curve_points(node_id, &points)
    }

    fn update_curve_point(&mut self, node_id: NodeId, index: usize, point: Vec3) -> bool {
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

    fn set_curve_points(&mut self, node_id: NodeId, points: &[[f32; 3]]) -> bool {
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

    pub(super) fn ensure_ffd_lattice_points(&mut self, node_id: NodeId) {
        let params = {
            let Some(node) = self.project.graph.node(node_id) else {
                return;
            };
            if node.name != "FFD" {
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

    fn update_ffd_point(&mut self, node_id: NodeId, index: usize, point: Vec3) -> bool {
        let raw = {
            let Some(node) = self.project.graph.node(node_id) else {
                return false;
            };
            if node.name != "FFD" {
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

    fn set_ffd_points(&mut self, node_id: NodeId, points: &[[f32; 3]]) -> bool {
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

    fn ffd_input_bounds(&self, node_id: NodeId) -> Option<(Vec3, Vec3)> {
        if let Some(input_id) = input_node_for(&self.project.graph, node_id, 0) {
            if let Some(geometry) = self.eval_state.geometry_for_node(input_id) {
                if let Some(bounds) = geometry_bounds(geometry) {
                    return Some(bounds);
                }
            }
        }
        let geometry = self.eval_state.geometry_for_node(node_id)?;
        geometry_bounds(geometry)
    }

    fn apply_group_selection(
        &mut self,
        node_id: NodeId,
        action: SelectionAction,
        indices: BTreeSet<usize>,
    ) {
        let Some(node) = self.project.graph.node(node_id) else {
            return;
        };
        let current = parse_selection_indices(node.params.get_string("selection", ""));
        let mut next = current.clone();
        match action {
            SelectionAction::Replace => {
                next = indices;
            }
            SelectionAction::Add => {
                next.extend(indices);
            }
            SelectionAction::Remove => {
                for idx in indices {
                    next.remove(&idx);
                }
            }
        }
        if next == current {
            return;
        }
        let encoded = encode_selection_indices(&next);
        if self
            .project
            .graph
            .set_param(node_id, "selection".to_string(), ParamValue::String(encoded))
            .is_ok()
        {
            self.mark_eval_dirty();
        }
    }
}

struct TransformParams {
    translate: [f32; 3],
    rotate: [f32; 3],
    scale: [f32; 3],
    pivot: [f32; 3],
}

struct BoxParams {
    center: Vec3,
    size: Vec3,
}

fn transform_params(graph: &lobedo_core::Graph, node_id: NodeId) -> TransformParams {
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

fn transform_origin(graph: &lobedo_core::Graph, node_id: NodeId) -> Option<Vec3> {
    let params = transform_params(graph, node_id);
    let translate = Vec3::from(params.translate);
    let pivot = Vec3::from(params.pivot);
    Some(translate + pivot)
}

fn transform_quat(rotate_deg: [f32; 3]) -> Quat {
    let rot = Vec3::from(rotate_deg) * std::f32::consts::PI / 180.0;
    Quat::from_euler(EulerRot::XYZ, rot.x, rot.y, rot.z)
}

fn transform_basis(rotate_deg: [f32; 3]) -> Mat3 {
    Mat3::from_quat(transform_quat(rotate_deg))
}

fn quat_to_euler_deg(quat: Quat) -> [f32; 3] {
    let (x, y, z) = quat.to_euler(EulerRot::XYZ);
    [x.to_degrees(), y.to_degrees(), z.to_degrees()]
}

fn box_params(graph: &lobedo_core::Graph, node_id: NodeId) -> Option<BoxParams> {
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

fn axis_dir(axis: GizmoAxis) -> Vec3 {
    match axis {
        GizmoAxis::X => Vec3::X,
        GizmoAxis::Y => Vec3::Y,
        GizmoAxis::Z => Vec3::Z,
    }
}

fn axis_color(axis: GizmoAxis) -> Color32 {
    match axis {
        GizmoAxis::X => Color32::from_rgb(220, 80, 80),
        GizmoAxis::Y => Color32::from_rgb(80, 200, 120),
        GizmoAxis::Z => Color32::from_rgb(80, 120, 220),
    }
}

fn viewport_view_proj(camera: render::CameraState, rect: Rect, pixels_per_point: f32) -> Mat4 {
    let viewport_width = (rect.width() * pixels_per_point).max(1.0);
    let viewport_height = (rect.height() * pixels_per_point).max(1.0);
    let aspect = viewport_width / viewport_height;

    let target = Vec3::from(camera.target);
    let position = camera_position(camera);

    let view = Mat4::look_at_rh(position, target, Vec3::Y);
    let projection = Mat4::perspective_rh(45_f32.to_radians(), aspect, 0.01, 1000.0);
    projection * view
}

fn camera_position(camera: render::CameraState) -> Vec3 {
    let pitch = camera.pitch.clamp(-1.54, 1.54);
    let yaw = camera.yaw;
    let cos_pitch = pitch.cos();
    let sin_pitch = pitch.sin();
    let cos_yaw = yaw.cos();
    let sin_yaw = yaw.sin();
    let direction = Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw);
    let target = Vec3::from(camera.target);
    target + direction * camera.distance.max(0.1)
}

fn camera_forward(camera: render::CameraState) -> Vec3 {
    let position = camera_position(camera);
    let target = Vec3::from(camera.target);
    (target - position).normalize_or_zero()
}

fn project_world_to_screen(
    view_proj: Mat4,
    rect: Rect,
    world: Vec3,
) -> Option<Pos2> {
    let clip = view_proj * world.extend(1.0);
    if clip.w.abs() <= 1.0e-6 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    if !ndc.x.is_finite() || !ndc.y.is_finite() || !ndc.z.is_finite() {
        return None;
    }
    let x = rect.min.x + (ndc.x * 0.5 + 0.5) * rect.width();
    let y = rect.min.y + (0.5 - ndc.y * 0.5) * rect.height();
    Some(Pos2::new(x, y))
}

fn screen_ray(
    camera: render::CameraState,
    rect: Rect,
    pixels_per_point: f32,
    pos: Pos2,
) -> Option<(Vec3, Vec3)> {
    let view_proj = viewport_view_proj(camera, rect, pixels_per_point);
    let inv = view_proj.inverse();
    let ndc_x = ((pos.x - rect.min.x) / rect.width()) * 2.0 - 1.0;
    let ndc_y = 1.0 - ((pos.y - rect.min.y) / rect.height()) * 2.0;
    let near = inv.project_point3(Vec3::new(ndc_x, ndc_y, 0.0));
    let far = inv.project_point3(Vec3::new(ndc_x, ndc_y, 1.0));
    let dir = (far - near).normalize_or_zero();
    Some((near, dir))
}

fn raycast_plane_y(
    camera: render::CameraState,
    rect: Rect,
    pixels_per_point: f32,
    pos: Pos2,
    y: f32,
) -> Option<Vec3> {
    let (origin, dir) = screen_ray(camera, rect, pixels_per_point, pos)?;
    if dir.y.abs() <= 1.0e-6 {
        return None;
    }
    let t = (y - origin.y) / dir.y;
    if t < 0.0 {
        return None;
    }
    Some(origin + dir * t)
}

fn raycast_plane(
    camera: render::CameraState,
    rect: Rect,
    pixels_per_point: f32,
    pos: Pos2,
    plane_origin: Vec3,
    plane_normal: Vec3,
) -> Option<Vec3> {
    let (origin, dir) = screen_ray(camera, rect, pixels_per_point, pos)?;
    let denom = plane_normal.dot(dir);
    if denom.abs() <= 1.0e-6 {
        return None;
    }
    let t = (plane_origin - origin).dot(plane_normal) / denom;
    if t < 0.0 {
        return None;
    }
    Some(origin + dir * t)
}

fn gizmo_scale(
    view_proj: Mat4,
    rect: Rect,
    origin: Vec3,
    target_px: f32,
) -> f32 {
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

fn pick_gizmo_hit(
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
            let dist = distance_to_polyline(mouse, &points);
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
        let dist = distance_to_segment(mouse, origin_screen, end_screen);
        if dist < best_dist {
            best_dist = dist;
            best = Some(GizmoHit::Axis(axis));
        }
    }

    if best_dist <= threshold { best } else { None }
}

fn apply_transform_drag(
    app: &mut LobedoApp,
    drag: TransformDrag,
    rect: Rect,
    pixels_per_point: f32,
    mouse: Pos2,
) {
    let view_proj = viewport_view_proj(app.camera_state(), rect, pixels_per_point);
    let origin_screen =
        if let Some(screen) = project_world_to_screen(view_proj, rect, drag.origin) {
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
            scale = Vec3::new(
                scale.x.max(0.001),
                scale.y.max(0.001),
                scale.z.max(0.001),
            );
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

fn apply_box_drag(
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
            let forward = camera_forward(app.camera_state());
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

fn axis_drag_delta(
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

fn axis_index(axis: GizmoAxis) -> usize {
    match axis {
        GizmoAxis::X => 0,
        GizmoAxis::Y => 1,
        GizmoAxis::Z => 2,
    }
}

fn draw_transform_gizmo(
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

fn draw_box_handles(app: &LobedoApp, ui: &egui::Ui, rect: Rect, node_id: NodeId) {
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
    let helper = if axis_dir.x.abs() < 0.9 {
        Vec3::X
    } else {
        Vec3::Y
    };
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
    let helper = if axis_dir.x.abs() < 0.9 {
        Vec3::X
    } else {
        Vec3::Y
    };
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

fn draw_curve_overlay(
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

fn draw_curve_handles(app: &LobedoApp, ui: &egui::Ui, rect: Rect, node_id: NodeId) {
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

const FFD_MIN_AXIS_SIZE: f32 = 1.0e-6;

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
        let tz = if res_z > 1 {
            z as f32 / (res_z - 1) as f32
        } else {
            0.0
        };
        let pz = min.z + size.z * tz;
        for y in 0..res_y {
            let ty = if res_y > 1 {
                y as f32 / (res_y - 1) as f32
            } else {
                0.0
            };
            let py = min.y + size.y * ty;
            for x in 0..res_x {
                let tx = if res_x > 1 {
                    x as f32 / (res_x - 1) as f32
                } else {
                    0.0
                };
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

fn draw_ffd_lattice_overlay(app: &LobedoApp, ui: &egui::Ui, rect: Rect, node_id: NodeId) {
    let Some(node) = app.project.graph.node(node_id) else {
        return;
    };
    if node.name != "FFD" {
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
                if let (Some(a), Some(b)) =
                    (project_world_to_screen(view_proj, rect, a), project_world_to_screen(view_proj, rect, b))
                {
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
                if let (Some(a), Some(b)) =
                    (project_world_to_screen(view_proj, rect, a), project_world_to_screen(view_proj, rect, b))
                {
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
                if let (Some(a), Some(b)) =
                    (project_world_to_screen(view_proj, rect, a), project_world_to_screen(view_proj, rect, b))
                {
                    painter.line_segment([a, b], stroke);
                }
            }
        }
    }
}

fn draw_ffd_lattice_handles(app: &LobedoApp, ui: &egui::Ui, rect: Rect, node_id: NodeId) {
    let Some(node) = app.project.graph.node(node_id) else {
        return;
    };
    if node.name != "FFD" {
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
struct FfdHandlePick {
    index: usize,
    axis: Option<GizmoAxis>,
    point: Vec3,
}

fn pick_ffd_handle(
    camera: render::CameraState,
    rect: Rect,
    pixels_per_point: f32,
    mouse: Pos2,
    node_id: NodeId,
    graph: &lobedo_core::Graph,
) -> Option<FfdHandlePick> {
    let node = graph.node(node_id)?;
    if node.name != "FFD" {
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
            let dist = distance_to_segment(mouse, screen, end_screen);
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

pub(super) fn input_node_for(
    graph: &lobedo_core::Graph,
    node_id: NodeId,
    input_index: usize,
) -> Option<NodeId> {
    let node = graph.node(node_id)?;
    let input_pin = *node.inputs.get(input_index)?;
    for link in graph.links() {
        if link.to == input_pin {
            return graph.pin(link.from).map(|pin| pin.node);
        }
    }
    None
}

fn box_handle_positions(center: Vec3, size: Vec3) -> Vec<(BoxHandle, Vec3)> {
    let mut handles = Vec::with_capacity(7);
    let mut half = size.abs() * 0.5;
    half = Vec3::new(
        half.x.max(0.001),
        half.y.max(0.001),
        half.z.max(0.001),
    );
    handles.push((BoxHandle::Center, center));
    handles.push((BoxHandle::Face { axis: GizmoAxis::X, sign: 1.0 }, center + Vec3::X * half.x));
    handles.push((BoxHandle::Face { axis: GizmoAxis::X, sign: -1.0 }, center - Vec3::X * half.x));
    handles.push((BoxHandle::Face { axis: GizmoAxis::Y, sign: 1.0 }, center + Vec3::Y * half.y));
    handles.push((BoxHandle::Face { axis: GizmoAxis::Y, sign: -1.0 }, center - Vec3::Y * half.y));
    handles.push((BoxHandle::Face { axis: GizmoAxis::Z, sign: 1.0 }, center + Vec3::Z * half.z));
    handles.push((BoxHandle::Face { axis: GizmoAxis::Z, sign: -1.0 }, center - Vec3::Z * half.z));
    handles
}

fn pick_box_handle(
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

#[derive(Clone, Copy)]
struct CurveHandlePick {
    index: usize,
    axis: Option<GizmoAxis>,
    point: Vec3,
}

fn pick_curve_handle(
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
            let dist = distance_to_segment(mouse, screen, end_screen);
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


fn distance_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ap = p - a;
    let ab = b - a;
    let t = (ap.dot(ab) / ab.dot(ab)).clamp(0.0, 1.0);
    let closest = a + ab * t;
    (p - closest).length()
}

fn distance_to_polyline(p: Pos2, points: &[Pos2]) -> f32 {
    if points.len() < 2 {
        return f32::INFINITY;
    }
    let mut best = f32::INFINITY;
    for segment in points.windows(2) {
        let dist = distance_to_segment(p, segment[0], segment[1]);
        best = best.min(dist);
    }
    best
}

fn selection_action(modifiers: egui::Modifiers) -> SelectionAction {
    if modifiers.ctrl || modifiers.command {
        SelectionAction::Remove
    } else if modifiers.shift {
        SelectionAction::Add
    } else {
        SelectionAction::Replace
    }
}

fn parse_selection_indices(value: &str) -> BTreeSet<usize> {
    let mut set = BTreeSet::new();
    for token in value.split(|c: char| c.is_whitespace() || c == ',' || c == ';') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        if let Ok(index) = token.parse::<usize>() {
            set.insert(index);
        }
    }
    set
}

fn encode_selection_indices(indices: &BTreeSet<usize>) -> String {
    let mut out = String::new();
    for (idx, value) in indices.iter().enumerate() {
        if idx > 0 {
            out.push(' ');
        }
        out.push_str(&value.to_string());
    }
    out
}

fn group_selection_settings(
    graph: &lobedo_core::Graph,
    node_id: NodeId,
) -> Option<(AttributeDomain, bool)> {
    let node = graph.node(node_id)?;
    if node.name != "Group" {
        return None;
    }
    let domain = node.params.get_int("domain", 2).clamp(0, 2);
    let domain = match domain {
        0 => AttributeDomain::Point,
        1 => AttributeDomain::Vertex,
        _ => AttributeDomain::Primitive,
    };
    let allow_backface = node.params.get_bool("select_backface", false);
    Some((domain, allow_backface))
}

enum SelectionSource<'a> {
    Mesh(&'a render::RenderMesh),
    Splats(&'a render::RenderSplats),
}

fn resolve_selection_source<'a>(
    scene: &'a render::RenderScene,
    domain: AttributeDomain,
) -> Option<SelectionSource<'a>> {
    match domain {
        AttributeDomain::Point => {
            if let Some(mesh) = scene.mesh() {
                if !mesh.positions.is_empty() {
                    return Some(SelectionSource::Mesh(mesh));
                }
            }
            scene.splats().map(SelectionSource::Splats)
        }
        AttributeDomain::Vertex => scene.mesh().map(SelectionSource::Mesh),
        AttributeDomain::Primitive => {
            if let Some(mesh) = scene.mesh() {
                if !mesh.indices.is_empty() {
                    return Some(SelectionSource::Mesh(mesh));
                }
            }
            scene.splats().map(SelectionSource::Splats)
        }
        AttributeDomain::Detail => None,
    }
}

fn pick_selection_index(
    scene: &render::RenderScene,
    domain: AttributeDomain,
    view_proj: Mat4,
    rect: Rect,
    mouse: Pos2,
    camera: render::CameraState,
    allow_backface: bool,
) -> Option<usize> {
    let threshold = 12.0;
    let source = resolve_selection_source(scene, domain)?;
    let camera_pos = camera_position(camera);
    match (source, domain) {
        (SelectionSource::Mesh(mesh), AttributeDomain::Point) => pick_nearest_index(
            mesh.positions
                .iter()
                .enumerate()
                .filter(|(idx, pos)| {
                    allow_backface
                        || is_front_facing_point(mesh, *idx, Vec3::from(**pos), camera_pos)
                })
                .map(|(idx, pos)| (idx, Vec3::from(*pos))),
            view_proj,
            rect,
            mouse,
            threshold,
        ),
        (SelectionSource::Mesh(mesh), AttributeDomain::Vertex) => pick_nearest_index(
            mesh.indices
                .iter()
                .enumerate()
                .filter_map(|(idx, pos)| {
                    let point_index = *pos as usize;
                    let world = mesh.positions.get(point_index).copied()?;
                    if !allow_backface
                        && !is_front_facing_vertex(mesh, idx, point_index, Vec3::from(world), camera_pos)
                    {
                        return None;
                    }
                    Some((idx, Vec3::from(world)))
                }),
            view_proj,
            rect,
            mouse,
            threshold,
        ),
        (SelectionSource::Mesh(mesh), AttributeDomain::Primitive) => pick_primitive_index(
            mesh,
            view_proj,
            rect,
            mouse,
            camera_pos,
            allow_backface,
        ),
        (SelectionSource::Splats(splats), _) => pick_nearest_index(
            splats
                .positions
                .iter()
                .enumerate()
                .map(|(idx, pos)| (idx, Vec3::from(*pos))),
            view_proj,
            rect,
            mouse,
            threshold,
        ),
        _ => None,
    }
}

fn selection_indices_in_rect(
    scene: &render::RenderScene,
    domain: AttributeDomain,
    view_proj: Mat4,
    rect: Rect,
    selection_rect: Rect,
    camera: render::CameraState,
    allow_backface: bool,
) -> BTreeSet<usize> {
    let mut out = BTreeSet::new();
    let Some(source) = resolve_selection_source(scene, domain) else {
        return out;
    };
    let camera_pos = camera_position(camera);
    match (source, domain) {
        (SelectionSource::Mesh(mesh), AttributeDomain::Point) => {
            for (idx, pos) in mesh.positions.iter().enumerate() {
                if !allow_backface
                    && !is_front_facing_point(mesh, idx, Vec3::from(*pos), camera_pos)
                {
                    continue;
                }
                if let Some(screen) =
                    project_world_to_screen(view_proj, rect, Vec3::from(*pos))
                {
                    if selection_rect.contains(screen) {
                        out.insert(idx);
                    }
                }
            }
        }
        (SelectionSource::Mesh(mesh), AttributeDomain::Vertex) => {
            for (idx, pos) in mesh.indices.iter().enumerate() {
                if let Some(world) = mesh.positions.get(*pos as usize) {
                    if !allow_backface
                        && !is_front_facing_vertex(
                            mesh,
                            idx,
                            *pos as usize,
                            Vec3::from(*world),
                            camera_pos,
                        )
                    {
                        continue;
                    }
                    if let Some(screen) =
                        project_world_to_screen(view_proj, rect, Vec3::from(*world))
                    {
                        if selection_rect.contains(screen) {
                            out.insert(idx);
                        }
                    }
                }
            }
        }
        (SelectionSource::Mesh(mesh), AttributeDomain::Primitive) => {
            for (idx, tri) in mesh.indices.chunks_exact(3).enumerate() {
                let (Some(p0), Some(p1), Some(p2)) = (
                    mesh.positions.get(tri[0] as usize),
                    mesh.positions.get(tri[1] as usize),
                    mesh.positions.get(tri[2] as usize),
                ) else {
                    continue;
                };
                if !allow_backface
                    && !is_front_facing_primitive(
                        Vec3::from(*p0),
                        Vec3::from(*p1),
                    Vec3::from(*p2),
                    camera_pos,
                )
                {
                    continue;
                }
                let (Some(s0), Some(s1), Some(s2)) = (
                    project_world_to_screen(view_proj, rect, Vec3::from(*p0)),
                    project_world_to_screen(view_proj, rect, Vec3::from(*p1)),
                    project_world_to_screen(view_proj, rect, Vec3::from(*p2)),
                ) else {
                    continue;
                };
                if selection_rect.contains(s0)
                    || selection_rect.contains(s1)
                    || selection_rect.contains(s2)
                    || rect_corners_in_triangle(selection_rect, s0, s1, s2)
                {
                    out.insert(idx);
                }
            }
        }
        (SelectionSource::Splats(splats), _) => {
            for (idx, pos) in splats.positions.iter().enumerate() {
                if let Some(screen) =
                    project_world_to_screen(view_proj, rect, Vec3::from(*pos))
                {
                    if selection_rect.contains(screen) {
                        out.insert(idx);
                    }
                }
            }
        }
        _ => {}
    }
    out
}

fn pick_nearest_index<I>(
    iter: I,
    view_proj: Mat4,
    rect: Rect,
    mouse: Pos2,
    threshold: f32,
) -> Option<usize>
where
    I: Iterator<Item = (usize, Vec3)>,
{
    let mut best = None;
    let mut best_dist = threshold;
    for (idx, world) in iter {
        let Some(screen) = project_world_to_screen(view_proj, rect, world) else {
            continue;
        };
        let dist = (screen - mouse).length();
        if dist <= best_dist {
            best_dist = dist;
            best = Some(idx);
        }
    }
    best
}

fn pick_primitive_index(
    mesh: &render::RenderMesh,
    view_proj: Mat4,
    rect: Rect,
    mouse: Pos2,
    camera_pos: Vec3,
    allow_backface: bool,
) -> Option<usize> {
    let threshold = 12.0;
    let mut best_idx = None;
    let mut best_dist = threshold;
    let mut best_depth = f32::INFINITY;
    for (idx, tri) in mesh.indices.chunks_exact(3).enumerate() {
        let (Some(p0), Some(p1), Some(p2)) = (
            mesh.positions.get(tri[0] as usize),
            mesh.positions.get(tri[1] as usize),
            mesh.positions.get(tri[2] as usize),
        ) else {
            continue;
        };
        if !allow_backface
            && !is_front_facing_primitive(
                Vec3::from(*p0),
                Vec3::from(*p1),
                Vec3::from(*p2),
                camera_pos,
            )
        {
            continue;
        }
        let (s0, d0) = match project_world_to_screen_with_depth(view_proj, rect, Vec3::from(*p0)) {
            Some(value) => value,
            None => continue,
        };
        let (s1, d1) = match project_world_to_screen_with_depth(view_proj, rect, Vec3::from(*p1)) {
            Some(value) => value,
            None => continue,
        };
        let (s2, d2) = match project_world_to_screen_with_depth(view_proj, rect, Vec3::from(*p2)) {
            Some(value) => value,
            None => continue,
        };
        let depth = (d0 + d1 + d2) / 3.0;
        if point_in_triangle(mouse, s0, s1, s2, 0.5) {
            if depth < best_depth {
                best_depth = depth;
                best_dist = 0.0;
                best_idx = Some(idx);
            }
            continue;
        }
        let dist = distance_to_triangle_edges(mouse, s0, s1, s2);
        if dist <= threshold && (dist < best_dist || (dist == best_dist && depth < best_depth)) {
            best_dist = dist;
            best_depth = depth;
            best_idx = Some(idx);
        }
    }
    best_idx
}

fn project_world_to_screen_with_depth(
    view_proj: Mat4,
    rect: Rect,
    world: Vec3,
) -> Option<(Pos2, f32)> {
    let clip = view_proj * world.extend(1.0);
    if clip.w.abs() <= 1.0e-6 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    if !ndc.x.is_finite() || !ndc.y.is_finite() || !ndc.z.is_finite() {
        return None;
    }
    let x = rect.min.x + (ndc.x * 0.5 + 0.5) * rect.width();
    let y = rect.min.y + (0.5 - ndc.y * 0.5) * rect.height();
    Some((Pos2::new(x, y), ndc.z))
}

fn point_in_triangle(p: Pos2, a: Pos2, b: Pos2, c: Pos2, tol: f32) -> bool {
    let ab = b - a;
    let bc = c - b;
    let ca = a - c;
    let ap = p - a;
    let bp = p - b;
    let cp = p - c;
    let cross1 = ab.x * ap.y - ab.y * ap.x;
    let cross2 = bc.x * bp.y - bc.y * bp.x;
    let cross3 = ca.x * cp.y - ca.y * cp.x;
    let has_neg = cross1 < -tol || cross2 < -tol || cross3 < -tol;
    let has_pos = cross1 > tol || cross2 > tol || cross3 > tol;
    !(has_neg && has_pos)
}

fn distance_to_triangle_edges(p: Pos2, a: Pos2, b: Pos2, c: Pos2) -> f32 {
    let d0 = distance_to_segment(p, a, b);
    let d1 = distance_to_segment(p, b, c);
    let d2 = distance_to_segment(p, c, a);
    d0.min(d1).min(d2)
}

fn rect_corners_in_triangle(rect: Rect, a: Pos2, b: Pos2, c: Pos2) -> bool {
    let corners = [
        rect.min,
        Pos2::new(rect.max.x, rect.min.y),
        rect.max,
        Pos2::new(rect.min.x, rect.max.y),
    ];
    corners
        .into_iter()
        .any(|corner| point_in_triangle(corner, a, b, c, 0.0))
}


fn draw_group_selection_overlay(app: &LobedoApp, ui: &egui::Ui, rect: Rect, node_id: NodeId) {
    let Some(scene) = app.last_scene.as_ref() else {
        return;
    };
    let Some((domain, _)) = group_selection_settings(&app.project.graph, node_id) else {
        return;
    };
    let Some(node) = app.project.graph.node(node_id) else {
        return;
    };
    let selection = parse_selection_indices(node.params.get_string("selection", ""));
    if selection.is_empty() {
        return;
    }
    let view_proj = viewport_view_proj(app.camera_state(), rect, ui.ctx().pixels_per_point());
    let painter = ui.painter();
    let point_color = Color32::from_rgb(255, 210, 120);
    let line_color = Color32::from_rgb(255, 235, 170);
    let stroke = Stroke::new(1.0, line_color);
    let point_radius = 3.0;
    match resolve_selection_source(scene, domain) {
        Some(SelectionSource::Mesh(mesh)) => match domain {
            AttributeDomain::Point => {
                for idx in selection {
                    let Some(pos) = mesh.positions.get(idx) else {
                        continue;
                    };
                    if let Some(screen) =
                        project_world_to_screen(view_proj, rect, Vec3::from(*pos))
                    {
                        painter.circle_filled(screen, point_radius, point_color);
                    }
                }
            }
            AttributeDomain::Vertex => {
                for idx in selection {
                    let Some(point_index) = mesh.indices.get(idx).copied() else {
                        continue;
                    };
                    let Some(pos) = mesh.positions.get(point_index as usize) else {
                        continue;
                    };
                    if let Some(screen) =
                        project_world_to_screen(view_proj, rect, Vec3::from(*pos))
                    {
                        painter.circle_filled(screen, point_radius, point_color);
                    }
                }
            }
            AttributeDomain::Primitive => {
                for idx in selection {
                    let tri = match mesh.indices.get(idx * 3..idx * 3 + 3) {
                        Some(tri) => tri,
                        None => continue,
                    };
                    let Some(p0) = mesh.positions.get(tri[0] as usize) else {
                        continue;
                    };
                    let Some(p1) = mesh.positions.get(tri[1] as usize) else {
                        continue;
                    };
                    let Some(p2) = mesh.positions.get(tri[2] as usize) else {
                        continue;
                    };
                    let Some(s0) = project_world_to_screen(view_proj, rect, Vec3::from(*p0)) else {
                        continue;
                    };
                    let Some(s1) = project_world_to_screen(view_proj, rect, Vec3::from(*p1)) else {
                        continue;
                    };
                    let Some(s2) = project_world_to_screen(view_proj, rect, Vec3::from(*p2)) else {
                        continue;
                    };
                    painter.line_segment([s0, s1], stroke);
                    painter.line_segment([s1, s2], stroke);
                    painter.line_segment([s2, s0], stroke);
                }
            }
            _ => {}
        },
        Some(SelectionSource::Splats(splats)) => {
            for idx in selection {
                let Some(pos) = splats.positions.get(idx) else {
                    continue;
                };
                if let Some(screen) =
                    project_world_to_screen(view_proj, rect, Vec3::from(*pos))
                {
                    painter.circle_filled(screen, point_radius, point_color);
                }
            }
        }
        _ => {}
    }
}

fn is_front_facing_point(
    mesh: &render::RenderMesh,
    point_index: usize,
    world: Vec3,
    camera_pos: Vec3,
) -> bool {
    let normal = mesh
        .normals
        .get(point_index)
        .copied()
        .map(Vec3::from)
        .unwrap_or(Vec3::ZERO);
    is_front_facing(normal, world, camera_pos)
}

fn is_front_facing_vertex(
    mesh: &render::RenderMesh,
    vertex_index: usize,
    point_index: usize,
    world: Vec3,
    camera_pos: Vec3,
) -> bool {
    let normal = if let Some(corner) = mesh.corner_normals.as_ref() {
        corner
            .get(vertex_index)
            .copied()
            .map(Vec3::from)
            .unwrap_or(Vec3::ZERO)
    } else {
        mesh.normals
            .get(point_index)
            .copied()
            .map(Vec3::from)
            .unwrap_or(Vec3::ZERO)
    };
    is_front_facing(normal, world, camera_pos)
}

fn is_front_facing_primitive(p0: Vec3, p1: Vec3, p2: Vec3, camera_pos: Vec3) -> bool {
    let normal = (p1 - p0).cross(p2 - p0);
    is_front_facing(normal, (p0 + p1 + p2) / 3.0, camera_pos)
}

fn is_front_facing(normal: Vec3, world: Vec3, camera_pos: Vec3) -> bool {
    if normal.length_squared() <= 1.0e-6 {
        return true;
    }
    let view_dir = (camera_pos - world).normalize_or_zero();
    normal.normalize_or_zero().dot(view_dir) > 0.0
}
