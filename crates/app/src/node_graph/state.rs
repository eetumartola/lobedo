use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use egui::{vec2, Color32, Frame, LayerId, Pos2, Rect, Sense, Stroke, Ui, UiBuilder};
use egui_snarl::ui::{BackgroundPattern, SnarlStyle};
use egui_snarl::{InPinId, OutPinId, Snarl};

use lobedo_core::{Graph, GraphNote, NodeId, ProgressEvent, ProgressSink, ProjectSettings};

use super::viewer::NodeGraphViewer;

const NOTE_MIN_WIDTH: f32 = 200.0;
const NOTE_MIN_HEIGHT: f32 = 120.0;
const NOTE_DEFAULT_WIDTH: f32 = 240.0;
const NOTE_DEFAULT_HEIGHT: f32 = 160.0;

#[derive(Clone, Copy)]
pub(super) struct SnarlNode {
    pub(super) core_id: NodeId,
}

pub struct NodeGraphState {
    pub(super) snarl: Snarl<SnarlNode>,
    pub(super) core_to_snarl: HashMap<NodeId, egui_snarl::NodeId>,
    pub(super) snarl_to_core: HashMap<egui_snarl::NodeId, NodeId>,
    pub(super) next_pos: Pos2,
    pub(super) needs_wire_sync: bool,
    pub(super) selected_node: Option<NodeId>,
    pub(super) node_ui_rects: HashMap<egui_snarl::NodeId, Rect>,
    pub(super) prev_node_ui_rects: HashMap<egui_snarl::NodeId, Rect>,
    pub(super) header_button_rects: HashMap<egui_snarl::NodeId, HeaderButtonRects>,
    pub(super) dragging_node: Option<egui_snarl::NodeId>,
    pub(super) pin_drag_active: bool,
    pub(super) add_menu_open: bool,
    pub(super) add_menu_screen_pos: Pos2,
    pub(super) add_menu_graph_pos: Pos2,
    pub(super) add_menu_filter: String,
    pub(super) add_menu_focus: bool,
    pub(super) pending_wire: Option<PendingWire>,
    pub(super) info_request: Option<NodeInfoRequest>,
    pub(super) wrangle_help_request: Option<Pos2>,
    pub(super) help_page_node: Option<String>,
    pub(super) graph_transform: GraphTransformState,
    pub(super) pending_transform: Option<egui::emath::TSTransform>,
    pub(super) input_pin_positions: Rc<RefCell<HashMap<InPinId, Pos2>>>,
    pub(super) output_pin_positions: Rc<RefCell<HashMap<OutPinId, Pos2>>>,
    pub(super) error_nodes: HashSet<NodeId>,
    pub(super) error_messages: HashMap<NodeId, String>,
    pub(super) dirty_nodes: HashSet<NodeId>,
    pub(super) node_menu_request: Option<NodeMenuRequest>,
    pub(super) node_menu_open: bool,
    pub(super) node_menu_screen_pos: Pos2,
    pub(super) node_menu_node: Option<NodeId>,
    pub(super) pending_write_request: Option<WriteRequest>,
    pub(super) progress_state: Arc<Mutex<NodeProgressState>>,
    pub(super) selected_note: Option<u64>,
    pub(super) last_changed: bool,
    pub(super) layout_changed: bool,
}

#[derive(Clone, Copy)]
pub(super) struct GraphTransformState {
    pub(super) to_global: egui::emath::TSTransform,
    pub(super) valid: bool,
}

#[derive(Clone, Copy)]
pub struct NodeInfoRequest {
    pub node_id: NodeId,
    pub screen_pos: Pos2,
}

#[derive(Clone, Copy)]
pub enum WriteRequestKind {
    Obj,
    Gltf,
    Splat,
}

#[derive(Clone, Copy)]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub struct WriteRequest {
    pub node_id: NodeId,
    pub kind: WriteRequestKind,
}

