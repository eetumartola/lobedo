use std::collections::{BTreeMap, BTreeSet};

use glam::{Mat4, Vec3};

use crate::attributes::{
    AttributeDomain, AttributeError, AttributeInfo, AttributeRef, AttributeStorage, AttributeType,
    MeshAttributes,
};

pub use crate::mesh_primitives::{make_box, make_grid, make_tube, make_uv_sphere};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

#[derive(Debug, Clone, Default)]
pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub normals: Option<Vec<[f32; 3]>>,
    pub corner_normals: Option<Vec<[f32; 3]>>,
    pub uvs: Option<Vec<[f32; 2]>>,
    pub attributes: MeshAttributes,
    pub groups: MeshGroups,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeshGroups {
    point: BTreeMap<String, Vec<bool>>,
    vertex: BTreeMap<String, Vec<bool>>,
    primitive: BTreeMap<String, Vec<bool>>,
}

impl MeshGroups {
    pub fn map(&self, domain: AttributeDomain) -> &BTreeMap<String, Vec<bool>> {
        match domain {
            AttributeDomain::Point => &self.point,
            AttributeDomain::Vertex => &self.vertex,
            AttributeDomain::Primitive => &self.primitive,
            AttributeDomain::Detail => &self.primitive,
        }
    }

    pub fn map_mut(&mut self, domain: AttributeDomain) -> &mut BTreeMap<String, Vec<bool>> {
        match domain {
            AttributeDomain::Point => &mut self.point,
            AttributeDomain::Vertex => &mut self.vertex,
            AttributeDomain::Primitive => &mut self.primitive,
            AttributeDomain::Detail => &mut self.primitive,
        }
    }
}

impl Mesh {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_positions_indices(positions: Vec<[f32; 3]>, indices: Vec<u32>) -> Self {
        Self {
            positions,
            indices,
            normals: None,
            corner_normals: None,
            uvs: None,
            attributes: MeshAttributes::default(),
            groups: MeshGroups::default(),
        }
    }

    pub fn attribute_domain_len(&self, domain: AttributeDomain) -> usize {
        match domain {
            AttributeDomain::Point => self.positions.len(),
            AttributeDomain::Vertex => self.indices.len(),
            AttributeDomain::Primitive => self.indices.len() / 3,
            AttributeDomain::Detail => 1,
        }
    }

    pub fn list_attributes(&self) -> Vec<AttributeInfo> {
        let mut list = Vec::new();
        if !self.positions.is_empty() {
            list.push(AttributeInfo {
                name: "P".to_string(),
                domain: AttributeDomain::Point,
                data_type: AttributeType::Vec3,
                len: self.positions.len(),
                implicit: true,
            });
        }
        if let Some(normals) = &self.normals {
            list.push(AttributeInfo {
                name: "N".to_string(),
                domain: AttributeDomain::Point,
                data_type: AttributeType::Vec3,
                len: normals.len(),
                implicit: true,
            });
        }
        if let Some(normals) = &self.corner_normals {
            list.push(AttributeInfo {
                name: "N".to_string(),
                domain: AttributeDomain::Vertex,
                data_type: AttributeType::Vec3,
                len: normals.len(),
                implicit: true,
            });
        }
        for domain in AttributeDomain::ALL {
            for (name, storage) in self.attributes.map(domain) {
                list.push(AttributeInfo {
                    name: name.clone(),
                    domain,
                    data_type: storage.data_type(),
                    len: storage.len(),
                    implicit: false,
                });
            }
        }
        list
    }

