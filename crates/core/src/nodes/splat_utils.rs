use glam::Vec3;

use crate::splat::SplatGeo;

pub fn splat_bounds(splats: &SplatGeo) -> (Vec3, Vec3) {
    let mut iter = splats.positions.iter();
    let first = iter
        .next()
        .copied()
        .unwrap_or([0.0, 0.0, 0.0]);
    let mut min = Vec3::from(first);
    let mut max = Vec3::from(first);
    for p in iter {
        let v = Vec3::from(*p);
        min = min.min(v);
        max = max.max(v);
    }
    (min, max)
}

pub fn splat_bounds_indices(splats: &SplatGeo, indices: &[usize]) -> (Vec3, Vec3) {
    let mut iter = indices.iter().copied();
    let first = iter
        .next()
        .and_then(|idx| splats.positions.get(idx).copied())
        .unwrap_or([0.0, 0.0, 0.0]);
    let mut min = Vec3::from(first);
    let mut max = Vec3::from(first);
    for idx in iter {
        if let Some(position) = splats.positions.get(idx) {
            let pos = Vec3::from(*position);
            min = min.min(pos);
            max = max.max(pos);
        }
    }
    (min, max)
}

pub fn splat_cell_key(position: Vec3, min: Vec3, inv_cell: f32) -> (i32, i32, i32) {
    let shifted = (position - min) * inv_cell;
    (
        shifted.x.floor() as i32,
        shifted.y.floor() as i32,
        shifted.z.floor() as i32,
    )
}
