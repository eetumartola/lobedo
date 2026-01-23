pub use lobedo_scene::{
    SceneCurve as RenderCurve, SceneDrawable as RenderDrawable, SceneMesh as RenderMesh,
    SceneSplats as RenderSplats, SceneVolume as RenderVolume,
    SceneVolumeKind as RenderVolumeKind, SelectionShape,
};

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
