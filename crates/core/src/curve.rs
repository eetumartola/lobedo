use glam::Mat4;

use crate::attributes::{AttributeDomain, MeshAttributes};
use crate::mesh::MeshGroups;

#[derive(Debug, Clone, Default)]
pub struct Curve {
    pub points: Vec<[f32; 3]>,
    pub closed: bool,
    pub attributes: MeshAttributes,
    pub groups: MeshGroups,
}

impl Curve {
    pub fn new(points: Vec<[f32; 3]>, closed: bool) -> Self {
        Self {
            points,
            closed,
            attributes: MeshAttributes::default(),
            groups: MeshGroups::default(),
        }
    }

    pub fn primitive_count(&self) -> usize {
        if self.points.len() < 2 {
            0
        } else if self.closed {
            self.points.len()
        } else {
            self.points.len() - 1
        }
    }

    pub fn attribute_domain_len(&self, domain: AttributeDomain) -> usize {
        match domain {
            AttributeDomain::Point => self.points.len(),
            AttributeDomain::Vertex => self.primitive_count() * 2,
            AttributeDomain::Primitive => self.primitive_count(),
            AttributeDomain::Detail => 1,
        }
    }

    pub fn transform(&mut self, matrix: Mat4) {
        for point in &mut self.points {
            let v = matrix.transform_point3((*point).into());
            *point = v.to_array();
        }
    }
}

pub fn parse_curve_points(text: &str) -> Vec<[f32; 3]> {
    let mut points = Vec::new();
    for chunk in text.split(';') {
        let cleaned = chunk.replace(',', " ");
        let mut iter = cleaned.split_whitespace();
        let x = iter.next().and_then(|v| v.parse::<f32>().ok());
        let y = iter.next().and_then(|v| v.parse::<f32>().ok());
        let z = iter.next().and_then(|v| v.parse::<f32>().ok());
        if let (Some(x), Some(y), Some(z)) = (x, y, z) {
            points.push([x, y, z]);
        }
    }
    points
}

pub fn encode_curve_points(points: &[[f32; 3]]) -> String {
    points
        .iter()
        .map(|p| format!("{:.6} {:.6} {:.6}", p[0], p[1], p[2]))
        .collect::<Vec<_>>()
        .join("; ")
}

pub fn sample_catmull_rom(points: &[[f32; 3]], steps: usize, closed: bool) -> Vec<[f32; 3]> {
    if points.len() < 2 {
        return points.to_vec();
    }
    let steps = steps.max(1);
    let count = points.len();
    let mut samples = Vec::new();
    let segment_count = if closed { count } else { count - 1 };

    for i in 0..segment_count {
        let i0 = if i == 0 { if closed { count - 1 } else { 0 } } else { i - 1 };
        let i1 = i;
        let i2 = (i + 1) % count;
        let i3 = if i + 2 < count {
            i + 2
        } else if closed {
            (i + 2) % count
        } else {
            i2
        };
        let p0 = points[i0];
        let p1 = points[i1];
        let p2 = points[i2];
        let p3 = points[i3];

        for step in 0..steps {
            let t = step as f32 / steps as f32;
            let t2 = t * t;
            let t3 = t2 * t;
            let mut out = [0.0f32; 3];
            for axis in 0..3 {
                let v0 = p0[axis];
                let v1 = p1[axis];
                let v2 = p2[axis];
                let v3 = p3[axis];
                out[axis] = 0.5
                    * (2.0 * v1
                        + (-v0 + v2) * t
                        + (2.0 * v0 - 5.0 * v1 + 4.0 * v2 - v3) * t2
                        + (-v0 + 3.0 * v1 - 3.0 * v2 + v3) * t3);
            }
            samples.push(out);
        }
    }
    if !closed {
        if let Some(last) = points.last() {
            samples.push(*last);
        }
    }
    samples
}
