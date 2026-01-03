#[derive(Debug, Clone, Default)]
pub struct SplatGeo {
    pub positions: Vec<[f32; 3]>,
    pub rotations: Vec<[f32; 4]>,
    pub scales: Vec<[f32; 3]>,
    pub opacity: Vec<f32>,
    pub sh0: Vec<[f32; 3]>,
}

impl SplatGeo {
    pub fn with_len(count: usize) -> Self {
        Self {
            positions: vec![[0.0, 0.0, 0.0]; count],
            rotations: vec![[0.0, 0.0, 0.0, 1.0]; count],
            scales: vec![[1.0, 1.0, 1.0]; count],
            opacity: vec![1.0; count],
            sh0: vec![[1.0, 1.0, 1.0]; count],
        }
    }

    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    pub fn validate(&self) -> Result<(), String> {
        let count = self.positions.len();
        if self.rotations.len() != count
            || self.scales.len() != count
            || self.opacity.len() != count
            || self.sh0.len() != count
        {
            return Err("SplatGeo arrays have inconsistent lengths".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum PlyFormat {
    Ascii,
    BinaryLittle,
    BinaryBig,
}

#[derive(Debug)]
struct PlyHeader {
    format: PlyFormat,
    vertex_count: usize,
    vertex_properties: Vec<String>,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_splat_ply(path: &str) -> Result<SplatGeo, String> {
    let data = std::fs::read(path).map_err(|err| err.to_string())?;
    let text = std::str::from_utf8(&data).map_err(|_| "PLY must be ASCII".to_string())?;
    parse_splat_ply(text)
}

#[cfg(target_arch = "wasm32")]
pub fn load_splat_ply(_path: &str) -> Result<SplatGeo, String> {
    Err("Read Splats is not supported in web builds".to_string())
}

fn parse_splat_ply(data: &str) -> Result<SplatGeo, String> {
    let mut lines = data.lines();
    let header = parse_header(&mut lines)?;
    match header.format {
        PlyFormat::Ascii => {}
        PlyFormat::BinaryLittle | PlyFormat::BinaryBig => {
            return Err("Binary PLY is not supported yet".to_string());
        }
    }

    let indices = SplatPropertyIndices::from_properties(&header.vertex_properties);
    if indices.x.is_none() || indices.y.is_none() || indices.z.is_none() {
        return Err("PLY is missing position properties (x, y, z)".to_string());
    }

    let mut splats = SplatGeo::with_len(header.vertex_count);
    let mut read = 0usize;
    while read < header.vertex_count {
        let line = lines
            .next()
            .ok_or_else(|| "Unexpected end of PLY vertex data".to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let values: Vec<f32> = line
            .split_whitespace()
            .map(|token| token.parse::<f32>().map_err(|_| "Invalid PLY value".to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        if values.len() < header.vertex_properties.len() {
            return Err("PLY vertex row has too few values".to_string());
        }

        let x = values[indices.x.unwrap()];
        let y = values[indices.y.unwrap()];
        let z = values[indices.z.unwrap()];
        splats.positions[read] = [x, y, z];

        if let Some(idx) = indices.opacity {
            splats.opacity[read] = values[idx];
        }

        if let (Some(sx), Some(sy), Some(sz)) = (indices.scale[0], indices.scale[1], indices.scale[2])
        {
            splats.scales[read] = [values[sx], values[sy], values[sz]];
        }

        if let (Some(r0), Some(r1), Some(r2), Some(r3)) =
            (indices.rot[0], indices.rot[1], indices.rot[2], indices.rot[3])
        {
            splats.rotations[read] = [values[r0], values[r1], values[r2], values[r3]];
        }

        if let (Some(c0), Some(c1), Some(c2)) = (indices.sh0[0], indices.sh0[1], indices.sh0[2]) {
            splats.sh0[read] = [values[c0], values[c1], values[c2]];
        } else if let (Some(r), Some(g), Some(b)) =
            (indices.color[0], indices.color[1], indices.color[2])
        {
            let mut color = [values[r], values[g], values[b]];
            if color.iter().any(|v| *v > 1.5) {
                color = [color[0] / 255.0, color[1] / 255.0, color[2] / 255.0];
            }
            splats.sh0[read] = color;
        }

        read += 1;
    }

    splats.validate()?;
    Ok(splats)
}

fn parse_header<'a, I>(lines: &mut I) -> Result<PlyHeader, String>
where
    I: Iterator<Item = &'a str>,
{
    let first = lines
        .next()
        .ok_or_else(|| "PLY header is missing".to_string())?;
    if first.trim() != "ply" {
        return Err("Not a PLY file".to_string());
    }

    let mut format = None;
    let mut vertex_count = None;
    let mut vertex_properties = Vec::new();
    let mut in_vertex = false;

    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with("comment") {
            continue;
        }
        if line == "end_header" {
            break;
        }

        let mut parts = line.split_whitespace();
        let Some(tag) = parts.next() else {
            continue;
        };
        match tag {
            "format" => {
                let fmt = parts.next().unwrap_or("");
                format = Some(match fmt {
                    "ascii" => PlyFormat::Ascii,
                    "binary_little_endian" => PlyFormat::BinaryLittle,
                    "binary_big_endian" => PlyFormat::BinaryBig,
                    _ => return Err("Unknown PLY format".to_string()),
                });
            }
            "element" => {
                let name = parts.next().unwrap_or("");
                let count = parts
                    .next()
                    .ok_or_else(|| "Malformed PLY element".to_string())?
                    .parse::<usize>()
                    .map_err(|_| "Malformed PLY element count".to_string())?;
                in_vertex = name == "vertex";
                if in_vertex {
                    vertex_count = Some(count);
                }
            }
            "property" if in_vertex => {
                let prop_type = parts.next().unwrap_or("");
                if prop_type == "list" {
                    return Err("PLY vertex list properties are not supported".to_string());
                }
                let name = parts.next().unwrap_or("").to_string();
                if name.is_empty() {
                    return Err("PLY property missing name".to_string());
                }
                vertex_properties.push(name);
            }
            _ => {}
        }
    }

    let format = format.ok_or_else(|| "PLY format not specified".to_string())?;
    let vertex_count = vertex_count.ok_or_else(|| "PLY has no vertex element".to_string())?;
    Ok(PlyHeader {
        format,
        vertex_count,
        vertex_properties,
    })
}

#[derive(Default)]
struct SplatPropertyIndices {
    x: Option<usize>,
    y: Option<usize>,
    z: Option<usize>,
    opacity: Option<usize>,
    scale: [Option<usize>; 3],
    rot: [Option<usize>; 4],
    sh0: [Option<usize>; 3],
    color: [Option<usize>; 3],
}

impl SplatPropertyIndices {
    fn from_properties(properties: &[String]) -> Self {
        let mut indices = SplatPropertyIndices::default();
        for (idx, name) in properties.iter().enumerate() {
            match name.as_str() {
                "x" => indices.x = Some(idx),
                "y" => indices.y = Some(idx),
                "z" => indices.z = Some(idx),
                "opacity" => indices.opacity = Some(idx),
                "scale_0" | "scale_x" => indices.scale[0] = Some(idx),
                "scale_1" | "scale_y" => indices.scale[1] = Some(idx),
                "scale_2" | "scale_z" => indices.scale[2] = Some(idx),
                "rot_0" | "rotation_0" | "q_w" => indices.rot[0] = Some(idx),
                "rot_1" | "rotation_1" | "q_x" => indices.rot[1] = Some(idx),
                "rot_2" | "rotation_2" | "q_y" => indices.rot[2] = Some(idx),
                "rot_3" | "rotation_3" | "q_z" => indices.rot[3] = Some(idx),
                "f_dc_0" | "sh0_0" => indices.sh0[0] = Some(idx),
                "f_dc_1" | "sh0_1" => indices.sh0[1] = Some(idx),
                "f_dc_2" | "sh0_2" => indices.sh0[2] = Some(idx),
                "red" | "r" => indices.color[0] = Some(idx),
                "green" | "g" => indices.color[1] = Some(idx),
                "blue" | "b" => indices.color[2] = Some(idx),
                _ => {}
            }
        }
        indices
    }
}

#[cfg(test)]
mod tests {
    use super::parse_splat_ply;

    #[test]
    fn parse_ascii_ply_positions_and_sh0() {
        let data = "\
ply
format ascii 1.0
element vertex 2
property float x
property float y
property float z
property float opacity
property float scale_0
property float scale_1
property float scale_2
property float f_dc_0
property float f_dc_1
property float f_dc_2
end_header
0 0 0 0.5 1 1 1 0.1 0.2 0.3
1 2 3 1.0 2 2 2 0.4 0.5 0.6
";

        let splats = parse_splat_ply(data).expect("parse");
        assert_eq!(splats.len(), 2);
        assert!((splats.opacity[0] - 0.5).abs() < 1.0e-6);
        assert_eq!(splats.sh0[1], [0.4, 0.5, 0.6]);
    }
}
