use std::collections::{HashMap, HashSet};
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use lobedo_core::{
    evaluate_geometry_graph, Mesh, SceneDrawable, SceneSnapshot, SceneSplats, ShadingMode,
};
use render::{
    RenderDrawable, RenderMaterial, RenderMesh, RenderScene, RenderSplats, RenderTexture,
    SelectionShape, ViewportDebug, ViewportShadingMode,
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
                    let selection_shape =
                        selection_shape_for_group(&self.project.graph, self.node_graph.selected_node_id());
                    let scene = scene_to_render_with_template(
                        &snapshot,
                        template_mesh.as_ref(),
                        selection_shape,
                    );
                    self.apply_scene(scene);
                } else {
                    if let Some(renderer) = &self.viewport_renderer {
                        renderer.clear_scene();
                    }
                    self.pending_scene = None;
                    self.last_scene = None;
                }

                if !output_valid {
                    if let Some(renderer) = &self.viewport_renderer {
                        renderer.clear_scene();
                    }
                    self.pending_scene = None;
                    self.last_scene = None;
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

    pub(super) fn apply_scene(&mut self, scene: RenderScene) {
        self.last_scene = Some(scene.clone());
        if let Some(renderer) = &self.viewport_renderer {
            renderer.set_scene(scene);
        } else {
            self.pending_scene = Some(scene);
        }
    }

    pub(super) fn sync_selection_overlay(&mut self) {
        let Some(scene) = self.last_scene.clone() else {
            self.last_selection_key = None;
            return;
        };
        let selection = selection_shape_for_group(
            &self.project.graph,
            self.node_graph.selected_node_id(),
        );
        let selection_key = self
            .node_graph
            .selected_node_id()
            .and_then(|node_id| self.project.graph.node(node_id).map(|node| (node_id, node.param_version)));
        if selection_key == self.last_selection_key
            && selection == scene.selection_shape
        {
            return;
        }

        self.last_selection_key = selection_key;
        let mut scene = scene;
        scene.selection_shape = selection;
        self.apply_scene(scene);
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
            pause_render: self.pause_viewport,
        }
    }
}

pub(super) fn scene_to_render_with_template(
    scene: &SceneSnapshot,
    template: Option<&Mesh>,
    selection_shape: Option<SelectionShape>,
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

    let base_color = if mesh_has_colors || !scene.materials.is_empty() {
        [1.0, 1.0, 1.0]
    } else {
        scene.base_color
    };

    let (materials, textures) = render_materials_from_scene(scene);

    RenderScene {
        drawables,
        base_color,
        template_mesh: template.map(render_mesh_from_mesh),
        selection_shape,
        materials,
        textures,
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
        uvs: mesh.uvs.clone(),
        corner_uvs: mesh.corner_uvs.clone(),
        corner_materials: mesh.corner_materials.clone(),
    }
}

fn render_splats_from_scene(splats: &SceneSplats) -> RenderSplats {
    let mut colors = splats.colors.clone();
    let mut opacity = splats.opacity.clone();
    let mut scales = splats.scales.clone();

    for value in &mut opacity {
        let logit = value.clamp(-9.21034, 9.21034);
        *value = 1.0 / (1.0 + (-logit).exp());
    }

    for value in &mut scales {
        let sx = value[0].clamp(-10.0, 10.0).exp();
        let sy = value[1].clamp(-10.0, 10.0).exp();
        let sz = value[2].clamp(-10.0, 10.0).exp();
        *value = [sx, sy, sz];
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

const MAX_MATERIAL_TEXTURES: usize = 1;

fn render_materials_from_scene(
    scene: &SceneSnapshot,
) -> (Vec<RenderMaterial>, Vec<RenderTexture>) {
    let mut materials = Vec::new();
    let mut textures = Vec::new();
    let mut texture_lookup: HashMap<String, usize> = HashMap::new();

    for material in &scene.materials {
        let mut base_color_texture = None;
        if let Some(path) = material.base_color_texture.as_ref() {
            if let Some(&index) = texture_lookup.get(path) {
                base_color_texture = Some(index);
            } else if textures.len() < MAX_MATERIAL_TEXTURES {
                match load_render_texture(path) {
                    Some(texture) => {
                        let index = textures.len();
                        textures.push(texture);
                        texture_lookup.insert(path.clone(), index);
                        base_color_texture = Some(index);
                    }
                    None => {
                        tracing::warn!("failed to load texture: {path}");
                    }
                }
            } else if !texture_lookup.contains_key(path) {
                tracing::warn!("only one texture is supported right now; skipping {path}");
            } else {
                tracing::warn!("texture limit reached, skipping: {path}");
            }
        }

        materials.push(RenderMaterial {
            base_color: material.base_color,
            metallic: material.metallic,
            roughness: material.roughness,
            base_color_texture,
        });
    }

    if materials.is_empty() {
        materials.push(RenderMaterial {
            base_color: [1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            base_color_texture: None,
        });
    }

    (materials, textures)
}

fn load_render_texture(path: &str) -> Option<RenderTexture> {
    let bytes = load_texture_bytes(path)?;
    let image = match image::load_from_memory(&bytes) {
        Ok(image) => image,
        Err(err) => {
            tracing::warn!("texture decode failed for {path}: {err}");
            return None;
        }
    };
    let rgba = image.to_rgba8();
    Some(RenderTexture {
        width: rgba.width(),
        height: rgba.height(),
        pixels: rgba.into_raw(),
    })
}

fn load_texture_bytes(path: &str) -> Option<Vec<u8>> {
    if let Some(bytes) = lobedo_core::load_bytes(path) {
        return Some(bytes);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        fs::read(path).ok()
    }
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
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

fn selection_shape_for_group(
    graph: &lobedo_core::Graph,
    node_id: Option<lobedo_core::NodeId>,
) -> Option<SelectionShape> {
    let node_id = node_id?;
    let node = graph.node(node_id)?;
    if node.name != "Group" {
        return None;
    }
    let shape = node.params.get_string("shape", "box").to_lowercase();
    match shape.as_str() {
        "box" => {
            let center = node.params.get_vec3("center", [0.0, 0.0, 0.0]);
            let size = node.params.get_vec3("size", [1.0, 1.0, 1.0]);
            Some(SelectionShape::Box { center, size })
        }
        "sphere" => {
            let center = node.params.get_vec3("center", [0.0, 0.0, 0.0]);
            let mut size = node.params.get_vec3("size", [1.0, 1.0, 1.0]);
            if size == [1.0, 1.0, 1.0] {
                let radius = node.params.get_float("radius", 1.0);
                if (radius - 1.0).abs() > f32::EPSILON {
                    size = [radius * 2.0, radius * 2.0, radius * 2.0];
                }
            }
            Some(SelectionShape::Sphere { center, size })
        }
        "plane" => {
            let origin = node.params.get_vec3("plane_origin", [0.0, 0.0, 0.0]);
            let normal = node.params.get_vec3("plane_normal", [0.0, 1.0, 0.0]);
            let size = node.params.get_vec3("size", [1.0, 1.0, 1.0]);
            Some(SelectionShape::Plane {
                origin,
                normal,
                size,
            })
        }
        _ => None,
    }
}
