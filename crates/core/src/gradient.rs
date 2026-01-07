#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorStop {
    pub pos: f32,
    pub color: [f32; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorGradient {
    pub stops: Vec<ColorStop>,
}

impl Default for ColorGradient {
    fn default() -> Self {
        Self {
            stops: vec![
                ColorStop {
                    pos: 0.0,
                    color: [0.0, 0.0, 0.0],
                },
                ColorStop {
                    pos: 1.0,
                    color: [1.0, 1.0, 1.0],
                },
            ],
        }
    }
}

impl fmt::Display for ColorGradient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.stops.is_empty() {
            return Ok(());
        }
        let mut stops = self.stops.clone();
        normalize_stops(&mut stops);
        for (index, stop) in stops.into_iter().enumerate() {
            if index > 0 {
                f.write_str(";")?;
            }
            write!(
                f,
                "{:.3}:{:.3},{:.3},{:.3}",
                stop.pos, stop.color[0], stop.color[1], stop.color[2]
            )?;
        }
        Ok(())
    }
}

impl ColorGradient {
    pub fn sample(&self, t: f32) -> [f32; 3] {
        if self.stops.is_empty() {
            return [1.0, 1.0, 1.0];
        }
        let t = t.clamp(0.0, 1.0);
        if self.stops.len() == 1 {
            return self.stops[0].color;
        }
        let mut prev = self.stops[0];
        for stop in &self.stops[1..] {
            if t <= stop.pos {
                let denom = (stop.pos - prev.pos).max(1.0e-6);
                let u = ((t - prev.pos) / denom).clamp(0.0, 1.0);
                return [
                    lerp(prev.color[0], stop.color[0], u),
                    lerp(prev.color[1], stop.color[1], u),
                    lerp(prev.color[2], stop.color[2], u),
                ];
            }
            prev = *stop;
        }
        prev.color
    }

    pub fn endpoints(&self) -> (usize, usize) {
        if self.stops.is_empty() {
            return (0, 0);
        }
        let mut min_idx = 0usize;
        let mut max_idx = 0usize;
        let mut min_pos = self.stops[0].pos;
        let mut max_pos = self.stops[0].pos;
        for (idx, stop) in self.stops.iter().enumerate().skip(1) {
            if stop.pos < min_pos {
                min_pos = stop.pos;
                min_idx = idx;
            }
            if stop.pos > max_pos {
                max_pos = stop.pos;
                max_idx = idx;
            }
        }
        (min_idx, max_idx)
    }
}

pub fn parse_color_gradient(value: &str) -> ColorGradient {
    ColorGradient::parse(value).unwrap_or_default()
}

impl ColorGradient {
    pub fn parse(value: &str) -> Option<Self> {
        let mut stops = Vec::new();
        let cleaned = value.trim();
        if cleaned.is_empty() {
            return None;
        }
        for token in cleaned.split(';') {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            let (pos_str, color_str) = token
                .split_once(':')
                .or_else(|| token.split_once('='))
                .unwrap_or((token, ""));
            let pos = pos_str.trim().parse::<f32>().ok()?;
            let color = parse_color(color_str.trim())?;
            stops.push(ColorStop { pos, color });
        }
        if stops.is_empty() {
            return None;
        }
        normalize_stops(&mut stops);
        if stops.len() == 1 {
            let stop = stops[0];
            stops.push(ColorStop {
                pos: 1.0,
                color: stop.color,
            });
        }
        Some(ColorGradient { stops })
    }
}

fn normalize_stops(stops: &mut [ColorStop]) {
    for stop in stops.iter_mut() {
        if !stop.pos.is_finite() {
            stop.pos = 0.0;
        }
        stop.pos = stop.pos.clamp(0.0, 1.0);
        stop.color = clamp_color(stop.color);
    }
    stops.sort_by(|a, b| {
        a.pos
            .partial_cmp(&b.pos)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

fn parse_color(value: &str) -> Option<[f32; 3]> {
    if value.is_empty() {
        return None;
    }
    let trimmed = value.trim();
    if let Some(hex) = trimmed.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
            return Some([r, g, b]);
        }
    }
    let mut parts = trimmed
        .split([',', ' '])
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>();
    if parts.len() < 3 {
        return None;
    }
    parts.truncate(3);
    let mut values = [0.0f32; 3];
    for (idx, part) in parts.iter().enumerate() {
        values[idx] = part.parse::<f32>().ok()?;
    }
    if values.iter().any(|v| *v > 1.5) {
        values = [values[0] / 255.0, values[1] / 255.0, values[2] / 255.0];
    }
    Some(clamp_color(values))
}

fn clamp_color(color: [f32; 3]) -> [f32; 3] {
    [
        color[0].clamp(0.0, 1.0),
        color[1].clamp(0.0, 1.0),
        color[2].clamp(0.0, 1.0),
    ]
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
use std::fmt;
