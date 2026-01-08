use std::collections::BTreeSet;

use crate::attributes::{AttributeDomain, AttributeStorage, MeshAttributes, StringTableAttribute};
use crate::material::MaterialLibrary;
use crate::mesh::Mesh;
use crate::curve::Curve;
use crate::splat::SplatGeo;
use crate::volume::Volume;

#[derive(Debug, Clone, Default)]
pub struct Geometry {
    pub meshes: Vec<Mesh>,
    pub splats: Vec<SplatGeo>,
    pub curves: Vec<Curve>,
    pub volumes: Vec<Volume>,
    pub materials: MaterialLibrary,
}

impl Geometry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_mesh(mesh: Mesh) -> Self {
        Self {
            meshes: vec![mesh],
            splats: Vec::new(),
            curves: Vec::new(),
            volumes: Vec::new(),
            materials: MaterialLibrary::default(),
        }
    }

    pub fn with_splats(splats: SplatGeo) -> Self {
        Self {
            meshes: Vec::new(),
            splats: vec![splats],
            curves: Vec::new(),
            volumes: Vec::new(),
            materials: MaterialLibrary::default(),
        }
    }

    pub fn with_curve(points: Vec<[f32; 3]>, closed: bool) -> Self {
        let point_count = points.len();
        let mesh = Mesh::with_positions_indices(points, Vec::new());
        let indices = (0..point_count as u32).collect();
        let curve = Curve::new(indices, closed);
        Self {
            meshes: vec![mesh],
            splats: Vec::new(),
            curves: vec![curve],
            volumes: Vec::new(),
            materials: MaterialLibrary::default(),
        }
    }

    pub fn with_volume(volume: Volume) -> Self {
        Self {
            meshes: Vec::new(),
            splats: Vec::new(),
            curves: Vec::new(),
            volumes: vec![volume],
            materials: MaterialLibrary::default(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.meshes.is_empty()
            && self.splats.is_empty()
            && self.curves.is_empty()
            && self.volumes.is_empty()
    }

    pub fn append(&mut self, mut other: Geometry) {
        let mut self_mesh = take_merged_mesh(&mut self.meshes);
        let mut other_mesh = take_merged_mesh(&mut other.meshes);
        let point_offset = self_mesh
            .as_ref()
            .map(|mesh| mesh.positions.len() as u32)
            .unwrap_or(0);

        match (self_mesh.take(), other_mesh.take()) {
            (Some(a), Some(b)) => {
                self.meshes = vec![Mesh::merge(&[a, b])];
            }
            (Some(a), None) => {
                self.meshes = vec![a];
            }
            (None, Some(b)) => {
                self.meshes = vec![b];
            }
            (None, None) => {
                self.meshes.clear();
            }
        }

        self.splats.append(&mut other.splats);
        for mut curve in other.curves {
            if point_offset > 0 {
                curve.offset_indices(point_offset);
            }
            self.curves.push(curve);
        }
        self.volumes.append(&mut other.volumes);
        self.materials.merge(&other.materials);
    }

    pub fn merged_mesh(&self) -> Option<Mesh> {
        match self.meshes.len() {
            0 => None,
            1 => Some(self.meshes[0].clone()),
            _ => Some(Mesh::merge(&self.meshes)),
        }
    }

    pub fn merged_splats(&self) -> Option<SplatGeo> {
        match self.splats.len() {
            0 => None,
            1 => Some(self.splats[0].clone()),
            _ => Some(merge_splats(&self.splats)),
        }
    }
}

fn take_merged_mesh(meshes: &mut Vec<Mesh>) -> Option<Mesh> {
    match meshes.len() {
        0 => None,
        1 => meshes.pop(),
        _ => {
            let merged = Mesh::merge(meshes);
            meshes.clear();
            Some(merged)
        }
    }
}

pub fn merge_splats(splats: &[SplatGeo]) -> SplatGeo {
    let total: usize = splats.iter().map(|s| s.len()).sum();
    let mut merged = SplatGeo::default();
    let max_coeffs = splats
        .iter()
        .map(|s| s.sh_coeffs)
        .max()
        .unwrap_or(0);
    merged.positions.reserve(total);
    merged.rotations.reserve(total);
    merged.scales.reserve(total);
    merged.opacity.reserve(total);
    merged.sh0.reserve(total);
    if max_coeffs > 0 {
        merged.sh_coeffs = max_coeffs;
        merged.sh_rest.reserve(total * max_coeffs);
    }
    for splat in splats {
        merged.positions.extend_from_slice(&splat.positions);
        merged.rotations.extend_from_slice(&splat.rotations);
        merged.scales.extend_from_slice(&splat.scales);
        merged.opacity.extend_from_slice(&splat.opacity);
        merged.sh0.extend_from_slice(&splat.sh0);
        if max_coeffs > 0 {
            let coeffs = splat.sh_coeffs;
            if coeffs == 0 {
                merged
                    .sh_rest
                    .extend(std::iter::repeat_n([0.0, 0.0, 0.0], splat.len() * max_coeffs));
            } else {
                for i in 0..splat.len() {
                    let base = i * coeffs;
                    for c in 0..max_coeffs {
                        let value = if c < coeffs {
                            splat.sh_rest[base + c]
                        } else {
                            [0.0, 0.0, 0.0]
                        };
                        merged.sh_rest.push(value);
                    }
                }
            }
        }
    }

    merged.attributes = merge_splat_attributes(splats);
    merged.groups = merge_splat_groups(splats);
    merged
}

fn merge_splat_attributes(splats: &[SplatGeo]) -> MeshAttributes {
    let mut merged = MeshAttributes::default();
    if splats.is_empty() {
        return merged;
    }

    for domain in [AttributeDomain::Point, AttributeDomain::Primitive, AttributeDomain::Detail] {
        let first = splats[0].attributes.map(domain);
        for (name, storage) in first {
            let data_type = storage.data_type();
            let mut compatible = true;
            for splat in &splats[1..] {
                let Some(other) = splat.attributes.get(domain, name) else {
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
                    for splat in &splats[1..] {
                        let Some(other) = splat.attributes.get(domain, name) else {
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
                        AttributeStorage::StringTable(_) => {
                            AttributeStorage::StringTable(StringTableAttribute::new(Vec::new(), Vec::new()))
                        }
                    };
                    for splat in splats {
                        let expected = splat.attribute_domain_len(domain);
                        let Some(current) = splat.attributes.get(domain, name) else {
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
                            (
                                AttributeStorage::StringTable(out),
                                AttributeStorage::StringTable(values),
                            ) => {
                                if !merge_string_table_attribute(out, values) {
                                    compatible = false;
                                    break;
                                }
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

fn merge_splat_groups(splats: &[SplatGeo]) -> crate::mesh::MeshGroups {
    let mut merged = crate::mesh::MeshGroups::default();
    if splats.is_empty() {
        return merged;
    }

    for domain in [AttributeDomain::Point, AttributeDomain::Primitive] {
        let mut names = BTreeSet::new();
        for splat in splats {
            names.extend(splat.groups.map(domain).keys().cloned());
        }
        for name in names {
            let mut values = Vec::new();
            for splat in splats {
                let len = splat.attribute_domain_len(domain);
                if let Some(group) = splat.groups.map(domain).get(&name) {
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

fn merge_string_table_attribute(
    combined: &mut StringTableAttribute,
    source: &StringTableAttribute,
) -> bool {
    if source.indices.is_empty() {
        return true;
    }

    let mut lookup: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for (idx, value) in combined.values.iter().enumerate() {
        lookup.insert(value.clone(), idx as u32);
    }

    for &index in &source.indices {
        let value = source.values.get(index as usize).cloned().unwrap_or_default();
        let entry = if let Some(&existing) = lookup.get(&value) {
            existing
        } else {
            let new_index = combined.values.len() as u32;
            combined.values.push(value.clone());
            lookup.insert(value, new_index);
            new_index
        };
        combined.indices.push(entry);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_splats_concatenates() {
        let mut a = SplatGeo::with_len(1);
        a.positions[0] = [1.0, 2.0, 3.0];
        let mut b = SplatGeo::with_len(1);
        b.positions[0] = [4.0, 5.0, 6.0];

        let merged = merge_splats(&[a, b]);
        assert_eq!(merged.positions.len(), 2);
        assert_eq!(merged.positions[0], [1.0, 2.0, 3.0]);
        assert_eq!(merged.positions[1], [4.0, 5.0, 6.0]);
    }

    #[test]
    fn merge_splats_pads_sh_coeffs() {
        let mut a = SplatGeo::with_len_and_sh(1, 3);
        a.sh_rest[0] = [1.0, 2.0, 3.0];
        let b = SplatGeo::with_len(1);

        let merged = merge_splats(&[a, b]);
        assert_eq!(merged.sh_coeffs, 3);
        assert_eq!(merged.sh_rest.len(), 2 * 3);
        assert_eq!(merged.sh_rest[0], [1.0, 2.0, 3.0]);
        assert_eq!(merged.sh_rest[3], [0.0, 0.0, 0.0]);
    }
}
