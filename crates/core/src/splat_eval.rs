use std::collections::BTreeMap;

use crate::eval::EvalReport;
use crate::geometry_eval::{evaluate_geometry_graph, GeometryEvalState};
use crate::graph::{Graph, GraphError, NodeId};
use crate::splat::SplatGeo;

#[derive(Debug, Default)]
pub struct SplatEvalState {
    pub geometry: GeometryEvalState,
    outputs: BTreeMap<NodeId, SplatGeo>,
}

#[derive(Debug)]
pub struct SplatEvalResult {
    pub report: EvalReport,
    pub output: Option<SplatGeo>,
}

impl SplatEvalState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn splats_for_node(&self, node_id: NodeId) -> Option<&SplatGeo> {
        self.outputs.get(&node_id)
    }
}

pub fn evaluate_splat_graph(
    graph: &Graph,
    output: NodeId,
    state: &mut SplatEvalState,
) -> Result<SplatEvalResult, GraphError> {
    let result = evaluate_geometry_graph(graph, output, &mut state.geometry)?;
    state.outputs.clear();
    for node in graph.nodes() {
        if let Some(geometry) = state.geometry.geometry_for_node(node.id) {
            if let Some(splats) = geometry.merged_splats() {
                state.outputs.insert(node.id, splats);
            }
        }
    }

    let output_splats = result.output.as_ref().and_then(|geo| geo.merged_splats());
    Ok(SplatEvalResult {
        report: result.report,
        output: output_splats,
    })
}