    pub fn attribute(&self, domain: AttributeDomain, name: &str) -> Option<AttributeRef<'_>> {
        match (name, domain) {
            ("P", AttributeDomain::Point) => Some(AttributeRef::Vec3(self.positions.as_slice())),
            ("N", AttributeDomain::Point) => self
                .normals
                .as_ref()
                .map(|normals| AttributeRef::Vec3(normals.as_slice())),
            ("N", AttributeDomain::Vertex) => self
                .corner_normals
                .as_ref()
                .map(|normals| AttributeRef::Vec3(normals.as_slice())),
            _ => self
                .attributes
                .get(domain, name)
                .map(AttributeStorage::as_ref),
        }
    }

    pub fn attribute_with_precedence(
        &self,
        name: &str,
    ) -> Option<(AttributeDomain, AttributeRef<'_>)> {
        if name == "P" {
            return self
                .attribute(AttributeDomain::Point, name)
                .map(|attr| (AttributeDomain::Point, attr));
        }

        if let Some(attr) = self.attribute(AttributeDomain::Vertex, name) {
            return Some((AttributeDomain::Vertex, attr));
        }
        if let Some(attr) = self.attribute(AttributeDomain::Point, name) {
            return Some((AttributeDomain::Point, attr));
        }
        if let Some(attr) = self.attribute(AttributeDomain::Primitive, name) {
            return Some((AttributeDomain::Primitive, attr));
        }
        if let Some(attr) = self.attribute(AttributeDomain::Detail, name) {
            return Some((AttributeDomain::Detail, attr));
        }
        None
    }

    pub fn set_attribute(
        &mut self,
        domain: AttributeDomain,
        name: impl Into<String>,
        storage: AttributeStorage,
    ) -> Result<(), AttributeError> {
        let name = name.into();
        let expected_len = self.attribute_domain_len(domain);
        let actual_len = storage.len();
        if expected_len != 0 && actual_len != expected_len {
            return Err(AttributeError::InvalidLength {
                expected: expected_len,
                actual: actual_len,
            });
        }

        match (name.as_str(), domain) {
            ("P", AttributeDomain::Point) => {
                if storage.data_type() != AttributeType::Vec3 {
                    return Err(AttributeError::InvalidType {
                        expected: AttributeType::Vec3,
                        actual: storage.data_type(),
                    });
                }
                if let AttributeStorage::Vec3(values) = storage {
                    self.positions = values;
                    return Ok(());
                }
            }
            ("P", _) => return Err(AttributeError::InvalidDomain),
            ("N", AttributeDomain::Point) => {
                if storage.data_type() != AttributeType::Vec3 {
                    return Err(AttributeError::InvalidType {
                        expected: AttributeType::Vec3,
                        actual: storage.data_type(),
                    });
                }
                if let AttributeStorage::Vec3(values) = storage {
                    self.normals = Some(values);
                    return Ok(());
                }
            }
            ("N", AttributeDomain::Vertex) => {
                if storage.data_type() != AttributeType::Vec3 {
                    return Err(AttributeError::InvalidType {
                        expected: AttributeType::Vec3,
                        actual: storage.data_type(),
                    });
                }
                if let AttributeStorage::Vec3(values) = storage {
                    self.corner_normals = Some(values);
                    return Ok(());
                }
            }
            _ => {}
        }

        self.attributes.map_mut(domain).insert(name, storage);
        Ok(())
    }

    pub fn remove_attribute(
        &mut self,
        domain: AttributeDomain,
        name: &str,
    ) -> Option<AttributeStorage> {
        match (name, domain) {
            ("P", AttributeDomain::Point) => None,
            ("N", AttributeDomain::Point) => {
                self.normals = None;
                None
            }
            ("N", AttributeDomain::Vertex) => {
                self.corner_normals = None;
                None
            }
            _ => self.attributes.remove(domain, name),
        }
    }

    pub fn bounds(&self) -> Option<Aabb> {
        let mut iter = self.positions.iter();
        let first = iter.next()?;
        let mut min = *first;
        let mut max = *first;

        for p in iter {
            min[0] = min[0].min(p[0]);
            min[1] = min[1].min(p[1]);
            min[2] = min[2].min(p[2]);
            max[0] = max[0].max(p[0]);
            max[1] = max[1].max(p[1]);
            max[2] = max[2].max(p[2]);
        }

        Some(Aabb { min, max })
    }

    pub fn compute_normals(&mut self) -> bool {
        if !self.indices.len().is_multiple_of(3) || self.positions.is_empty() {
            return false;
        }

        let mut accum = vec![Vec3::ZERO; self.positions.len()];

        for tri in self.indices.chunks_exact(3) {
            let i0 = tri[0] as usize;
            let i1 = tri[1] as usize;
            let i2 = tri[2] as usize;
            if i0 >= self.positions.len()
                || i1 >= self.positions.len()
                || i2 >= self.positions.len()
            {
                continue;
            }

            let p0 = Vec3::from(self.positions[i0]);
            let p1 = Vec3::from(self.positions[i1]);
            let p2 = Vec3::from(self.positions[i2]);
            let normal = (p1 - p0).cross(p2 - p0);
            accum[i0] += normal;
            accum[i1] += normal;
            accum[i2] += normal;
        }

        let normals = accum
            .into_iter()
            .map(|n| {
                let len = n.length();
                if len > 0.0 {
                    (n / len).to_array()
                } else {
                    [0.0, 1.0, 0.0]
                }
            })
            .collect();

        self.normals = Some(normals);
        self.corner_normals = None;
        true
    }

    pub fn compute_normals_with_threshold(&mut self, threshold_degrees: f32) -> bool {
        if !self.indices.len().is_multiple_of(3) || self.positions.is_empty() {
            return false;
        }

        let threshold = threshold_degrees.clamp(0.0, 180.0);
        if threshold >= 179.9 {
            return self.compute_normals();
        }

        let cos_threshold = threshold.to_radians().cos();
        let tri_count = self.indices.len() / 3;
        let mut face_normals = Vec::with_capacity(tri_count);
        let mut face_indices = Vec::with_capacity(tri_count);

        for tri in self.indices.chunks_exact(3) {
            let i0 = tri[0] as usize;
            let i1 = tri[1] as usize;
            let i2 = tri[2] as usize;
            if i0 >= self.positions.len()
                || i1 >= self.positions.len()
                || i2 >= self.positions.len()
            {
                return false;
            }
            let p0 = Vec3::from(self.positions[i0]);
            let p1 = Vec3::from(self.positions[i1]);
            let p2 = Vec3::from(self.positions[i2]);
            let normal = (p1 - p0).cross(p2 - p0);
            let normal = if normal.length_squared() > 0.0 {
                normal.normalize()
            } else {
                Vec3::Y
            };
            face_normals.push(normal);
            face_indices.push([i0, i1, i2]);
        }

        let mut groups = std::collections::HashMap::new();
        for (index, position) in self.positions.iter().enumerate() {
            let key = quantize_position(*position);
            groups.entry(key).or_insert_with(Vec::new).push(index);
        }

        let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); self.positions.len()];
        for (face_index, indices) in face_indices.iter().enumerate() {
            for &pos_index in indices {
                let key = quantize_position(self.positions[pos_index]);
                if let Some(group) = groups.get(&key) {
                    for &member in group {
                        adjacency[member].push(face_index);
                    }
                } else {
                    adjacency[pos_index].push(face_index);
                }
            }
        }

        let mut corner_normals = Vec::with_capacity(self.indices.len());
        for (face_index, indices) in face_indices.iter().enumerate() {
            let face_normal = face_normals[face_index];
            for &pos_index in indices {
                let mut sum = Vec3::ZERO;
                for &adj_face in &adjacency[pos_index] {
                    let candidate = face_normals[adj_face];
                    if candidate.dot(face_normal) >= cos_threshold {
                        sum += candidate;
                    }
                }
                let sum = if sum.length_squared() > 0.0 {
                    sum.normalize()
                } else {
                    face_normal
                };
                corner_normals.push(sum.to_array());
            }
        }

        let _ = self.compute_normals();
        self.corner_normals = Some(corner_normals);
        true
    }

    pub fn transform(&mut self, matrix: Mat4) {
        for p in &mut self.positions {
            let v = matrix.transform_point3(Vec3::from(*p));
            *p = v.to_array();
        }

        if let Some(normals) = &mut self.normals {
            let normal_matrix = matrix.inverse().transpose();
            for n in normals {
                let v = normal_matrix.transform_vector3(Vec3::from(*n));
                let len = v.length();
                *n = if len > 0.0 {
                    (v / len).to_array()
                } else {
                    [0.0, 1.0, 0.0]
                };
            }
        }

        if let Some(corner_normals) = &mut self.corner_normals {
            let normal_matrix = matrix.inverse().transpose();
            for n in corner_normals {
                let v = normal_matrix.transform_vector3(Vec3::from(*n));
                let len = v.length();
                *n = if len > 0.0 {
                    (v / len).to_array()
                } else {
                    [0.0, 1.0, 0.0]
                };
            }
        }
    }

    pub fn merge(meshes: &[Mesh]) -> Mesh {
        let mut merged = Mesh::default();
        let mut vertex_offset = 0u32;
        let mut include_normals = true;
        let mut include_uvs = true;
        let mut include_corner_normals = true;

        for mesh in meshes {
            include_normals &= mesh.normals.is_some();
            include_uvs &= mesh.uvs.is_some();
            include_corner_normals &= mesh.corner_normals.is_some();
        }

        for mesh in meshes {
            merged.positions.extend_from_slice(&mesh.positions);
            merged
                .indices
                .extend(mesh.indices.iter().map(|i| i + vertex_offset));
            vertex_offset += mesh.positions.len() as u32;
        }

        if include_normals {
            let mut normals = Vec::new();
            for mesh in meshes {
                normals.extend_from_slice(mesh.normals.as_ref().unwrap());
            }
            merged.normals = Some(normals);
        }

        if include_uvs {
            let mut uvs = Vec::new();
            for mesh in meshes {
                uvs.extend_from_slice(mesh.uvs.as_ref().unwrap());
            }
            merged.uvs = Some(uvs);
        }

        if include_corner_normals {
            let mut corner_normals = Vec::new();
            for mesh in meshes {
                corner_normals.extend_from_slice(mesh.corner_normals.as_ref().unwrap());
            }
            merged.corner_normals = Some(corner_normals);
        }

        merged.attributes = merge_attributes(meshes);
        merged.groups = merge_groups(meshes);
        merged
    }
}

