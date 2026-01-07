use egui_wgpu::wgpu::util::DeviceExt as _;

use crate::scene::RenderScene;

use super::mesh::{
    bounds_from_positions, bounds_vertices, build_vertices, normals_vertices,
    selection_shape_vertices, wireframe_vertices,
};
use super::pipeline::{MaterialGpu, PipelineState};

pub(super) fn apply_scene_to_pipeline(
    device: &egui_wgpu::wgpu::Device,
    queue: &egui_wgpu::wgpu::Queue,
    pipeline: &mut PipelineState,
    scene: &RenderScene,
) {
    if let Some(mesh) = scene.mesh() {
        let (vertices, indices) = build_vertices(mesh);
        pipeline.mesh_cache.upload_or_update(
            device,
            pipeline.mesh_id,
            bytemuck::cast_slice(&vertices),
            std::mem::size_of::<super::mesh::Vertex>(),
            &indices,
        );

        pipeline.mesh_vertices = vertices;
        pipeline.index_count = indices.len() as u32;
        pipeline.point_count = pipeline.mesh_vertices.len() as u32;
        pipeline.point_positions = mesh.positions.clone();
        pipeline.point_size = -1.0;
        pipeline.mesh_bounds = bounds_from_positions(&mesh.positions);

        let normals_vertices = normals_vertices(&pipeline.mesh_vertices, pipeline.normals_length);
        pipeline.normals_buffer =
            device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
                label: Some("lobedo_normals_vertices"),
                contents: bytemuck::cast_slice(&normals_vertices),
                usage: egui_wgpu::wgpu::BufferUsages::VERTEX
                    | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            });
        pipeline.normals_count = normals_vertices.len() as u32;
    } else {
        pipeline.mesh_vertices.clear();
        pipeline.index_count = 0;
        pipeline.point_positions.clear();
        pipeline.point_count = 0;
        pipeline.point_size = -1.0;
        pipeline.normals_count = 0;
    }

    if let Some(splats) = scene.splats() {
        pipeline.splat_positions = splats.positions.clone();
        pipeline.splat_sh0 = splats.sh0.clone();
        pipeline.splat_sh_coeffs = splats.sh_coeffs;
        pipeline.splat_sh_rest = splats.sh_rest.clone();
        pipeline.splat_sh0_is_coeff = splats.sh0_is_coeff;
        pipeline.splat_opacity = splats.opacity.clone();
        pipeline.splat_scales = splats.scales.clone();
        pipeline.splat_rotations = splats.rotations.clone();
        pipeline.splat_point_size = -1.0;
        pipeline.splat_buffers.clear();
        pipeline.splat_counts.clear();
        pipeline.splat_scissors.clear();
        if pipeline.mesh_vertices.is_empty() {
            pipeline.mesh_bounds = bounds_from_positions(&pipeline.splat_positions);
        }
    } else {
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
    }

    if pipeline.mesh_vertices.is_empty() && pipeline.splat_positions.is_empty() {
        pipeline.mesh_bounds = ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
    }

    let bounds_vertices = bounds_vertices(pipeline.mesh_bounds.0, pipeline.mesh_bounds.1);
    pipeline.bounds_buffer =
        device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
            label: Some("lobedo_bounds_vertices"),
            contents: bytemuck::cast_slice(&bounds_vertices),
            usage: egui_wgpu::wgpu::BufferUsages::VERTEX,
        });
    pipeline.bounds_count = bounds_vertices.len() as u32;

    let template_lines = if let Some(template) = &scene.template_mesh {
        wireframe_vertices(&template.positions, &template.indices)
    } else {
        Vec::new()
    };
    if template_lines.is_empty() {
        pipeline.template_count = 0;
    } else {
        pipeline.template_buffer =
            device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
                label: Some("lobedo_template_vertices"),
                contents: bytemuck::cast_slice(&template_lines),
                usage: egui_wgpu::wgpu::BufferUsages::VERTEX
                    | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            });
        pipeline.template_count = template_lines.len() as u32;
    }

    let selection_lines = scene
        .selection_shape
        .as_ref()
        .map(selection_shape_vertices)
        .unwrap_or_default();
    if selection_lines.is_empty() {
        pipeline.selection_count = 0;
    } else {
        pipeline.selection_buffer =
            device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
                label: Some("lobedo_selection_vertices"),
                contents: bytemuck::cast_slice(&selection_lines),
                usage: egui_wgpu::wgpu::BufferUsages::VERTEX
                    | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            });
        pipeline.selection_count = selection_lines.len() as u32;
    }

    apply_materials_to_pipeline(device, queue, pipeline, scene);
}

fn apply_materials_to_pipeline(
    device: &egui_wgpu::wgpu::Device,
    queue: &egui_wgpu::wgpu::Queue,
    pipeline: &mut PipelineState,
    scene: &RenderScene,
) {
    let mut materials = Vec::new();
    if scene.materials.is_empty() {
        materials.push(MaterialGpu {
            base_color: [1.0, 1.0, 1.0, 0.0],
            params: [0.5, -1.0, 0.0, 0.0],
        });
    } else {
        for material in &scene.materials {
            let tex_index = material
                .base_color_texture
                .map(|idx| idx as f32)
                .unwrap_or(-1.0);
            materials.push(MaterialGpu {
                base_color: [
                    material.base_color[0],
                    material.base_color[1],
                    material.base_color[2],
                    material.metallic,
                ],
                params: [material.roughness.clamp(0.0, 1.0), tex_index, 0.0, 0.0],
            });
        }
    }
    pipeline.material_buffer = device.create_buffer_init(
        &egui_wgpu::wgpu::util::BufferInitDescriptor {
            label: Some("lobedo_materials"),
            contents: bytemuck::cast_slice(&materials),
            usage: egui_wgpu::wgpu::BufferUsages::STORAGE
                | egui_wgpu::wgpu::BufferUsages::COPY_DST,
        },
    );

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
    let mut active_texture = fallback_texture;
    if let Some(texture) = scene.textures.first() {
        let expected_len = texture
            .width
            .saturating_mul(texture.height)
            .saturating_mul(4) as usize;
        if texture.width > 0
            && texture.height > 0
            && texture.pixels.len() == expected_len
        {
            active_texture = device.create_texture_with_data(
                queue,
                &egui_wgpu::wgpu::TextureDescriptor {
                    label: Some("lobedo_material_texture"),
                    size: egui_wgpu::wgpu::Extent3d {
                        width: texture.width,
                        height: texture.height,
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
                texture.pixels.as_slice(),
            );
        }
    }
    let active_view = active_texture.create_view(&Default::default());
    pipeline.material_bind_group = device.create_bind_group(
        &egui_wgpu::wgpu::BindGroupDescriptor {
            label: Some("lobedo_viewport_material_bind_group"),
            layout: &pipeline.material_bind_group_layout,
            entries: &[
                egui_wgpu::wgpu::BindGroupEntry {
                    binding: 0,
                    resource: pipeline.material_buffer.as_entire_binding(),
                },
                egui_wgpu::wgpu::BindGroupEntry {
                    binding: 1,
                    resource: egui_wgpu::wgpu::BindingResource::Sampler(
                        &pipeline.material_sampler,
                    ),
                },
                egui_wgpu::wgpu::BindGroupEntry {
                    binding: 2,
                    resource: egui_wgpu::wgpu::BindingResource::TextureView(&active_view),
                },
            ],
        },
    );
    pipeline.material_texture = active_texture;
    pipeline.material_texture_view = active_view;
}
