use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Material {
    pub name: String,
    pub base_color: [f32; 3],
    pub metallic: f32,
    pub roughness: f32,
    pub base_color_texture: Option<String>,
}

impl Material {
    pub fn new(name: String) -> Self {
        Self {
            name,
            base_color: [1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            base_color_texture: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MaterialLibrary {
    materials: BTreeMap<String, Material>,
}

impl MaterialLibrary {
    pub fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }

    pub fn insert(&mut self, material: Material) {
        self.materials.insert(material.name.clone(), material);
    }

    pub fn get(&self, name: &str) -> Option<&Material> {
        self.materials.get(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Material> {
        self.materials.values()
    }

    pub fn merge(&mut self, other: &MaterialLibrary) {
        for material in other.materials.values() {
            self.materials.insert(material.name.clone(), material.clone());
        }
    }
}
