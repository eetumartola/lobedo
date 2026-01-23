use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use glam::{Quat, Vec3};
use lobedo_core::{BuiltinNodeKind, NodeId};
use std::collections::BTreeSet;

mod viewport_tools_curve;
mod viewport_tools_ffd;
mod viewport_tools_gizmo;
mod viewport_tools_math;
mod viewport_tools_selection;

use viewport_tools_curve::{draw_curve_handles, draw_curve_overlay, pick_curve_handle};
use viewport_tools_ffd::{draw_ffd_lattice_handles, draw_ffd_lattice_overlay, pick_ffd_handle};
use viewport_tools_gizmo::{
    apply_box_drag, apply_transform_drag, axis_dir, axis_drag_delta, box_params, draw_box_handles,
    draw_transform_gizmo, pick_box_handle, pick_gizmo_hit, transform_basis, transform_origin,
    transform_params, transform_quat,
};
use viewport_tools_math::{camera_forward, raycast_plane, raycast_plane_y, viewport_view_proj};
use viewport_tools_selection::{
    draw_group_selection_overlay, group_selection_settings, pick_selection_index,
    selection_action, selection_indices_in_rect,
};
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
        if node.builtin_kind() != Some(BuiltinNodeKind::Group) {
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
        if matches!(
            node.builtin_kind(),
            Some(BuiltinNodeKind::Transform | BuiltinNodeKind::CopyTransform)
        ) {
            Some(node_id)
        } else {
            None
        }
    }

    fn selected_box_node(&self) -> Option<NodeId> {
        let node_id = self.node_graph.selected_node_id()?;
        let node = self.project.graph.node(node_id)?;
        match node.builtin_kind() {
            Some(BuiltinNodeKind::Box) => Some(node_id),
            Some(BuiltinNodeKind::Group | BuiltinNodeKind::Delete) => {
                let shape = node.params.get_string("shape", "box").to_lowercase();
                if shape == "box" {
                    Some(node_id)
                } else {
                    None
                }
            }
            Some(BuiltinNodeKind::SplatHeal) => {
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

    }

pub(super) fn input_node_for(
    graph: &lobedo_core::Graph,
    node_id: NodeId,
    input_index: usize,
) -> Option<NodeId> {
    graph.input_node(node_id, input_index)
}
