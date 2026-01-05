use std::sync::{Arc, Mutex};

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use egui::epaint::Rect;
use egui_wgpu::wgpu::util::DeviceExt as _;
use egui_wgpu::{CallbackResources, CallbackTrait};

use super::callback_helpers::{light_view_projection, sort_splats_by_depth};
use super::mesh::{
    normals_vertices, point_cross_vertices, splat_billboard_vertices, SplatBillboardInputs,
};
use super::pipeline::{apply_scene_to_pipeline, ensure_offscreen_targets, PipelineState, Uniforms};
use super::{ViewportDebug, ViewportSceneState, ViewportShadingMode, ViewportStatsState};
use crate::camera::{camera_position, camera_view_proj, CameraState};
use glam::{Mat3, Mat4, Vec3};

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
            callback_resources.insert(PipelineState::new(device, self.target_format));
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
                        apply_scene_to_pipeline(device, pipeline, scene);
                        pipeline.base_color = scene.base_color;
                    }
                    None => {
                        pipeline.mesh_vertices.clear();
                        pipeline.index_count = 0;
                        pipeline.mesh_bounds = ([0.0; 3], [0.0; 3]);
                        pipeline.base_color = [0.7, 0.72, 0.75];
                        pipeline.template_count = 0;
                        pipeline.selection_count = 0;
                        pipeline.splat_positions.clear();
                        pipeline.splat_colors.clear();
                        pipeline.splat_opacity.clear();
                        pipeline.splat_scales.clear();
                        pipeline.splat_rotations.clear();
                        pipeline.splat_count = 0;
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
                    self.debug.depth_near,
                    self.debug.depth_far,
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
                    shadow_pass.set_bind_group(0, &pipeline.shadow_bind_group, &[]);
                    shadow_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    shadow_pass.set_index_buffer(
                        mesh.index_buffer.slice(..),
                        egui_wgpu::wgpu::IndexFormat::Uint32,
                    );
                    shadow_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
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
                    render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    render_pass.set_index_buffer(
                        mesh.index_buffer.slice(..),
                        egui_wgpu::wgpu::IndexFormat::Uint32,
                    );
                    render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                }
            }

            render_pass.set_pipeline(&pipeline.line_pipeline);
            render_pass.set_bind_group(0, &pipeline.uniform_bind_group, &[]);

            if self.debug.show_points || pipeline.index_count == 0 {
                let camera_distance = (camera_pos - Vec3::from(self.camera.target)).length();
                let desired_size =
                    (self.debug.point_size.max(1.0) * camera_distance * 0.002).clamp(0.0005, 2.0);
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
                    let sorted = sort_splats_by_depth(
                        &pipeline.splat_positions,
                        &pipeline.splat_colors,
                        &pipeline.splat_opacity,
                        &pipeline.splat_scales,
                        &pipeline.splat_rotations,
                        camera_pos,
                        forward,
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
                    let splat_vertices = splat_billboard_vertices(
                        SplatBillboardInputs {
                            positions: &positions,
                            colors: &colors,
                            opacities: &opacity,
                            scales: &scales,
                            rotations: &rotations,
                            view: Mat4::look_at_rh(camera_pos, target, Vec3::Y),
                            viewport: [width as f32, height as f32],
                            fov_y: 45_f32.to_radians(),
                            world_transform: Mat3::from_diagonal(Vec3::new(1.0, -1.0, 1.0)),
                        },
                    );
                    pipeline.splat_buffer =
                        device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
                            label: Some("lobedo_splat_vertices"),
                            contents: bytemuck::cast_slice(&splat_vertices),
                            usage: egui_wgpu::wgpu::BufferUsages::VERTEX,
                        });
                    pipeline.splat_count = splat_vertices.len() as u32;
                    pipeline.splat_point_size = 0.0;
                    pipeline.splat_last_right = right.to_array();
                    pipeline.splat_last_up = up.to_array();
                    pipeline.splat_last_camera_pos = camera_pos.to_array();
                    pipeline.splat_last_viewport = [width, height];
                    pipeline.last_splat_rebuild = Some(now);
                }
                if pipeline.splat_count > 0 {
                    render_pass.set_pipeline(&pipeline.splat_pipeline);
                    render_pass.set_bind_group(0, &pipeline.uniform_bind_group, &[]);
                    render_pass.set_vertex_buffer(0, pipeline.splat_buffer.slice(..));
                    render_pass.draw(0..pipeline.splat_count, 0..1);
                }
            }

            render_pass.set_pipeline(&pipeline.line_pipeline);
            render_pass.set_bind_group(0, &pipeline.uniform_bind_group, &[]);

            if pipeline.template_count > 0 {
                render_pass.set_vertex_buffer(0, pipeline.template_buffer.slice(..));
                render_pass.draw(0..pipeline.template_count, 0..1);
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
