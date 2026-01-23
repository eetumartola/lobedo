use std::collections::BTreeSet;

use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use glam::{Mat4, Vec3};

use lobedo_core::{AttributeDomain, BuiltinNodeKind, NodeId, ParamValue};

use super::LobedoApp;
use super::SelectionAction;
use super::viewport_tools_math::{
    camera_position, distance_to_triangle_edges, point_in_triangle,
    project_world_to_screen, project_world_to_screen_with_depth, rect_corners_in_triangle,
    viewport_view_proj,
};

impl LobedoApp {
    pub(super) fn apply_group_selection(
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

pub(super) fn parse_selection_indices(value: &str) -> BTreeSet<usize> {
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

pub(super) fn encode_selection_indices(indices: &BTreeSet<usize>) -> String {
    let mut out = String::new();
    for (idx, value) in indices.iter().enumerate() {
        if idx > 0 {
            out.push(' ');
        }
        out.push_str(&value.to_string());
    }
    out
}

pub(super) fn group_selection_settings(
    graph: &lobedo_core::Graph,
    node_id: NodeId,
) -> Option<(AttributeDomain, bool)> {
    let node = graph.node(node_id)?;
    if node.builtin_kind() != Some(BuiltinNodeKind::Group) {
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

pub(super) fn pick_selection_index(
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
                    allow_backface || is_front_facing_point(mesh, *idx, Vec3::from(**pos), camera_pos)
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
                    let corner = mesh
                        .corner_indices
                        .get(idx)
                        .copied()
                        .unwrap_or(idx as u32) as usize;
                    Some((corner, Vec3::from(world)))
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

pub(super) fn selection_indices_in_rect(
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
                if !allow_backface && !is_front_facing_point(mesh, idx, Vec3::from(*pos), camera_pos)
                {
                    continue;
                }
                if let Some(screen) = project_world_to_screen(view_proj, rect, Vec3::from(*pos)) {
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
                            let corner = mesh
                                .corner_indices
                                .get(idx)
                                .copied()
                                .unwrap_or(idx as u32) as usize;
                            out.insert(corner);
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
                    let face = mesh
                        .tri_to_face
                        .get(idx)
                        .copied()
                        .unwrap_or(idx as u32) as usize;
                    out.insert(face);
                }
            }
        }
        (SelectionSource::Splats(splats), _) => {
            for (idx, pos) in splats.positions.iter().enumerate() {
                if let Some(screen) = project_world_to_screen(view_proj, rect, Vec3::from(*pos)) {
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
        let (s0, d0) =
            match project_world_to_screen_with_depth(view_proj, rect, Vec3::from(*p0)) {
                Some(value) => value,
                None => continue,
            };
        let (s1, d1) =
            match project_world_to_screen_with_depth(view_proj, rect, Vec3::from(*p1)) {
                Some(value) => value,
                None => continue,
            };
        let (s2, d2) =
            match project_world_to_screen_with_depth(view_proj, rect, Vec3::from(*p2)) {
                Some(value) => value,
                None => continue,
            };
        let depth = (d0 + d1 + d2) / 3.0;
        let face = mesh
            .tri_to_face
            .get(idx)
            .copied()
            .unwrap_or(idx as u32) as usize;
        if point_in_triangle(mouse, s0, s1, s2, 0.5) {
            if depth < best_depth {
                best_depth = depth;
                best_dist = 0.0;
                best_idx = Some(face);
            }
            continue;
        }
        let dist = distance_to_triangle_edges(mouse, s0, s1, s2);
        if dist <= threshold && (dist < best_dist || (dist == best_dist && depth < best_depth)) {
            best_dist = dist;
            best_depth = depth;
            best_idx = Some(face);
        }
    }
    best_idx
}

pub(super) fn draw_group_selection_overlay(
    app: &LobedoApp,
    ui: &egui::Ui,
    rect: Rect,
    node_id: NodeId,
) {
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
                    if let Some(screen) = project_world_to_screen(view_proj, rect, Vec3::from(*pos))
                    {
                        painter.circle_filled(screen, point_radius, point_color);
                    }
                }
            }
            AttributeDomain::Vertex => {
                for (tri_corner_idx, corner_idx) in mesh.corner_indices.iter().enumerate() {
                    if !selection.contains(&(*corner_idx as usize)) {
                        continue;
                    }
                    let Some(point_index) = mesh.indices.get(tri_corner_idx).copied() else {
                        continue;
                    };
                    let Some(pos) = mesh.positions.get(point_index as usize) else {
                        continue;
                    };
                    if let Some(screen) = project_world_to_screen(view_proj, rect, Vec3::from(*pos))
                    {
                        painter.circle_filled(screen, point_radius, point_color);
                    }
                }
            }
            AttributeDomain::Primitive => {
                if !mesh.poly_face_counts.is_empty() && !mesh.poly_indices.is_empty() {
                    let mut cursor = 0usize;
                    for (face_idx, count) in mesh.poly_face_counts.iter().enumerate() {
                        let count = *count as usize;
                        if count < 2 || cursor + count > mesh.poly_indices.len() {
                            cursor = cursor.saturating_add(count);
                            continue;
                        }
                        if !selection.contains(&face_idx) {
                            cursor += count;
                            continue;
                        }
                        for i in 0..count {
                            let a = mesh.poly_indices[cursor + i] as usize;
                            let b = mesh.poly_indices[cursor + (i + 1) % count] as usize;
                            let (Some(p0), Some(p1)) = (mesh.positions.get(a), mesh.positions.get(b))
                            else {
                                continue;
                            };
                            let Some(s0) =
                                project_world_to_screen(view_proj, rect, Vec3::from(*p0))
                            else {
                                continue;
                            };
                            let Some(s1) =
                                project_world_to_screen(view_proj, rect, Vec3::from(*p1))
                            else {
                                continue;
                            };
                            painter.line_segment([s0, s1], stroke);
                        }
                        cursor += count;
                    }
                } else {
                    for (tri_idx, tri) in mesh.indices.chunks_exact(3).enumerate() {
                        let face_idx = mesh
                            .tri_to_face
                            .get(tri_idx)
                            .copied()
                            .unwrap_or(tri_idx as u32) as usize;
                        if !selection.contains(&face_idx) {
                            continue;
                        }
                        let Some(p0) = mesh.positions.get(tri[0] as usize) else {
                            continue;
                        };
                        let Some(p1) = mesh.positions.get(tri[1] as usize) else {
                            continue;
                        };
                        let Some(p2) = mesh.positions.get(tri[2] as usize) else {
                            continue;
                        };
                        let Some(s0) =
                            project_world_to_screen(view_proj, rect, Vec3::from(*p0))
                        else {
                            continue;
                        };
                        let Some(s1) =
                            project_world_to_screen(view_proj, rect, Vec3::from(*p1))
                        else {
                            continue;
                        };
                        let Some(s2) =
                            project_world_to_screen(view_proj, rect, Vec3::from(*p2))
                        else {
                            continue;
                        };
                        painter.line_segment([s0, s1], stroke);
                        painter.line_segment([s1, s2], stroke);
                        painter.line_segment([s2, s0], stroke);
                    }
                }
            }
            _ => {}
        },
        Some(SelectionSource::Splats(splats)) => {
            for idx in selection {
                let Some(pos) = splats.positions.get(idx) else {
                    continue;
                };
                if let Some(screen) = project_world_to_screen(view_proj, rect, Vec3::from(*pos))
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

pub(super) fn selection_action(modifiers: egui::Modifiers) -> SelectionAction {
    if modifiers.ctrl || modifiers.command {
        SelectionAction::Remove
    } else if modifiers.shift {
        SelectionAction::Add
    } else {
        SelectionAction::Replace
    }
}
