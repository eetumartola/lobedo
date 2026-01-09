use glam::{Mat4, Vec3};

use crate::volume::{Volume, VolumeKind};

#[derive(Clone, Copy)]
pub struct VolumeSampler<'a> {
    volume: &'a Volume,
    world_to_volume: Mat4,
    outside: f32,
}

impl<'a> VolumeSampler<'a> {
    pub fn new(volume: &'a Volume) -> Self {
        let outside = outside_value(volume);
        let world_to_volume = safe_inverse(volume.transform);
        Self {
            volume,
            world_to_volume,
            outside,
        }
    }

    pub fn sample_world(&self, world_pos: Vec3) -> f32 {
        sample_volume(self.volume, self.world_to_volume, world_pos, self.outside)
    }
}

pub fn outside_value(volume: &Volume) -> f32 {
    match volume.kind {
        VolumeKind::Sdf => 1.0e6,
        VolumeKind::Density => 0.0,
    }
}

pub fn sample_volume(
    volume: &Volume,
    world_to_volume: Mat4,
    world_pos: Vec3,
    outside: f32,
) -> f32 {
    let local = world_to_volume.transform_point3(world_pos);
    let origin = Vec3::from(volume.origin);
    let voxel = volume.voxel_size.max(1.0e-6);
    let gx = (local.x - origin.x) / voxel;
    let gy = (local.y - origin.y) / voxel;
    let gz = (local.z - origin.z) / voxel;

    let nx = volume.dims[0] as i32;
    let ny = volume.dims[1] as i32;
    let nz = volume.dims[2] as i32;
    if nx <= 0 || ny <= 0 || nz <= 0 {
        return outside;
    }
    if gx < 0.0
        || gy < 0.0
        || gz < 0.0
        || gx > (nx - 1) as f32
        || gy > (ny - 1) as f32
        || gz > (nz - 1) as f32
    {
        return outside;
    }

    let x0 = gx.floor() as i32;
    let y0 = gy.floor() as i32;
    let z0 = gz.floor() as i32;
    let x1 = (x0 + 1).min(nx - 1);
    let y1 = (y0 + 1).min(ny - 1);
    let z1 = (z0 + 1).min(nz - 1);

    let fx = if x0 == x1 { 0.0 } else { gx - x0 as f32 };
    let fy = if y0 == y1 { 0.0 } else { gy - y0 as f32 };
    let fz = if z0 == z1 { 0.0 } else { gz - z0 as f32 };

    let c000 = volume.values[volume.value_index(x0 as u32, y0 as u32, z0 as u32)];
    let c100 = volume.values[volume.value_index(x1 as u32, y0 as u32, z0 as u32)];
    let c010 = volume.values[volume.value_index(x0 as u32, y1 as u32, z0 as u32)];
    let c110 = volume.values[volume.value_index(x1 as u32, y1 as u32, z0 as u32)];
    let c001 = volume.values[volume.value_index(x0 as u32, y0 as u32, z1 as u32)];
    let c101 = volume.values[volume.value_index(x1 as u32, y0 as u32, z1 as u32)];
    let c011 = volume.values[volume.value_index(x0 as u32, y1 as u32, z1 as u32)];
    let c111 = volume.values[volume.value_index(x1 as u32, y1 as u32, z1 as u32)];

    let c00 = c000 + (c100 - c000) * fx;
    let c10 = c010 + (c110 - c010) * fx;
    let c01 = c001 + (c101 - c001) * fx;
    let c11 = c011 + (c111 - c011) * fx;
    let c0 = c00 + (c10 - c00) * fy;
    let c1 = c01 + (c11 - c01) * fy;
    c0 + (c1 - c0) * fz
}

pub fn safe_inverse(mat: Mat4) -> Mat4 {
    let inv = mat.inverse();
    if inv.x_axis.is_finite()
        && inv.y_axis.is_finite()
        && inv.z_axis.is_finite()
        && inv.w_axis.is_finite()
    {
        inv
    } else {
        Mat4::IDENTITY
    }
}
