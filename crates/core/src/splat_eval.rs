use std::collections::BTreeMap;

use crate::eval::{evaluate_from_with, EvalReport, EvalState};
use crate::graph::{Graph, GraphError, NodeId};
use crate::nodes_builtin::{builtin_kind_from_name, compute_splat_node};
use crate::splat::SplatGeo;

#[derive(Debug, Default)]
pub struct SplatEvalState {
    pub eval: EvalState,
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
    let outputs = &mut state.outputs;
    let report = evaluate_from_with(graph, output, &mut state.eval, |node_id, params| {
        let node = graph
            .node(node_id)
            .ok_or_else(|| "missing node".to_string())?;
        let kind = builtin_kind_from_name(&node.name)
            .ok_or_else(|| format!("unknown node type {}", node.name))?;

        let mut input_splats = Vec::with_capacity(node.inputs.len());
        for pin_id in &node.inputs {
            let link = graph.links().find(|link| link.to == *pin_id);
            let splat = if let Some(link) = link {
                let from_pin = graph
                    .pin(link.from)
                    .ok_or_else(|| "missing upstream pin".to_string())?;
                let upstream_id = from_pin.node;
                let splat = outputs
                    .get(&upstream_id)
                    .ok_or_else(|| format!("missing upstream output {:?}", upstream_id))?;
                Some(splat.clone())
            } else {
                None
            };
            input_splats.push(splat);
        }

        let inputs: Vec<SplatGeo> = input_splats.into_iter().flatten().collect();
        let splats = compute_splat_node(kind, params, &inputs)?;
        outputs.insert(node_id, splats);
        Ok(())
    })?;

    if !report.output_valid {
        for err in &report.errors {
            match err {
                crate::eval::EvalError::Node { node, .. } => {
                    outputs.remove(node);
                }
                crate::eval::EvalError::Upstream { node, upstream } => {
                    outputs.remove(node);
                    for upstream_node in upstream {
                        outputs.remove(upstream_node);
                    }
                }
            }
        }
        outputs.remove(&output);
        return Ok(SplatEvalResult {
            report,
            output: None,
        });
    }

    let output_splats = outputs.get(&output).cloned();
    Ok(SplatEvalResult {
        report,
        output: output_splats,
    })
}
