use std::collections::HashMap;

use glam::Vec3;

use crate::attributes::AttributeDomain;
use crate::graph::NodeParams;
use crate::nodes::group_utils::{mask_has_any, splat_group_mask};

use crate::splat::SplatGeo;

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

pub fn split_splats_by_group(
    splats: &SplatGeo,
    params: &NodeParams,
    target_domain: AttributeDomain,
) -> Option<(Vec<usize>, Vec<usize>)> {
    let mask = splat_group_mask(splats, params, target_domain);
    if !mask_has_any(mask.as_deref()) {
        return None;
    }
    let mut selected = Vec::new();
    let mut unselected = Vec::new();
    for idx in 0..splats.len() {
        let selected_here = mask
            .as_ref()
            .map(|mask| mask.get(idx).copied().unwrap_or(false))
            .unwrap_or(true);
        if selected_here {
            selected.push(idx);
        } else {
            unselected.push(idx);
        }
    }
    if selected.is_empty() {
        return None;
    }
    Some((selected, unselected))
}

pub struct SpatialHash {
    min: Vec3,
    inv_cell: f32,
    cells: HashMap<(i32, i32, i32), Vec<usize>>,
}

impl SpatialHash {
    pub fn build(positions: &[[f32; 3]], cell_size: f32) -> Option<Self> {
        if positions.is_empty() || !cell_size.is_finite() || cell_size <= 1.0e-6 {
            return None;
        }
        let mut min = Vec3::from(positions[0]);
        for pos in positions.iter().skip(1) {
            min = min.min(Vec3::from(*pos));
        }
        let inv_cell = 1.0 / cell_size;
        let mut cells: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();
        for (idx, pos) in positions.iter().enumerate() {
            let key = splat_cell_key(Vec3::from(*pos), min, inv_cell);
            cells.entry(key).or_default().push(idx);
        }
        Some(Self {
            min,
            inv_cell,
            cells,
        })
    }

    pub fn nearest(
        &self,
        positions: &[[f32; 3]],
        position: Vec3,
        radius: f32,
    ) -> Option<(usize, f32)> {
        if radius <= 1.0e-6 {
            return None;
        }
        let base = splat_cell_key(position, self.min, self.inv_cell);
        let range = (radius * self.inv_cell).ceil() as i32;
        let r2 = radius * radius;
        let mut best = None;
        for dz in -range..=range {
            for dy in -range..=range {
                for dx in -range..=range {
                    let key = (base.0 + dx, base.1 + dy, base.2 + dz);
                    let Some(list) = self.cells.get(&key) else {
                        continue;
                    };
                    for &idx in list {
                        let delta = Vec3::from(positions[idx]) - position;
                        let dist2 = delta.length_squared();
                        if dist2 <= r2 {
                            match best {
                                Some((_, best_dist2)) if dist2 >= best_dist2 => {}
                                _ => best = Some((idx, dist2)),
                            }
                        }
                    }
                }
            }
        }
        best.map(|(idx, dist2)| (idx, dist2.sqrt()))
    }

    pub fn neighbors_in_radius(
        &self,
        positions: &[[f32; 3]],
        idx: usize,
        radius: f32,
        out: &mut Vec<usize>,
    ) {
        out.clear();
        if radius <= 1.0e-6 {
            return;
        }
        let pos = Vec3::from(positions[idx]);
        let base = splat_cell_key(pos, self.min, self.inv_cell);
        let range = (radius * self.inv_cell).ceil() as i32;
        let r2 = radius * radius;
        for dz in -range..=range {
            for dy in -range..=range {
                for dx in -range..=range {
                    let key = (base.0 + dx, base.1 + dy, base.2 + dz);
                    let Some(list) = self.cells.get(&key) else {
                        continue;
                    };
                    for &other in list {
                        if other == idx {
                            continue;
                        }
                        let delta = Vec3::from(positions[other]) - pos;
                        if delta.length_squared() <= r2 {
                            out.push(other);
                        }
                    }
                }
            }
        }
    }
}
