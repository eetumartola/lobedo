use std::collections::{HashMap, HashSet};
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use lobedo_core::{
    evaluate_geometry_graph, Mesh, SceneDrawable, SceneSnapshot, SceneSplats, ShadingMode,
};
use render::{
    RenderDrawable, RenderMesh, RenderScene, RenderSplats, ViewportDebug, ViewportShadingMode,
};

use super::{DisplayState, LobedoApp};

impl LobedoApp {
    pub(super) fn mark_eval_dirty(&mut self) {
        self.eval_dirty = true;
        self.last_param_change = Some(Instant::now());
    }

    pub(super) fn evaluate_if_needed(&mut self) {
        if !self.eval_dirty {
            return;
        }

        if let Some(last_change) = self.last_param_change {
            let debounce = Duration::from_millis(150);
            if last_change.elapsed() < debounce {
                return;
            }
        }

        self.eval_dirty = false;
        self.last_param_change = None;
        self.evaluate_graph();
    }

    pub(super) fn evaluate_graph(&mut self) {
        let display_node = self.project.graph.display_node();
        let display_node = match display_node {
            None => {
                if self.last_display_state != DisplayState::Missing {
                    tracing::warn!("no display flag set; nothing to evaluate");
                    self.last_display_state = DisplayState::Missing;
                }
                if let Some(renderer) = &self.viewport_renderer {
                    renderer.clear_scene();
                }
                self.pending_scene = None;
                self.node_graph
                    .set_error_state(HashSet::new(), HashMap::new());
                return;
            }
            Some(node) => node,
        };
        self.last_display_state = DisplayState::Ok;
        let template_nodes = self.project.graph.template_nodes();

        let start = Instant::now();
        match evaluate_geometry_graph(&self.project.graph, display_node, &mut self.eval_state) {
            Ok(result) => {
                self.last_eval_ms = Some(start.elapsed().as_secs_f32() * 1000.0);
                let output_valid = result.report.output_valid;
                let mut error_nodes = HashSet::new();
                let mut error_messages = HashMap::new();
                merge_error_state(&result.report, &mut error_nodes, &mut error_messages);
                self.last_eval_report = Some(result.report);

                if let Some(geometry) = result.output {
                    let snapshot = SceneSnapshot::from_geometry(&geometry, [0.7, 0.72, 0.75]);
                    let template_mesh = if output_valid {
                        collect_template_meshes(
                            &self.project.graph,
                            display_node,
                            &template_nodes,
                            &mut self.eval_state,
                            &mut error_nodes,
                            &mut error_messages,
                        )
                    } else {
                        None
                    };
                    let scene = scene_to_render_with_template(&snapshot, template_mesh.as_ref());
                    if let Some(renderer) = &self.viewport_renderer {
                        renderer.set_scene(scene);
                    } else {
                        self.pending_scene = Some(scene);
                    }
                } else {
                    if let Some(renderer) = &self.viewport_renderer {
                        renderer.clear_scene();
                    }
                    self.pending_scene = None;
                }

                if !output_valid {
                    if let Some(renderer) = &self.viewport_renderer {
                        renderer.clear_scene();
                    }
                    self.pending_scene = None;
                }
                self.node_graph.set_error_state(error_nodes, error_messages);
            }
            Err(err) => {
                tracing::error!("eval failed: {:?}", err);
                self.node_graph
                    .set_error_state(HashSet::new(), HashMap::new());
            }
        }
    }

    pub(super) fn viewport_debug(&self) -> ViewportDebug {
        let shading_mode = match self.project.settings.render_debug.shading_mode {
            ShadingMode::Lit => ViewportShadingMode::Lit,
            ShadingMode::Normals => ViewportShadingMode::Normals,
            ShadingMode::Depth => ViewportShadingMode::Depth,
        };
        ViewportDebug {
            show_grid: self.project.settings.render_debug.show_grid,
            show_axes: self.project.settings.render_debug.show_axes,
            show_normals: self.project.settings.render_debug.show_normals,
            show_bounds: self.project.settings.render_debug.show_bounds,
            normal_length: self.project.settings.render_debug.normal_length,
            shading_mode,
            depth_near: self.project.settings.render_debug.depth_near,
            depth_far: self.project.settings.render_debug.depth_far,
            show_points: self.project.settings.render_debug.show_points,
            show_splats: self.project.settings.render_debug.show_splats,
            point_size: self.project.settings.render_debug.point_size,
            key_shadows: self.project.settings.render_debug.key_shadows,
        }
    }
}

