use std::collections::BTreeMap;

use crate::eval::{evaluate_from_with, EvalReport, EvalState};
use crate::geometry::Geometry;
use crate::graph::{Graph, GraphError, NodeId};
use crate::nodes_builtin::{builtin_kind_from_name, compute_geometry_node, BuiltinNodeKind};

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
    let outputs = &mut state.outputs;
    let report = evaluate_from_with(graph, output, &mut state.eval, |node_id, params| {
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

        let inputs = match kind {
            BuiltinNodeKind::Transform
            | BuiltinNodeKind::CopyTransform
            | BuiltinNodeKind::Delete
            | BuiltinNodeKind::Prune
            | BuiltinNodeKind::Regularize
            | BuiltinNodeKind::Group
            | BuiltinNodeKind::Normal
            | BuiltinNodeKind::Scatter
            | BuiltinNodeKind::Color
            | BuiltinNodeKind::Noise
            | BuiltinNodeKind::AttributeMath
            | BuiltinNodeKind::Wrangle
            | BuiltinNodeKind::ObjOutput
            | BuiltinNodeKind::Output => {
                if let Some(geometry) = input_geometries.first().and_then(|geo| geo.clone()) {
                    vec![geometry]
                } else {
                    let name = input_names
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "in".to_string());
                    return Err(format!("missing input '{}'", name));
                }
            }
            BuiltinNodeKind::CopyToPoints => {
                let source = input_geometries.first().and_then(|geo| geo.clone());
                let template = input_geometries.get(1).and_then(|geo| geo.clone());
                if source.is_none() {
                    let name = input_names
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "source".to_string());
                    return Err(format!("missing input '{}'", name));
                }
                if template.is_none() {
                    let name = input_names
                        .get(1)
                        .cloned()
                        .unwrap_or_else(|| "template".to_string());
                    return Err(format!("missing input '{}'", name));
                }
                vec![source.unwrap(), template.unwrap()]
            }
            BuiltinNodeKind::Merge => input_geometries.into_iter().flatten().collect(),
            _ => Vec::new(),
        };

        if matches!(kind, BuiltinNodeKind::Merge) && inputs.is_empty() {
            return Err("Merge requires at least one geometry input".to_string());
        }

        let geometry = compute_geometry_node(kind, params, &inputs)?;
        outputs.insert(node_id, geometry);
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
