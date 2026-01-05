use std::collections::BTreeMap;

use crate::eval::EvalReport;
use crate::geometry_eval::{evaluate_geometry_graph, GeometryEvalState};
use crate::graph::{Graph, GraphError, NodeId};
use crate::mesh::Mesh;

#[derive(Debug, Default)]
pub struct MeshEvalState {
    pub geometry: GeometryEvalState,
    outputs: BTreeMap<NodeId, Mesh>,
}

#[derive(Debug)]
pub struct MeshEvalResult {
    pub report: EvalReport,
    pub output: Option<Mesh>,
}

impl MeshEvalState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mesh_for_node(&self, node_id: NodeId) -> Option<&Mesh> {
        self.outputs.get(&node_id)
    }
}

pub fn evaluate_mesh_graph(
    graph: &Graph,
    output: NodeId,
    state: &mut MeshEvalState,
) -> Result<MeshEvalResult, GraphError> {
    let result = evaluate_geometry_graph(graph, output, &mut state.geometry)?;
    state.outputs.clear();
    for node in graph.nodes() {
        if let Some(geometry) = state.geometry.geometry_for_node(node.id) {
            if let Some(mesh) = geometry.merged_mesh() {
                state.outputs.insert(node.id, mesh);
            }
        }
    }

    let output_mesh = result.output.as_ref().and_then(|geo| geo.merged_mesh());
    Ok(MeshEvalResult {
        report: result.report,
        output: output_mesh,
    })
}
