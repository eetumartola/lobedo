use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use egui::{vec2, Color32, Frame, Pos2, Rect, Stroke, Ui};
use egui_snarl::ui::{BackgroundPattern, SnarlStyle};
use egui_snarl::{InPinId, OutPinId, Snarl};

use lobedo_core::{Graph, NodeId};

use super::viewer::NodeGraphViewer;

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
    pub(super) add_menu_open: bool,
    pub(super) add_menu_screen_pos: Pos2,
    pub(super) add_menu_graph_pos: Pos2,
    pub(super) add_menu_filter: String,
    pub(super) add_menu_focus: bool,
    pub(super) pending_wire: Option<PendingWire>,
    pub(super) info_request: Option<NodeInfoRequest>,
    pub(super) wrangle_help_request: Option<Pos2>,
    pub(super) graph_transform: GraphTransformState,
    pub(super) pending_transform: Option<egui::emath::TSTransform>,
    pub(super) input_pin_positions: Rc<RefCell<HashMap<InPinId, Pos2>>>,
    pub(super) output_pin_positions: Rc<RefCell<HashMap<OutPinId, Pos2>>>,
    pub(super) error_nodes: HashSet<NodeId>,
    pub(super) error_messages: HashMap<NodeId, String>,
    pub(super) node_menu_request: Option<NodeMenuRequest>,
    pub(super) node_menu_open: bool,
    pub(super) node_menu_screen_pos: Pos2,
    pub(super) node_menu_node: Option<NodeId>,
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
            add_menu_open: false,
            add_menu_screen_pos: Pos2::new(0.0, 0.0),
            add_menu_graph_pos: Pos2::new(0.0, 0.0),
            add_menu_filter: String::new(),
            add_menu_focus: false,
            pending_wire: None,
            info_request: None,
            wrangle_help_request: None,
            graph_transform: GraphTransformState {
                to_global: egui::emath::TSTransform::IDENTITY,
                valid: false,
            },
            pending_transform: None,
            input_pin_positions: Rc::new(RefCell::new(HashMap::new())),
            output_pin_positions: Rc::new(RefCell::new(HashMap::new())),
            error_nodes: HashSet::new(),
            error_messages: HashMap::new(),
            node_menu_request: None,
            node_menu_open: false,
            node_menu_screen_pos: Pos2::new(0.0, 0.0),
            node_menu_node: None,
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
    pub(super) display: Rect,
    pub(super) template: Rect,
    pub(super) help: Option<Rect>,
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

    pub fn show(&mut self, ui: &mut Ui, graph: &mut Graph, eval_dirty: &mut bool) {
        self.ensure_nodes(graph);
        if self.needs_wire_sync {
            self.sync_wires(graph);
            self.needs_wire_sync = false;
        }

        self.prev_node_ui_rects = std::mem::take(&mut self.node_ui_rects);
        self.node_ui_rects.clear();
        self.header_button_rects.clear();
        self.input_pin_positions.borrow_mut().clear();
        self.output_pin_positions.borrow_mut().clear();
        self.last_changed = false;
        self.layout_changed = false;

        let mut viewer = NodeGraphViewer {
            graph,
            core_to_snarl: &mut self.core_to_snarl,
            snarl_to_core: &mut self.snarl_to_core,
            next_pos: &mut self.next_pos,
            selected_node: &mut self.selected_node,
            node_rects: &mut self.node_ui_rects,
            header_button_rects: &mut self.header_button_rects,
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
        for (node, rect) in &self.node_ui_rects {
            let Some(prev) = self.prev_node_ui_rects.get(node) else {
                continue;
            };
            if (rect.center() - prev.center()).length_sq() > 0.5 {
                self.layout_changed = true;
                break;
            }
        }

        if viewer_changed {
            if !self.layout_changed {
                *eval_dirty = true;
                self.needs_wire_sync = true;
            }
        }

        if self.add_menu_open && self.show_add_menu(ui, graph) {
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

    pub fn set_error_state(&mut self, nodes: HashSet<NodeId>, messages: HashMap<NodeId, String>) {
        self.error_nodes = nodes;
        self.error_messages = messages;
    }

    pub fn selected_node_id(&self) -> Option<NodeId> {
        self.selected_node
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
        let zoom = 1.0 - (scroll_delta * 0.1 / 100.0);
        let next_scale = (base.scaling * zoom).clamp(0.1, 4.0);
        let graph_pos = base.inverse() * screen_pos;
        let translation = screen_pos.to_vec2() - graph_pos.to_vec2() * next_scale;
        self.pending_transform = Some(egui::emath::TSTransform::new(translation, next_scale));
    }

    pub fn fit_to_rect(&mut self, panel_rect: Rect) {
        if self.node_ui_rects.is_empty() {
            return;
        }
        let base = if self.graph_transform.valid {
            self.graph_transform.to_global
        } else {
            egui::emath::TSTransform::IDENTITY
        };
        let inv = base.inverse();
        let mut min = Pos2::new(f32::INFINITY, f32::INFINITY);
        let mut max = Pos2::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
        for rect in self.node_ui_rects.values() {
            let graph_min = inv * rect.min;
            let graph_max = inv * rect.max;
            min.x = min.x.min(graph_min.x);
            min.y = min.y.min(graph_min.y);
            max.x = max.x.max(graph_max.x);
            max.y = max.y.max(graph_max.y);
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
}

pub(super) enum PendingWire {
    FromOutputs(Vec<OutPinId>),
    FromInputs(Vec<InPinId>),
}
