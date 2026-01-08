use crate::attributes::{AttributeDomain, AttributeRef};
use crate::curve::Curve;
use crate::geometry::Geometry;
use crate::material::Material;
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
    pub opacity: Vec<f32>,
    pub scales: Vec<[f32; 3]>,
    pub rotations: Vec<[f32; 4]>,
}

#[derive(Debug, Clone)]
pub struct SceneCurve {
    pub points: Vec<[f32; 3]>,
    pub closed: bool,
}

#[derive(Debug, Clone)]
pub enum SceneDrawable {
    Mesh(SceneMesh),
    Splats(SceneSplats),
    Curve(SceneCurve),
}

#[derive(Debug, Clone)]
pub struct SceneSnapshot {
    pub drawables: Vec<SceneDrawable>,
    pub base_color: [f32; 3],
    pub materials: Vec<Material>,
}

type UvData = (Option<Vec<[f32; 2]>>, Option<Vec<[f32; 2]>>);

impl SceneMesh {
    pub fn from_mesh(mesh: &Mesh) -> Self {
        Self::from_mesh_with_materials(mesh, &std::collections::HashMap::new())
    }

    pub fn from_mesh_with_materials(
        mesh: &Mesh,
        material_lookup: &std::collections::HashMap<String, u32>,
    ) -> Self {
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

        let (uvs, corner_uvs) = mesh_uvs(mesh);
        let corner_materials = mesh_materials(mesh, material_lookup);

        Self {
            positions: mesh.positions.clone(),
            normals,
            indices: mesh.indices.clone(),
            corner_normals,
            colors,
            corner_colors,
            uvs,
            corner_uvs,
            corner_materials,
        }
    }
}

impl SceneSplats {
    pub fn from_splats(splats: &SplatGeo) -> Self {
        Self {
            positions: splats.positions.clone(),
            sh0: splats.sh0.clone(),
            sh_coeffs: splats.sh_coeffs,
            sh_rest: splats.sh_rest.clone(),
            opacity: splats.opacity.clone(),
            scales: splats.scales.clone(),
            rotations: splats.rotations.clone(),
        }
    }
}

impl SceneCurve {
    pub fn from_curve(curve: &Curve, positions: &[[f32; 3]]) -> Self {
        Self {
            points: curve.resolved_points(positions),
            closed: curve.closed,
        }
    }
}

impl SceneSnapshot {
    pub fn from_mesh(mesh: &Mesh, base_color: [f32; 3]) -> Self {
        Self {
            drawables: vec![SceneDrawable::Mesh(SceneMesh::from_mesh(mesh))],
            base_color,
            materials: Vec::new(),
        }
    }

    pub fn from_splats(splats: &SplatGeo, base_color: [f32; 3]) -> Self {
        Self {
            drawables: vec![SceneDrawable::Splats(SceneSplats::from_splats(splats))],
            base_color,
            materials: Vec::new(),
        }
    }

    pub fn from_geometry(geometry: &Geometry, base_color: [f32; 3]) -> Self {
        let mut drawables = Vec::new();
        let materials: Vec<Material> = geometry.materials.iter().cloned().collect();
        let mut material_lookup = std::collections::HashMap::new();
        for (idx, material) in materials.iter().enumerate() {
            material_lookup.insert(material.name.clone(), idx as u32);
        }
        let mesh = geometry.merged_mesh();
        if let Some(mesh) = mesh.as_ref() {
            drawables.push(SceneDrawable::Mesh(SceneMesh::from_mesh_with_materials(
                mesh,
                &material_lookup,
            )));
        }
        for splats in &geometry.splats {
            drawables.push(SceneDrawable::Splats(SceneSplats::from_splats(splats)));
        }
        let curve_points = mesh
            .as_ref()
            .map(|mesh| mesh.positions.as_slice())
            .unwrap_or(&[]);
        for curve in &geometry.curves {
            drawables.push(SceneDrawable::Curve(SceneCurve::from_curve(
                curve,
                curve_points,
            )));
        }
        Self {
            drawables,
            base_color,
            materials,
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

    pub fn curves(&self) -> Vec<&SceneCurve> {
        self.drawables
            .iter()
            .filter_map(|drawable| match drawable {
                SceneDrawable::Curve(curve) => Some(curve),
                _ => None,
            })
            .collect()
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

fn attr_vec2(attr: AttributeRef<'_>) -> Option<Vec<[f32; 2]>> {
    match attr {
        AttributeRef::Vec2(values) => Some(values.to_vec()),
        AttributeRef::Vec3(values) => Some(values.iter().map(|v| [v[0], v[1]]).collect()),
        AttributeRef::Vec4(values) => Some(values.iter().map(|v| [v[0], v[1]]).collect()),
        _ => None,
    }
}

fn mesh_uvs(mesh: &Mesh) -> UvData {
    let mut uvs = mesh.uvs.clone();
    if let Some(attr) = mesh.attribute(AttributeDomain::Point, "uv") {
        if let Some(values) = attr_vec2(attr) {
            if values.len() == mesh.positions.len() {
                uvs = Some(values);
            }
        }
    }

    let mut corner_uvs = None;
    if let Some(attr) = mesh.attribute(AttributeDomain::Vertex, "uv") {
        if let Some(values) = attr_vec2(attr) {
            if values.len() == mesh.indices.len() {
                corner_uvs = Some(values);
            }
        }
    }
    if corner_uvs.is_none() {
        if let Some(uvs) = &uvs {
            if uvs.len() == mesh.positions.len() && !mesh.indices.is_empty() {
                let mut expanded = Vec::with_capacity(mesh.indices.len());
                for &idx in &mesh.indices {
                    expanded.push(*uvs.get(idx as usize).unwrap_or(&[0.0, 0.0]));
                }
                corner_uvs = Some(expanded);
            }
        }
    }

    (uvs, corner_uvs)
}

fn mesh_materials(
    mesh: &Mesh,
    material_lookup: &std::collections::HashMap<String, u32>,
) -> Option<Vec<u32>> {
    let attr = mesh.attribute(AttributeDomain::Primitive, "material")?;
    let AttributeRef::StringTable(table) = attr else {
        return None;
    };
    let tri_count = mesh.indices.len() / 3;
    if tri_count == 0 || table.indices.len() != tri_count {
        return None;
    }
    let mut out = Vec::with_capacity(mesh.indices.len());
    for &mat_idx in &table.indices {
        let name = table
            .values
            .get(mat_idx as usize)
            .map(|value| value.as_str())
            .unwrap_or("");
        let resolved = material_lookup.get(name).copied().unwrap_or(0);
        out.extend_from_slice(&[resolved; 3]);
    }
    Some(out)
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
