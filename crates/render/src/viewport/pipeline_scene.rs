use egui_wgpu::wgpu::util::DeviceExt as _;

use crate::scene::RenderScene;

use super::mesh::{
    bounds_from_positions, bounds_vertices, build_vertices, normals_vertices,
    selection_shape_vertices, wireframe_vertices,
};
use super::pipeline::PipelineState;

pub(super) fn apply_scene_to_pipeline(
    device: &egui_wgpu::wgpu::Device,
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
        pipeline.splat_colors = splats.colors.clone();
        pipeline.splat_opacity = splats.opacity.clone();
        pipeline.splat_scales = splats.scales.clone();
        pipeline.splat_rotations = splats.rotations.clone();
        pipeline.splat_point_size = -1.0;
        pipeline.splat_buffers.clear();
        pipeline.splat_counts.clear();
        if pipeline.mesh_vertices.is_empty() {
            pipeline.mesh_bounds = bounds_from_positions(&pipeline.splat_positions);
        }
    } else {
        pipeline.splat_positions.clear();
        pipeline.splat_colors.clear();
        pipeline.splat_opacity.clear();
        pipeline.splat_scales.clear();
        pipeline.splat_rotations.clear();
        pipeline.splat_buffers.clear();
        pipeline.splat_counts.clear();
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
}