impl Default for NodeGraphState {
    fn default() -> Self {
        Self {
            snarl: Snarl::new(),
            core_to_snarl: HashMap::new(),
            snarl_to_core: HashMap::new(),
            next_pos: Pos2::new(0.0, 0.0),
            needs_wire_sync: true,
            selected_node: None,
            node_ui_rects: HashMap::new(),
            prev_node_ui_rects: HashMap::new(),
            header_button_rects: HashMap::new(),
            dragging_node: None,
            pin_drag_active: false,
            add_menu_open: false,
            add_menu_screen_pos: Pos2::new(0.0, 0.0),
            add_menu_graph_pos: Pos2::new(0.0, 0.0),
            add_menu_filter: String::new(),
            add_menu_focus: false,
            pending_wire: None,
            info_request: None,
            wrangle_help_request: None,
            help_page_node: None,
            graph_transform: GraphTransformState {
                to_global: egui::emath::TSTransform::IDENTITY,
                valid: false,
            },
            pending_transform: None,
            input_pin_positions: Rc::new(RefCell::new(HashMap::new())),
            output_pin_positions: Rc::new(RefCell::new(HashMap::new())),
            error_nodes: HashSet::new(),
            error_messages: HashMap::new(),
            dirty_nodes: HashSet::new(),
            node_menu_request: None,
            node_menu_open: false,
            node_menu_screen_pos: Pos2::new(0.0, 0.0),
            node_menu_node: None,
            pending_write_request: None,
            progress_state: Arc::new(Mutex::new(NodeProgressState::default())),
            selected_note: None,
            last_changed: false,
            layout_changed: false,
        }
    }
}

pub(super) struct NodeMenuRequest {
    pub(super) node_id: NodeId,
    pub(super) screen_pos: Pos2,
}

#[derive(Clone, Copy)]
pub(super) struct HeaderButtonRects {
    pub(super) bypass: Rect,
    pub(super) display: Rect,
    pub(super) template: Rect,
    pub(super) help: Option<Rect>,
}

impl HeaderButtonRects {
    fn hit_test(&self, pos: Pos2) -> bool {
        self.bypass.contains(pos)
            || self.display.contains(pos)
            || self.template.contains(pos)
            || self.help.is_some_and(|rect| rect.contains(pos))
    }
}

#[derive(Clone, Copy)]
pub(super) struct NodeProgressView {
    pub(super) fraction: f32,
    pub(super) active: bool,
}

#[derive(Default)]
pub(super) struct NodeProgressState {
    entries: HashMap<NodeId, NodeProgressEntry>,
}

#[derive(Clone)]
struct NodeProgressEntry {
    fraction: f32,
    started_at: Instant,
    last_update: Instant,
    finished_at: Option<Instant>,
}

#[derive(Clone, Default, PartialEq)]
pub struct NodeGraphLayout {
    pub positions: HashMap<NodeId, Pos2>,
    pub selected: Option<NodeId>,
}

