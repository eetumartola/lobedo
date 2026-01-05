use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

use egui_wgpu::wgpu;
use egui_wgpu::wgpu::util::DeviceExt as _;

pub struct GpuMesh {
    pub data: GpuMeshData,
    pub index_count: u32,
    hash: u64,
}

pub enum GpuMeshData {
    Indexed {
        vertex_buffer: wgpu::Buffer,
        index_buffers: Vec<wgpu::Buffer>,
        index_counts: Vec<u32>,
    },
    NonIndexed {
        vertex_buffers: Vec<wgpu::Buffer>,
        vertex_counts: Vec<u32>,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct GpuMeshCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub uploads: u64,
    pub mesh_count: u32,
}

pub struct GpuMeshCache {
    meshes: HashMap<u64, GpuMesh>,
    hits: AtomicU64,
    misses: AtomicU64,
    uploads: AtomicU64,
}

impl GpuMeshCache {
    pub fn new() -> Self {
        Self {
            meshes: HashMap::new(),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            uploads: AtomicU64::new(0),
        }
    }

    pub fn get(&self, mesh_id: u64) -> Option<&GpuMesh> {
        let mesh = self.meshes.get(&mesh_id);
        if mesh.is_some() {
            self.hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
        }
        mesh
    }

    pub fn upload_or_update(
        &mut self,
        device: &wgpu::Device,
        mesh_id: u64,
        vertices: &[u8],
        vertex_stride: usize,
        indices: &[u32],
    ) -> &GpuMesh {
        let hash = hash_mesh(vertices, indices);
        let index_count = indices.len() as u32;
        let needs_upload = self
            .meshes
            .get(&mesh_id)
            .map(|mesh| mesh.hash != hash || mesh.index_count != index_count)
            .unwrap_or(true);

        if needs_upload {
            let max_buffer_size = device.limits().max_buffer_size as usize;
            let vertex_stride = vertex_stride.max(1);
            let vertex_buffer_size = vertices.len();
            let index_buffer_size = indices.len() * std::mem::size_of::<u32>();
            let data = if vertex_buffer_size <= max_buffer_size
                && index_buffer_size <= max_buffer_size
            {
                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("lobedo_mesh_vertices"),
                    contents: vertices,
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("lobedo_mesh_indices"),
                    contents: bytemuck::cast_slice(indices),
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                });
                GpuMeshData::Indexed {
                    vertex_buffer,
                    index_buffers: vec![index_buffer],
                    index_counts: vec![indices.len() as u32],
                }
            } else if vertex_buffer_size <= max_buffer_size {
                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("lobedo_mesh_vertices"),
                    contents: vertices,
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let max_indices =
                    (max_buffer_size / std::mem::size_of::<u32>()).max(1);
                let mut index_buffers = Vec::new();
                let mut index_counts = Vec::new();
                for chunk in indices.chunks(max_indices) {
                    let buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("lobedo_mesh_indices"),
                            contents: bytemuck::cast_slice(chunk),
                            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                        });
                    index_buffers.push(buffer);
                    index_counts.push(chunk.len() as u32);
                }
                GpuMeshData::Indexed {
                    vertex_buffer,
                    index_buffers,
                    index_counts,
                }
            } else {
                let max_vertices = (max_buffer_size / vertex_stride).max(1);
                let mut vertex_buffers = Vec::new();
                let mut vertex_counts = Vec::new();
                if indices.is_empty() {
                    let chunk_size = max_vertices * vertex_stride;
                    for chunk in vertices.chunks(chunk_size) {
                        if chunk.is_empty() {
                            continue;
                        }
                        let buffer = device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("lobedo_mesh_vertices"),
                                contents: chunk,
                                usage: wgpu::BufferUsages::VERTEX,
                            });
                        vertex_buffers.push(buffer);
                        vertex_counts.push((chunk.len() / vertex_stride) as u32);
                    }
                } else {
                    let mut chunk_bytes = Vec::with_capacity(max_vertices * vertex_stride);
                    let mut count = 0usize;
                    let zero_vertex = vec![0u8; vertex_stride];
                    for &index in indices {
                        let start = index as usize * vertex_stride;
                        if start + vertex_stride <= vertices.len() {
                            chunk_bytes
                                .extend_from_slice(&vertices[start..start + vertex_stride]);
                        } else {
                            chunk_bytes.extend_from_slice(&zero_vertex);
                        }
                        count += 1;
                        if count == max_vertices {
                            let buffer = device
                                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                    label: Some("lobedo_mesh_vertices"),
                                    contents: &chunk_bytes,
                                    usage: wgpu::BufferUsages::VERTEX,
                                });
                            vertex_buffers.push(buffer);
                            vertex_counts.push(count as u32);
                            chunk_bytes.clear();
                            count = 0;
                        }
                    }
                    if count > 0 {
                        let buffer = device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("lobedo_mesh_vertices"),
                                contents: &chunk_bytes,
                                usage: wgpu::BufferUsages::VERTEX,
                            });
                        vertex_buffers.push(buffer);
                        vertex_counts.push(count as u32);
                    }
                }
                GpuMeshData::NonIndexed {
                    vertex_buffers,
                    vertex_counts,
                }
            };
            let mesh = GpuMesh {
                data,
                index_count,
                hash,
            };
            self.meshes.insert(mesh_id, mesh);
            self.uploads.fetch_add(1, Ordering::Relaxed);
        }

        self.meshes.get(&mesh_id).expect("mesh cache insert")
    }

    pub fn stats_snapshot(&self) -> GpuMeshCacheStats {
        GpuMeshCacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            uploads: self.uploads.load(Ordering::Relaxed),
            mesh_count: self.meshes.len() as u32,
        }
    }
}

fn hash_mesh(vertices: &[u8], indices: &[u32]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    vertices.hash(&mut hasher);
    indices.hash(&mut hasher);
    hasher.finish()
}
