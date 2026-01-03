use crate::attributes::{AttributeDomain, AttributeRef};
use crate::mesh::Mesh;
use crate::splat::SplatGeo;

#[derive(Debug, Clone)]
pub struct SceneMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub corner_normals: Option<Vec<[f32; 3]>>,
    pub colors: Option<Vec<[f32; 3]>>,
    pub corner_colors: Option<Vec<[f32; 3]>>,
}

#[derive(Debug, Clone)]
pub struct SceneSplats {
    pub positions: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 3]>,
    pub opacity: Vec<f32>,
    pub scales: Vec<[f32; 3]>,
    pub rotations: Vec<[f32; 4]>,
}

#[derive(Debug, Clone)]
pub enum SceneDrawable {
    Mesh(SceneMesh),
    Splats(SceneSplats),
}

#[derive(Debug, Clone)]
pub struct SceneSnapshot {
    pub drawables: Vec<SceneDrawable>,
    pub base_color: [f32; 3],
}

impl SceneMesh {
    pub fn from_mesh(mesh: &Mesh) -> Self {
        let mut normals = fallback_normals(mesh);
        let mut corner_normals = mesh.corner_normals.clone();
        if let Some((domain, attr)) = mesh.attribute_with_precedence("N") {
            if let Some(values) = attr_vec3(attr) {
                match domain {
                    AttributeDomain::Vertex => {
                        if values.len() == mesh.indices.len() {
                            corner_normals = Some(values);
                        }
                    }
                    AttributeDomain::Point => {
                        if values.len() == mesh.positions.len() {
                            normals = values;
                            corner_normals = None;
                        }
                    }
                    AttributeDomain::Primitive => {
                        if let Some(expanded) = expand_primitive_vec3(mesh, &values) {
                            corner_normals = Some(expanded);
                        }
                    }
                    AttributeDomain::Detail => {
                        if let Some(value) = values.first().copied() {
                            if mesh.indices.is_empty() {
                                normals = vec![value; mesh.positions.len()];
                                corner_normals = None;
                            } else {
                                corner_normals = Some(vec![value; mesh.indices.len()]);
                            }
                        }
                    }
                }
            }
        }

        let mut colors = None;
        let mut corner_colors = None;
        if let Some((domain, attr)) = mesh.attribute_with_precedence("Cd") {
            if let Some(values) = attr_vec3(attr) {
                match domain {
                    AttributeDomain::Vertex => {
                        if values.len() == mesh.indices.len() {
                            corner_colors = Some(values);
                        }
                    }
                    AttributeDomain::Point => {
                        if values.len() == mesh.positions.len() {
                            colors = Some(values);
                        }
                    }
                    AttributeDomain::Primitive => {
                        if let Some(expanded) = expand_primitive_vec3(mesh, &values) {
                            corner_colors = Some(expanded);
                        }
                    }
                    AttributeDomain::Detail => {
                        if let Some(value) = values.first().copied() {
                            if mesh.indices.is_empty() {
                                colors = Some(vec![value; mesh.positions.len()]);
                            } else {
                                corner_colors = Some(vec![value; mesh.indices.len()]);
                            }
                        }
                    }
                }
            }
        }

        Self {
            positions: mesh.positions.clone(),
            normals,
            indices: mesh.indices.clone(),
            corner_normals,
            colors,
            corner_colors,
        }
    }
}

impl SceneSplats {
    pub fn from_splats(splats: &SplatGeo) -> Self {
        Self {
            positions: splats.positions.clone(),
            colors: splats.sh0.clone(),
            opacity: splats.opacity.clone(),
            scales: splats.scales.clone(),
            rotations: splats.rotations.clone(),
        }
    }
}

impl SceneSnapshot {
    pub fn from_mesh(mesh: &Mesh, base_color: [f32; 3]) -> Self {
        Self {
            drawables: vec![SceneDrawable::Mesh(SceneMesh::from_mesh(mesh))],
            base_color,
        }
    }

    pub fn from_splats(splats: &SplatGeo, base_color: [f32; 3]) -> Self {
        Self {
            drawables: vec![SceneDrawable::Splats(SceneSplats::from_splats(splats))],
            base_color,
        }
    }

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
}

fn fallback_normals(mesh: &Mesh) -> Vec<[f32; 3]> {
    match &mesh.normals {
        Some(normals) => normals.clone(),
        None => {
            let mut temp = mesh.clone();
            temp.compute_normals();
            temp.normals
                .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; mesh.positions.len()])
        }
    }
}

fn attr_vec3(attr: AttributeRef<'_>) -> Option<Vec<[f32; 3]>> {
    match attr {
        AttributeRef::Vec3(values) => Some(values.to_vec()),
        AttributeRef::Vec4(values) => Some(values.iter().map(|v| [v[0], v[1], v[2]]).collect()),
        _ => None,
    }
}

fn expand_primitive_vec3(mesh: &Mesh, values: &[[f32; 3]]) -> Option<Vec<[f32; 3]>> {
    let tri_count = mesh.indices.len() / 3;
    if values.len() != tri_count {
        return None;
    }
    let mut expanded = Vec::with_capacity(mesh.indices.len());
    for value in values {
        expanded.extend_from_slice(&[*value; 3]);
    }
    Some(expanded)
}
