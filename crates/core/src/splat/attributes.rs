use crate::attributes::{
    AttributeDomain, AttributeError, AttributeInfo, AttributeRef, AttributeStorage, AttributeType,
};

use super::SplatGeo;

impl SplatGeo {
    pub fn attribute_domain_len(&self, domain: AttributeDomain) -> usize {
        match domain {
            AttributeDomain::Point => self.positions.len(),
            AttributeDomain::Vertex => 0,
            AttributeDomain::Primitive => self.positions.len(),
            AttributeDomain::Detail => {
                if self.positions.is_empty() {
                    0
                } else {
                    1
                }
            }
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
        if !self.rotations.is_empty() {
            list.push(AttributeInfo {
                name: "orient".to_string(),
                domain: AttributeDomain::Point,
                data_type: AttributeType::Vec4,
                len: self.rotations.len(),
                implicit: true,
            });
        }
        if !self.scales.is_empty() {
            list.push(AttributeInfo {
                name: "scale".to_string(),
                domain: AttributeDomain::Point,
                data_type: AttributeType::Vec3,
                len: self.scales.len(),
                implicit: true,
            });
        }
        if !self.opacity.is_empty() {
            list.push(AttributeInfo {
                name: "opacity".to_string(),
                domain: AttributeDomain::Point,
                data_type: AttributeType::Float,
                len: self.opacity.len(),
                implicit: true,
            });
        }
        if !self.sh0.is_empty() {
            list.push(AttributeInfo {
                name: "Cd".to_string(),
                domain: AttributeDomain::Point,
                data_type: AttributeType::Vec3,
                len: self.sh0.len(),
                implicit: true,
            });
        }
        for domain in [AttributeDomain::Point, AttributeDomain::Primitive, AttributeDomain::Detail] {
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
            ("Cd", AttributeDomain::Point)
            | ("Cd", AttributeDomain::Primitive)
            | ("sh0", AttributeDomain::Point)
            | ("sh0", AttributeDomain::Primitive) => {
                Some(AttributeRef::Vec3(self.sh0.as_slice()))
            }
            ("opacity", AttributeDomain::Point) | ("opacity", AttributeDomain::Primitive) => {
                Some(AttributeRef::Float(self.opacity.as_slice()))
            }
            ("scale", AttributeDomain::Point) | ("scale", AttributeDomain::Primitive) => {
                Some(AttributeRef::Vec3(self.scales.as_slice()))
            }
            ("orient", AttributeDomain::Point)
            | ("orient", AttributeDomain::Primitive)
            | ("rot", AttributeDomain::Point)
            | ("rot", AttributeDomain::Primitive) => {
                Some(AttributeRef::Vec4(self.rotations.as_slice()))
            }
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
            ("Cd", AttributeDomain::Point)
            | ("Cd", AttributeDomain::Primitive)
            | ("sh0", AttributeDomain::Point)
            | ("sh0", AttributeDomain::Primitive) => {
                if storage.data_type() != AttributeType::Vec3 {
                    return Err(AttributeError::InvalidType {
                        expected: AttributeType::Vec3,
                        actual: storage.data_type(),
                    });
                }
                if let AttributeStorage::Vec3(values) = storage {
                    self.sh0 = values;
                    return Ok(());
                }
            }
            ("opacity", AttributeDomain::Point) | ("opacity", AttributeDomain::Primitive) => {
                if storage.data_type() != AttributeType::Float {
                    return Err(AttributeError::InvalidType {
                        expected: AttributeType::Float,
                        actual: storage.data_type(),
                    });
                }
                if let AttributeStorage::Float(values) = storage {
                    self.opacity = values;
                    return Ok(());
                }
            }
            ("scale", AttributeDomain::Point) | ("scale", AttributeDomain::Primitive) => {
                if storage.data_type() != AttributeType::Vec3 {
                    return Err(AttributeError::InvalidType {
                        expected: AttributeType::Vec3,
                        actual: storage.data_type(),
                    });
                }
                if let AttributeStorage::Vec3(values) = storage {
                    self.scales = values;
                    return Ok(());
                }
            }
            ("orient", AttributeDomain::Point)
            | ("orient", AttributeDomain::Primitive)
            | ("rot", AttributeDomain::Point)
            | ("rot", AttributeDomain::Primitive) => {
                if storage.data_type() != AttributeType::Vec4 {
                    return Err(AttributeError::InvalidType {
                        expected: AttributeType::Vec4,
                        actual: storage.data_type(),
                    });
                }
                if let AttributeStorage::Vec4(values) = storage {
                    self.rotations = values;
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
            ("Cd", AttributeDomain::Point)
            | ("Cd", AttributeDomain::Primitive)
            | ("sh0", AttributeDomain::Point)
            | ("sh0", AttributeDomain::Primitive) => {
                self.sh0.clear();
                None
            }
            ("opacity", AttributeDomain::Point) | ("opacity", AttributeDomain::Primitive) => {
                self.opacity.clear();
                None
            }
            ("scale", AttributeDomain::Point) | ("scale", AttributeDomain::Primitive) => {
                self.scales.clear();
                None
            }
            ("orient", AttributeDomain::Point)
            | ("orient", AttributeDomain::Primitive)
            | ("rot", AttributeDomain::Point)
            | ("rot", AttributeDomain::Primitive) => {
                self.rotations.clear();
                None
            }
            _ => self.attributes.remove(domain, name),
        }
    }
}