fn merge_attributes(meshes: &[Mesh]) -> MeshAttributes {
    let mut merged = MeshAttributes::default();
    if meshes.is_empty() {
        return merged;
    }

    for domain in AttributeDomain::ALL {
        let first = meshes[0].attributes.map(domain);
        for (name, storage) in first {
            let data_type = storage.data_type();
            let mut compatible = true;
            for mesh in &meshes[1..] {
                let Some(other) = mesh.attributes.get(domain, name) else {
                    compatible = false;
                    break;
                };
                if other.data_type() != data_type {
                    compatible = false;
                    break;
                }
            }
            if !compatible {
                continue;
            }

            match domain {
                AttributeDomain::Detail => {
                    let mut all_equal = true;
                    for mesh in &meshes[1..] {
                        let Some(other) = mesh.attributes.get(domain, name) else {
                            all_equal = false;
                            break;
                        };
                        if other != storage {
                            all_equal = false;
                            break;
                        }
                    }
                    if all_equal {
                        merged.map_mut(domain).insert(name.clone(), storage.clone());
                    }
                }
                _ => {
                    let mut combined = match storage {
                        AttributeStorage::Float(_) => AttributeStorage::Float(Vec::new()),
                        AttributeStorage::Int(_) => AttributeStorage::Int(Vec::new()),
                        AttributeStorage::Vec2(_) => AttributeStorage::Vec2(Vec::new()),
                        AttributeStorage::Vec3(_) => AttributeStorage::Vec3(Vec::new()),
                        AttributeStorage::Vec4(_) => AttributeStorage::Vec4(Vec::new()),
                    };

                    for mesh in meshes {
                        let expected = mesh.attribute_domain_len(domain);
                        let Some(current) = mesh.attributes.get(domain, name) else {
                            continue;
                        };
                        if expected != 0 && current.len() != expected {
                            compatible = false;
                            break;
                        }
                        match (&mut combined, current) {
                            (AttributeStorage::Float(out), AttributeStorage::Float(values)) => {
                                out.extend_from_slice(values);
                            }
                            (AttributeStorage::Int(out), AttributeStorage::Int(values)) => {
                                out.extend_from_slice(values);
                            }
                            (AttributeStorage::Vec2(out), AttributeStorage::Vec2(values)) => {
                                out.extend_from_slice(values);
                            }
                            (AttributeStorage::Vec3(out), AttributeStorage::Vec3(values)) => {
                                out.extend_from_slice(values);
                            }
                            (AttributeStorage::Vec4(out), AttributeStorage::Vec4(values)) => {
                                out.extend_from_slice(values);
                            }
                            _ => {
                                compatible = false;
                                break;
                            }
                        }
                    }

                    if compatible {
                        merged.map_mut(domain).insert(name.clone(), combined);
                    }
                }
            }
        }
    }

    merged
}

