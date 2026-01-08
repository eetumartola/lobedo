#[derive(Debug, Clone, Default)]
pub struct Curve {
    pub indices: Vec<u32>,
    pub closed: bool,
}

impl Curve {
    pub fn new(indices: Vec<u32>, closed: bool) -> Self {
        Self { indices, closed }
    }

    pub fn primitive_count(&self) -> usize {
        if self.indices.len() < 2 {
            0
        } else if self.closed {
            self.indices.len()
        } else {
            self.indices.len() - 1
        }
    }

    pub fn offset_indices(&mut self, offset: u32) {
        for index in &mut self.indices {
            *index = index.saturating_add(offset);
        }
    }

    pub fn resolved_points(&self, positions: &[[f32; 3]]) -> Vec<[f32; 3]> {
        let mut out = Vec::with_capacity(self.indices.len());
        for &index in &self.indices {
            if let Some(point) = positions.get(index as usize) {
                out.push(*point);
            }
        }
        out
    }

    pub fn remap_indices(&self, mapping: &[u32]) -> Option<Self> {
        if mapping.is_empty() {
            return None;
        }
        let mut indices = Vec::with_capacity(self.indices.len());
        for &index in &self.indices {
            let mapped = mapping.get(index as usize).copied().unwrap_or(u32::MAX);
            if mapped != u32::MAX {
                indices.push(mapped);
            }
        }
        if indices.len() < 2 {
            return None;
        }
        let mut out = self.clone();
        out.indices = indices;
        Some(out)
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
