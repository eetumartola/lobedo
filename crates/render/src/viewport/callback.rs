use std::sync::{Arc, Mutex};

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use egui::epaint::Rect;
use egui_wgpu::wgpu::util::DeviceExt as _;
use egui_wgpu::{CallbackResources, CallbackTrait};

use super::callback_helpers::{
    light_view_projection, sort_splats_by_depth, splat_color_from_sh,
};
use super::mesh::{
    normals_vertices, point_cross_vertices, splat_billboard_vertices, splat_billboards,
    splat_vertices_from_billboards, SplatBillboardInputs, SplatVertex,
};
use super::pipeline::{apply_scene_to_pipeline, ensure_offscreen_targets, PipelineState, Uniforms};
use super::{
    ViewportDebug, ViewportSceneState, ViewportShadingMode, ViewportSplatShadingMode,
    ViewportStatsState,
};
use crate::camera::{camera_position, camera_view_proj, CameraState};
use crate::mesh_cache::GpuMeshData;
use glam::{Mat3, Mat4, Vec2, Vec3};

pub(super) struct ViewportCallback {
    pub(super) target_format: egui_wgpu::wgpu::TextureFormat,
    pub(super) rect: Rect,
    pub(super) camera: CameraState,
    pub(super) debug: ViewportDebug,
    pub(super) stats: Arc<Mutex<ViewportStatsState>>,
    pub(super) scene: Arc<Mutex<ViewportSceneState>>,
}

