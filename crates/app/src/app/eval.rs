use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::thread;

#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use lobedo_core::{
    build_skirt_preview_mesh, evaluate_geometry_graph_with_progress,
    Geometry, GeometryEvalState, Mesh, NodeId, ProgressSink, SceneCurve, SceneDrawable,
    SceneSnapshot, SceneSplats, SceneVolume,
    ShadingMode, SplatShadingMode, VolumeKind,
};
use render::{
    RenderCurve, RenderDrawable, RenderMaterial, RenderMesh, RenderScene, RenderSplats,
    RenderTexture, RenderVolume, RenderVolumeKind, SelectionShape, ViewportDebug,
    ViewportShadingMode, ViewportSplatShadingMode,
};

use super::{viewport_tools::input_node_for, DisplayState, LobedoApp};

pub(crate) struct EvalJob {
    receiver: Receiver<EvalResult>,
}

struct EvalResult {
    revision: u64,
    display_node: NodeId,
    eval_state: GeometryEvalState,
    report: lobedo_core::EvalReport,
    error_nodes: HashSet<NodeId>,
    error_messages: HashMap<NodeId, String>,
    template_mesh: Option<Mesh>,
    scene: Option<RenderScene>,
    last_eval_ms: Option<f32>,
}

impl LobedoApp {
    pub(super) fn mark_eval_dirty(&mut self) {
        self.eval_dirty = true;
        self.last_param_change = Some(Instant::now());
    }

