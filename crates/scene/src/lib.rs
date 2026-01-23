#[derive(Debug, Clone)]
pub struct SceneMaterial {
    pub name: String,
    pub base_color: [f32; 3],
    pub metallic: f32,
    pub roughness: f32,
    pub base_color_texture: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SceneMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub tri_to_face: Vec<u32>,
    pub corner_indices: Vec<u32>,
    pub poly_indices: Vec<u32>,
    pub poly_face_counts: Vec<u32>,
    pub corner_normals: Option<Vec<[f32; 3]>>,
    pub colors: Option<Vec<[f32; 3]>>,
    pub corner_colors: Option<Vec<[f32; 3]>>,
    pub uvs: Option<Vec<[f32; 2]>>,
    pub corner_uvs: Option<Vec<[f32; 2]>>,
    pub corner_materials: Option<Vec<u32>>,
}

#[derive(Debug, Clone)]
pub struct SceneSplats {
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
pub struct SceneCurve {
    pub points: Vec<[f32; 3]>,
    pub closed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SceneVolumeKind {
    Density,
    Sdf,
}

#[derive(Debug, Clone)]
pub struct SceneVolume {
    pub kind: SceneVolumeKind,
    pub origin: [f32; 3],
    pub dims: [u32; 3],
    pub voxel_size: f32,
    pub values: Vec<f32>,
    pub transform: glam::Mat4,
    pub density_scale: f32,
    pub sdf_band: f32,
}

#[derive(Debug, Clone)]
pub enum SceneDrawable {
    Mesh(SceneMesh),
    Splats(SceneSplats),
    Curve(SceneCurve),
    Volume(SceneVolume),
}

#[derive(Debug, Clone)]
pub struct SceneSnapshot {
    pub drawables: Vec<SceneDrawable>,
    pub base_color: [f32; 3],
    pub materials: Vec<SceneMaterial>,
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

impl SceneSnapshot {
    pub fn mesh(&self) -> Option<&SceneMesh> {
        self.drawables.iter().find_map(|drawable| match drawable {
            SceneDrawable::Mesh(mesh) => Some(mesh),
            _ => None,
        })
    }

    pub fn splats(&self) -> Option<&SceneSplats> {
        self.drawables.iter().find_map(|drawable| match drawable {
            SceneDrawable::Splats(splats) => Some(splats),
            _ => None,
        })
    }

    pub fn curves(&self) -> Vec<&SceneCurve> {
        self.drawables
            .iter()
            .filter_map(|drawable| match drawable {
                SceneDrawable::Curve(curve) => Some(curve),
                _ => None,
            })
            .collect()
    }

    pub fn volume(&self) -> Option<&SceneVolume> {
        self.drawables.iter().find_map(|drawable| match drawable {
            SceneDrawable::Volume(volume) => Some(volume),
            _ => None,
        })
    }
}
