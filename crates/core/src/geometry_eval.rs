use std::collections::BTreeMap;

use crate::eval::{evaluate_from_with_progress, EvalReport, EvalState};
use crate::geometry::Geometry;
use crate::graph::{Graph, GraphError, NodeId, PinId, PinType};
use crate::image_data::ImageData;
use crate::nodes_builtin::{
    builtin_kind_from_id, builtin_kind_from_name, compute_geometry_node, input_policy,
    BuiltinNodeKind, InputPolicy,
};
use crate::progress::ProgressSink;

#[derive(Debug, Default)]
pub struct GeometryEvalState {
    pub eval: EvalState,
    outputs: BTreeMap<NodeId, Geometry>,
    image_outputs: BTreeMap<PinId, ImageData>,
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

    pub fn image_for_pin(&self, pin_id: PinId) -> Option<&ImageData> {
        self.image_outputs.get(&pin_id)
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
    let image_outputs = &mut state.image_outputs;
    let report = evaluate_from_with_progress(
        graph,
        output,
        &mut state.eval,
        progress,
        |node_id, params| {
        let node = graph
            .node(node_id)
            .ok_or_else(|| "missing node".to_string())?;
        let kind = if node.kind_id.is_empty() {
            builtin_kind_from_name(&node.name)
        } else {
            builtin_kind_from_id(&node.kind_id)
                .or_else(|| builtin_kind_from_name(&node.name))
        }
        .ok_or_else(|| {
            if node.kind_id.is_empty() {
                format!("unknown node type {}", node.name)
            } else {
                format!("unknown node kind {} ({})", node.kind_id, node.name)
            }
        })?;

        let mut input_geometries = Vec::new();
        let mut input_images = Vec::new();
        let mut input_names = Vec::with_capacity(node.inputs.len());
        for pin_id in &node.inputs {
            let pin = graph
                .pin(*pin_id)
                .ok_or_else(|| "missing input pin".to_string())?;
            input_names.push(pin.name.clone());
            let link = graph.input_link(*pin_id);
            match pin.pin_type {
                PinType::Image => {
                    let image = if let Some(link) = link {
                        image_outputs.get(&link.from).cloned()
                    } else {
                        None
                    };
                    input_images.push(image);
                }
                _ => {
                    let geometry = if let Some(link) = link {
                        let from_pin = graph
                            .pin(link.from)
                            .ok_or_else(|| "missing upstream pin".to_string())?;
                        let upstream_id = from_pin.node;
                        let geometry = outputs
                            .get(&upstream_id)
                            .ok_or_else(|| format!("missing upstream output {upstream_id:?}"))?;
                        Some(geometry.clone())
                    } else {
                        None
                    };
                    input_geometries.push(geometry);
                }
            }
        }

        if node.bypass {
            match kind {
                BuiltinNodeKind::DepthImage => {
                    let Some(input) = input_images.first().cloned().flatten() else {
                        return Err("Depth Image requires an image input".to_string());
                    };
                    let width = input.width();
                    let height = input.height();
                    let depth = ImageData::from_depth(
                        width,
                        height,
                        vec![0.0f32; (width * height) as usize],
                    )
                    .map_err(|err| err.to_string())?;
                    let seg = ImageData::from_seg(
                        width,
                        height,
                        vec![0u32; (width * height) as usize],
                    )
                    .map_err(|err| err.to_string())?;
                    let out_images = [input, depth, seg];
                    for (idx, pin_id) in node.outputs.iter().enumerate() {
                        if let Some(pin) = graph.pin(*pin_id) {
                            if pin.pin_type == PinType::Image {
                                if let Some(image) = out_images.get(idx).cloned() {
                                    image_outputs.insert(*pin_id, image);
                                }
                            }
                        }
                    }
                }
                BuiltinNodeKind::ImagePreview => {
                    outputs.insert(node_id, Geometry::default());
                }
                BuiltinNodeKind::DepthToSplats => {
                    outputs.insert(node_id, Geometry::default());
                }
                _ => {
                    let geometry = input_geometries
                        .first()
                        .cloned()
                        .flatten()
                        .unwrap_or_default();
                    outputs.insert(node_id, geometry);
                }
            }
            return Ok(());
        }

        match kind {
            BuiltinNodeKind::Image => {
                let image = crate::nodes::image::compute(params)?;
                for pin_id in &node.outputs {
                    if let Some(pin) = graph.pin(*pin_id) {
                        if pin.pin_type == PinType::Image {
                            image_outputs.insert(*pin_id, image.clone());
                        }
                    }
                }
            }
            BuiltinNodeKind::DepthImage => {
                let Some(input) = input_images.first().cloned().flatten() else {
                    return Err("Depth Image requires an image input".to_string());
                };
                let (color, depth, seg) = crate::nodes::depth_image::compute(params, &input)?;
                let out_images = [color, depth, seg];
                for (idx, pin_id) in node.outputs.iter().enumerate() {
                    if let Some(pin) = graph.pin(*pin_id) {
                        if pin.pin_type == PinType::Image {
                            if let Some(image) = out_images.get(idx).cloned() {
                                image_outputs.insert(*pin_id, image);
                            }
                        }
                    }
                }
            }
            BuiltinNodeKind::DepthToSplats => {
                let mut images = input_images.into_iter();
                let color = images
                    .next()
                    .flatten()
                    .ok_or_else(|| "Depth to Splats requires color input".to_string())?;
                let depth = images
                    .next()
                    .flatten()
                    .ok_or_else(|| "Depth to Splats requires depth input".to_string())?;
                let seg = images.next().flatten();
                let geometry = crate::nodes::depth_to_splats::compute(
                    params,
                    &color,
                    &depth,
                    seg.as_ref(),
                )?;
                outputs.insert(node_id, geometry);
            }
            BuiltinNodeKind::ImagePreview => {
                let image = input_images
                    .first()
                    .cloned()
                    .flatten()
                    .ok_or_else(|| "Image Preview requires an image input".to_string())?;
                let geometry = crate::nodes::image_preview::compute(params, &image)?;
                outputs.insert(node_id, geometry);
            }
            _ => {
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
                                return Err(format!("missing input '{name}'"));
                            };
                            inputs.push(geometry);
                        }
                        inputs
                    }
                    InputPolicy::RequireAtLeast(min) => {
                        let inputs: Vec<Geometry> =
                            input_geometries.into_iter().flatten().collect();
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
            }
        }
        Ok(())
    },
    )?;

    if !report.output_valid {
        for err in &report.errors {
            match err {
                crate::eval::EvalError::Node { node, .. } => {
                    outputs.remove(node);
                    if let Some(node) = graph.node(*node) {
                        for pin_id in &node.outputs {
                            image_outputs.remove(pin_id);
                        }
                    }
                }
                crate::eval::EvalError::Upstream { node, upstream } => {
                    outputs.remove(node);
                    if let Some(node) = graph.node(*node) {
                        for pin_id in &node.outputs {
                            image_outputs.remove(pin_id);
                        }
                    }
                    for upstream_node in upstream {
                        outputs.remove(upstream_node);
                        if let Some(node) = graph.node(*upstream_node) {
                            for pin_id in &node.outputs {
                                image_outputs.remove(pin_id);
                            }
                        }
                    }
                }
            }
        }
        outputs.remove(&output);
        if let Some(node) = graph.node(output) {
            for pin_id in &node.outputs {
                image_outputs.remove(pin_id);
            }
        }
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