fn merge_groups(meshes: &[Mesh]) -> MeshGroups {
    let mut merged = MeshGroups::default();
    if meshes.is_empty() {
        return merged;
    }

    for domain in [
        AttributeDomain::Point,
        AttributeDomain::Vertex,
        AttributeDomain::Primitive,
    ] {
        let mut names = BTreeSet::new();
        for mesh in meshes {
            names.extend(mesh.groups.map(domain).keys().cloned());
        }
        for name in names {
            let mut values = Vec::new();
            for mesh in meshes {
                let len = mesh.attribute_domain_len(domain);
                if let Some(group) = mesh.groups.map(domain).get(&name) {
                    if group.len() == len {
                        values.extend_from_slice(group);
                    } else {
                        values.extend(std::iter::repeat_n(false, len));
                    }
                } else {
                    values.extend(std::iter::repeat_n(false, len));
                }
            }
            merged.map_mut(domain).insert(name, values);
        }
    }

    merged
}

fn quantize_position(position: [f32; 3]) -> (i32, i32, i32) {
    let epsilon = 1.0e-5;
    (
        (position[0] / epsilon).round() as i32,
        (position[1] / epsilon).round() as i32,
        (position[2] / epsilon).round() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_for_simple_points() {
        let mesh =
            Mesh::with_positions_indices(vec![[1.0, -2.0, 0.5], [-3.0, 4.0, 2.0]], vec![0, 1, 0]);
        let bounds = mesh.bounds().expect("bounds");
        assert_eq!(bounds.min, [-3.0, -2.0, 0.5]);
        assert_eq!(bounds.max, [1.0, 4.0, 2.0]);
    }

    #[test]
    fn normals_for_triangle() {
        let mut mesh = Mesh::with_positions_indices(
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            vec![0, 1, 2],
        );
        assert!(mesh.compute_normals());
        let normals = mesh.normals.expect("normals");
        for n in normals {
            assert!((n[2] - 1.0).abs() < 0.001);
        }
    }

    #[test]
    fn merge_offsets_indices() {
        let mesh_a = Mesh::with_positions_indices(vec![[0.0, 0.0, 0.0]], vec![0]);
        let mesh_b = Mesh::with_positions_indices(vec![[1.0, 0.0, 0.0]], vec![0]);
        let merged = Mesh::merge(&[mesh_a, mesh_b]);
        assert_eq!(merged.indices, vec![0, 1]);
    }
}