impl NodeGraphState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn error_message(&self, node_id: NodeId) -> Option<&String> {
        self.error_messages.get(&node_id)
    }

    pub fn take_write_request(&mut self) -> Option<WriteRequest> {
        self.pending_write_request.take()
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        graph: &mut Graph,
        settings: &mut ProjectSettings,
        eval_dirty: &mut bool,
        eval_state: &lobedo_core::EvalState,
    ) {
        self.ensure_nodes(graph);
        if self.needs_wire_sync {
            self.sync_wires(graph);
            self.needs_wire_sync = false;
        }

        self.prev_node_ui_rects = std::mem::take(&mut self.node_ui_rects);
        self.node_ui_rects.clear();
        self.last_changed = false;
        let (skip_header_click, header_changed) = self.handle_header_click(ui, graph);
        if header_changed {
            self.last_changed = true;
            *eval_dirty = true;
            self.needs_wire_sync = true;
        }
        self.header_button_rects.clear();
        self.input_pin_positions.borrow_mut().clear();
        self.output_pin_positions.borrow_mut().clear();
        self.layout_changed = false;
        let progress_snapshot = self.progress_snapshot();
        if !progress_snapshot.is_empty() {
            ui.ctx().request_repaint();
        }

        let dim_nodes = self.compute_dim_nodes(graph, eval_state);
        let mut viewer = NodeGraphViewer {
            graph,
            selected_node: &mut self.selected_node,
            node_rects: &mut self.node_ui_rects,
            header_button_rects: &mut self.header_button_rects,
            progress: &progress_snapshot,
            graph_transform: &mut self.graph_transform,
            pending_transform: &mut self.pending_transform,
            input_pin_positions: Rc::clone(&self.input_pin_positions),
            output_pin_positions: Rc::clone(&self.output_pin_positions),
            add_menu_open: &mut self.add_menu_open,
            add_menu_screen_pos: &mut self.add_menu_screen_pos,
            add_menu_graph_pos: &mut self.add_menu_graph_pos,
            add_menu_filter: &mut self.add_menu_filter,
            add_menu_focus: &mut self.add_menu_focus,
            pending_wire: &mut self.pending_wire,
            node_menu_request: &mut self.node_menu_request,
            wrangle_help_request: &mut self.wrangle_help_request,
            error_nodes: &self.error_nodes,
            error_messages: &self.error_messages,
            dim_nodes: &dim_nodes,
            skip_header_click,
            changed: false,
        };
        let style = SnarlStyle {
            pin_size: Some(10.0),
            bg_frame: Some(Frame::NONE.fill(Color32::from_rgb(18, 18, 18))),
            bg_pattern: Some(BackgroundPattern::grid(vec2(64.0, 64.0), 0.0)),
            bg_pattern_stroke: Some(Stroke::new(1.0, Color32::from_rgb(26, 26, 26))),
            collapsible: Some(false),
            ..SnarlStyle::default()
        };
        self.snarl.show(&mut viewer, &style, "node_graph", ui);
        let viewer_changed = viewer.changed;
        self.last_changed |= viewer_changed;
        drop(viewer);

        if self.show_notes(ui, settings) {
            self.last_changed = true;
            self.layout_changed = true;
        }
        for (node, rect) in &self.node_ui_rects {
            let Some(prev) = self.prev_node_ui_rects.get(node) else {
                continue;
            };
            if (rect.center() - prev.center()).length_sq() > 0.5 {
                self.layout_changed = true;
                break;
            }
        }

        self.sync_graph_positions(graph);

        if viewer_changed && !self.layout_changed {
            *eval_dirty = true;
            self.needs_wire_sync = true;
        }

        if self.add_menu_open && self.show_add_menu(ui, graph, settings) {
            self.last_changed = true;
            *eval_dirty = true;
            self.needs_wire_sync = true;
        }

        self.update_drag_state(ui);

        if self.handle_drop_on_wire(ui, graph) {
            self.last_changed = true;
            *eval_dirty = true;
            self.needs_wire_sync = true;
        }

        if let Some(request) = self.node_menu_request.take() {
            self.node_menu_open = true;
            self.node_menu_node = Some(request.node_id);
            self.node_menu_screen_pos = request.screen_pos;
        }
        if self.node_menu_open && self.show_node_menu(ui, graph) {
            self.last_changed = true;
            *eval_dirty = true;
            self.needs_wire_sync = true;
        }
    }

    pub fn take_changed(&mut self) -> bool {
        let changed = self.last_changed;
        self.last_changed = false;
        changed
    }

    pub fn take_layout_changed(&mut self) -> bool {
        let changed = self.layout_changed;
        self.layout_changed = false;
        changed
    }

    fn handle_header_click(&mut self, ui: &Ui, graph: &mut Graph) -> (bool, bool) {
        let clicked_pos = ui.input(|i| {
            if i.pointer.primary_clicked() {
                i.pointer.interact_pos()
            } else {
                None
            }
        });
        let Some(pos_screen) = clicked_pos else {
            return (false, false);
        };
        let pos = if self.graph_transform.valid {
            self.graph_transform.to_global.inverse() * pos_screen
        } else {
            pos_screen
        };
        for (snarl_id, rects) in &self.header_button_rects {
            if rects.bypass.contains(pos) {
                if let Some(core_id) = self.snarl_to_core.get(snarl_id) {
                    let _ = graph.toggle_bypass_node(*core_id);
                    return (true, true);
                }
            }
            if rects.display.contains(pos) {
                if let Some(core_id) = self.snarl_to_core.get(snarl_id) {
                    let _ = graph.toggle_display_node(*core_id);
                    return (true, true);
                }
            }
            if rects.template.contains(pos) {
                if let Some(core_id) = self.snarl_to_core.get(snarl_id) {
                    let _ = graph.toggle_template_node(*core_id);
                    return (true, true);
                }
            }
            if rects.help.is_some_and(|rect| rect.contains(pos)) {
                self.wrangle_help_request = Some(pos_screen);
                return (true, false);
            }
        }
        (false, false)
    }

    fn compute_dim_nodes(
        &self,
        graph: &Graph,
        eval_state: &lobedo_core::EvalState,
    ) -> HashSet<NodeId> {
        let mut dim_nodes = HashSet::new();
        let mut dirty_cache = HashMap::new();
        let mut dirty_visiting = HashSet::new();
        for node in graph.nodes() {
            if lobedo_core::node_dirty(
                graph,
                eval_state,
                node.id,
                &mut dirty_cache,
                &mut dirty_visiting,
            ) {
                dim_nodes.insert(node.id);
            }
        }
        dim_nodes
    }

    pub fn preflight_flag_click(&self, ui: &Ui) -> bool {
        let clicked_pos = ui.input(|i| {
            if i.pointer.primary_clicked() {
                i.pointer.interact_pos()
            } else {
                None
            }
        });
        let Some(pos) = clicked_pos else {
            return false;
        };
        let pos = if self.graph_transform.valid {
            self.graph_transform.to_global.inverse() * pos
        } else {
            pos
        };
        self.header_button_rects
            .values()
            .any(|rects| rects.hit_test(pos))
    }


    pub(super) fn add_note(
        &mut self,
        settings: &mut ProjectSettings,
        graph_pos: Pos2,
    ) -> bool {
        let id = settings.next_note_id.max(1);
        settings.next_note_id = id.saturating_add(1);
        settings.graph_notes.push(GraphNote {
            id,
            position: [graph_pos.x, graph_pos.y],
            size: [NOTE_DEFAULT_WIDTH, NOTE_DEFAULT_HEIGHT],
            text: String::new(),
        });
        true
    }

    fn show_notes(
        &mut self,
        ui: &mut Ui,
        settings: &mut ProjectSettings,
    ) -> bool {
        if settings.graph_notes.is_empty() {
            return false;
        }
        let mut hit_note = false;
        let snarl_id = ui.make_persistent_id("node_graph");
        let snarl_layer_id = LayerId::new(ui.layer_id().order, snarl_id);
        let mut note_ui = ui.new_child(
            UiBuilder::new()
                .layer_id(snarl_layer_id)
                .max_rect(ui.max_rect())
                .sense(Sense::click()),
        );
        note_ui.set_clip_rect(ui.clip_rect());
        let to_global = if self.graph_transform.valid {
            self.graph_transform.to_global
        } else {
            egui::emath::TSTransform::IDENTITY
        };
        let mut changed = false;
        for note in &mut settings.graph_notes {
            let note_pos = Pos2::new(note.position[0], note.position[1]);
            let graph_size = vec2(
                note.size[0].max(NOTE_MIN_WIDTH),
                note.size[1].max(NOTE_MIN_HEIGHT),
            );
            let note_rect = Rect::from_min_size(note_pos, graph_size);
            let note_rect = Rect::from_min_max(to_global * note_rect.min, to_global * note_rect.max);
            if !note_rect.intersects(note_ui.clip_rect()) {
                continue;
            }
            let id = note_ui.make_persistent_id(("graph_note", note.id));
            let mut drag_delta = vec2(0.0, 0.0);
            let mut note_changed = false;
            let selected = self.selected_note == Some(note.id);
            {
                let painter = note_ui.painter();
                painter.rect_filled(note_rect, 4.0, Color32::from_rgb(255, 236, 140));
                painter.rect_stroke(
                    note_rect,
                    4.0,
                    if selected {
                        Stroke::new(2.0, Color32::from_rgb(230, 140, 40))
                    } else {
                        Stroke::new(1.0, Color32::from_rgb(210, 180, 80))
                    },
                    egui::StrokeKind::Inside,
                );
                let header_height = 18.0;
                let header_bottom = note_rect.min.y + header_height;
                let header_rect = Rect::from_min_max(
                    note_rect.min,
                    Pos2::new(note_rect.max.x, header_bottom),
                );
                painter.rect_filled(header_rect, 2.0, Color32::from_rgb(255, 226, 110));
            }
            let header_height = 18.0;
            let header_bottom = note_rect.min.y + header_height;
            let header_rect = Rect::from_min_max(
                note_rect.min,
                Pos2::new(note_rect.max.x, header_bottom),
            );
            let header_resp = note_ui.interact(
                header_rect,
                id.with("header"),
                Sense::click_and_drag(),
            );
            if header_resp.dragged() {
                drag_delta = header_resp.drag_delta();
                hit_note = true;
            }
            if header_resp.clicked() || header_resp.drag_started() {
                self.selected_note = Some(note.id);
                self.selected_node = None;
                hit_note = true;
            }
            let body_rect = Rect::from_min_max(
                Pos2::new(note_rect.min.x + 6.0, header_bottom + 4.0),
                Pos2::new(note_rect.max.x - 6.0, note_rect.max.y - 6.0),
            );
            if body_rect.is_positive() {
                let text_edit = egui::TextEdit::multiline(&mut note.text)
                    .id(id.with("text"))
                    .frame(false)
                    .desired_width(body_rect.width());
                let response = note_ui.put(body_rect, text_edit);
                if response.changed() {
                    note_changed = true;
                }
                if response.clicked() {
                    self.selected_note = Some(note.id);
                    self.selected_node = None;
                    hit_note = true;
                }
            }
            note_ui.input(|input| {
                if !input.pointer.any_pressed() {
                    return;
                }
                let Some(pos) = input.pointer.interact_pos() else {
                    return;
                };
                if !note_rect.contains(pos) || header_rect.contains(pos) || body_rect.contains(pos)
                {
                    return;
                }
                self.selected_note = Some(note.id);
                self.selected_node = None;
                hit_note = true;
            });
            if drag_delta.length_sq() > 0.0 {
                let scale = to_global.scaling.max(0.0001);
                let delta = drag_delta / scale;
                note.position[0] += delta.x;
                note.position[1] += delta.y;
                note_changed = true;
            }
            if note_changed {
                changed = true;
            }
        }
        if note_ui.input(|i| i.pointer.any_pressed()) && !hit_note {
            self.selected_note = None;
        }
        changed
    }

    pub fn set_error_state(&mut self, nodes: HashSet<NodeId>, messages: HashMap<NodeId, String>) {
        self.error_nodes = nodes;
        self.error_messages = messages;
    }

    pub fn set_dirty_nodes(&mut self, nodes: HashSet<NodeId>) -> bool {
        if self.dirty_nodes == nodes {
            return false;
        }
        self.dirty_nodes = nodes;
        true
    }

    pub fn selected_node_id(&self) -> Option<NodeId> {
        self.selected_node
    }

    pub fn selected_note_id(&self) -> Option<u64> {
        self.selected_note
    }

    pub fn delete_selected_note(&mut self, settings: &mut ProjectSettings) -> bool {
        let Some(note_id) = self.selected_note else {
            return false;
        };
        let before = settings.graph_notes.len();
        settings.graph_notes.retain(|note| note.id != note_id);
        let removed = settings.graph_notes.len() != before;
        if removed {
            self.selected_note = None;
        }
        removed
    }

    pub fn delete_selected_node(&mut self, graph: &mut Graph) -> bool {
        let Some(node_id) = self.selected_node else {
            return false;
        };
        self.delete_node(graph, node_id)
    }

    pub fn delete_node(&mut self, graph: &mut Graph, node_id: NodeId) -> bool {
        let Some(node) = graph.node(node_id) else {
            return false;
        };
        let input_pins = node.inputs.clone();
        let output_pins = node.outputs.clone();
        let mut upstream_outputs = HashSet::new();
        let mut downstream_inputs = Vec::new();
        for link in graph.links() {
            if input_pins.contains(&link.to) {
                upstream_outputs.insert(link.from);
            }
            if output_pins.contains(&link.from) {
                downstream_inputs.push(link.to);
            }
        }

        if !graph.remove_node(node_id) {
            return false;
        }

        if upstream_outputs.len() == 1 && !downstream_inputs.is_empty() {
            let from = *upstream_outputs.iter().next().unwrap();
            for to in downstream_inputs {
                let _ = graph.add_link(from, to);
            }
        }
        if let Some(snarl_id) = self.core_to_snarl.remove(&node_id) {
            self.snarl_to_core.remove(&snarl_id);
            let _ = self.snarl.remove_node(snarl_id);
        }
        if self.selected_node.as_ref() == Some(&node_id) {
            self.selected_node = None;
        }
        self.needs_wire_sync = true;
        true
    }

    pub fn node_at_screen_pos(&self, pos: Pos2) -> Option<NodeId> {
        let pos = if self.graph_transform.valid {
            self.graph_transform.to_global.inverse() * pos
        } else {
            pos
        };
        let snarl_node = self.node_at_pos(pos)?;
        self.snarl_to_core.get(&snarl_node).copied()
    }

    pub fn take_info_request(&mut self) -> Option<NodeInfoRequest> {
        self.info_request.take()
    }

    pub fn take_wrangle_help_request(&mut self) -> Option<Pos2> {
        self.wrangle_help_request.take()
    }

    pub fn zoom_at(&mut self, screen_pos: Pos2, scroll_delta: f32) {
        if scroll_delta.abs() <= 0.0 {
            return;
        }
        let base = if self.graph_transform.valid {
            self.graph_transform.to_global
        } else {
            egui::emath::TSTransform::IDENTITY
        };
        let zoom = 1.0 + (scroll_delta * 0.2 / 100.0);
        let next_scale = (base.scaling * zoom).clamp(0.1, 4.0);
        let graph_pos = base.inverse() * screen_pos;
        let translation = screen_pos.to_vec2() - graph_pos.to_vec2() * next_scale;
        self.pending_transform = Some(egui::emath::TSTransform::new(translation, next_scale));
    }

    pub fn fit_to_rect(&mut self, panel_rect: Rect) {
        if self.node_ui_rects.is_empty() {
            return;
        }
        let mut min = Pos2::new(f32::INFINITY, f32::INFINITY);
        let mut max = Pos2::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
        for rect in self.node_ui_rects.values() {
            min.x = min.x.min(rect.min.x);
            min.y = min.y.min(rect.min.y);
            max.x = max.x.max(rect.max.x);
            max.y = max.y.max(rect.max.y);
        }
        if !min.is_finite() || !max.is_finite() {
            return;
        }
        let bounds = Rect::from_min_max(min, max);
        let size = bounds.size().max(vec2(1.0, 1.0));
        let padded = panel_rect.shrink(32.0);
        if !padded.is_positive() {
            return;
        }
        let scale_x = padded.width() / size.x;
        let scale_y = padded.height() / size.y;
        let scale = scale_x.min(scale_y).clamp(0.1, 4.0);
        let translation = padded.center().to_vec2() - bounds.center().to_vec2() * scale;
        self.pending_transform = Some(egui::emath::TSTransform::new(translation, scale));
    }

    pub fn progress_sink(&self) -> ProgressSink {
        let shared = Arc::clone(&self.progress_state);
        Arc::new(move |event| {
            if let Ok(mut state) = shared.lock() {
                state.on_event(event);
            }
        })
    }

    fn progress_snapshot(&self) -> HashMap<NodeId, NodeProgressView> {
        let now = Instant::now();
        if let Ok(mut state) = self.progress_state.lock() {
            state.snapshot(now)
        } else {
            HashMap::new()
        }
    }
}

