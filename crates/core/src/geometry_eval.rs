use std::collections::BTreeMap;

use crate::eval::{evaluate_from_with_progress, EvalReport, EvalState};
use crate::geometry::Geometry;
use crate::graph::{Graph, GraphError, NodeId};
use crate::nodes_builtin::{builtin_kind_from_name, compute_geometry_node, input_policy, InputPolicy};
use crate::progress::ProgressSink;

#[derive(Debug, Default)]
pub struct GeometryEvalState {
    pub eval: EvalState,
    outputs: BTreeMap<NodeId, Geometry>,
}

#[derive(Debug)]
pub struct GeometryEvalResult {
    pub report: EvalReport,
    pub output: Option<Geometry>,
}

impl GeometryEvalState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn geometry_for_node(&self, node_id: NodeId) -> Option<&Geometry> {
        self.outputs.get(&node_id)
    }
}

pub fn evaluate_geometry_graph(
    graph: &Graph,
    output: NodeId,
    state: &mut GeometryEvalState,
) -> Result<GeometryEvalResult, GraphError> {
    evaluate_geometry_graph_with_progress(graph, output, state, None)
}

pub fn evaluate_geometry_graph_with_progress(
    graph: &Graph,
    output: NodeId,
    state: &mut GeometryEvalState,
    progress: Option<ProgressSink>,
) -> Result<GeometryEvalResult, GraphError> {
    let outputs = &mut state.outputs;
    let report = evaluate_from_with_progress(
        graph,
        output,
        &mut state.eval,
        progress,
        |node_id, params| {
        let node = graph
            .node(node_id)
            .ok_or_else(|| "missing node".to_string())?;
        let kind = builtin_kind_from_name(&node.name)
            .ok_or_else(|| format!("unknown node type {}", node.name))?;

        let mut input_geometries = Vec::with_capacity(node.inputs.len());
        let mut input_names = Vec::with_capacity(node.inputs.len());
        for pin_id in &node.inputs {
            let pin = graph
                .pin(*pin_id)
                .ok_or_else(|| "missing input pin".to_string())?;
            input_names.push(pin.name.clone());
            let link = graph.links().find(|link| link.to == *pin_id);
            let geometry = if let Some(link) = link {
                let from_pin = graph
                    .pin(link.from)
                    .ok_or_else(|| "missing upstream pin".to_string())?;
                let upstream_id = from_pin.node;
                let geometry = outputs
                    .get(&upstream_id)
                    .ok_or_else(|| format!("missing upstream output {:?}", upstream_id))?;
                Some(geometry.clone())
            } else {
                None
            };
            input_geometries.push(geometry);
        }

        if node.bypass {
            let geometry = input_geometries
                .first()
                .cloned()
                .flatten()
                .unwrap_or_default();
            outputs.insert(node_id, geometry);
            return Ok(());
        }

        let inputs = match input_policy(kind) {
            InputPolicy::None => Vec::new(),
            InputPolicy::RequireAll => {
                let mut inputs = Vec::with_capacity(input_geometries.len());
                for (idx, geometry) in input_geometries.into_iter().enumerate() {
                    let Some(geometry) = geometry else {
                        let name = input_names
                            .get(idx)
                            .cloned()
                            .unwrap_or_else(|| "in".to_string());
                        return Err(format!("missing input '{}'", name));
                    };
                    inputs.push(geometry);
                }
                inputs
            }
            InputPolicy::RequireAtLeast(min) => {
                let inputs: Vec<Geometry> = input_geometries.into_iter().flatten().collect();
                if inputs.len() < min {
                    let suffix = if min == 1 { "" } else { "s" };
                    return Err(format!(
                        "{} requires at least {} input{}",
                        kind.name(),
                        min,
                        suffix
                    ));
                }
                inputs
            }
        };

        let geometry = compute_geometry_node(kind, params, &inputs)?;
        outputs.insert(node_id, geometry);
        Ok(())
    },
    )?;

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
        return Ok(GeometryEvalResult {
            report,
            output: None,
        });
    }

    let output_geometry = outputs.get(&output).cloned();
    Ok(GeometryEvalResult {
        report,
        output: output_geometry,
    })
}
