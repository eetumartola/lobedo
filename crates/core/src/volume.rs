use glam::{Mat4, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VolumeKind {
    Density,
    Sdf,
}

#[derive(Debug, Clone)]
pub struct Volume {
    pub kind: VolumeKind,
    pub origin: [f32; 3],
    pub dims: [u32; 3],
    pub voxel_size: f32,
    pub values: Vec<f32>,
    pub transform: Mat4,
    pub density_scale: f32,
    pub sdf_band: f32,
}

impl Volume {
    pub fn new(
        kind: VolumeKind,
        origin: [f32; 3],
        dims: [u32; 3],
        voxel_size: f32,
        values: Vec<f32>,
    ) -> Self {
        Self {
            kind,
            origin,
            dims,
            voxel_size,
            values,
            transform: Mat4::IDENTITY,
            density_scale: 1.0,
            sdf_band: voxel_size.max(1.0e-6) * 2.0,
        }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn local_bounds(&self) -> (Vec3, Vec3) {
        let min = Vec3::from(self.origin);
        let size = Vec3::new(
            self.dims[0].saturating_sub(1) as f32 * self.voxel_size,
            self.dims[1].saturating_sub(1) as f32 * self.voxel_size,
            self.dims[2].saturating_sub(1) as f32 * self.voxel_size,
        );
        let max = min + size;
        (min, max)
    }

    pub fn world_bounds(&self) -> (Vec3, Vec3) {
        let (min, max) = self.local_bounds();
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
            let world = self.transform.transform_point3(corner);
            world_min = world_min.min(world);
            world_max = world_max.max(world);
        }
        (world_min, world_max)
    }

    pub fn value_index(&self, x: u32, y: u32, z: u32) -> usize {
        let nx = self.dims[0].max(1);
        let ny = self.dims[1].max(1);
        (z * nx * ny + y * nx + x) as usize
    }
}
