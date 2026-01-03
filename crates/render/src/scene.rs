#[derive(Debug, Clone)]
pub struct RenderMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub corner_normals: Option<Vec<[f32; 3]>>,
    pub colors: Option<Vec<[f32; 3]>>,
    pub corner_colors: Option<Vec<[f32; 3]>>,
}

#[derive(Debug, Clone)]
pub struct RenderSplats {
    pub positions: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 3]>,
    pub opacity: Vec<f32>,
    pub scales: Vec<[f32; 3]>,
    pub rotations: Vec<[f32; 4]>,
}

#[derive(Debug, Clone)]
pub enum RenderDrawable {
    Mesh(RenderMesh),
    Splats(RenderSplats),
}

#[derive(Debug, Clone)]
pub struct RenderScene {
    pub drawables: Vec<RenderDrawable>,
    pub base_color: [f32; 3],
    pub template_mesh: Option<RenderMesh>,
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
}
