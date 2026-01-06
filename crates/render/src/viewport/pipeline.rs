use egui_wgpu::wgpu::util::DeviceExt as _;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use crate::mesh_cache::GpuMeshCache;

use super::mesh::{
    bounds_vertices, cube_mesh, grid_and_axes, normals_vertices, point_cross_vertices, LineVertex,
    SplatVertex, Vertex, LINE_ATTRIBUTES, SPLAT_ATTRIBUTES, VERTEX_ATTRIBUTES,
};
use super::pipeline_shaders::{create_blit_shader, create_main_shader};
use super::pipeline_targets::{create_offscreen_targets, create_shadow_targets};

pub(super) use super::pipeline_scene::apply_scene_to_pipeline;

pub(super) const DEPTH_FORMAT: egui_wgpu::wgpu::TextureFormat =
    egui_wgpu::wgpu::TextureFormat::Depth24Plus;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Uniforms {
    pub(super) view_proj: [[f32; 4]; 4],
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
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct MaterialGpu {
    pub(super) base_color: [f32; 4],
    pub(super) params: [f32; 4],
}

pub(super) struct PipelineState {
    pub(super) mesh_pipeline: egui_wgpu::wgpu::RenderPipeline,
    pub(super) shadow_pipeline: egui_wgpu::wgpu::RenderPipeline,
    pub(super) line_pipeline: egui_wgpu::wgpu::RenderPipeline,
    pub(super) splat_pipeline: egui_wgpu::wgpu::RenderPipeline,
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
    pub(super) material_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) material_bind_group: egui_wgpu::wgpu::BindGroup,
    pub(super) material_bind_group_layout: egui_wgpu::wgpu::BindGroupLayout,
    pub(super) material_sampler: egui_wgpu::wgpu::Sampler,
    pub(super) material_texture: egui_wgpu::wgpu::Texture,
    pub(super) material_texture_view: egui_wgpu::wgpu::TextureView,
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
    pub(super) splat_colors: Vec<[f32; 3]>,
    pub(super) splat_opacity: Vec<f32>,
    pub(super) splat_scales: Vec<[f32; 3]>,
    pub(super) splat_rotations: Vec<[f32; 4]>,
    pub(super) splat_point_size: f32,
    pub(super) splat_buffers: Vec<egui_wgpu::wgpu::Buffer>,
    pub(super) splat_counts: Vec<u32>,
    pub(super) splat_last_right: [f32; 3],
    pub(super) splat_last_up: [f32; 3],
    pub(super) splat_last_camera_pos: [f32; 3],
    pub(super) splat_last_viewport: [u32; 2],
    pub(super) scene_version: u64,
    pub(super) base_color: [f32; 3],
    pub(super) grid_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) grid_count: u32,
    pub(super) axes_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) axes_count: u32,
    pub(super) normals_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) normals_count: u32,
    pub(super) normals_length: f32,
    pub(super) bounds_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) bounds_count: u32,
    pub(super) template_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) template_count: u32,
    pub(super) selection_buffer: egui_wgpu::wgpu::Buffer,
    pub(super) selection_count: u32,
    pub(super) last_splat_rebuild: Option<Instant>,
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
                            view_dimension: egui_wgpu::wgpu::TextureViewDimension::D2,
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
            params: [0.5, -1.0, 0.0, 0.0],
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
        let material_texture_view = fallback_texture.create_view(&Default::default());

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

        let pipeline_layout =
            device.create_pipeline_layout(&egui_wgpu::wgpu::PipelineLayoutDescriptor {
                label: Some("lobedo_viewport_layout"),
                bind_group_layouts: &[&uniform_layout, &material_bind_group_layout],
                push_constant_ranges: &[],
            });
        let shadow_pipeline_layout =
            device.create_pipeline_layout(&egui_wgpu::wgpu::PipelineLayoutDescriptor {
                label: Some("lobedo_viewport_shadow_layout"),
                bind_group_layouts: &[&uniform_layout, &material_bind_group_layout],
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
                    cull_mode: Some(egui_wgpu::wgpu::Face::Front),
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

        let splat_pipeline =
            device.create_render_pipeline(&egui_wgpu::wgpu::RenderPipelineDescriptor {
                label: Some("lobedo_viewport_splats"),
                layout: Some(&pipeline_layout),
                vertex: egui_wgpu::wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_splat"),
                    compilation_options: egui_wgpu::wgpu::PipelineCompilationOptions::default(),
                    buffers: &[egui_wgpu::wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<SplatVertex>()
                            as egui_wgpu::wgpu::BufferAddress,
                        step_mode: egui_wgpu::wgpu::VertexStepMode::Vertex,
                        attributes: &SPLAT_ATTRIBUTES,
                    }],
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
        let point_lines = point_cross_vertices(&point_positions, point_size);
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

        Self {
            mesh_pipeline,
            shadow_pipeline,
            line_pipeline,
            splat_pipeline,
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
            material_buffer,
            material_bind_group,
            material_bind_group_layout,
            material_sampler,
            material_texture: fallback_texture,
            material_texture_view,
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
            splat_colors: Vec::new(),
            splat_opacity: Vec::new(),
            splat_scales: Vec::new(),
            splat_rotations: Vec::new(),
            splat_point_size: -1.0,
            splat_buffers: Vec::new(),
            splat_counts: Vec::new(),
            splat_last_right: [0.0, 0.0, 0.0],
            splat_last_up: [0.0, 0.0, 0.0],
            splat_last_camera_pos: [0.0, 0.0, 0.0],
            splat_last_viewport: [0, 0],
            scene_version: 0,
            base_color: [0.7, 0.72, 0.75],
            grid_buffer,
            grid_count: grid_vertices.len() as u32,
            axes_buffer,
            axes_count: axes_vertices.len() as u32,
            normals_buffer,
            normals_count: normals_vertices.len() as u32,
            normals_length,
            bounds_buffer,
            bounds_count: bounds_vertices.len() as u32,
            template_buffer,
            template_count: 0,
            selection_buffer,
            selection_count: 0,
            last_splat_rebuild: None,
        }
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
