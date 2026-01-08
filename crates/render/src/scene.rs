#[derive(Debug, Clone)]
pub struct RenderMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub corner_normals: Option<Vec<[f32; 3]>>,
    pub colors: Option<Vec<[f32; 3]>>,
    pub corner_colors: Option<Vec<[f32; 3]>>,
    pub uvs: Option<Vec<[f32; 2]>>,
    pub corner_uvs: Option<Vec<[f32; 2]>>,
    pub corner_materials: Option<Vec<u32>>,
}

#[derive(Debug, Clone)]
pub struct RenderSplats {
    pub positions: Vec<[f32; 3]>,
    pub sh0: Vec<[f32; 3]>,
    pub sh_coeffs: usize,
    pub sh_rest: Vec<[f32; 3]>,
    pub sh0_is_coeff: bool,
    pub opacity: Vec<f32>,
    pub scales: Vec<[f32; 3]>,
    pub rotations: Vec<[f32; 4]>,
}

#[derive(Debug, Clone)]
pub struct RenderCurve {
    pub points: Vec<[f32; 3]>,
    pub closed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderVolumeKind {
    Density,
    Sdf,
}

#[derive(Debug, Clone)]
pub struct RenderVolume {
    pub kind: RenderVolumeKind,
    pub origin: [f32; 3],
    pub dims: [u32; 3],
    pub voxel_size: f32,
    pub values: Vec<f32>,
    pub transform: glam::Mat4,
    pub density_scale: f32,
    pub sdf_band: f32,
}

#[derive(Debug, Clone)]
pub enum RenderDrawable {
    Mesh(RenderMesh),
    Splats(RenderSplats),
    Curve(RenderCurve),
    Volume(RenderVolume),
}

#[derive(Debug, Clone)]
pub struct RenderMaterial {
    pub base_color: [f32; 3],
    pub metallic: f32,
    pub roughness: f32,
    pub base_color_texture: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct RenderTexture {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectionShape {
    Box { center: [f32; 3], size: [f32; 3] },
    Sphere { center: [f32; 3], size: [f32; 3] },
    Plane {
        origin: [f32; 3],
        normal: [f32; 3],
        size: [f32; 3],
    },
}

#[derive(Debug, Clone)]
pub struct RenderScene {
    pub drawables: Vec<RenderDrawable>,
    pub base_color: [f32; 3],
    pub template_mesh: Option<RenderMesh>,
    pub selection_shape: Option<SelectionShape>,
    pub materials: Vec<RenderMaterial>,
    pub textures: Vec<RenderTexture>,
}

impl RenderScene {
    pub fn mesh(&self) -> Option<&RenderMesh> {
        self.drawables.iter().find_map(|drawable| match drawable {
            RenderDrawable::Mesh(mesh) => Some(mesh),
            _ => None,
        })
    }

    pub fn splats(&self) -> Option<&RenderSplats> {
        self.drawables.iter().find_map(|drawable| match drawable {
            RenderDrawable::Splats(splats) => Some(splats),
            _ => None,
        })
    }

    pub fn curves(&self) -> Vec<&RenderCurve> {
        self.drawables
            .iter()
            .filter_map(|drawable| match drawable {
                RenderDrawable::Curve(curve) => Some(curve),
                _ => None,
            })
            .collect()
    }

    pub fn volume(&self) -> Option<&RenderVolume> {
        self.drawables.iter().find_map(|drawable| match drawable {
            RenderDrawable::Volume(volume) => Some(volume),
            _ => None,
        })
    }
}
