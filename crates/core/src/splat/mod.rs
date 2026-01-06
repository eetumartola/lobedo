use crate::attributes::MeshAttributes;
use crate::mesh::MeshGroups;

pub use crate::splat_ply::{load_splat_ply_with_mode, save_splat_ply, SplatLoadMode};

const SPLAT_LOG_SCALE_MIN: f32 = -10.0;
const SPLAT_LOG_SCALE_MAX: f32 = 10.0;
const SPLAT_ALPHA_MIN: f32 = 1.0e-4;
const SPLAT_ALPHA_MAX: f32 = 1.0 - 1.0e-4;

#[derive(Debug, Clone, Default)]
pub struct SplatGeo {
    pub positions: Vec<[f32; 3]>,
    pub rotations: Vec<[f32; 4]>,
    pub scales: Vec<[f32; 3]>,
    pub opacity: Vec<f32>,
    pub sh0: Vec<[f32; 3]>,
    pub sh_coeffs: usize,
    pub sh_rest: Vec<[f32; 3]>,
    pub attributes: MeshAttributes,
    pub groups: MeshGroups,
}

impl SplatGeo {
    pub fn with_len(count: usize) -> Self {
        Self {
            positions: vec![[0.0, 0.0, 0.0]; count],
            rotations: vec![[0.0, 0.0, 0.0, 1.0]; count],
            scales: vec![[0.0, 0.0, 0.0]; count],
            opacity: vec![1.0; count],
            sh0: vec![[1.0, 1.0, 1.0]; count],
            sh_coeffs: 0,
            sh_rest: Vec::new(),
            attributes: MeshAttributes::default(),
            groups: MeshGroups::default(),
        }
    }

    pub fn with_len_and_sh(count: usize, sh_coeffs: usize) -> Self {
        let mut splats = Self::with_len(count);
        if sh_coeffs > 0 {
            splats.sh_coeffs = sh_coeffs;
            splats.sh_rest = vec![[0.0, 0.0, 0.0]; count * sh_coeffs];
        }
        splats
    }

    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }
}

mod attributes;
mod math;
mod sh;
mod transform;
mod validate;

#[cfg(test)]
mod tests;
