use egui_wgpu::wgpu::util::DeviceExt as _;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use crate::mesh_cache::GpuMeshCache;

use super::mesh::{
    bounds_vertices, cube_mesh, grid_and_axes, normals_vertices, point_cross_vertices_color,
    splat_corner_vertices, LineVertex, SplatCorner, SplatInstance, Vertex, LINE_ATTRIBUTES,
    SPLAT_CORNER_ATTRIBUTES, SPLAT_INSTANCE_ATTRIBUTES, VERTEX_ATTRIBUTES,
};
use super::pipeline_shaders::{
    create_blit_shader, create_main_shader, create_splat_compute_shader,
};
use super::pipeline_targets::{create_offscreen_targets, create_shadow_targets};

pub(super) use super::pipeline_scene::apply_scene_to_pipeline;

pub(super) const DEPTH_FORMAT: egui_wgpu::wgpu::TextureFormat =
    egui_wgpu::wgpu::TextureFormat::Depth24Plus;
pub(super) const SPLAT_BUCKET_DEFAULT: u32 = 4096;
pub(super) const SPLAT_BUCKET_CHUNK: u32 = 256;
const SPLAT_INSTANCE_STRIDE_FLOATS: u32 = 14;
const SPLAT_INSTANCE_STRIDE_BYTES: u64 = SPLAT_INSTANCE_STRIDE_FLOATS as u64 * 4;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Uniforms {
    pub(super) view_proj: [[f32; 4]; 4],
    pub(super) inv_view_proj: [[f32; 4]; 4],
    pub(super) light_view_proj: [[f32; 4]; 4],
    pub(super) key_dir: [f32; 3],
    pub(super) _pad0: f32,
    pub(super) fill_dir: [f32; 3],
    pub(super) _pad1: f32,
    pub(super) rim_dir: [f32; 3],
    pub(super) _pad2: f32,
    pub(super) camera_pos: [f32; 3],
    pub(super) _pad3: f32,
    pub(super) base_color: [f32; 3],
    pub(super) _pad4: f32,
    pub(super) light_params: [f32; 4],
    pub(super) debug_params: [f32; 4],
    pub(super) shadow_params: [f32; 4],
    pub(super) splat_params: [f32; 4],
    pub(super) splat_view_x: [f32; 3],
    pub(super) _pad5: f32,
    pub(super) splat_view_y: [f32; 3],
    pub(super) _pad6: f32,
    pub(super) splat_view_z: [f32; 3],
    pub(super) _pad7: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct MaterialGpu {
    pub(super) base_color: [f32; 4],
    pub(super) params: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct VolumeParams {
    pub(super) origin: [f32; 3],
    pub(super) voxel_size: f32,
    pub(super) dims: [u32; 3],
    pub(super) kind: u32,
    pub(super) params: [f32; 4],
    pub(super) world_to_volume: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct SplatComputeParams {
    pub(super) splat_count: u32,
    pub(super) sh_coeffs: u32,
    pub(super) bucket_count: u32,
    pub(super) flags: u32,
    pub(super) near: f32,
    pub(super) far: f32,
    pub(super) vertex_count: u32,
    pub(super) _pad0: u32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct SplatGpuData {
    pub(super) pos_opacity: [f32; 4],
    pub(super) scale: [f32; 4],
    pub(super) rotation: [f32; 4],
    pub(super) sh0: [f32; 4],
}

pub(super) struct SplatGpuResources {
    pub(super) supported: bool,
    pub(super) capacity: u32,
    pub(super) count: u32,
    pub(super) sh_coeffs: u32,
    pub(super) bucket_capacity: u32,
    pub(super) data_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) sh_rest_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) instances_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) bucket_counts: egui_wgpu::wgpu::Buffer,
    pub(super) bucket_offsets: egui_wgpu::wgpu::Buffer,
    pub(super) chunk_sums: egui_wgpu::wgpu::Buffer,
    pub(super) chunk_offsets: egui_wgpu::wgpu::Buffer,
    pub(super) params_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) indirect_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) bind_group_layout: egui_wgpu::wgpu::BindGroupLayout,
    pub(super) bind_group: egui_wgpu::wgpu::BindGroup,
    pub(super) clear_pipeline: egui_wgpu::wgpu::ComputePipeline,
    pub(super) count_pipeline: egui_wgpu::wgpu::ComputePipeline,
    pub(super) prefix_local_pipeline: egui_wgpu::wgpu::ComputePipeline,
    pub(super) prefix_chunk_pipeline: egui_wgpu::wgpu::ComputePipeline,
    pub(super) prefix_add_pipeline: egui_wgpu::wgpu::ComputePipeline,
    pub(super) scatter_pipeline: egui_wgpu::wgpu::ComputePipeline,
}

pub(super) struct PipelineState {
    pub(super) mesh_pipeline: egui_wgpu::wgpu::RenderPipeline,
    pub(super) shadow_pipeline: egui_wgpu::wgpu::RenderPipeline,
    pub(super) line_pipeline: egui_wgpu::wgpu::RenderPipeline,
    pub(super) splat_pipeline: egui_wgpu::wgpu::RenderPipeline,
    pub(super) splat_depth_pipeline: egui_wgpu::wgpu::RenderPipeline,
    pub(super) splat_overdraw_pipeline: egui_wgpu::wgpu::RenderPipeline,
    pub(super) volume_pipeline: egui_wgpu::wgpu::RenderPipeline,
    pub(super) blit_pipeline: egui_wgpu::wgpu::RenderPipeline,
    pub(super) blit_bind_group: egui_wgpu::wgpu::BindGroup,
    pub(super) blit_bind_group_layout: egui_wgpu::wgpu::BindGroupLayout,
    pub(super) blit_sampler: egui_wgpu::wgpu::Sampler,
    pub(super) _shadow_texture: egui_wgpu::wgpu::Texture,
    pub(super) shadow_view: egui_wgpu::wgpu::TextureView,
    pub(super) _shadow_sampler: egui_wgpu::wgpu::Sampler,
    pub(super) _shadow_size: u32,
    pub(super) offscreen_texture: egui_wgpu::wgpu::Texture,
    pub(super) offscreen_view: egui_wgpu::wgpu::TextureView,
    pub(super) depth_texture: egui_wgpu::wgpu::Texture,
    pub(super) depth_view: egui_wgpu::wgpu::TextureView,
    pub(super) offscreen_size: [u32; 2],
    pub(super) uniform_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) uniform_bind_group: egui_wgpu::wgpu::BindGroup,
    pub(super) shadow_uniform_bind_group: egui_wgpu::wgpu::BindGroup,
    pub(super) material_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) material_bind_group: egui_wgpu::wgpu::BindGroup,
    pub(super) material_bind_group_layout: egui_wgpu::wgpu::BindGroupLayout,
    pub(super) material_sampler: egui_wgpu::wgpu::Sampler,
    pub(super) material_texture: egui_wgpu::wgpu::Texture,
    pub(super) material_texture_view: egui_wgpu::wgpu::TextureView,
    pub(super) volume_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) volume_bind_group: egui_wgpu::wgpu::BindGroup,
    pub(super) volume_bind_group_layout: egui_wgpu::wgpu::BindGroupLayout,
    pub(super) volume_texture: egui_wgpu::wgpu::Texture,
    pub(super) volume_view: egui_wgpu::wgpu::TextureView,
    pub(super) volume_present: bool,
    pub(super) mesh_cache: GpuMeshCache,
    pub(super) mesh_id: u64,
    pub(super) mesh_vertices: Vec<Vertex>,
    pub(super) point_positions: Vec<[f32; 3]>,
    pub(super) mesh_bounds: ([f32; 3], [f32; 3]),
    pub(super) index_count: u32,
    pub(super) point_count: u32,
    pub(super) point_size: f32,
    pub(super) point_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) splat_positions: Vec<[f32; 3]>,
    pub(super) splat_sh0: Vec<[f32; 3]>,
    pub(super) splat_sh_coeffs: usize,
    pub(super) splat_sh_rest: Vec<[f32; 3]>,
    pub(super) splat_sh0_is_coeff: bool,
    pub(super) splat_opacity: Vec<f32>,
    pub(super) splat_scales: Vec<[f32; 3]>,
    pub(super) splat_rotations: Vec<[f32; 4]>,
    pub(super) splat_base_colors: Vec<[f32; 3]>,
    pub(super) splat_color_cache_scene: u64,
    pub(super) splat_color_cache_len: usize,
    pub(super) splat_color_cache_sh0_is_coeff: bool,
    pub(super) splat_point_size: f32,
    pub(super) splat_corner_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) splat_corner_count: u32,
    pub(super) splat_instance_buffers: Vec<egui_wgpu::wgpu::Buffer>,
    pub(super) splat_instance_counts: Vec<u32>,
    pub(super) splat_scissors: Vec<[u32; 4]>,
    pub(super) splat_gpu: SplatGpuResources,
    pub(super) splat_last_right: [f32; 3],
    pub(super) splat_last_up: [f32; 3],
    pub(super) splat_last_camera_pos: [f32; 3],
    pub(super) splat_last_viewport: [u32; 2],
    pub(super) splat_last_bucket_count: u32,
    pub(super) splat_last_log_depth: bool,
    pub(super) splat_last_full_sh: bool,
    pub(super) scene_version: u64,
    pub(super) base_color: [f32; 3],
    pub(super) grid_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) grid_count: u32,
    pub(super) axes_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) axes_count: u32,
    pub(super) normals_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) normals_count: u32,
    pub(super) normals_length: f32,
    pub(super) has_normals: bool,
    pub(super) bounds_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) bounds_count: u32,
    pub(super) curve_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) curve_count: u32,
    pub(super) template_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) template_count: u32,
    pub(super) selection_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) selection_count: u32,
    pub(super) last_splat_rebuild: Option<Instant>,
}