impl NodeProgressState {
    fn on_event(&mut self, event: ProgressEvent) {
        let now = Instant::now();
        match event {
            ProgressEvent::Start { node } => {
                self.entries.insert(
                    node,
                    NodeProgressEntry {
                        fraction: 0.0,
                        started_at: now,
                        last_update: now,
                        finished_at: None,
                    },
                );
            }
            ProgressEvent::Advance { node, fraction } => {
                let entry = self.entries.entry(node).or_insert(NodeProgressEntry {
                    fraction: 0.0,
                    started_at: now,
                    last_update: now,
                    finished_at: None,
                });
                entry.fraction = fraction.clamp(0.0, 1.0);
                entry.last_update = now;
            }
            ProgressEvent::Finish { node } => {
                let entry = self.entries.entry(node).or_insert(NodeProgressEntry {
                    fraction: 0.0,
                    started_at: now,
                    last_update: now,
                    finished_at: None,
                });
                entry.fraction = 1.0;
                entry.last_update = now;
                entry.finished_at = Some(now);
            }
        }
    }

    fn snapshot(&mut self, now: Instant) -> HashMap<NodeId, NodeProgressView> {
        const SHOW_DELAY_SECS: f32 = 1.0;
        const SHOW_AFTER_FINISH_SECS: f32 = 0.6;
        let mut result = HashMap::new();
        self.entries.retain(|node_id, entry| {
            let elapsed = now.duration_since(entry.started_at).as_secs_f32();
            if elapsed < SHOW_DELAY_SECS {
                return true;
            }
            let active = entry.finished_at.is_none();
            if let Some(done_at) = entry.finished_at {
                if now.duration_since(done_at).as_secs_f32() > SHOW_AFTER_FINISH_SECS {
                    return false;
                }
            }
            result.insert(
                *node_id,
                NodeProgressView {
                    fraction: entry.fraction,
                    active,
                },
            );
            true
        });
        result
    }
}

pub(super) enum PendingWire {
    FromOutputs(Vec<OutPinId>),
    FromInputs(Vec<InPinId>),
}