impl CallbackTrait for ViewportCallback {
    fn prepare(
        &self,
        device: &egui_wgpu::wgpu::Device,
        queue: &egui_wgpu::wgpu::Queue,
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut egui_wgpu::wgpu::CommandEncoder,
        callback_resources: &mut CallbackResources,
    ) -> Vec<egui_wgpu::wgpu::CommandBuffer> {
        if callback_resources.get::<PipelineState>().is_none() {
            callback_resources.insert(PipelineState::new(
                device,
                queue,
                self.target_format,
            ));
        }

        let view_proj = camera_view_proj(self.camera, self.rect, screen_descriptor);
        let camera_pos = camera_position(self.camera);
        let target = glam::Vec3::from(self.camera.target);
        let forward = (target - camera_pos).normalize_or_zero();
        let mut right = forward.cross(Vec3::Y).normalize_or_zero();
        if right.length_squared() == 0.0 {
            right = Vec3::X;
        }
        let up = right.cross(forward).normalize_or_zero();
        let key_dir = (-forward + right * 0.6 + up * 0.8).normalize_or_zero();
        let fill_dir = (-forward - right * 0.4 + up * 0.2).normalize_or_zero();
        let rim_dir = (forward + up * 0.6).normalize_or_zero();
        let shading_mode = match self.debug.shading_mode {
            ViewportShadingMode::Lit => 0.0,
            ViewportShadingMode::Normals => 1.0,
            ViewportShadingMode::Depth => 2.0,
            ViewportShadingMode::SplatOpacity => 3.0,
            ViewportShadingMode::SplatScale => 4.0,
            ViewportShadingMode::SplatOverdraw => 5.0,
        };
        let (debug_min, debug_max) = match self.debug.shading_mode {
            ViewportShadingMode::SplatOpacity
            | ViewportShadingMode::SplatScale
            | ViewportShadingMode::SplatOverdraw => {
                (self.debug.splat_debug_min, self.debug.splat_debug_max)
            }
            _ => (self.debug.depth_near, self.debug.depth_far),
        };

        if let Some(pipeline) = callback_resources.get_mut::<PipelineState>() {
            let width = (self.rect.width() * screen_descriptor.pixels_per_point)
                .round()
                .max(1.0) as u32;
            let height = (self.rect.height() * screen_descriptor.pixels_per_point)
                .round()
                .max(1.0) as u32;
            let viewport_changed = pipeline.offscreen_size != [width, height];
            ensure_offscreen_targets(device, pipeline, self.target_format, width, height);

            let (scene_version, scene) = if let Ok(scene_state) = self.scene.lock() {
                (scene_state.version, scene_state.scene.clone())
            } else {
                (pipeline.scene_version, None)
            };
            let scene_changed = scene_version != pipeline.scene_version;
            if scene_changed {
                match scene.as_deref() {
                    Some(scene) => {
                        apply_scene_to_pipeline(device, queue, pipeline, scene);
                        pipeline.base_color = scene.base_color;
                    }
                    None => {
                        pipeline.mesh_vertices.clear();
                        pipeline.index_count = 0;
                        pipeline.mesh_bounds = ([0.0; 3], [0.0; 3]);
                        pipeline.base_color = [0.7, 0.72, 0.75];
                        pipeline.template_count = 0;
                        pipeline.curve_count = 0;
                        pipeline.selection_count = 0;
                        pipeline.splat_positions.clear();
                        pipeline.splat_sh0.clear();
                        pipeline.splat_sh_coeffs = 0;
                        pipeline.splat_sh_rest.clear();
                        pipeline.splat_sh0_is_coeff = false;
                        pipeline.splat_opacity.clear();
                        pipeline.splat_scales.clear();
                        pipeline.splat_rotations.clear();
                        pipeline.splat_buffers.clear();
                        pipeline.splat_counts.clear();
                        pipeline.splat_scissors.clear();
                        pipeline.splat_point_size = -1.0;
                        pipeline.splat_last_right = [0.0, 0.0, 0.0];
                        pipeline.splat_last_up = [0.0, 0.0, 0.0];
                        pipeline.splat_last_camera_pos = [0.0, 0.0, 0.0];
                        pipeline.splat_last_viewport = [0, 0];
                    }
                }
                pipeline.scene_version = scene_version;
            }
            if self.debug.pause_render && !scene_changed && !viewport_changed {
                return Vec::new();
            }

            let light_view_proj = light_view_projection(pipeline.mesh_bounds, key_dir);
            let shadow_enabled = self.debug.key_shadows && pipeline.index_count > 0;
            let bounds_min = Vec3::from(pipeline.mesh_bounds.0);
            let bounds_max = Vec3::from(pipeline.mesh_bounds.1);
            let radius = ((bounds_max - bounds_min) * 0.5).length().max(0.5);
            let shadow_bias = (radius * 0.00025).clamp(0.000025, 0.005);
            let normal_bias = 0.0;
            let shadow_texel = 1.0 / pipeline._shadow_size.max(1) as f32;

            let uniforms = Uniforms {
                view_proj: view_proj.to_cols_array_2d(),
                light_view_proj: light_view_proj.to_cols_array_2d(),
                key_dir: key_dir.to_array(),
                _pad0: 0.0,
                fill_dir: fill_dir.to_array(),
                _pad1: 0.0,
                rim_dir: rim_dir.to_array(),
                _pad2: 0.0,
                camera_pos: camera_pos.to_array(),
                _pad3: 0.0,
                base_color: pipeline.base_color,
                _pad4: 0.0,
                light_params: [1.0, 0.45, 0.5, 0.15],
                debug_params: [
                    shading_mode,
                    debug_min,
                    debug_max,
                    self.debug.point_size,
                ],
                shadow_params: [
                    if shadow_enabled { 1.0 } else { 0.0 },
                    shadow_bias,
                    shadow_texel,
                    normal_bias,
                ],
            };

            queue.write_buffer(&pipeline.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

            if self.debug.show_normals
                && (self.debug.normal_length - pipeline.normals_length).abs() > 0.0001
            {
                let normals_vertices =
                    normals_vertices(&pipeline.mesh_vertices, self.debug.normal_length);
                queue.write_buffer(
                    &pipeline.normals_buffer,
                    0,
                    bytemuck::cast_slice(&normals_vertices),
                );
                pipeline.normals_length = self.debug.normal_length;
            }

            if let Ok(mut stats_state) = self.stats.lock() {
                let now = Instant::now();
                if let Some(last) = stats_state.last_frame {
                    let dt = (now - last).as_secs_f32();
                    if dt > 0.0 {
                        let fps = 1.0 / dt;
                        let frame_ms = dt * 1000.0;
                        let alpha = 0.1;
                        if stats_state.stats.fps == 0.0 {
                            stats_state.stats.fps = fps;
                            stats_state.stats.frame_time_ms = frame_ms;
                        } else {
                            stats_state.stats.fps += (fps - stats_state.stats.fps) * alpha;
                            stats_state.stats.frame_time_ms +=
                                (frame_ms - stats_state.stats.frame_time_ms) * alpha;
                        }
                    }
                }
                stats_state.last_frame = Some(now);

                let cache_stats = pipeline.mesh_cache.stats_snapshot();
                stats_state.stats.mesh_count = cache_stats.mesh_count;
                stats_state.stats.cache_hits = cache_stats.hits;
                stats_state.stats.cache_misses = cache_stats.misses;
                stats_state.stats.cache_uploads = cache_stats.uploads;
                stats_state.stats.vertex_count = pipeline.mesh_vertices.len() as u32;
                stats_state.stats.triangle_count = pipeline.index_count / 3;
            }

            let mesh = if pipeline.mesh_vertices.is_empty() {
                None
            } else {
                pipeline.mesh_cache.get(pipeline.mesh_id)
            };
            if shadow_enabled {
                if let Some(mesh) = &mesh {
                    let mut shadow_pass =
                        _egui_encoder.begin_render_pass(&egui_wgpu::wgpu::RenderPassDescriptor {
                            label: Some("lobedo_shadow_pass"),
                            color_attachments: &[],
                            depth_stencil_attachment: Some(
                                egui_wgpu::wgpu::RenderPassDepthStencilAttachment {
                                    view: &pipeline.shadow_view,
                                    depth_ops: Some(egui_wgpu::wgpu::Operations {
                                        load: egui_wgpu::wgpu::LoadOp::Clear(1.0),
                                        store: egui_wgpu::wgpu::StoreOp::Store,
                                    }),
                                    stencil_ops: None,
                                },
                            ),
                            occlusion_query_set: None,
                            timestamp_writes: None,
                        });
                    shadow_pass.set_pipeline(&pipeline.shadow_pipeline);
                    shadow_pass.set_bind_group(0, &pipeline.uniform_bind_group, &[]);
                    shadow_pass.set_bind_group(1, &pipeline.material_bind_group, &[]);
                    match &mesh.data {
                        GpuMeshData::Indexed {
                            vertex_buffer,
                            index_buffers,
                            index_counts,
                        } => {
                            shadow_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                            for (buffer, count) in
                                index_buffers.iter().zip(index_counts.iter())
                            {
                                if *count == 0 {
                                    continue;
                                }
                                shadow_pass.set_index_buffer(
                                    buffer.slice(..),
                                    egui_wgpu::wgpu::IndexFormat::Uint32,
                                );
                                shadow_pass.draw_indexed(0..*count, 0, 0..1);
                            }
                        }
                        GpuMeshData::NonIndexed {
                            vertex_buffers,
                            vertex_counts,
                        } => {
                            for (buffer, count) in
                                vertex_buffers.iter().zip(vertex_counts.iter())
                            {
                                if *count == 0 {
                                    continue;
                                }
                                shadow_pass.set_vertex_buffer(0, buffer.slice(..));
                                shadow_pass.draw(0..*count, 0..1);
                            }
                        }
                    }
                }
            }

            let mut render_pass =
                _egui_encoder.begin_render_pass(&egui_wgpu::wgpu::RenderPassDescriptor {
                    label: Some("lobedo_viewport_offscreen"),
                    color_attachments: &[Some(egui_wgpu::wgpu::RenderPassColorAttachment {
                        view: &pipeline.offscreen_view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: egui_wgpu::wgpu::Operations {
                            load: egui_wgpu::wgpu::LoadOp::Clear(egui_wgpu::wgpu::Color {
                                r: 28.0 / 255.0,
                                g: 28.0 / 255.0,
                                b: 28.0 / 255.0,
                                a: 1.0,
                            }),
                            store: egui_wgpu::wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(
                        egui_wgpu::wgpu::RenderPassDepthStencilAttachment {
                            view: &pipeline.depth_view,
                            depth_ops: Some(egui_wgpu::wgpu::Operations {
                                load: egui_wgpu::wgpu::LoadOp::Clear(1.0),
                                store: egui_wgpu::wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        },
                    ),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

            render_pass.set_viewport(0.0, 0.0, width as f32, height as f32, 0.0, 1.0);
            if let Some(mesh) = mesh {
                if !self.debug.show_points && pipeline.index_count > 0 {
                    render_pass.set_pipeline(&pipeline.mesh_pipeline);
                    render_pass.set_bind_group(0, &pipeline.uniform_bind_group, &[]);
                    render_pass.set_bind_group(1, &pipeline.material_bind_group, &[]);
                    match &mesh.data {
                        GpuMeshData::Indexed {
                            vertex_buffer,
                            index_buffers,
                            index_counts,
                        } => {
                            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                            for (buffer, count) in
                                index_buffers.iter().zip(index_counts.iter())
                            {
                                if *count == 0 {
                                    continue;
                                }
                                render_pass.set_index_buffer(
                                    buffer.slice(..),
                                    egui_wgpu::wgpu::IndexFormat::Uint32,
                                );
                                render_pass.draw_indexed(0..*count, 0, 0..1);
                            }
                        }
                        GpuMeshData::NonIndexed {
                            vertex_buffers,
                            vertex_counts,
                        } => {
                            for (buffer, count) in
                                vertex_buffers.iter().zip(vertex_counts.iter())
                            {
                                if *count == 0 {
                                    continue;
                                }
                                render_pass.set_vertex_buffer(0, buffer.slice(..));
                                render_pass.draw(0..*count, 0..1);
                            }
                        }
                    }
                }
            }

            render_pass.set_pipeline(&pipeline.line_pipeline);
            render_pass.set_bind_group(0, &pipeline.uniform_bind_group, &[]);
            render_pass.set_bind_group(1, &pipeline.material_bind_group, &[]);

            if self.debug.show_points || pipeline.index_count == 0 {
                let camera_distance = (camera_pos - Vec3::from(self.camera.target)).length();
                let pixel_size = self.debug.point_size.max(1.0);
                let viewport_height = height.max(1) as f32;
                let fov = 45_f32.to_radians();
                let world_per_pixel =
                    2.0 * camera_distance.max(0.1) * (fov * 0.5).tan() / viewport_height;
                let desired_size = (pixel_size * world_per_pixel).max(0.0001);
                if pipeline.point_size < 0.0 || (desired_size - pipeline.point_size).abs() > 0.0001
                {
                    let point_vertices =
                        point_cross_vertices(&pipeline.point_positions, desired_size);
                    pipeline.point_buffer =
                        device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
                            label: Some("lobedo_point_vertices"),
                            contents: bytemuck::cast_slice(&point_vertices),
                            usage: egui_wgpu::wgpu::BufferUsages::VERTEX,
                        });
                    pipeline.point_count = point_vertices.len() as u32;
                    pipeline.point_size = desired_size;
                }
                if pipeline.point_count > 0 {
                    render_pass.set_vertex_buffer(0, pipeline.point_buffer.slice(..));
                    render_pass.draw(0..pipeline.point_count, 0..1);
                }
            }

            if self.debug.show_splats && !pipeline.splat_positions.is_empty() {
                const SPLAT_REBUILD_MAX_FPS: f32 = 30.0;
                const SPLAT_BUFFER_SOFT_LIMIT: u64 = 64 * 1024 * 1024;
                let tile_size = self.debug.splat_tile_size.max(1);
                let tile_threshold = self.debug.splat_tile_threshold as usize;
                let right_delta = right - Vec3::from(pipeline.splat_last_right);
                let up_delta = up - Vec3::from(pipeline.splat_last_up);
                let camera_delta = camera_pos - Vec3::from(pipeline.splat_last_camera_pos);
                let viewport_changed = pipeline.splat_last_viewport != [width, height];
                let needs_rebuild = pipeline.splat_point_size < 0.0
                    || right_delta.length_squared() > 1.0e-6
                    || up_delta.length_squared() > 1.0e-6
                    || camera_delta.length_squared() > 1.0e-6
                    || viewport_changed;
                let now = Instant::now();
                let elapsed = pipeline
                    .last_splat_rebuild
                    .map(|last| (now - last).as_secs_f32())
                    .unwrap_or(f32::INFINITY);
                let interval = 1.0 / SPLAT_REBUILD_MAX_FPS.max(1.0);
                let allow_rebuild = scene_changed || viewport_changed || elapsed >= interval;
                if needs_rebuild && allow_rebuild {
                    let world_transform =
                        Mat3::from_diagonal(Vec3::new(1.0, -1.0, 1.0));
                    let use_full_sh = matches!(
                        self.debug.splat_shading_mode,
                        ViewportSplatShadingMode::FullSh
                    ) && matches!(self.debug.shading_mode, ViewportShadingMode::Lit);
                    let sh_coeffs = pipeline.splat_sh_coeffs;
                    let sh0_is_coeff = pipeline.splat_sh0_is_coeff;
                    let sh_rest = &pipeline.splat_sh_rest;
                    let mut base_colors = Vec::with_capacity(pipeline.splat_positions.len());
                    for (idx, pos) in pipeline.splat_positions.iter().enumerate() {
                        let sh0 = pipeline
                            .splat_sh0
                            .get(idx)
                            .copied()
                            .unwrap_or([1.0, 1.0, 1.0]);
                        let base = idx.saturating_mul(sh_coeffs);
                        let rest = if sh_coeffs > 0 && base + sh_coeffs <= sh_rest.len() {
                            &sh_rest[base..base + sh_coeffs]
                        } else {
                            &[]
                        };
                        let view_dir = if use_full_sh && sh0_is_coeff {
                            let center = world_transform * Vec3::from(*pos);
                            world_transform.transpose() * (camera_pos - center)
                        } else {
                            Vec3::Z
                        };
                        base_colors.push(splat_color_from_sh(
                            sh0,
                            rest,
                            sh_coeffs,
                            sh0_is_coeff,
                            use_full_sh,
                            view_dir,
                        ));
                    }
                    let vertex_bytes = std::mem::size_of::<SplatVertex>() as u64;
                    let max_buffer_size =
                        device.limits().max_buffer_size.min(SPLAT_BUFFER_SOFT_LIMIT);
                    let splat_bytes = vertex_bytes.saturating_mul(6);
                    let max_splats = (max_buffer_size / splat_bytes)
                        .min(u32::MAX as u64)
                        .max(1) as usize;
                    let view = Mat4::look_at_rh(camera_pos, target, Vec3::Y);
                    let allow_tile_binning =
                        self.debug.splat_tile_binning && !cfg!(target_arch = "wasm32");
                    let use_tile_binning = allow_tile_binning
                        && (tile_threshold == 0
                            || pipeline.splat_positions.len() >= tile_threshold);
                    let mut buffers = Vec::new();
                    let mut counts = Vec::new();
                    let mut scissors = Vec::new();
                    if use_tile_binning {
                        let billboards = splat_billboards(SplatBillboardInputs {
                            positions: &pipeline.splat_positions,
                            colors: &base_colors,
                            opacities: &pipeline.splat_opacity,
                            scales: &pipeline.splat_scales,
                            rotations: &pipeline.splat_rotations,
                            view,
                            viewport: [width as f32, height as f32],
                            fov_y: 45_f32.to_radians(),
                            world_transform,
                        });
                        let mut depths = Vec::with_capacity(billboards.len());
                        for billboard in &billboards {
                            let center = Vec3::from(billboard.center);
                            depths.push((center - camera_pos).dot(forward));
                        }

                        let tiles_x = width.div_ceil(tile_size);
                        let tiles_y = height.div_ceil(tile_size);
                        let tile_count = tiles_x.saturating_mul(tiles_y) as usize;
                        let mut bins: Vec<Vec<usize>> = vec![Vec::new(); tile_count];
                        let width_f = width as f32;
                        let height_f = height as f32;
                        let corners = [
                            (-1.0, -1.0),
                            (1.0, -1.0),
                            (1.0, 1.0),
                            (-1.0, 1.0),
                        ];
                        for (idx, billboard) in billboards.iter().enumerate() {
                            let center = Vec3::from(billboard.center);
                            let clip = view_proj * center.extend(1.0);
                            if clip.w <= 1.0e-6 || !clip.w.is_finite() {
                                continue;
                            }
                            let ndc = Vec2::new(clip.x / clip.w, clip.y / clip.w);
                            let axis1 = Vec2::from(billboard.axis1_ndc);
                            let axis2 = Vec2::from(billboard.axis2_ndc);
                            let mut min_x = f32::INFINITY;
                            let mut max_x = f32::NEG_INFINITY;
                            let mut min_y = f32::INFINITY;
                            let mut max_y = f32::NEG_INFINITY;
                            for (sx, sy) in corners {
                                let corner = ndc + axis1 * sx + axis2 * sy;
                                let px = (corner.x * 0.5 + 0.5) * width_f;
                                let py = (0.5 - corner.y * 0.5) * height_f;
                                min_x = min_x.min(px);
                                max_x = max_x.max(px);
                                min_y = min_y.min(py);
                                max_y = max_y.max(py);
                            }
                            if max_x < 0.0
                                || max_y < 0.0
                                || min_x > width_f
                                || min_y > height_f
                            {
                                continue;
                            }
                            min_x = min_x.clamp(0.0, width_f - 1.0);
                            max_x = max_x.clamp(0.0, width_f - 1.0);
                            min_y = min_y.clamp(0.0, height_f - 1.0);
                            max_y = max_y.clamp(0.0, height_f - 1.0);
                            let tile_min_x = (min_x as u32) / tile_size;
                            let tile_max_x = (max_x as u32) / tile_size;
                            let tile_min_y = (min_y as u32) / tile_size;
                            let tile_max_y = (max_y as u32) / tile_size;
                            for ty in tile_min_y..=tile_max_y {
                                for tx in tile_min_x..=tile_max_x {
                                    let tile_index = (ty * tiles_x + tx) as usize;
                                    if let Some(bin) = bins.get_mut(tile_index) {
                                        bin.push(idx);
                                    }
                                }
                            }
                        }
                        for (tile_index, bin) in bins.iter_mut().enumerate() {
                            if bin.is_empty() {
                                continue;
                            }
                            bin.sort_by(|a, b| {
                                depths
                                    .get(*b)
                                    .unwrap_or(&0.0)
                                    .partial_cmp(depths.get(*a).unwrap_or(&0.0))
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            });
                            let tile_x = (tile_index as u32) % tiles_x;
                            let tile_y = (tile_index as u32) / tiles_x;
                            let x0 = tile_x * tile_size;
                            let y0 = tile_y * tile_size;
                            let x1 = (x0 + tile_size).min(width);
                            let y1 = (y0 + tile_size).min(height);
                            let scissor = [x0, y0, (x1 - x0).max(1), (y1 - y0).max(1)];
                            for chunk in bin.chunks(max_splats) {
                                let splat_vertices =
                                    splat_vertices_from_billboards(&billboards, Some(chunk));
                                let count = splat_vertices.len() as u32;
                                if count == 0 {
                                    continue;
                                }
                                let bytes = bytemuck::cast_slice(&splat_vertices);
                                let buffer = device.create_buffer_init(
                                    &egui_wgpu::wgpu::util::BufferInitDescriptor {
                                        label: Some("lobedo_splat_vertices"),
                                        contents: bytes,
                                        usage: egui_wgpu::wgpu::BufferUsages::VERTEX
                                            | egui_wgpu::wgpu::BufferUsages::COPY_DST,
                                    },
                                );
                                buffers.push(buffer);
                                counts.push(count);
                                scissors.push(scissor);
                            }
                        }
                    } else {
                        let sorted = sort_splats_by_depth(
                            &pipeline.splat_positions,
                            &base_colors,
                            &pipeline.splat_opacity,
                            &pipeline.splat_scales,
                            &pipeline.splat_rotations,
                            camera_pos,
                            forward,
                            world_transform,
                        );
                        let mut positions = Vec::with_capacity(sorted.len());
                        let mut colors = Vec::with_capacity(sorted.len());
                        let mut opacity = Vec::with_capacity(sorted.len());
                        let mut scales = Vec::with_capacity(sorted.len());
                        let mut rotations = Vec::with_capacity(sorted.len());
                        for entry in sorted {
                            positions.push(entry.position);
                            colors.push(entry.color);
                            opacity.push(entry.opacity);
                            scales.push(entry.scale);
                            rotations.push(entry.rotation);
                        }
                        for (chunk_index, chunk_range) in
                            (0..positions.len()).step_by(max_splats).enumerate()
                        {
                            let end = (chunk_range + max_splats).min(positions.len());
                            let splat_vertices = splat_billboard_vertices(SplatBillboardInputs {
                                positions: &positions[chunk_range..end],
                                colors: &colors[chunk_range..end],
                                opacities: &opacity[chunk_range..end],
                                scales: &scales[chunk_range..end],
                                rotations: &rotations[chunk_range..end],
                                view,
                                viewport: [width as f32, height as f32],
                                fov_y: 45_f32.to_radians(),
                                world_transform,
                            });
                            let count = splat_vertices.len() as u32;
                            if count == 0 {
                                continue;
                            }
                            let bytes = bytemuck::cast_slice(&splat_vertices);
                            let buffer = match pipeline.splat_buffers.get(chunk_index) {
                                Some(existing) if existing.size() >= bytes.len() as u64 => {
                                    queue.write_buffer(existing, 0, bytes);
                                    existing.clone()
                                }
                                _ => device.create_buffer_init(
                                    &egui_wgpu::wgpu::util::BufferInitDescriptor {
                                        label: Some("lobedo_splat_vertices"),
                                        contents: bytes,
                                        usage: egui_wgpu::wgpu::BufferUsages::VERTEX
                                            | egui_wgpu::wgpu::BufferUsages::COPY_DST,
                                    },
                                ),
                            };
                            buffers.push(buffer);
                            counts.push(count);
                        }
                    }
                    pipeline.splat_buffers = buffers;
                    pipeline.splat_counts = counts;
                    pipeline.splat_scissors = scissors;
                    pipeline.splat_point_size = 0.0;
                    pipeline.splat_last_right = right.to_array();
                    pipeline.splat_last_up = up.to_array();
                    pipeline.splat_last_camera_pos = camera_pos.to_array();
                    pipeline.splat_last_viewport = [width, height];
                    pipeline.last_splat_rebuild = Some(now);
                }
                if !pipeline.splat_buffers.is_empty() {
                    let splat_pipeline = if matches!(
                        self.debug.shading_mode,
                        ViewportShadingMode::SplatOverdraw
                    ) {
                        &pipeline.splat_overdraw_pipeline
                    } else {
                        &pipeline.splat_pipeline
                    };
                    render_pass.set_pipeline(splat_pipeline);
                    render_pass.set_bind_group(0, &pipeline.uniform_bind_group, &[]);
                    render_pass.set_bind_group(1, &pipeline.material_bind_group, &[]);
                    let use_scissors = !pipeline.splat_scissors.is_empty()
                        && pipeline.splat_scissors.len() == pipeline.splat_counts.len()
                        && pipeline.splat_scissors.len() == pipeline.splat_buffers.len();
                    if use_scissors {
                        for ((buffer, count), scissor) in pipeline
                            .splat_buffers
                            .iter()
                            .zip(pipeline.splat_counts.iter())
                            .zip(pipeline.splat_scissors.iter())
                        {
                            if *count == 0 {
                                continue;
                            }
                            render_pass.set_scissor_rect(
                                scissor[0],
                                scissor[1],
                                scissor[2],
                                scissor[3],
                            );
                            render_pass.set_vertex_buffer(0, buffer.slice(..));
                            render_pass.draw(0..*count, 0..1);
                        }
                        render_pass.set_scissor_rect(0, 0, width, height);
                    } else {
                        for (buffer, count) in pipeline
                            .splat_buffers
                            .iter()
                            .zip(pipeline.splat_counts.iter())
                        {
                            if *count == 0 {
                                continue;
                            }
                            render_pass.set_vertex_buffer(0, buffer.slice(..));
                            render_pass.draw(0..*count, 0..1);
                        }
                    }
                }
            }

            render_pass.set_pipeline(&pipeline.line_pipeline);
            render_pass.set_bind_group(0, &pipeline.uniform_bind_group, &[]);
            render_pass.set_bind_group(1, &pipeline.material_bind_group, &[]);

            if pipeline.template_count > 0 {
                render_pass.set_vertex_buffer(0, pipeline.template_buffer.slice(..));
                render_pass.draw(0..pipeline.template_count, 0..1);
            }

            if pipeline.curve_count > 0 {
                render_pass.set_vertex_buffer(0, pipeline.curve_buffer.slice(..));
                render_pass.draw(0..pipeline.curve_count, 0..1);
            }

            if pipeline.selection_count > 0 {
                render_pass.set_vertex_buffer(0, pipeline.selection_buffer.slice(..));
                render_pass.draw(0..pipeline.selection_count, 0..1);
            }

            if self.debug.show_grid && pipeline.grid_count > 0 {
                render_pass.set_vertex_buffer(0, pipeline.grid_buffer.slice(..));
                render_pass.draw(0..pipeline.grid_count, 0..1);
            }

            if self.debug.show_axes && pipeline.axes_count > 0 {
                render_pass.set_vertex_buffer(0, pipeline.axes_buffer.slice(..));
                render_pass.draw(0..pipeline.axes_count, 0..1);
            }

            if self.debug.show_normals && pipeline.normals_count > 0 {
                render_pass.set_vertex_buffer(0, pipeline.normals_buffer.slice(..));
                render_pass.draw(0..pipeline.normals_count, 0..1);
            }

            if self.debug.show_bounds && pipeline.bounds_count > 0 {
                render_pass.set_vertex_buffer(0, pipeline.bounds_buffer.slice(..));
                render_pass.draw(0..pipeline.bounds_count, 0..1);
            }
        }

        Vec::new()
    }

    fn paint(
        &self,
        info: egui::epaint::PaintCallbackInfo,
        render_pass: &mut egui_wgpu::wgpu::RenderPass<'static>,
        callback_resources: &CallbackResources,
    ) {
        let viewport = info.viewport_in_pixels();
        if viewport.width_px <= 0 || viewport.height_px <= 0 {
            return;
        }

        let clip = info.clip_rect_in_pixels();
        if clip.width_px <= 0 || clip.height_px <= 0 {
            return;
        }

        let Some(pipeline) = callback_resources.get::<PipelineState>() else {
            return;
        };

        render_pass.set_viewport(
            viewport.left_px as f32,
            viewport.top_px as f32,
            viewport.width_px as f32,
            viewport.height_px as f32,
            0.0,
            1.0,
        );
        render_pass.set_scissor_rect(
            clip.left_px.max(0) as u32,
            clip.top_px.max(0) as u32,
            clip.width_px.max(0) as u32,
            clip.height_px.max(0) as u32,
        );
        render_pass.set_pipeline(&pipeline.blit_pipeline);
        render_pass.set_bind_group(0, &pipeline.blit_bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