impl SplatGpuResources {
    fn new(
        device: &egui_wgpu::wgpu::Device,
        uniform_buffer: &egui_wgpu::wgpu::Buffer,
    ) -> Self {
        let supported = !cfg!(target_arch = "wasm32");
        let shader = create_splat_compute_shader(device);
        let bind_group_layout =
            device.create_bind_group_layout(&egui_wgpu::wgpu::BindGroupLayoutDescriptor {
                label: Some("lobedo_viewport_splat_compute_layout"),
                entries: &[
                    egui_wgpu::wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: egui_wgpu::wgpu::ShaderStages::COMPUTE,
                        ty: egui_wgpu::wgpu::BindingType::Buffer {
                            ty: egui_wgpu::wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    egui_wgpu::wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: egui_wgpu::wgpu::ShaderStages::COMPUTE,
                        ty: egui_wgpu::wgpu::BindingType::Buffer {
                            ty: egui_wgpu::wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    egui_wgpu::wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: egui_wgpu::wgpu::ShaderStages::COMPUTE,
                        ty: egui_wgpu::wgpu::BindingType::Buffer {
                            ty: egui_wgpu::wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    egui_wgpu::wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: egui_wgpu::wgpu::ShaderStages::COMPUTE,
                        ty: egui_wgpu::wgpu::BindingType::Buffer {
                            ty: egui_wgpu::wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    egui_wgpu::wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: egui_wgpu::wgpu::ShaderStages::COMPUTE,
                        ty: egui_wgpu::wgpu::BindingType::Buffer {
                            ty: egui_wgpu::wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    egui_wgpu::wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: egui_wgpu::wgpu::ShaderStages::COMPUTE,
                        ty: egui_wgpu::wgpu::BindingType::Buffer {
                            ty: egui_wgpu::wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    egui_wgpu::wgpu::BindGroupLayoutEntry {
                        binding: 6,
                        visibility: egui_wgpu::wgpu::ShaderStages::COMPUTE,
                        ty: egui_wgpu::wgpu::BindingType::Buffer {
                            ty: egui_wgpu::wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    egui_wgpu::wgpu::BindGroupLayoutEntry {
                        binding: 7,
                        visibility: egui_wgpu::wgpu::ShaderStages::COMPUTE,
                        ty: egui_wgpu::wgpu::BindingType::Buffer {
                            ty: egui_wgpu::wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    egui_wgpu::wgpu::BindGroupLayoutEntry {
                        binding: 8,
                        visibility: egui_wgpu::wgpu::ShaderStages::COMPUTE,
                        ty: egui_wgpu::wgpu::BindingType::Buffer {
                            ty: egui_wgpu::wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    egui_wgpu::wgpu::BindGroupLayoutEntry {
                        binding: 9,
                        visibility: egui_wgpu::wgpu::ShaderStages::COMPUTE,
                        ty: egui_wgpu::wgpu::BindingType::Buffer {
                            ty: egui_wgpu::wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let pipeline_layout =
            device.create_pipeline_layout(&egui_wgpu::wgpu::PipelineLayoutDescriptor {
                label: Some("lobedo_viewport_splat_compute_pipeline_layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });
        let clear_pipeline =
            device.create_compute_pipeline(&egui_wgpu::wgpu::ComputePipelineDescriptor {
                label: Some("lobedo_viewport_splat_clear"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("cs_clear"),
                compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
        let count_pipeline =
            device.create_compute_pipeline(&egui_wgpu::wgpu::ComputePipelineDescriptor {
                label: Some("lobedo_viewport_splat_count"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("cs_count"),
                compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
        let prefix_local_pipeline =
            device.create_compute_pipeline(&egui_wgpu::wgpu::ComputePipelineDescriptor {
                label: Some("lobedo_viewport_splat_prefix_local"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("cs_prefix_local"),
                compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
        let prefix_chunk_pipeline =
            device.create_compute_pipeline(&egui_wgpu::wgpu::ComputePipelineDescriptor {
                label: Some("lobedo_viewport_splat_prefix_chunk"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("cs_prefix_chunk"),
                compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
        let prefix_add_pipeline =
            device.create_compute_pipeline(&egui_wgpu::wgpu::ComputePipelineDescriptor {
                label: Some("lobedo_viewport_splat_prefix_add"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("cs_prefix_add"),
                compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
        let scatter_pipeline =
            device.create_compute_pipeline(&egui_wgpu::wgpu::ComputePipelineDescriptor {
                label: Some("lobedo_viewport_splat_scatter"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("cs_scatter"),
                compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let params_buffer =
            device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
                label: Some("lobedo_splat_compute_params"),
                contents: bytemuck::bytes_of(&SplatComputeParams {
                    splat_count: 0,
                    sh_coeffs: 0,
                    bucket_count: SPLAT_BUCKET_DEFAULT,
                    flags: 0,
                    near: 0.02,
                    far: 20.0,
                    vertex_count: 0,
                    _pad0: 0,
                }),
                usage: egui_wgpu::wgpu::BufferUsages::UNIFORM
                    | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            });

        let empty_storage = |label: &str| {
            device.create_buffer(&egui_wgpu::wgpu::BufferDescriptor {
                label: Some(label),
                size: 16,
                usage: egui_wgpu::wgpu::BufferUsages::STORAGE
                    | egui_wgpu::wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };

        let data_buffer = empty_storage("lobedo_splat_gpu_data");
        let sh_rest_buffer = empty_storage("lobedo_splat_gpu_sh_rest");
        let instances_buffer = device.create_buffer(&egui_wgpu::wgpu::BufferDescriptor {
            label: Some("lobedo_splat_gpu_instances"),
            size: 16,
            usage: egui_wgpu::wgpu::BufferUsages::STORAGE
                | egui_wgpu::wgpu::BufferUsages::VERTEX
                | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bucket_counts = device.create_buffer(&egui_wgpu::wgpu::BufferDescriptor {
            label: Some("lobedo_splat_gpu_bucket_counts"),
            size: SPLAT_BUCKET_DEFAULT as u64 * 4,
            usage: egui_wgpu::wgpu::BufferUsages::STORAGE
                | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bucket_offsets = device.create_buffer(&egui_wgpu::wgpu::BufferDescriptor {
            label: Some("lobedo_splat_gpu_bucket_offsets"),
            size: SPLAT_BUCKET_DEFAULT as u64 * 4,
            usage: egui_wgpu::wgpu::BufferUsages::STORAGE
                | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let chunk_count = SPLAT_BUCKET_DEFAULT.div_ceil(SPLAT_BUCKET_CHUNK).max(1);
        let chunk_sums = device.create_buffer(&egui_wgpu::wgpu::BufferDescriptor {
            label: Some("lobedo_splat_gpu_chunk_sums"),
            size: chunk_count as u64 * 4,
            usage: egui_wgpu::wgpu::BufferUsages::STORAGE
                | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let chunk_offsets = device.create_buffer(&egui_wgpu::wgpu::BufferDescriptor {
            label: Some("lobedo_splat_gpu_chunk_offsets"),
            size: chunk_count as u64 * 4,
            usage: egui_wgpu::wgpu::BufferUsages::STORAGE
                | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let indirect_buffer =
            device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
                label: Some("lobedo_splat_gpu_indirect"),
                contents: bytemuck::cast_slice(&[0u32, 0u32, 0u32, 0u32]),
                usage: egui_wgpu::wgpu::BufferUsages::STORAGE
                    | egui_wgpu::wgpu::BufferUsages::INDIRECT
                    | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            });

        let bind_group =
            Self::build_bind_group(
                device,
                &bind_group_layout,
                uniform_buffer,
                &params_buffer,
                &data_buffer,
                &sh_rest_buffer,
                &bucket_counts,
                &bucket_offsets,
                &chunk_sums,
                &chunk_offsets,
                &instances_buffer,
                &indirect_buffer,
            );

        Self {
            supported,
            capacity: 0,
            count: 0,
            sh_coeffs: 0,
            bucket_capacity: SPLAT_BUCKET_DEFAULT,
            data_buffer,
            sh_rest_buffer,
            instances_buffer,
            bucket_counts,
            bucket_offsets,
            chunk_sums,
            chunk_offsets,
            params_buffer,
            indirect_buffer,
            bind_group_layout,
            bind_group,
            clear_pipeline,
            count_pipeline,
            prefix_local_pipeline,
            prefix_chunk_pipeline,
            prefix_add_pipeline,
            scatter_pipeline,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn build_bind_group(
        device: &egui_wgpu::wgpu::Device,
        layout: &egui_wgpu::wgpu::BindGroupLayout,
        uniform_buffer: &egui_wgpu::wgpu::Buffer,
        params_buffer: &egui_wgpu::wgpu::Buffer,
        data_buffer: &egui_wgpu::wgpu::Buffer,
        sh_rest_buffer: &egui_wgpu::wgpu::Buffer,
        bucket_counts: &egui_wgpu::wgpu::Buffer,
        bucket_offsets: &egui_wgpu::wgpu::Buffer,
        chunk_sums: &egui_wgpu::wgpu::Buffer,
        chunk_offsets: &egui_wgpu::wgpu::Buffer,
        instances_buffer: &egui_wgpu::wgpu::Buffer,
        indirect_buffer: &egui_wgpu::wgpu::Buffer,
    ) -> egui_wgpu::wgpu::BindGroup {
        device.create_bind_group(&egui_wgpu::wgpu::BindGroupDescriptor {
            label: Some("lobedo_viewport_splat_compute_group"),
            layout,
            entries: &[
                egui_wgpu::wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                egui_wgpu::wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
                egui_wgpu::wgpu::BindGroupEntry {
                    binding: 2,
                    resource: data_buffer.as_entire_binding(),
                },
                egui_wgpu::wgpu::BindGroupEntry {
                    binding: 3,
                    resource: sh_rest_buffer.as_entire_binding(),
                },
                egui_wgpu::wgpu::BindGroupEntry {
                    binding: 4,
                    resource: bucket_counts.as_entire_binding(),
                },
                egui_wgpu::wgpu::BindGroupEntry {
                    binding: 5,
                    resource: bucket_offsets.as_entire_binding(),
                },
                egui_wgpu::wgpu::BindGroupEntry {
                    binding: 6,
                    resource: chunk_sums.as_entire_binding(),
                },
                egui_wgpu::wgpu::BindGroupEntry {
                    binding: 7,
                    resource: chunk_offsets.as_entire_binding(),
                },
                egui_wgpu::wgpu::BindGroupEntry {
                    binding: 8,
                    resource: instances_buffer.as_entire_binding(),
                },
                egui_wgpu::wgpu::BindGroupEntry {
                    binding: 9,
                    resource: indirect_buffer.as_entire_binding(),
                },
            ],
        })
    }
}

impl PipelineState {
    pub(super) fn new(
        device: &egui_wgpu::wgpu::Device,
        queue: &egui_wgpu::wgpu::Queue,
        target_format: egui_wgpu::wgpu::TextureFormat,
    ) -> Self {
        let shader = create_main_shader(device);

        let uniform_buffer =
            device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
                label: Some("lobedo_viewport_uniforms"),
                contents: bytemuck::bytes_of(&Uniforms {
                    view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
                    inv_view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
                    light_view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
                    key_dir: [0.6, 1.0, 0.2],
                    _pad0: 0.0,
                    fill_dir: [-0.4, 0.4, 0.2],
                    _pad1: 0.0,
                    rim_dir: [0.0, 0.6, -0.8],
                    _pad2: 0.0,
                    camera_pos: [0.0, 0.0, 5.0],
                    _pad3: 0.0,
                    base_color: [0.7, 0.72, 0.75],
                    _pad4: 0.0,
                    light_params: [1.0, 0.4, 0.5, 0.15],
                    debug_params: [0.0, 0.5, 20.0, 4.0],
                    shadow_params: [0.0, 0.002, 0.0, 0.0],
                    splat_params: [1.0, 1.0, 45_f32.to_radians(), 0.02],
                    splat_view_x: [1.0, 0.0, 0.0],
                    _pad5: 0.0,
                    splat_view_y: [0.0, 1.0, 0.0],
                    _pad6: 0.0,
                    splat_view_z: [0.0, 0.0, 1.0],
                    _pad7: 0.0,
                }),
                usage: egui_wgpu::wgpu::BufferUsages::UNIFORM
                    | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            });

        let uniform_layout =
            device.create_bind_group_layout(&egui_wgpu::wgpu::BindGroupLayoutDescriptor {
                label: Some("lobedo_viewport_uniform_layout"),
                entries: &[
                    egui_wgpu::wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: egui_wgpu::wgpu::ShaderStages::VERTEX
                            | egui_wgpu::wgpu::ShaderStages::FRAGMENT,
                        ty: egui_wgpu::wgpu::BindingType::Buffer {
                            ty: egui_wgpu::wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    egui_wgpu::wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: egui_wgpu::wgpu::ShaderStages::FRAGMENT,
                        ty: egui_wgpu::wgpu::BindingType::Texture {
                            sample_type: egui_wgpu::wgpu::TextureSampleType::Depth,
                            view_dimension: egui_wgpu::wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    egui_wgpu::wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: egui_wgpu::wgpu::ShaderStages::FRAGMENT,
                        ty: egui_wgpu::wgpu::BindingType::Sampler(
                            egui_wgpu::wgpu::SamplerBindingType::Comparison,
                        ),
                        count: None,
                    },
                ],
            });
        let shadow_uniform_layout =
            device.create_bind_group_layout(&egui_wgpu::wgpu::BindGroupLayoutDescriptor {
                label: Some("lobedo_viewport_shadow_uniform_layout"),
                entries: &[egui_wgpu::wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: egui_wgpu::wgpu::ShaderStages::VERTEX,
                    ty: egui_wgpu::wgpu::BindingType::Buffer {
                        ty: egui_wgpu::wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let material_bind_group_layout =
            device.create_bind_group_layout(&egui_wgpu::wgpu::BindGroupLayoutDescriptor {
                label: Some("lobedo_viewport_material_layout"),
                entries: &[
                    egui_wgpu::wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: egui_wgpu::wgpu::ShaderStages::VERTEX
                            | egui_wgpu::wgpu::ShaderStages::FRAGMENT,
                        ty: egui_wgpu::wgpu::BindingType::Buffer {
                            ty: egui_wgpu::wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    egui_wgpu::wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: egui_wgpu::wgpu::ShaderStages::FRAGMENT,
                        ty: egui_wgpu::wgpu::BindingType::Sampler(
                            egui_wgpu::wgpu::SamplerBindingType::Filtering,
                        ),
                        count: None,
                    },
                    egui_wgpu::wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: egui_wgpu::wgpu::ShaderStages::FRAGMENT,
                        ty: egui_wgpu::wgpu::BindingType::Texture {
                            sample_type: egui_wgpu::wgpu::TextureSampleType::Float {
                                filterable: true,
                            },
                            view_dimension: egui_wgpu::wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });
        let volume_bind_group_layout =
            device.create_bind_group_layout(&egui_wgpu::wgpu::BindGroupLayoutDescriptor {
                label: Some("lobedo_viewport_volume_layout"),
                entries: &[
                    egui_wgpu::wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: egui_wgpu::wgpu::ShaderStages::FRAGMENT,
                        ty: egui_wgpu::wgpu::BindingType::Buffer {
                            ty: egui_wgpu::wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    egui_wgpu::wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: egui_wgpu::wgpu::ShaderStages::FRAGMENT,
                        ty: egui_wgpu::wgpu::BindingType::Texture {
                            sample_type: egui_wgpu::wgpu::TextureSampleType::Float {
                                filterable: false,
                            },
                            view_dimension: egui_wgpu::wgpu::TextureViewDimension::D3,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });

        let shadow_size = 1024;
        let (shadow_texture, shadow_view) = create_shadow_targets(device, shadow_size);
        let shadow_sampler = device.create_sampler(&egui_wgpu::wgpu::SamplerDescriptor {
            label: Some("lobedo_shadow_sampler"),
            address_mode_u: egui_wgpu::wgpu::AddressMode::ClampToEdge,
            address_mode_v: egui_wgpu::wgpu::AddressMode::ClampToEdge,
            address_mode_w: egui_wgpu::wgpu::AddressMode::ClampToEdge,
            mag_filter: egui_wgpu::wgpu::FilterMode::Linear,
            min_filter: egui_wgpu::wgpu::FilterMode::Linear,
            compare: Some(egui_wgpu::wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        let material_sampler = device.create_sampler(&egui_wgpu::wgpu::SamplerDescriptor {
            label: Some("lobedo_material_sampler"),
            mag_filter: egui_wgpu::wgpu::FilterMode::Linear,
            min_filter: egui_wgpu::wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let default_material = MaterialGpu {
            base_color: [1.0, 1.0, 1.0, 0.0],
            params: [0.5, -1.0, 1.0, 1.0],
        };
        let material_buffer =
            device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
                label: Some("lobedo_materials"),
                contents: bytemuck::cast_slice(&[default_material]),
                usage: egui_wgpu::wgpu::BufferUsages::STORAGE
                    | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            });
        let fallback_texture = device.create_texture_with_data(
            queue,
            &egui_wgpu::wgpu::TextureDescriptor {
                label: Some("lobedo_material_fallback"),
                size: egui_wgpu::wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: egui_wgpu::wgpu::TextureDimension::D2,
                format: egui_wgpu::wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: egui_wgpu::wgpu::TextureUsages::TEXTURE_BINDING
                    | egui_wgpu::wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            egui_wgpu::wgpu::util::TextureDataOrder::LayerMajor,
            &[255, 255, 255, 255],
        );
        let material_texture_view = fallback_texture.create_view(
            &egui_wgpu::wgpu::TextureViewDescriptor {
                dimension: Some(egui_wgpu::wgpu::TextureViewDimension::D2Array),
                array_layer_count: Some(1),
                ..Default::default()
            },
        );

        let uniform_bind_group = device.create_bind_group(&egui_wgpu::wgpu::BindGroupDescriptor {
            label: Some("lobedo_viewport_uniform_bind_group"),
            layout: &uniform_layout,
            entries: &[
                egui_wgpu::wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                egui_wgpu::wgpu::BindGroupEntry {
                    binding: 1,
                    resource: egui_wgpu::wgpu::BindingResource::TextureView(&shadow_view),
                },
                egui_wgpu::wgpu::BindGroupEntry {
                    binding: 2,
                    resource: egui_wgpu::wgpu::BindingResource::Sampler(&shadow_sampler),
                },
            ],
        });
        let shadow_uniform_bind_group =
            device.create_bind_group(&egui_wgpu::wgpu::BindGroupDescriptor {
                label: Some("lobedo_viewport_shadow_uniform_bind_group"),
                layout: &shadow_uniform_layout,
                entries: &[egui_wgpu::wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }],
            });
        let material_bind_group = {
            device.create_bind_group(&egui_wgpu::wgpu::BindGroupDescriptor {
                label: Some("lobedo_viewport_material_bind_group"),
                layout: &material_bind_group_layout,
                entries: &[
                    egui_wgpu::wgpu::BindGroupEntry {
                        binding: 0,
                        resource: material_buffer.as_entire_binding(),
                    },
                    egui_wgpu::wgpu::BindGroupEntry {
                        binding: 1,
                        resource: egui_wgpu::wgpu::BindingResource::Sampler(&material_sampler),
                    },
                    egui_wgpu::wgpu::BindGroupEntry {
                        binding: 2,
                        resource: egui_wgpu::wgpu::BindingResource::TextureView(
                            &material_texture_view,
                        ),
                    },
                ],
            })
        };

        let splat_gpu = SplatGpuResources::new(device, &uniform_buffer);

        let volume_buffer = device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
            label: Some("lobedo_volume_params"),
            contents: bytemuck::bytes_of(&VolumeParams {
                origin: [0.0, 0.0, 0.0],
                voxel_size: 1.0,
                dims: [0, 0, 0],
                kind: 0,
                params: [1.0, 1.0, 1.0, 0.0],
                world_to_volume: glam::Mat4::IDENTITY.to_cols_array_2d(),
            }),
            usage: egui_wgpu::wgpu::BufferUsages::UNIFORM | egui_wgpu::wgpu::BufferUsages::COPY_DST,
        });
        let volume_texture = device.create_texture_with_data(
            queue,
            &egui_wgpu::wgpu::TextureDescriptor {
                label: Some("lobedo_volume_texture"),
                size: egui_wgpu::wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: egui_wgpu::wgpu::TextureDimension::D3,
                format: egui_wgpu::wgpu::TextureFormat::R32Float,
                usage: egui_wgpu::wgpu::TextureUsages::TEXTURE_BINDING
                    | egui_wgpu::wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            egui_wgpu::wgpu::util::TextureDataOrder::LayerMajor,
            bytemuck::cast_slice(&[0.0f32]),
        );
        let volume_view = volume_texture.create_view(&egui_wgpu::wgpu::TextureViewDescriptor {
            dimension: Some(egui_wgpu::wgpu::TextureViewDimension::D3),
            ..Default::default()
        });
        let volume_bind_group = device.create_bind_group(&egui_wgpu::wgpu::BindGroupDescriptor {
            label: Some("lobedo_volume_bind_group"),
            layout: &volume_bind_group_layout,
            entries: &[
                egui_wgpu::wgpu::BindGroupEntry {
                    binding: 0,
                    resource: volume_buffer.as_entire_binding(),
                },
                egui_wgpu::wgpu::BindGroupEntry {
                    binding: 1,
                    resource: egui_wgpu::wgpu::BindingResource::TextureView(&volume_view),
                },
            ],
        });

        let pipeline_layout =
            device.create_pipeline_layout(&egui_wgpu::wgpu::PipelineLayoutDescriptor {
                label: Some("lobedo_viewport_layout"),
                bind_group_layouts: &[&uniform_layout, &material_bind_group_layout],
                push_constant_ranges: &[],
            });
        let volume_pipeline_layout =
            device.create_pipeline_layout(&egui_wgpu::wgpu::PipelineLayoutDescriptor {
                label: Some("lobedo_viewport_volume_layout"),
                bind_group_layouts: &[&uniform_layout, &material_bind_group_layout, &volume_bind_group_layout],
                push_constant_ranges: &[],
            });
        let shadow_pipeline_layout =
            device.create_pipeline_layout(&egui_wgpu::wgpu::PipelineLayoutDescriptor {
                label: Some("lobedo_viewport_shadow_layout"),
                bind_group_layouts: &[&shadow_uniform_layout],
                push_constant_ranges: &[],
            });

        let mesh_pipeline =
            device.create_render_pipeline(&egui_wgpu::wgpu::RenderPipelineDescriptor {
                label: Some("lobedo_viewport_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: egui_wgpu::wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                    buffers: &[egui_wgpu::wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>()
                            as egui_wgpu::wgpu::BufferAddress,
                        step_mode: egui_wgpu::wgpu::VertexStepMode::Vertex,
                        attributes: &VERTEX_ATTRIBUTES,
                    }],
                },
                fragment: Some(egui_wgpu::wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(egui_wgpu::wgpu::ColorTargetState {
                        format: target_format,
                        blend: Some(egui_wgpu::wgpu::BlendState::REPLACE),
                        write_mask: egui_wgpu::wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: egui_wgpu::wgpu::PrimitiveState {
                    topology: egui_wgpu::wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: Some(egui_wgpu::wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: egui_wgpu::wgpu::CompareFunction::LessEqual,
                    stencil: egui_wgpu::wgpu::StencilState::default(),
                    bias: egui_wgpu::wgpu::DepthBiasState::default(),
                }),
                multisample: egui_wgpu::wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let shadow_pipeline =
            device.create_render_pipeline(&egui_wgpu::wgpu::RenderPipelineDescriptor {
                label: Some("lobedo_viewport_shadow"),
                layout: Some(&shadow_pipeline_layout),
                vertex: egui_wgpu::wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_shadow"),
                    compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                    buffers: &[egui_wgpu::wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>()
                            as egui_wgpu::wgpu::BufferAddress,
                        step_mode: egui_wgpu::wgpu::VertexStepMode::Vertex,
                        attributes: &VERTEX_ATTRIBUTES,
                    }],
                },
                fragment: None,
                primitive: egui_wgpu::wgpu::PrimitiveState {
                    topology: egui_wgpu::wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: Some(egui_wgpu::wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: Some(egui_wgpu::wgpu::DepthStencilState {
                    format: egui_wgpu::wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: egui_wgpu::wgpu::CompareFunction::LessEqual,
                    stencil: egui_wgpu::wgpu::StencilState::default(),
                    bias: egui_wgpu::wgpu::DepthBiasState {
                        constant: 1,
                        slope_scale: 1.0,
                        clamp: 0.0,
                    },
                }),
                multisample: egui_wgpu::wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let line_pipeline =
            device.create_render_pipeline(&egui_wgpu::wgpu::RenderPipelineDescriptor {
                label: Some("lobedo_viewport_lines"),
                layout: Some(&pipeline_layout),
                vertex: egui_wgpu::wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_line"),
                    compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                    buffers: &[egui_wgpu::wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<LineVertex>()
                            as egui_wgpu::wgpu::BufferAddress,
                        step_mode: egui_wgpu::wgpu::VertexStepMode::Vertex,
                        attributes: &LINE_ATTRIBUTES,
                    }],
                },
                fragment: Some(egui_wgpu::wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_line"),
                    compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(egui_wgpu::wgpu::ColorTargetState {
                        format: target_format,
                        blend: Some(egui_wgpu::wgpu::BlendState::REPLACE),
                        write_mask: egui_wgpu::wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: egui_wgpu::wgpu::PrimitiveState {
                    topology: egui_wgpu::wgpu::PrimitiveTopology::LineList,
                    ..Default::default()
                },
                depth_stencil: Some(egui_wgpu::wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: false,
                    depth_compare: egui_wgpu::wgpu::CompareFunction::LessEqual,
                    stencil: egui_wgpu::wgpu::StencilState::default(),
                    bias: egui_wgpu::wgpu::DepthBiasState::default(),
                }),
                multisample: egui_wgpu::wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let splat_depth_pipeline =
            device.create_render_pipeline(&egui_wgpu::wgpu::RenderPipelineDescriptor {
                label: Some("lobedo_viewport_splats_depth"),
                layout: Some(&pipeline_layout),
                vertex: egui_wgpu::wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_splat"),
                    compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                    buffers: &[
                        egui_wgpu::wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<SplatCorner>()
                                as egui_wgpu::wgpu::BufferAddress,
                            step_mode: egui_wgpu::wgpu::VertexStepMode::Vertex,
                            attributes: &SPLAT_CORNER_ATTRIBUTES,
                        },
                        egui_wgpu::wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<SplatInstance>()
                                as egui_wgpu::wgpu::BufferAddress,
                            step_mode: egui_wgpu::wgpu::VertexStepMode::Instance,
                            attributes: &SPLAT_INSTANCE_ATTRIBUTES,
                        },
                    ],
                },
                fragment: Some(egui_wgpu::wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_splat"),
                    compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(egui_wgpu::wgpu::ColorTargetState {
                        format: target_format,
                        blend: None,
                        write_mask: egui_wgpu::wgpu::ColorWrites::empty(),
                    })],
                }),
                primitive: egui_wgpu::wgpu::PrimitiveState {
                    topology: egui_wgpu::wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(egui_wgpu::wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: egui_wgpu::wgpu::CompareFunction::LessEqual,
                    stencil: egui_wgpu::wgpu::StencilState::default(),
                    bias: egui_wgpu::wgpu::DepthBiasState::default(),
                }),
                multisample: egui_wgpu::wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let splat_pipeline =
            device.create_render_pipeline(&egui_wgpu::wgpu::RenderPipelineDescriptor {
                label: Some("lobedo_viewport_splats"),
                layout: Some(&pipeline_layout),
                vertex: egui_wgpu::wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_splat"),
                    compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                    buffers: &[
                        egui_wgpu::wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<SplatCorner>()
                                as egui_wgpu::wgpu::BufferAddress,
                            step_mode: egui_wgpu::wgpu::VertexStepMode::Vertex,
                            attributes: &SPLAT_CORNER_ATTRIBUTES,
                        },
                        egui_wgpu::wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<SplatInstance>()
                                as egui_wgpu::wgpu::BufferAddress,
                            step_mode: egui_wgpu::wgpu::VertexStepMode::Instance,
                            attributes: &SPLAT_INSTANCE_ATTRIBUTES,
                        },
                    ],
                },
                fragment: Some(egui_wgpu::wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_splat"),
                    compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(egui_wgpu::wgpu::ColorTargetState {
                        format: target_format,
                        blend: Some(egui_wgpu::wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: egui_wgpu::wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: egui_wgpu::wgpu::PrimitiveState {
                    topology: egui_wgpu::wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(egui_wgpu::wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: false,
                    depth_compare: egui_wgpu::wgpu::CompareFunction::LessEqual,
                    stencil: egui_wgpu::wgpu::StencilState::default(),
                    bias: egui_wgpu::wgpu::DepthBiasState::default(),
                }),
                multisample: egui_wgpu::wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let splat_overdraw_pipeline =
            device.create_render_pipeline(&egui_wgpu::wgpu::RenderPipelineDescriptor {
                label: Some("lobedo_viewport_splats_overdraw"),
                layout: Some(&pipeline_layout),
                vertex: egui_wgpu::wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_splat"),
                    compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                    buffers: &[
                        egui_wgpu::wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<SplatCorner>()
                                as egui_wgpu::wgpu::BufferAddress,
                            step_mode: egui_wgpu::wgpu::VertexStepMode::Vertex,
                            attributes: &SPLAT_CORNER_ATTRIBUTES,
                        },
                        egui_wgpu::wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<SplatInstance>()
                                as egui_wgpu::wgpu::BufferAddress,
                            step_mode: egui_wgpu::wgpu::VertexStepMode::Instance,
                            attributes: &SPLAT_INSTANCE_ATTRIBUTES,
                        },
                    ],
                },
                fragment: Some(egui_wgpu::wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_splat"),
                    compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(egui_wgpu::wgpu::ColorTargetState {
                        format: target_format,
                        blend: Some(egui_wgpu::wgpu::BlendState {
                            color: egui_wgpu::wgpu::BlendComponent {
                                src_factor: egui_wgpu::wgpu::BlendFactor::One,
                                dst_factor: egui_wgpu::wgpu::BlendFactor::One,
                                operation: egui_wgpu::wgpu::BlendOperation::Add,
                            },
                            alpha: egui_wgpu::wgpu::BlendComponent {
                                src_factor: egui_wgpu::wgpu::BlendFactor::One,
                                dst_factor: egui_wgpu::wgpu::BlendFactor::One,
                                operation: egui_wgpu::wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: egui_wgpu::wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: egui_wgpu::wgpu::PrimitiveState {
                    topology: egui_wgpu::wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(egui_wgpu::wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: false,
                    depth_compare: egui_wgpu::wgpu::CompareFunction::LessEqual,
                    stencil: egui_wgpu::wgpu::StencilState::default(),
                    bias: egui_wgpu::wgpu::DepthBiasState::default(),
                }),
                multisample: egui_wgpu::wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let volume_pipeline =
            device.create_render_pipeline(&egui_wgpu::wgpu::RenderPipelineDescriptor {
                label: Some("lobedo_viewport_volume"),
                layout: Some(&volume_pipeline_layout),
                vertex: egui_wgpu::wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_volume"),
                    compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                fragment: Some(egui_wgpu::wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_volume"),
                    compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(egui_wgpu::wgpu::ColorTargetState {
                        format: target_format,
                        blend: Some(egui_wgpu::wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: egui_wgpu::wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: egui_wgpu::wgpu::PrimitiveState {
                    topology: egui_wgpu::wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: Some(egui_wgpu::wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: false,
                    depth_compare: egui_wgpu::wgpu::CompareFunction::LessEqual,
                    stencil: egui_wgpu::wgpu::StencilState::default(),
                    bias: egui_wgpu::wgpu::DepthBiasState::default(),
                }),
                multisample: egui_wgpu::wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let blit_shader = create_blit_shader(device);

        let blit_bind_group_layout =
            device.create_bind_group_layout(&egui_wgpu::wgpu::BindGroupLayoutDescriptor {
                label: Some("lobedo_viewport_blit_layout"),
                entries: &[
                    egui_wgpu::wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: egui_wgpu::wgpu::ShaderStages::FRAGMENT,
                        ty: egui_wgpu::wgpu::BindingType::Texture {
                            sample_type: egui_wgpu::wgpu::TextureSampleType::Float {
                                filterable: true,
                            },
                            view_dimension: egui_wgpu::wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    egui_wgpu::wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: egui_wgpu::wgpu::ShaderStages::FRAGMENT,
                        ty: egui_wgpu::wgpu::BindingType::Sampler(
                            egui_wgpu::wgpu::SamplerBindingType::Filtering,
                        ),
                        count: None,
                    },
                ],
            });

        let blit_sampler = device.create_sampler(&egui_wgpu::wgpu::SamplerDescriptor {
            label: Some("lobedo_viewport_blit_sampler"),
            mag_filter: egui_wgpu::wgpu::FilterMode::Linear,
            min_filter: egui_wgpu::wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let blit_pipeline_layout =
            device.create_pipeline_layout(&egui_wgpu::wgpu::PipelineLayoutDescriptor {
                label: Some("lobedo_viewport_blit_pipeline_layout"),
                bind_group_layouts: &[&blit_bind_group_layout],
                push_constant_ranges: &[],
            });

        let blit_pipeline =
            device.create_render_pipeline(&egui_wgpu::wgpu::RenderPipelineDescriptor {
                label: Some("lobedo_viewport_blit_pipeline"),
                layout: Some(&blit_pipeline_layout),
                vertex: egui_wgpu::wgpu::VertexState {
                    module: &blit_shader,
                    entry_point: Some("vs_blit"),
                    compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                fragment: Some(egui_wgpu::wgpu::FragmentState {
                    module: &blit_shader,
                    entry_point: Some("fs_blit"),
                    compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(egui_wgpu::wgpu::ColorTargetState {
                        format: target_format,
                        blend: Some(egui_wgpu::wgpu::BlendState::REPLACE),
                        write_mask: egui_wgpu::wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: egui_wgpu::wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: egui_wgpu::wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let (offscreen_texture, offscreen_view, depth_texture, depth_view) =
            create_offscreen_targets(device, target_format, 1, 1);
        let blit_bind_group = device.create_bind_group(&egui_wgpu::wgpu::BindGroupDescriptor {
            label: Some("lobedo_viewport_blit_group"),
            layout: &blit_bind_group_layout,
            entries: &[
                egui_wgpu::wgpu::BindGroupEntry {
                    binding: 0,
                    resource: egui_wgpu::wgpu::BindingResource::TextureView(&offscreen_view),
                },
                egui_wgpu::wgpu::BindGroupEntry {
                    binding: 1,
                    resource: egui_wgpu::wgpu::BindingResource::Sampler(&blit_sampler),
                },
            ],
        });

        let mesh = cube_mesh();
        let mut mesh_cache = GpuMeshCache::new();
        let mesh_id = 1;
        mesh_cache.upload_or_update(
            device,
            mesh_id,
            bytemuck::cast_slice(&mesh.vertices),
            std::mem::size_of::<Vertex>(),
            &mesh.indices,
        );
        let index_count = mesh.indices.len() as u32;
        let normals_length = 0.3;
        let normals_vertices = normals_vertices(&mesh.vertices, normals_length);
        let has_normals = !normals_vertices.is_empty();
        let normals_buffer =
            device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
                label: Some("lobedo_normals_vertices"),
                contents: bytemuck::cast_slice(&normals_vertices),
                usage: egui_wgpu::wgpu::BufferUsages::VERTEX
                    | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            });
        let bounds_vertices = bounds_vertices(mesh.bounds_min, mesh.bounds_max);
        let bounds_buffer =
            device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
                label: Some("lobedo_bounds_vertices"),
                contents: bytemuck::cast_slice(&bounds_vertices),
                usage: egui_wgpu::wgpu::BufferUsages::VERTEX,
            });
        let curve_buffer =
            device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
                label: Some("lobedo_curve_vertices"),
                contents: bytemuck::cast_slice(&[LineVertex {
                    position: [0.0, 0.0, 0.0],
                    color: [0.0, 0.0, 0.0],
                }]),
                usage: egui_wgpu::wgpu::BufferUsages::VERTEX
                    | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            });
        let template_buffer =
            device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
                label: Some("lobedo_template_vertices"),
                contents: bytemuck::cast_slice(&[LineVertex {
                    position: [0.0, 0.0, 0.0],
                    color: [0.0, 0.0, 0.0],
                }]),
                usage: egui_wgpu::wgpu::BufferUsages::VERTEX
                    | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            });
        let selection_buffer =
            device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
                label: Some("lobedo_selection_vertices"),
                contents: bytemuck::cast_slice(&[LineVertex {
                    position: [0.0, 0.0, 0.0],
                    color: [0.0, 0.0, 0.0],
                }]),
                usage: egui_wgpu::wgpu::BufferUsages::VERTEX
                    | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            });
        let (grid_vertices, axes_vertices) = grid_and_axes();
        let point_count = mesh.vertices.len() as u32;
        let point_positions: Vec<[f32; 3]> = mesh.vertices.iter().map(|v| v.position).collect();
        let point_size = 0.1;
        let point_lines =
            point_cross_vertices_color(&point_positions, point_size, [1.0, 0.9, 0.2]);
        let point_buffer =
            device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
                label: Some("lobedo_point_vertices"),
                contents: bytemuck::cast_slice(&point_lines),
                usage: egui_wgpu::wgpu::BufferUsages::VERTEX
                    | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            });
        let grid_buffer = device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
            label: Some("lobedo_grid_vertices"),
            contents: bytemuck::cast_slice(&grid_vertices),
            usage: egui_wgpu::wgpu::BufferUsages::VERTEX,
        });
        let axes_buffer = device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
            label: Some("lobedo_axes_vertices"),
            contents: bytemuck::cast_slice(&axes_vertices),
            usage: egui_wgpu::wgpu::BufferUsages::VERTEX,
        });
        let splat_corner_vertices = splat_corner_vertices();
        let splat_corner_buffer =
            device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
                label: Some("lobedo_splat_corners"),
                contents: bytemuck::cast_slice(&splat_corner_vertices),
                usage: egui_wgpu::wgpu::BufferUsages::VERTEX,
            });
        let splat_corner_count = splat_corner_vertices.len() as u32;

        Self {
            mesh_pipeline,
            shadow_pipeline,
            line_pipeline,
            splat_pipeline,
            splat_depth_pipeline,
            splat_overdraw_pipeline,
            volume_pipeline,
            blit_pipeline,
            blit_bind_group,
            blit_bind_group_layout,
            blit_sampler,
            _shadow_texture: shadow_texture,
            shadow_view,
            _shadow_sampler: shadow_sampler,
            _shadow_size: shadow_size,
            offscreen_texture,
            offscreen_view,
            depth_texture,
            depth_view,
            offscreen_size: [1, 1],
            uniform_buffer,
            uniform_bind_group,
            shadow_uniform_bind_group,
            material_buffer,
            material_bind_group,
            material_bind_group_layout,
            material_sampler,
            material_texture: fallback_texture,
            material_texture_view,
            volume_buffer,
            volume_bind_group,
            volume_bind_group_layout,
            volume_texture,
            volume_view,
            volume_present: false,
            mesh_cache,
            mesh_id,
            mesh_vertices: mesh.vertices,
            point_positions,
            mesh_bounds: (mesh.bounds_min, mesh.bounds_max),
            index_count,
            point_count,
            point_size,
            point_buffer,
            splat_positions: Vec::new(),
            splat_sh0: Vec::new(),
            splat_sh_coeffs: 0,
            splat_sh_rest: Vec::new(),
            splat_sh0_is_coeff: false,
            splat_opacity: Vec::new(),
            splat_scales: Vec::new(),
            splat_rotations: Vec::new(),
            splat_base_colors: Vec::new(),
            splat_color_cache_scene: 0,
            splat_color_cache_len: 0,
            splat_color_cache_sh0_is_coeff: false,
            splat_point_size: -1.0,
            splat_corner_buffer,
            splat_corner_count,
            splat_instance_buffers: Vec::new(),
            splat_instance_counts: Vec::new(),
            splat_scissors: Vec::new(),
            splat_gpu,
            splat_last_right: [0.0, 0.0, 0.0],
            splat_last_up: [0.0, 0.0, 0.0],
            splat_last_camera_pos: [0.0, 0.0, 0.0],
            splat_last_viewport: [0, 0],
            splat_last_bucket_count: SPLAT_BUCKET_DEFAULT,
            splat_last_log_depth: true,
            splat_last_full_sh: true,
            scene_version: 0,
            base_color: [0.7, 0.72, 0.75],
            grid_buffer,
            grid_count: grid_vertices.len() as u32,
            axes_buffer,
            axes_count: axes_vertices.len() as u32,
            normals_buffer,
            normals_count: if has_normals {
                normals_vertices.len() as u32
            } else {
                0
            },
            normals_length,
            has_normals,
            bounds_buffer,
            bounds_count: bounds_vertices.len() as u32,
            curve_buffer,
            curve_count: 0,
            template_buffer,
            template_count: 0,
            selection_buffer,
            selection_count: 0,
            last_splat_rebuild: None,
        }
    }

    pub(super) fn ensure_splat_gpu_buffers(
        &mut self,
        device: &egui_wgpu::wgpu::Device,
        queue: &egui_wgpu::wgpu::Queue,
    ) {
        if !self.splat_gpu.supported {
            return;
        }
        let count = self.splat_positions.len() as u32;
        self.splat_gpu.count = count;
        if count == 0 {
            return;
        }
        let requested_sh_coeffs = self.splat_sh_coeffs as u32;
        let max_buffer_size = device.limits().max_buffer_size.max(16);
        let max_by_data =
            (max_buffer_size / std::mem::size_of::<SplatGpuData>() as u64).max(1);
        let max_by_instances = (max_buffer_size / SPLAT_INSTANCE_STRIDE_BYTES).max(1);
        let mut capacity = count.next_power_of_two();
        if capacity == 0 {
            capacity = 1;
        }
        let max_capacity = max_by_data
            .min(max_by_instances)
            .min(u32::MAX as u64) as u32;
        if capacity > max_capacity {
            capacity = max_capacity.max(1);
        }
        let sh_rest_slots_max = if capacity == 0 {
            0
        } else {
            max_buffer_size / (capacity as u64 * std::mem::size_of::<[f32; 4]>() as u64)
        } as u32;
        let sh_coeffs =
            Self::select_supported_sh_coeffs(requested_sh_coeffs, sh_rest_slots_max);
        if sh_coeffs < requested_sh_coeffs {
            eprintln!(
                "Viewport splat GPU: clamped SH coeffs from {requested_sh_coeffs} to {sh_coeffs} to fit max buffer size ({max_buffer_size} bytes)."
            );
        }
        let needs_realloc =
            capacity > self.splat_gpu.capacity || sh_coeffs != self.splat_gpu.sh_coeffs;
        if needs_realloc {
            self.splat_gpu.capacity = capacity;
            self.splat_gpu.sh_coeffs = sh_coeffs;
            let data_size = capacity as u64 * std::mem::size_of::<SplatGpuData>() as u64;
            self.splat_gpu.data_buffer = device.create_buffer(&egui_wgpu::wgpu::BufferDescriptor {
                label: Some("lobedo_splat_gpu_data"),
                size: data_size.max(16),
                usage: egui_wgpu::wgpu::BufferUsages::STORAGE
                    | egui_wgpu::wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let sh_rest_len = if sh_coeffs > 0 {
                capacity as u64 * sh_coeffs as u64
            } else {
                1
            };
            let sh_rest_size = sh_rest_len
                .saturating_mul(std::mem::size_of::<[f32; 4]>() as u64)
                .max(16);
            self.splat_gpu.sh_rest_buffer =
                device.create_buffer(&egui_wgpu::wgpu::BufferDescriptor {
                    label: Some("lobedo_splat_gpu_sh_rest"),
                    size: sh_rest_size,
                    usage: egui_wgpu::wgpu::BufferUsages::STORAGE
                        | egui_wgpu::wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            let instance_size = capacity as u64 * SPLAT_INSTANCE_STRIDE_BYTES;
            self.splat_gpu.instances_buffer =
                device.create_buffer(&egui_wgpu::wgpu::BufferDescriptor {
                    label: Some("lobedo_splat_gpu_instances"),
                    size: instance_size.max(16),
                    usage: egui_wgpu::wgpu::BufferUsages::STORAGE
                        | egui_wgpu::wgpu::BufferUsages::VERTEX
                        | egui_wgpu::wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            self.splat_gpu.bind_group = SplatGpuResources::build_bind_group(
                device,
                &self.splat_gpu.bind_group_layout,
                &self.uniform_buffer,
                &self.splat_gpu.params_buffer,
                &self.splat_gpu.data_buffer,
                &self.splat_gpu.sh_rest_buffer,
                &self.splat_gpu.bucket_counts,
                &self.splat_gpu.bucket_offsets,
                &self.splat_gpu.chunk_sums,
                &self.splat_gpu.chunk_offsets,
                &self.splat_gpu.instances_buffer,
                &self.splat_gpu.indirect_buffer,
            );
        }
        if count > self.splat_gpu.capacity {
            // Too many splats for GPU path on this device; callback will fallback to CPU splat path.
            return;
        }

        let mut data = Vec::with_capacity(count as usize);
        for idx in 0..count as usize {
            let position = self
                .splat_positions
                .get(idx)
                .copied()
                .unwrap_or([0.0, 0.0, 0.0]);
            let opacity = self.splat_opacity.get(idx).copied().unwrap_or(1.0);
            let scale = self
                .splat_scales
                .get(idx)
                .copied()
                .unwrap_or([1.0, 1.0, 1.0]);
            let rotation = self
                .splat_rotations
                .get(idx)
                .copied()
                .unwrap_or([0.0, 0.0, 0.0, 1.0]);
            let sh0 = self.splat_sh0.get(idx).copied().unwrap_or([1.0, 1.0, 1.0]);
            data.push(SplatGpuData {
                pos_opacity: [position[0], position[1], position[2], opacity],
                scale: [scale[0], scale[1], scale[2], 0.0],
                rotation,
                sh0: [sh0[0], sh0[1], sh0[2], 0.0],
            });
        }
        if !data.is_empty() {
            queue.write_buffer(
                &self.splat_gpu.data_buffer,
                0,
                bytemuck::cast_slice(&data),
            );
        }

        if sh_coeffs > 0 {
            let coeffs = sh_coeffs as usize;
            let expected = count as usize * coeffs;
            let mut rest = Vec::with_capacity(expected);
            for idx in 0..count as usize {
                let base = idx.saturating_mul(coeffs);
                for c in 0..coeffs {
                    let value = self
                        .splat_sh_rest
                        .get(base + c)
                        .copied()
                        .unwrap_or([0.0, 0.0, 0.0]);
                    rest.push([value[0], value[1], value[2], 0.0]);
                }
            }
            if !rest.is_empty() {
                queue.write_buffer(
                    &self.splat_gpu.sh_rest_buffer,
                    0,
                    bytemuck::cast_slice(&rest),
                );
            }
        }
    }

    fn select_supported_sh_coeffs(requested: u32, max_fit: u32) -> u32 {
        let capped = requested.min(max_fit);
        if requested >= 15 && capped >= 15 {
            15
        } else if requested >= 8 && capped >= 8 {
            8
        } else if requested >= 3 && capped >= 3 {
            3
        } else {
            0
        }
    }

    pub(super) fn ensure_splat_gpu_bucket_capacity(
        &mut self,
        device: &egui_wgpu::wgpu::Device,
        bucket_count: u32,
    ) {
        if !self.splat_gpu.supported {
            return;
        }
        let desired = bucket_count.max(1);
        if desired <= self.splat_gpu.bucket_capacity {
            return;
        }
        let mut capacity = desired.next_power_of_two();
        if capacity == 0 {
            capacity = SPLAT_BUCKET_DEFAULT;
        }
        self.splat_gpu.bucket_capacity = capacity;
        self.splat_gpu.bucket_counts = device.create_buffer(&egui_wgpu::wgpu::BufferDescriptor {
            label: Some("lobedo_splat_gpu_bucket_counts"),
            size: capacity as u64 * 4,
            usage: egui_wgpu::wgpu::BufferUsages::STORAGE
                | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.splat_gpu.bucket_offsets = device.create_buffer(&egui_wgpu::wgpu::BufferDescriptor {
            label: Some("lobedo_splat_gpu_bucket_offsets"),
            size: capacity as u64 * 4,
            usage: egui_wgpu::wgpu::BufferUsages::STORAGE
                | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let chunk_count = capacity.div_ceil(SPLAT_BUCKET_CHUNK).max(1);
        self.splat_gpu.chunk_sums = device.create_buffer(&egui_wgpu::wgpu::BufferDescriptor {
            label: Some("lobedo_splat_gpu_chunk_sums"),
            size: chunk_count as u64 * 4,
            usage: egui_wgpu::wgpu::BufferUsages::STORAGE
                | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.splat_gpu.chunk_offsets = device.create_buffer(&egui_wgpu::wgpu::BufferDescriptor {
            label: Some("lobedo_splat_gpu_chunk_offsets"),
            size: chunk_count as u64 * 4,
            usage: egui_wgpu::wgpu::BufferUsages::STORAGE
                | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.splat_gpu.bind_group = SplatGpuResources::build_bind_group(
            device,
            &self.splat_gpu.bind_group_layout,
            &self.uniform_buffer,
            &self.splat_gpu.params_buffer,
            &self.splat_gpu.data_buffer,
            &self.splat_gpu.sh_rest_buffer,
            &self.splat_gpu.bucket_counts,
            &self.splat_gpu.bucket_offsets,
            &self.splat_gpu.chunk_sums,
            &self.splat_gpu.chunk_offsets,
            &self.splat_gpu.instances_buffer,
            &self.splat_gpu.indirect_buffer,
        );
    }
}

pub(super) fn ensure_offscreen_targets(
    device: &egui_wgpu::wgpu::Device,
    pipeline: &mut PipelineState,
    target_format: egui_wgpu::wgpu::TextureFormat,
    width: u32,
    height: u32,
) {
    let width = width.max(1);
    let height = height.max(1);
    if pipeline.offscreen_size == [width, height] {
        return;
    }

    let (offscreen_texture, offscreen_view, depth_texture, depth_view) =
        create_offscreen_targets(device, target_format, width, height);
    pipeline.offscreen_texture = offscreen_texture;
    pipeline.offscreen_view = offscreen_view;
    pipeline.depth_texture = depth_texture;
    pipeline.depth_view = depth_view;
    pipeline.offscreen_size = [width, height];
    pipeline.blit_bind_group = device.create_bind_group(&egui_wgpu::wgpu::BindGroupDescriptor {
        label: Some("lobedo_viewport_blit_group"),
        layout: &pipeline.blit_bind_group_layout,
        entries: &[
            egui_wgpu::wgpu::BindGroupEntry {
                binding: 0,
                resource: egui_wgpu::wgpu::BindingResource::TextureView(&pipeline.offscreen_view),
            },
            egui_wgpu::wgpu::BindGroupEntry {
                binding: 1,
                resource: egui_wgpu::wgpu::BindingResource::Sampler(&pipeline.blit_sampler),
            },
        ],
    });
}
