use egui_wgpu::wgpu::util::DeviceExt as _;

use crate::scene::{RenderDrawable, RenderScene, RenderSplats};

use super::mesh::{
    bounds_from_positions, bounds_vertices, build_vertices, curve_vertices,
    normals_vertices, point_cross_vertices_with_colors, selection_shape_vertices,
    wireframe_vertices, wireframe_vertices_ngon,
};
use super::pipeline::{MaterialGpu, PipelineState, VolumeParams};
use glam::Vec3;

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

        let has_normals = !mesh.normals.is_empty()
            || mesh
                .corner_normals
                .as_ref()
                .map(|n| !n.is_empty())
                .unwrap_or(false);
        pipeline.has_normals = has_normals;
        if has_normals {
            let normals_vertices =
                normals_vertices(&pipeline.mesh_vertices, pipeline.normals_length);
            pipeline.normals_buffer =
                device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
                    label: Some("lobedo_normals_vertices"),
                    contents: bytemuck::cast_slice(&normals_vertices),
                    usage: egui_wgpu::wgpu::BufferUsages::VERTEX
                        | egui_wgpu::wgpu::BufferUsages::COPY_DST,
                });
            pipeline.normals_count = normals_vertices.len() as u32;
        } else {
            pipeline.normals_count = 0;
        }
    } else {
        pipeline.mesh_vertices.clear();
        pipeline.index_count = 0;
        pipeline.point_positions.clear();
        pipeline.point_count = 0;
        pipeline.point_size = -1.0;
        pipeline.normals_count = 0;
        pipeline.has_normals = false;
    }

    let mut curve_lines = Vec::new();
    let mut curve_positions = Vec::new();
    for curve in scene.curves() {
        curve_lines.extend(curve_vertices(&curve.points, curve.closed));
        curve_positions.extend(curve.points.iter().copied());
    }

    if let Some(splats) = merged_scene_splats(scene) {
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

    let volume_bounds = apply_volume_to_pipeline(device, queue, pipeline, scene.volume());

    if pipeline.mesh_vertices.is_empty() && pipeline.splat_positions.is_empty() {
        if curve_positions.is_empty() {
            pipeline.mesh_bounds = ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
        } else {
            pipeline.mesh_bounds = bounds_from_positions(&curve_positions);
            if pipeline.point_positions.is_empty() {
                pipeline.point_positions = curve_positions.clone();
                pipeline.point_count = pipeline.point_positions.len() as u32;
                pipeline.point_size = -1.0;
            }
        }
    } else if !curve_positions.is_empty() {
        let (curve_min, curve_max) = bounds_from_positions(&curve_positions);
        for i in 0..3 {
            pipeline.mesh_bounds.0[i] = pipeline.mesh_bounds.0[i].min(curve_min[i]);
            pipeline.mesh_bounds.1[i] = pipeline.mesh_bounds.1[i].max(curve_max[i]);
        }
    }

    if let Some((vol_min, vol_max)) = volume_bounds {
        if pipeline.mesh_vertices.is_empty()
            && pipeline.splat_positions.is_empty()
            && curve_positions.is_empty()
        {
            pipeline.mesh_bounds = (vol_min, vol_max);
        } else {
            for i in 0..3 {
                pipeline.mesh_bounds.0[i] = pipeline.mesh_bounds.0[i].min(vol_min[i]);
                pipeline.mesh_bounds.1[i] = pipeline.mesh_bounds.1[i].max(vol_max[i]);
            }
        }
    }

    if curve_lines.is_empty() {
        pipeline.curve_count = 0;
    } else {
        pipeline.curve_buffer =
            device.create_buffer_init(&egui_wgpu::wgpu::util::BufferInitDescriptor {
                label: Some("lobedo_curve_vertices"),
                contents: bytemuck::cast_slice(&curve_lines),
                usage: egui_wgpu::wgpu::BufferUsages::VERTEX
                    | egui_wgpu::wgpu::BufferUsages::COPY_DST,
            });
        pipeline.curve_count = curve_lines.len() as u32;
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
        if !template.poly_indices.is_empty() {
            wireframe_vertices_ngon(
                &template.positions,
                &template.poly_indices,
                &template.poly_face_counts,
            )
        } else if !template.indices.is_empty() {
            wireframe_vertices(&template.positions, &template.indices)
        } else if !template.positions.is_empty() {
            let (min, max) = bounds_from_positions(&template.positions);
            let diag = (Vec3::from(max) - Vec3::from(min)).length();
            let size = (diag * 0.01).max(0.0005);
            let colors = vec![[0.6, 0.6, 0.6]; template.positions.len()];
            point_cross_vertices_with_colors(&template.positions, &colors, size)
        } else {
            Vec::new()
        }
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

fn merged_scene_splats(scene: &RenderScene) -> Option<RenderSplats> {
    let splats: Vec<&RenderSplats> = scene
        .drawables
        .iter()
        .filter_map(|drawable| match drawable {
            RenderDrawable::Splats(splats) => Some(splats),
            _ => None,
        })
        .collect();
    if splats.is_empty() {
        return None;
    }
    if splats.len() == 1 {
        return Some(splats[0].clone());
    }

    let total: usize = splats.iter().map(|s| s.positions.len()).sum();
    let max_coeffs = splats
        .iter()
        .map(|s| s.sh_coeffs)
        .max()
        .unwrap_or(0);
    let sh0_is_coeff = splats.iter().any(|s| s.sh0_is_coeff || s.sh_coeffs > 0);

    let mut merged = RenderSplats {
        positions: Vec::with_capacity(total),
        sh0: Vec::with_capacity(total),
        sh_coeffs: max_coeffs,
        sh_rest: Vec::with_capacity(total * max_coeffs),
        sh0_is_coeff,
        opacity: Vec::with_capacity(total),
        scales: Vec::with_capacity(total),
        rotations: Vec::with_capacity(total),
    };

    for splat in splats {
        let count = splat.positions.len();
        merged.positions.extend_from_slice(&splat.positions);
        merged.sh0.extend_from_slice(&splat.sh0);
        merged.opacity.extend_from_slice(&splat.opacity);
        merged.scales.extend_from_slice(&splat.scales);
        merged.rotations.extend_from_slice(&splat.rotations);

        if max_coeffs == 0 {
            continue;
        }
        let coeffs = splat.sh_coeffs;
        if coeffs == 0 {
            merged
                .sh_rest
                .extend(std::iter::repeat_n([0.0, 0.0, 0.0], count * max_coeffs));
        } else {
            for i in 0..count {
                let base = i * coeffs;
                for c in 0..max_coeffs {
                    let value = if c < coeffs {
                        splat.sh_rest[base + c]
                    } else {
                        [0.0, 0.0, 0.0]
                    };
                    merged.sh_rest.push(value);
                }
            }
        }
    }

    Some(merged)
}

fn apply_materials_to_pipeline(
    device: &egui_wgpu::wgpu::Device,
    queue: &egui_wgpu::wgpu::Queue,
    pipeline: &mut PipelineState,
    scene: &RenderScene,
) {
    let mut materials = Vec::new();
    let max_layers = device.limits().max_texture_array_layers.max(1) as usize;
    let available_layers = scene.textures.len().min(max_layers);
    if scene.textures.len() > max_layers {
        eprintln!(
            "texture limit ({max_layers}) exceeded; skipping {} texture(s)",
            scene.textures.len() - max_layers
        );
    }
    let max_dim = device.limits().max_texture_dimension_2d.max(1);
    let (mut texture_width, mut texture_height) = (1u32, 1u32);
    let mut valid_textures = vec![false; available_layers.max(1)];
    if available_layers > 0 {
        for (idx, texture) in scene.textures.iter().take(available_layers).enumerate() {
            let expected_len = texture
                .width
                .saturating_mul(texture.height)
                .saturating_mul(4) as usize;
            let valid = texture.width > 0
                && texture.height > 0
                && texture.width <= max_dim
                && texture.height <= max_dim
                && texture.pixels.len() == expected_len;
            valid_textures[idx] = valid;
            if !valid {
                eprintln!(
                    "texture {idx} invalid or too large ({}x{}, {} bytes)",
                    texture.width,
                    texture.height,
                    texture.pixels.len()
                );
            }
            if valid {
                texture_width = texture_width.max(texture.width);
                texture_height = texture_height.max(texture.height);
            }
        }
    }
    let mut uv_scales = vec![[1.0, 1.0]; available_layers.max(1)];
    for (idx, texture) in scene.textures.iter().take(available_layers).enumerate() {
        if valid_textures.get(idx).copied().unwrap_or(false) {
            uv_scales[idx] = [
                texture.width as f32 / texture_width as f32,
                texture.height as f32 / texture_height as f32,
            ];
        }
    }
    if scene.materials.is_empty() {
        materials.push(MaterialGpu {
            base_color: [1.0, 1.0, 1.0, 0.0],
            params: [0.5, -1.0, 1.0, 1.0],
        });
    } else {
        for material in &scene.materials {
            let tex_index = material
                .base_color_texture
                .filter(|idx| {
                    *idx < available_layers && valid_textures.get(*idx).copied().unwrap_or(false)
                })
                .map(|idx| idx as f32)
                .unwrap_or(-1.0);
            let uv_scale = material
                .base_color_texture
                .and_then(|idx| uv_scales.get(idx).copied())
                .unwrap_or([1.0, 1.0]);
            materials.push(MaterialGpu {
                base_color: [
                    material.base_color[0],
                    material.base_color[1],
                    material.base_color[2],
                    material.metallic,
                ],
                params: [
                    material.roughness.clamp(0.0, 1.0),
                    tex_index,
                    uv_scale[0],
                    uv_scale[1],
                ],
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
    let mut active_layers = 1;
    if available_layers > 0 {
        let layer_count = available_layers.max(1);
        let layer_stride = texture_width
            .saturating_mul(texture_height)
            .saturating_mul(4) as usize;
        let mut pixels = vec![255u8; layer_stride.saturating_mul(layer_count)];
        for (layer, texture) in scene.textures.iter().take(layer_count).enumerate() {
            if !valid_textures.get(layer).copied().unwrap_or(false) {
                continue;
            }
            let max_width = texture_width as usize;
            let tex_width = texture.width as usize;
            let tex_height = texture.height as usize;
            let layer_offset = layer * layer_stride;
            for y in 0..tex_height {
                let src_offset = y * tex_width * 4;
                let dst_offset = layer_offset + y * max_width * 4;
                let row_len = tex_width * 4;
                pixels[dst_offset..dst_offset + row_len]
                    .copy_from_slice(&texture.pixels[src_offset..src_offset + row_len]);
            }
        }
        active_texture = device.create_texture_with_data(
            queue,
            &egui_wgpu::wgpu::TextureDescriptor {
                label: Some("lobedo_material_texture_array"),
                size: egui_wgpu::wgpu::Extent3d {
                    width: texture_width.max(1),
                    height: texture_height.max(1),
                    depth_or_array_layers: layer_count as u32,
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
            pixels.as_slice(),
        );
        active_layers = layer_count as u32;
    }
    let active_view = active_texture.create_view(&egui_wgpu::wgpu::TextureViewDescriptor {
        dimension: Some(egui_wgpu::wgpu::TextureViewDimension::D2Array),
        array_layer_count: Some(active_layers),
        ..Default::default()
    });
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

fn apply_volume_to_pipeline(
    device: &egui_wgpu::wgpu::Device,
    queue: &egui_wgpu::wgpu::Queue,
    pipeline: &mut PipelineState,
    volume: Option<&crate::scene::RenderVolume>,
) -> Option<([f32; 3], [f32; 3])> {
    let Some(volume) = volume else {
        pipeline.volume_present = false;
        let params = empty_volume_params();
        queue.write_buffer(&pipeline.volume_buffer, 0, bytemuck::bytes_of(&params));
        return None;
    };

    let dims = volume.dims;
    let total = dims[0] as u64 * dims[1] as u64 * dims[2] as u64;
    if total == 0 || volume.values.len() as u64 != total {
        pipeline.volume_present = false;
        let params = empty_volume_params();
        queue.write_buffer(&pipeline.volume_buffer, 0, bytemuck::bytes_of(&params));
        return None;
    }

    let max_dim = device.limits().max_texture_dimension_3d.max(1);
    if dims[0] > max_dim || dims[1] > max_dim || dims[2] > max_dim {
        eprintln!(
            "volume dims exceed GPU limit ({}x{}x{} > {})",
            dims[0], dims[1], dims[2], max_dim
        );
        pipeline.volume_present = false;
        let params = empty_volume_params();
        queue.write_buffer(&pipeline.volume_buffer, 0, bytemuck::bytes_of(&params));
        return None;
    }

    let size = egui_wgpu::wgpu::Extent3d {
        width: dims[0].max(1),
        height: dims[1].max(1),
        depth_or_array_layers: dims[2].max(1),
    };
    pipeline.volume_texture = device.create_texture_with_data(
        queue,
        &egui_wgpu::wgpu::TextureDescriptor {
            label: Some("lobedo_volume_texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: egui_wgpu::wgpu::TextureDimension::D3,
            format: egui_wgpu::wgpu::TextureFormat::R32Float,
            usage: egui_wgpu::wgpu::TextureUsages::TEXTURE_BINDING
                | egui_wgpu::wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        },
        egui_wgpu::wgpu::util::TextureDataOrder::LayerMajor,
        bytemuck::cast_slice(volume.values.as_slice()),
    );
    pipeline.volume_view = pipeline.volume_texture.create_view(
        &egui_wgpu::wgpu::TextureViewDescriptor {
            dimension: Some(egui_wgpu::wgpu::TextureViewDimension::D3),
            ..Default::default()
        },
    );
    pipeline.volume_bind_group = device.create_bind_group(&egui_wgpu::wgpu::BindGroupDescriptor {
        label: Some("lobedo_volume_bind_group"),
        layout: &pipeline.volume_bind_group_layout,
        entries: &[
            egui_wgpu::wgpu::BindGroupEntry {
                binding: 0,
                resource: pipeline.volume_buffer.as_entire_binding(),
            },
            egui_wgpu::wgpu::BindGroupEntry {
                binding: 1,
                resource: egui_wgpu::wgpu::BindingResource::TextureView(&pipeline.volume_view),
            },
        ],
    });

    let world_to_volume = volume
        .transform
        .inverse()
        .to_cols_array_2d();
    let kind = match volume.kind {
        crate::scene::RenderVolumeKind::Density => 0u32,
        crate::scene::RenderVolumeKind::Sdf => 1u32,
    };
    let params = VolumeParams {
        origin: volume.origin,
        voxel_size: volume.voxel_size.max(1.0e-6),
        dims,
        kind,
        params: [
            volume.density_scale.max(0.0),
            volume.sdf_band.max(1.0e-6),
            1.0,
            0.0,
        ],
        world_to_volume,
    };
    queue.write_buffer(&pipeline.volume_buffer, 0, bytemuck::bytes_of(&params));
    pipeline.volume_present = true;

    let (min, max) = volume_world_bounds(volume);
    Some((min.to_array(), max.to_array()))
}

fn empty_volume_params() -> VolumeParams {
    VolumeParams {
        origin: [0.0, 0.0, 0.0],
        voxel_size: 1.0,
        dims: [0, 0, 0],
        kind: 0,
        params: [1.0, 1.0, 1.0, 0.0],
        world_to_volume: glam::Mat4::IDENTITY.to_cols_array_2d(),
    }
}

fn volume_world_bounds(volume: &crate::scene::RenderVolume) -> (Vec3, Vec3) {
    let min = Vec3::from(volume.origin);
    let size = Vec3::new(
        volume.dims[0] as f32 * volume.voxel_size,
        volume.dims[1] as f32 * volume.voxel_size,
        volume.dims[2] as f32 * volume.voxel_size,
    );
    let max = min + size;
    let corners = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::new(max.x, max.y, max.z),
    ];
    let mut world_min = Vec3::splat(f32::INFINITY);
    let mut world_max = Vec3::splat(f32::NEG_INFINITY);
    for corner in corners {
        let world = volume.transform.transform_point3(corner);
        world_min = world_min.min(world);
        world_max = world_max.max(world);
    }
    (world_min, world_max)
}