pub(super) fn scene_to_render_with_template(
    scene: &SceneSnapshot,
    template: Option<&Mesh>,
) -> RenderScene {
    let mut drawables = Vec::new();
    let mut mesh_has_colors = false;
    for drawable in &scene.drawables {
        match drawable {
            SceneDrawable::Mesh(mesh) => {
                mesh_has_colors |=
                    mesh.colors.is_some() || mesh.corner_colors.is_some();
                drawables.push(RenderDrawable::Mesh(render_mesh_from_scene(mesh)));
            }
            SceneDrawable::Splats(splats) => {
                drawables.push(RenderDrawable::Splats(render_splats_from_scene(splats)));
            }
        }
    }

    let base_color = if mesh_has_colors {
        [1.0, 1.0, 1.0]
    } else {
        scene.base_color
    };

    RenderScene {
        drawables,
        base_color,
        template_mesh: template.map(render_mesh_from_mesh),
    }
}

fn render_mesh_from_scene(mesh: &lobedo_core::SceneMesh) -> RenderMesh {
    RenderMesh {
        positions: mesh.positions.clone(),
        normals: mesh.normals.clone(),
        indices: mesh.indices.clone(),
        corner_normals: mesh.corner_normals.clone(),
        colors: mesh.colors.clone(),
        corner_colors: mesh.corner_colors.clone(),
    }
}

fn render_splats_from_scene(splats: &SceneSplats) -> RenderSplats {
    let mut colors = splats.colors.clone();
    let mut opacity = splats.opacity.clone();
    let mut scales = splats.scales.clone();

    let use_log_opacity = opacity.iter().any(|value| *value < 0.0 || *value > 1.0);
    if use_log_opacity {
        for value in &mut opacity {
            *value = 1.0 / (1.0 + (-*value).exp());
        }
    }

    let use_log_scale = scales
        .iter()
        .any(|value| value[0] < 0.0 || value[1] < 0.0 || value[2] < 0.0);
    if use_log_scale {
        for value in &mut scales {
            *value = [value[0].exp(), value[1].exp(), value[2].exp()];
        }
    }

    let use_sh0_colors = colors
        .iter()
        .any(|value| value[0] < 0.0 || value[1] < 0.0 || value[2] < 0.0);
    if use_sh0_colors {
        const SH_C0: f32 = 0.2820948;
        for value in &mut colors {
            *value = [
                value[0] * SH_C0 + 0.5,
                value[1] * SH_C0 + 0.5,
                value[2] * SH_C0 + 0.5,
            ];
        }
    }

    RenderSplats {
        positions: splats.positions.clone(),
        colors,
        opacity,
        scales,
        rotations: splats.rotations.clone(),
    }
}

fn render_mesh_from_mesh(mesh: &Mesh) -> RenderMesh {
    let snapshot = SceneSnapshot::from_mesh(mesh, [0.7, 0.72, 0.75]);
    let mesh = snapshot
        .mesh()
        .expect("mesh snapshot missing mesh");
    render_mesh_from_scene(mesh)
}

fn collect_template_meshes(
    graph: &lobedo_core::Graph,
    display_node: lobedo_core::NodeId,
    template_nodes: &[lobedo_core::NodeId],
    state: &mut lobedo_core::GeometryEvalState,
    error_nodes: &mut HashSet<lobedo_core::NodeId>,
    error_messages: &mut HashMap<lobedo_core::NodeId, String>,
) -> Option<Mesh> {
    let mut meshes = Vec::new();
    for node_id in template_nodes {
        if *node_id == display_node {
            continue;
        }
        match evaluate_geometry_graph(graph, *node_id, state) {
            Ok(result) => {
                merge_error_state(&result.report, error_nodes, error_messages);
                if result.report.output_valid {
                    if let Some(geometry) = result.output {
                        if let Some(mesh) = geometry.merged_mesh() {
                            meshes.push(mesh);
                        }
                    }
                }
            }
            Err(err) => {
                tracing::error!("template eval failed: {:?}", err);
            }
        }
    }
    if meshes.is_empty() {
        None
    } else {
        Some(Mesh::merge(&meshes))
    }
}

fn merge_error_state(
    report: &lobedo_core::EvalReport,
    nodes: &mut HashSet<lobedo_core::NodeId>,
    messages: &mut HashMap<lobedo_core::NodeId, String>,
) {
    for err in &report.errors {
        match err {
            lobedo_core::EvalError::Node { node, message } => {
                nodes.insert(*node);
                messages.entry(*node).or_insert_with(|| message.clone());
            }
            lobedo_core::EvalError::Upstream { node, upstream } => {
                nodes.insert(*node);
                messages
                    .entry(*node)
                    .or_insert_with(|| format!("Upstream error in nodes: {:?}", upstream));
                for upstream_node in upstream {
                    nodes.insert(*upstream_node);
                }
            }
        }
    }
}