    pub(super) fn evaluate_if_needed(&mut self, ctx: &egui::Context) {
        let url_revision = lobedo_core::url_revision();
        if url_revision != self.last_url_revision {
            self.last_url_revision = url_revision;
            self.mark_eval_dirty();
        }
        if self.poll_eval_job(ctx) {
            return;
        }

        if !self.eval_dirty {
            return;
        }

        let dragging = self.viewport_tools.is_dragging();
        let fps = self.viewport_fps();
        if dragging && fps < 1.0 {
            return;
        }
        let realtime = dragging && fps >= 1.0;
        if !realtime {
            if let Some(last_change) = self.last_param_change {
                let debounce = Duration::from_millis(150);
                if last_change.elapsed() < debounce {
                    return;
                }
            }
        }

        self.eval_dirty = false;
        self.last_param_change = None;

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.start_eval_job(ctx);
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.evaluate_graph();
        }
    }

    pub(super) fn evaluate_graph(&mut self) {
        let Some(display_node) = self.project.graph.display_node() else {
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
            self.last_template_mesh = None;
            return;
        };
        self.last_display_state = DisplayState::Ok;

        let revision = self.project.graph.revision();
        let graph = self.project.graph.clone();
        let template_nodes = graph.template_nodes();
        let selected_node = self.node_graph.selected_node_id();
        let progress = Some(self.node_graph.progress_sink());
        let eval_state = std::mem::take(&mut self.eval_state);
        let result = run_eval_job(
            graph,
            display_node,
            revision,
            template_nodes,
            selected_node,
            eval_state,
            progress,
        );
        self.apply_eval_result(result);
    }

    fn poll_eval_job(&mut self, ctx: &egui::Context) -> bool {
        let Some(job) = &self.eval_job else {
            return false;
        };
        match job.receiver.try_recv() {
            Ok(result) => {
                self.eval_job = None;
                self.apply_eval_result(result);
                false
            }
            Err(TryRecvError::Empty) => {
                ctx.request_repaint();
                true
            }
            Err(TryRecvError::Disconnected) => {
                self.eval_job = None;
                false
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn start_eval_job(&mut self, ctx: &egui::Context) {
        let Some(display_node) = self.project.graph.display_node() else {
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
            self.last_template_mesh = None;
            return;
        };
        self.last_display_state = DisplayState::Ok;

        let revision = self.project.graph.revision();
        let graph = self.project.graph.clone();
        let template_nodes = graph.template_nodes();
        let selected_node = self.node_graph.selected_node_id();
        let progress = Some(self.node_graph.progress_sink());
        let eval_state = std::mem::take(&mut self.eval_state);

        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = run_eval_job(
                graph,
                display_node,
                revision,
                template_nodes,
                selected_node,
                eval_state,
                progress,
            );
            let _ = sender.send(result);
        });
        self.eval_job = Some(EvalJob { receiver });
        ctx.request_repaint();
    }

    fn apply_eval_result(&mut self, result: EvalResult) {
        self.eval_state = result.eval_state;
        if self.project.graph.revision() != result.revision
            || self.project.graph.display_node() != Some(result.display_node)
        {
            self.eval_dirty = true;
            return;
        }

        self.last_eval_report = Some(result.report);
        self.last_eval_ms = result.last_eval_ms;
        self.last_template_mesh = result.template_mesh.clone();
        self.node_graph
            .set_error_state(result.error_nodes, result.error_messages);

        if let Some(scene) = result.scene {
            self.apply_scene(scene);
        } else {
            if let Some(renderer) = &self.viewport_renderer {
                renderer.clear_scene();
            }
            self.pending_scene = None;
            self.last_scene = None;
            self.last_template_mesh = None;
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
            self.last_preview_key = None;
            return;
        };
        let selection = selection_shape_for_node(
            &self.project.graph,
            self.node_graph.selected_node_id(),
        );
        let selection_key = self
            .node_graph
            .selected_node_id()
            .and_then(|node_id| self.project.graph.node(node_id).map(|node| (node_id, node.param_version)));
        let mut scene = scene;
        let mut changed = false;

        if selection_key != self.last_selection_key || selection != scene.selection_shape {
            self.last_selection_key = selection_key;
            scene.selection_shape = selection;
            changed = true;
        }

        if selection_key != self.last_preview_key {
            let preview = splat_merge_preview_mesh(
                &self.project.graph,
                self.node_graph.selected_node_id(),
                &mut self.eval_state,
                None,
            );
            let merged_template = merge_optional_meshes(self.last_template_mesh.clone(), preview);
            scene.template_mesh = merged_template.as_ref().map(render_mesh_from_mesh);
            self.last_preview_key = selection_key;
            changed = true;
        }

        if changed {
            self.apply_scene(scene);
        }
    }

    pub(super) fn viewport_debug(&self) -> ViewportDebug {
        let shading_mode = match self.project.settings.render_debug.shading_mode {
            ShadingMode::Lit => ViewportShadingMode::Lit,
            ShadingMode::Normals => ViewportShadingMode::Normals,
            ShadingMode::Depth => ViewportShadingMode::Depth,
            ShadingMode::SplatOpacity => ViewportShadingMode::SplatOpacity,
            ShadingMode::SplatScale => ViewportShadingMode::SplatScale,
            ShadingMode::SplatOverdraw => ViewportShadingMode::SplatOverdraw,
        };
        let splat_shading_mode = match self.project.settings.render_debug.splat_shading_mode {
            SplatShadingMode::ColorOnly => ViewportSplatShadingMode::ColorOnly,
            SplatShadingMode::FullSh => ViewportSplatShadingMode::FullSh,
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
            splat_debug_min: self.project.settings.render_debug.splat_debug_min,
            splat_debug_max: self.project.settings.render_debug.splat_debug_max,
            splat_shading_mode,
            splat_tile_binning: self.project.settings.render_debug.splat_tile_binning,
            splat_tile_size: self.project.settings.render_debug.splat_tile_size,
            splat_tile_threshold: self.project.settings.render_debug.splat_tile_threshold,
            show_points: self.project.settings.render_debug.show_points,
            show_splats: self.project.settings.render_debug.show_splats,
            point_size: self.project.settings.render_debug.point_size,
            key_shadows: self.project.settings.render_debug.key_shadows,
            pause_render: self.pause_viewport,
        }
    }

    fn viewport_fps(&self) -> f32 {
        let fps = self
            .viewport_renderer
            .as_ref()
            .map(|renderer| renderer.stats_snapshot().fps)
            .unwrap_or(60.0);
        if fps <= 0.0 {
            60.0
        } else {
            fps
        }
    }
}

