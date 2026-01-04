use std::collections::{HashMap, HashSet};

use lobedo_core::Graph;

use super::state::{NodeGraphLayout, NodeGraphState, SnarlNode};

impl NodeGraphState {
    pub fn layout_snapshot(&self) -> NodeGraphLayout {
        let mut positions = HashMap::new();
        for (_, pos, node) in self.snarl.nodes_pos_ids() {
            positions.insert(node.core_id, pos);
        }
        NodeGraphLayout {
            positions,
            selected: self.selected_node,
        }
    }

    pub fn restore_layout(&mut self, graph: &Graph, layout: &NodeGraphLayout) {
        self.reset();
        for node in graph.nodes() {
            let pos = layout
                .positions
                .get(&node.id)
                .copied()
                .unwrap_or(self.next_pos);
            let snarl_id = self.snarl.insert_node(pos, SnarlNode { core_id: node.id });
            self.core_to_snarl.insert(node.id, snarl_id);
            self.snarl_to_core.insert(snarl_id, node.id);
            self.advance_pos();
        }
        self.selected_node = layout
            .selected
            .filter(|selected| graph.node(*selected).is_some());
        self.needs_wire_sync = true;
    }

    pub(super) fn ensure_nodes(&mut self, graph: &Graph) {
        for node in graph.nodes() {
            if self.core_to_snarl.contains_key(&node.id) {
                continue;
            }

            let pos = self.next_pos;
            let snarl_id = self.snarl.insert_node(pos, SnarlNode { core_id: node.id });
            self.core_to_snarl.insert(node.id, snarl_id);
            self.snarl_to_core.insert(snarl_id, node.id);
            self.advance_pos();
            self.needs_wire_sync = true;
        }

        let mut to_remove = Vec::new();
        for (snarl_id, core_id) in &self.snarl_to_core {
            if graph.node(*core_id).is_none() {
                to_remove.push(*snarl_id);
            }
        }

        for snarl_id in to_remove {
            if let Some(core_id) = self.snarl_to_core.remove(&snarl_id) {
                self.core_to_snarl.remove(&core_id);
            }
            let _ = self.snarl.remove_node(snarl_id);
            self.needs_wire_sync = true;
        }

        if let Some(selected) = self.selected_node {
            if graph.node(selected).is_none() {
                self.selected_node = None;
            }
        }
    }

    pub(super) fn sync_wires(&mut self, graph: &Graph) {
        let mut desired = HashSet::new();
        for link in graph.links() {
            if let Some((out_pin, in_pin)) = self.snarl_link_for_core(graph, link.from, link.to) {
                desired.insert((out_pin, in_pin));
            }
        }

        let existing: Vec<_> = self.snarl.wires().collect();
        for (out_pin, in_pin) in existing {
            if !desired.contains(&(out_pin, in_pin)) {
                let _ = self.snarl.disconnect(out_pin, in_pin);
            }
        }

        for (out_pin, in_pin) in desired {
            let _ = self.snarl.connect(out_pin, in_pin);
        }
    }

    fn snarl_link_for_core(
        &self,
        graph: &Graph,
        from: lobedo_core::PinId,
        to: lobedo_core::PinId,
    ) -> Option<(egui_snarl::OutPinId, egui_snarl::InPinId)> {
        let from_pin = graph.pin(from)?;
        let to_pin = graph.pin(to)?;
        if from_pin.kind != lobedo_core::PinKind::Output || to_pin.kind != lobedo_core::PinKind::Input
        {
            return None;
        }

        let from_node = graph.node(from_pin.node)?;
        let to_node = graph.node(to_pin.node)?;
        let from_index = from_node.outputs.iter().position(|id| *id == from)?;
        let to_index = to_node.inputs.iter().position(|id| *id == to)?;

        let snarl_from = *self.core_to_snarl.get(&from_pin.node)?;
        let snarl_to = *self.core_to_snarl.get(&to_pin.node)?;

        Some((
            egui_snarl::OutPinId {
                node: snarl_from,
                output: from_index,
            },
            egui_snarl::InPinId {
                node: snarl_to,
                input: to_index,
            },
        ))
    }

    fn advance_pos(&mut self) {
        self.next_pos.x += 240.0;
        if self.next_pos.x > 1000.0 {
            self.next_pos.x = 0.0;
            self.next_pos.y += 200.0;
        }
    }
}
