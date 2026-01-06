use egui::{Pos2, Ui};
use egui_snarl::{InPinId, OutPinId};

use lobedo_core::{Graph, NodeId, PinId};

use super::state::{NodeGraphState, PendingWire};
use super::utils::{
    core_input_pin, core_output_pin, find_input_of_type, find_output_of_type,
    point_snarl_wire_distance,
};

impl NodeGraphState {
    pub(super) fn update_drag_state(&mut self, ui: &Ui) {
        let pressed = ui.input(|i| i.pointer.any_pressed());
        let pointer_down = ui.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
        if pressed {
            if let Some(pos) = ui.input(|i| i.pointer.press_origin()) {
                if let Some(node) = self.node_at_pos(pos) {
                    self.dragging_node = Some(node);
                }
            }
        }
        if pointer_down {
            if let Some(node) = self.find_moved_node() {
                self.dragging_node = Some(node);
            }
        } else if !ui.input(|i| i.pointer.any_released()) {
            self.dragging_node = None;
        }
    }

    pub(super) fn handle_drop_on_wire(&mut self, ui: &Ui, graph: &mut Graph) -> bool {
        if !ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary)) {
            return false;
        }
        let Some(drop_pos) =
            ui.input(|i| i.pointer.latest_pos().or_else(|| i.pointer.interact_pos()))
        else {
            return false;
        };
        let moved_node = self.dragging_node.or_else(|| self.find_moved_node());
        if moved_node.is_none() {
            return false;
        }
        let moved_node = moved_node.unwrap();
        self.dragging_node = None;
        let (wire_hit, _wire_dist) = self.find_wire_hit_with_dist(graph, drop_pos);
        let Some((out_pin, in_pin)) = wire_hit else {
            return false;
        };
        if out_pin.node == moved_node || in_pin.node == moved_node {
            return false;
        }
        self.insert_node_between_wire(graph, moved_node, out_pin, in_pin)
    }

    pub(super) fn find_moved_node(&self) -> Option<egui_snarl::NodeId> {
        let mut best = None;
        let mut best_dist = 0.0;
        for (node, rect) in &self.node_ui_rects {
            let Some(prev) = self.prev_node_ui_rects.get(node) else {
                continue;
            };
            let delta = rect.center() - prev.center();
            let dist = delta.length_sq();
            if dist > best_dist {
                best_dist = dist;
                best = Some(*node);
            }
        }
        if best_dist > 0.5 {
            best
        } else {
            None
        }
    }

    pub(super) fn node_at_pos(&self, pos: Pos2) -> Option<egui_snarl::NodeId> {
        let mut best = None;
        let mut best_area = f32::MAX;
        for (node, rect) in &self.node_ui_rects {
            if rect.contains(pos) {
                let area = rect.width() * rect.height();
                if area < best_area {
                    best_area = area;
                    best = Some(*node);
                }
            }
        }
        best
    }

    fn find_wire_hit_with_dist(
        &self,
        graph: &Graph,
        pos: Pos2,
    ) -> (Option<(OutPinId, InPinId)>, f32) {
        let mut best = None;
        let mut best_dist = f32::MAX;
        let threshold = 28.0;
        for (out_pin, in_pin) in self.snarl.wires() {
            let Some(out_pos) = self.pin_pos_for_output(graph, out_pin) else {
                continue;
            };
            let Some(in_pos) = self.pin_pos_for_input(graph, in_pin) else {
                continue;
            };
            let dist = point_snarl_wire_distance(pos, out_pos, in_pos);
            if dist < threshold && dist < best_dist {
                best = Some((out_pin, in_pin));
                best_dist = dist;
            }
        }
        (best, best_dist)
    }

    fn pin_pos_for_output(&self, graph: &Graph, pin: OutPinId) -> Option<Pos2> {
        if let Some(pos) = self.output_pin_positions.borrow().get(&pin).copied() {
            return Some(pos);
        }
        let rect = self.node_ui_rects.get(&pin.node)?;
        let core_id = *self.snarl_to_core.get(&pin.node)?;
        let node = graph.node(core_id)?;
        let count = node.outputs.len().max(1);
        let t = (pin.output as f32 + 1.0) / (count as f32 + 1.0);
        Some(Pos2::new(rect.right(), rect.top() + rect.height() * t))
    }

    fn pin_pos_for_input(&self, graph: &Graph, pin: InPinId) -> Option<Pos2> {
        if let Some(pos) = self.input_pin_positions.borrow().get(&pin).copied() {
            return Some(pos);
        }
        let rect = self.node_ui_rects.get(&pin.node)?;
        let core_id = *self.snarl_to_core.get(&pin.node)?;
        let node = graph.node(core_id)?;
        let count = node.inputs.len().max(1);
        let t = (pin.input as f32 + 1.0) / (count as f32 + 1.0);
        Some(Pos2::new(rect.left(), rect.top() + rect.height() * t))
    }

    fn insert_node_between_wire(
        &mut self,
        graph: &mut Graph,
        node: egui_snarl::NodeId,
        out_pin: OutPinId,
        in_pin: InPinId,
    ) -> bool {
        let Some(core_out) = self.core_pin_for_output(graph, out_pin) else {
            return false;
        };
        let Some(core_in) = self.core_pin_for_input(graph, in_pin) else {
            return false;
        };
        let Some(core_node) = self.snarl_to_core.get(&node).copied() else {
            return false;
        };
        let Some(out_pin_data) = graph.pin(core_out) else {
            return false;
        };
        let Some(in_pin_data) = graph.pin(core_in) else {
            return false;
        };
        let Some(node_data) = graph.node(core_node) else {
            return false;
        };
        let Some((new_in_pin, new_in_idx)) =
            find_input_of_type(graph, node_data, out_pin_data.pin_type)
        else {
            return false;
        };
        let Some((new_out_pin, new_out_idx)) =
            find_output_of_type(graph, node_data, in_pin_data.pin_type)
        else {
            return false;
        };

        let _ = graph.remove_link_between(core_out, core_in);
        let _ = self.snarl.disconnect(out_pin, in_pin);

        let new_in_snarl = InPinId {
            node,
            input: new_in_idx,
        };
        let new_out_snarl = OutPinId {
            node,
            output: new_out_idx,
        };

        if graph.add_link(core_out, new_in_pin).is_err() {
            let _ = graph.remove_links_for_pin(new_in_pin);
            self.snarl.drop_inputs(new_in_snarl);
            let _ = graph.add_link(core_out, new_in_pin);
        }
        let _ = self.snarl.connect(out_pin, new_in_snarl);

        if graph.add_link(new_out_pin, core_in).is_err() {
            let _ = graph.remove_links_for_pin(core_in);
            self.snarl.drop_inputs(in_pin);
            let _ = graph.add_link(new_out_pin, core_in);
        }
        let _ = self.snarl.connect(new_out_snarl, in_pin);
        true
    }

    pub(super) fn connect_pending_wire(
        &mut self,
        graph: &mut Graph,
        new_node: NodeId,
        pending: PendingWire,
    ) -> bool {
        let Some(snarl_node) = self.core_to_snarl.get(&new_node).copied() else {
            return false;
        };
        let Some(node_data) = graph.node(new_node) else {
            return false;
        };
        match pending {
            PendingWire::FromOutputs(out_pins) => {
                for out_pin in out_pins {
                    let Some(core_out) = self.core_pin_for_output(graph, out_pin) else {
                        continue;
                    };
                    let Some(pin_data) = graph.pin(core_out) else {
                        continue;
                    };
                    let Some((new_in_pin, new_in_idx)) =
                        find_input_of_type(graph, node_data, pin_data.pin_type)
                    else {
                        continue;
                    };
                    let new_in_snarl = InPinId {
                        node: snarl_node,
                        input: new_in_idx,
                    };
                    if graph.add_link(core_out, new_in_pin).is_err() {
                        let _ = graph.remove_links_for_pin(new_in_pin);
                        self.snarl.drop_inputs(new_in_snarl);
                        let _ = graph.add_link(core_out, new_in_pin);
                    }
                    let _ = self.snarl.connect(out_pin, new_in_snarl);
                    return true;
                }
                false
            }
            PendingWire::FromInputs(in_pins) => {
                let mut target_type = None;
                for in_pin in &in_pins {
                    if let Some(core_in) = self.core_pin_for_input(graph, *in_pin) {
                        if let Some(pin_data) = graph.pin(core_in) {
                            target_type = Some(pin_data.pin_type);
                            break;
                        }
                    }
                }
                let Some(target_type) = target_type else {
                    return false;
                };
                let Some((new_out_pin, new_out_idx)) =
                    find_output_of_type(graph, node_data, target_type)
                else {
                    return false;
                };
                let new_out_snarl = OutPinId {
                    node: snarl_node,
                    output: new_out_idx,
                };
                let mut connected = false;
                for in_pin in in_pins {
                    let Some(core_in) = self.core_pin_for_input(graph, in_pin) else {
                        continue;
                    };
                    let Some(pin_data) = graph.pin(core_in) else {
                        continue;
                    };
                    if pin_data.pin_type != target_type {
                        continue;
                    }
                    let _ = graph.remove_links_for_pin(core_in);
                    self.snarl.drop_inputs(in_pin);
                    if graph.add_link(new_out_pin, core_in).is_ok() {
                        let _ = self.snarl.connect(new_out_snarl, in_pin);
                        connected = true;
                    }
                }
                connected
            }
        }
    }

    fn core_pin_for_input(&self, graph: &Graph, pin: InPinId) -> Option<PinId> {
        let core_node = self.snarl_to_core.get(&pin.node).copied()?;
        core_input_pin(graph, core_node, pin.input)
    }

    fn core_pin_for_output(&self, graph: &Graph, pin: OutPinId) -> Option<PinId> {
        let core_node = self.snarl_to_core.get(&pin.node).copied()?;
        core_output_pin(graph, core_node, pin.output)
    }
}