fn run_eval_job(
    graph: lobedo_core::Graph,
    display_node: NodeId,
    revision: u64,
    template_nodes: Vec<NodeId>,
    selected_node: Option<NodeId>,
    mut eval_state: GeometryEvalState,
    progress: Option<ProgressSink>,
) -> EvalResult {
    let start = Instant::now();
    let mut error_nodes = HashSet::new();
    let mut error_messages = HashMap::new();
    let mut report = lobedo_core::EvalReport::default();
    let mut output: Option<Geometry> = None;
    match evaluate_geometry_graph_with_progress(&graph, display_node, &mut eval_state, progress.clone())
    {
        Ok(result) => {
            report = result.report;
            merge_error_state(&report, &mut error_nodes, &mut error_messages);
            output = result.output;
        }
        Err(err) => {
            error_nodes.insert(display_node);
            error_messages.insert(display_node, format!("Eval failed: {err:?}"));
            report.output_valid = false;
        }
    }

    let last_eval_ms = Some(start.elapsed().as_secs_f32() * 1000.0);
    let mut template_mesh = None;
    let mut scene = None;
    let output_valid = report.output_valid;

    if output_valid {
        if let Some(geometry) = output.as_ref() {
            let snapshot = SceneSnapshot::from_geometry(geometry, [0.7, 0.72, 0.75]);
            template_mesh = collect_template_meshes(
                &graph,
                display_node,
                &template_nodes,
                &mut eval_state,
                &mut error_nodes,
                &mut error_messages,
                progress.clone(),
            );
            let preview = splat_merge_preview_mesh(
                &graph,
                selected_node,
                &mut eval_state,
                progress.clone(),
            );
            let merged_template = merge_optional_meshes(template_mesh.clone(), preview);
            let selection_shape = selection_shape_for_node(&graph, selected_node);
            scene = Some(scene_to_render_with_template(
                &snapshot,
                merged_template.as_ref(),
                selection_shape,
            ));
        }
    }

    if !output_valid {
        template_mesh = None;
        scene = None;
    }

    EvalResult {
        revision,
        display_node,
        eval_state,
        report,
        error_nodes,
        error_messages,
        template_mesh,
        scene,
        last_eval_ms,
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
            SceneDrawable::Curve(curve) => {
                drawables.push(RenderDrawable::Curve(render_curve_from_scene(curve)));
            }
            SceneDrawable::Volume(volume) => {
                drawables.push(RenderDrawable::Volume(render_volume_from_scene(volume)));
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
        tri_to_face: mesh.tri_to_face.clone(),
        corner_indices: mesh.corner_indices.clone(),
        poly_indices: mesh.poly_indices.clone(),
        poly_face_counts: mesh.poly_face_counts.clone(),
        corner_normals: mesh.corner_normals.clone(),
        colors: mesh.colors.clone(),
        corner_colors: mesh.corner_colors.clone(),
        uvs: mesh.uvs.clone(),
        corner_uvs: mesh.corner_uvs.clone(),
        corner_materials: mesh.corner_materials.clone(),
    }
}

fn render_splats_from_scene(splats: &SceneSplats) -> RenderSplats {
    let sh0 = splats.sh0.clone();
    let mut opacity = splats.opacity.clone();
    let mut scales = splats.scales.clone();
    let sh_coeffs = splats.sh_coeffs;
    let sh_rest = splats.sh_rest.clone();

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

    let sh0_is_coeff = sh_coeffs > 0
        || sh0
            .iter()
            .any(|value| value[0] < 0.0 || value[1] < 0.0 || value[2] < 0.0);

    RenderSplats {
        positions: splats.positions.clone(),
        sh0,
        sh_coeffs,
        sh_rest,
        sh0_is_coeff,
        opacity,
        scales,
        rotations: splats.rotations.clone(),
    }
}

fn render_curve_from_scene(curve: &SceneCurve) -> RenderCurve {
    RenderCurve {
        points: curve.points.clone(),
        closed: curve.closed,
    }
}

fn render_volume_from_scene(volume: &SceneVolume) -> RenderVolume {
    let kind = match volume.kind {
        VolumeKind::Density => RenderVolumeKind::Density,
        VolumeKind::Sdf => RenderVolumeKind::Sdf,
    };
    RenderVolume {
        kind,
        origin: volume.origin,
        dims: volume.dims,
        voxel_size: volume.voxel_size,
        values: volume.values.clone(),
        transform: volume.transform,
        density_scale: volume.density_scale,
        sdf_band: volume.sdf_band,
    }
}

fn render_mesh_from_mesh(mesh: &Mesh) -> RenderMesh {
    let snapshot = SceneSnapshot::from_mesh(mesh, [0.7, 0.72, 0.75]);
    let mesh = snapshot
        .mesh()
        .expect("mesh snapshot missing mesh");
    render_mesh_from_scene(mesh)
}

const MAX_MATERIAL_TEXTURES: usize = 64;

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
                tracing::warn!(
                    "texture limit ({MAX_MATERIAL_TEXTURES}) reached; skipping {path}"
                );
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
    progress: Option<ProgressSink>,
) -> Option<Mesh> {
    let mut meshes = Vec::new();
    for node_id in template_nodes {
        if *node_id == display_node {
            continue;
        }
        match evaluate_geometry_graph_with_progress(graph, *node_id, state, progress.clone()) {
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

fn splat_merge_preview_mesh(
    graph: &lobedo_core::Graph,
    selected_node: Option<NodeId>,
    state: &mut lobedo_core::GeometryEvalState,
    progress: Option<ProgressSink>,
) -> Option<Mesh> {
    let node_id = selected_node?;
    let node = graph.node(node_id)?;
    if node.name != "Splat Merge" {
        return None;
    }
    let preview = node.params.get_bool("preview_skirt", false);
    if !preview {
        return None;
    }
    let method = node.params.get_int("method", 0).clamp(0, 1);
    if method != 1 {
        return None;
    }
    let input_a = input_node_for(graph, node_id, 0)?;
    let input_b = input_node_for(graph, node_id, 1)?;
    let geo_a = evaluate_geometry_graph_with_progress(graph, input_a, state, progress.clone())
        .ok()?
        .output?;
    let geo_b =
        evaluate_geometry_graph_with_progress(graph, input_b, state, progress).ok()?.output?;
    let splats_a = geo_a.merged_splats()?;
    let splats_b = geo_b.merged_splats()?;
    build_skirt_preview_mesh(&node.params, &splats_a, &splats_b)
}

fn merge_optional_meshes(a: Option<Mesh>, b: Option<Mesh>) -> Option<Mesh> {
    match (a, b) {
        (Some(a), Some(b)) => Some(Mesh::merge(&[a, b])),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
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

fn selection_shape_for_node(
    graph: &lobedo_core::Graph,
    node_id: Option<lobedo_core::NodeId>,
) -> Option<SelectionShape> {
    let node_id = node_id?;
    let node = graph.node(node_id)?;
    match node.name.as_str() {
        "Box" => {
            let center = node.params.get_vec3("center", [0.0, 0.0, 0.0]);       
            let size = node.params.get_vec3("size", [1.0, 1.0, 1.0]);
            Some(SelectionShape::Box { center, size })
        }
        "Group" | "Delete" => selection_shape_from_params(&node.params),
        "Splat Heal" => {
            let shape = node.params.get_string("heal_shape", "all").to_lowercase();
            match shape.as_str() {
                "box" => {
                    let center = node.params.get_vec3("heal_center", [0.0, 0.0, 0.0]);
                    let size = node.params.get_vec3("heal_size", [1.0, 1.0, 1.0]);
                    Some(SelectionShape::Box { center, size })
                }
                "sphere" => {
                    let center = node.params.get_vec3("heal_center", [0.0, 0.0, 0.0]);
                    let size = node.params.get_vec3("heal_size", [1.0, 1.0, 1.0]);
                    Some(SelectionShape::Sphere { center, size })
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn selection_shape_from_params(params: &lobedo_core::NodeParams) -> Option<SelectionShape> {
    let shape = params.get_string("shape", "box").to_lowercase();
    match shape.as_str() {
        "selection" => None,
        "box" => {
            let center = params.get_vec3("center", [0.0, 0.0, 0.0]);
            let size = params.get_vec3("size", [1.0, 1.0, 1.0]);
            Some(SelectionShape::Box { center, size })
        }
        "sphere" => {
            let center = params.get_vec3("center", [0.0, 0.0, 0.0]);
            let mut size = params.get_vec3("size", [1.0, 1.0, 1.0]);
            if size == [1.0, 1.0, 1.0] {
                let radius = params.get_float("radius", 1.0);
                if (radius - 1.0).abs() > f32::EPSILON {
                    size = [radius * 2.0, radius * 2.0, radius * 2.0];
                }
            }
            Some(SelectionShape::Sphere { center, size })
        }
        "plane" => {
            let origin = params.get_vec3("plane_origin", [0.0, 0.0, 0.0]);
            let normal = params.get_vec3("plane_normal", [0.0, 1.0, 0.0]);
            let size = params.get_vec3("size", [1.0, 1.0, 1.0]);
            Some(SelectionShape::Plane {
                origin,
                normal,
                size,
            })
        }
        _ => None,
    }
}
