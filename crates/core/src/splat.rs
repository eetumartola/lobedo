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

#[derive(Debug, Clone, Copy)]
enum PlyScalarType {
    Int8,
    Uint8,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Float32,
    Float64,
}

impl PlyScalarType {
    fn size(self) -> usize {
        match self {
            PlyScalarType::Int8 | PlyScalarType::Uint8 => 1,
            PlyScalarType::Int16 | PlyScalarType::Uint16 => 2,
            PlyScalarType::Int32 | PlyScalarType::Uint32 | PlyScalarType::Float32 => 4,
            PlyScalarType::Float64 => 8,
        }
    }
}

#[derive(Debug)]
struct PlyProperty {
    name: String,
    data_type: PlyScalarType,
}

#[derive(Debug)]
struct PlyHeader {
    format: PlyFormat,
    vertex_count: usize,
    vertex_properties: Vec<PlyProperty>,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_splat_ply(path: &str) -> Result<SplatGeo, String> {
    let data = std::fs::read(path).map_err(|err| err.to_string())?;
    parse_splat_ply_bytes(&data)
}

#[cfg(target_arch = "wasm32")]
pub fn load_splat_ply(_path: &str) -> Result<SplatGeo, String> {
    Err("Read Splats is not supported in web builds".to_string())
}

fn parse_splat_ply_bytes(data: &[u8]) -> Result<SplatGeo, String> {
    let (header, data_start) = parse_header_bytes(data)?;
    let indices = SplatPropertyIndices::from_properties(&header.vertex_properties);
    if indices.x.is_none() || indices.y.is_none() || indices.z.is_none() {
        return Err("PLY is missing position properties (x, y, z)".to_string());
    }

    match header.format {
        PlyFormat::Ascii => {
            let text = std::str::from_utf8(&data[data_start..])
                .map_err(|_| "PLY ASCII data is not UTF-8".to_string())?;
            parse_ascii_vertices(text, &header, &indices)
        }
        PlyFormat::BinaryLittle => parse_binary_vertices(&data[data_start..], &header, &indices, true),
        PlyFormat::BinaryBig => parse_binary_vertices(&data[data_start..], &header, &indices, false),
    }
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
                let data_type = parse_scalar_type(prop_type)?;
                let name = parts.next().unwrap_or("").to_string();
                if name.is_empty() {
                    return Err("PLY property missing name".to_string());
                }
                vertex_properties.push(PlyProperty { name, data_type });
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

fn parse_header_bytes(data: &[u8]) -> Result<(PlyHeader, usize), String> {
    let mut line_start = 0usize;
    let mut header_end = None;
    for (idx, byte) in data.iter().enumerate() {
        if *byte != b'\n' {
            continue;
        }
        let line_bytes = &data[line_start..idx];
        let line_str = std::str::from_utf8(line_bytes)
            .map_err(|_| "PLY header is not ASCII".to_string())?;
        let line = line_str.trim_end_matches('\r').trim();
        if line == "end_header" {
            header_end = Some(idx + 1);
            break;
        }
        line_start = idx + 1;
    }

    let header_end = header_end.ok_or_else(|| "PLY header is missing end_header".to_string())?;
    let header_text = std::str::from_utf8(&data[..header_end])
        .map_err(|_| "PLY header is not ASCII".to_string())?;
    let mut lines = header_text.lines();
    let header = parse_header(&mut lines)?;
    Ok((header, header_end))
}

fn parse_scalar_type(value: &str) -> Result<PlyScalarType, String> {
    match value {
        "char" | "int8" => Ok(PlyScalarType::Int8),
        "uchar" | "uint8" => Ok(PlyScalarType::Uint8),
        "short" | "int16" => Ok(PlyScalarType::Int16),
        "ushort" | "uint16" => Ok(PlyScalarType::Uint16),
        "int" | "int32" => Ok(PlyScalarType::Int32),
        "uint" | "uint32" => Ok(PlyScalarType::Uint32),
        "float" | "float32" => Ok(PlyScalarType::Float32),
        "double" | "float64" => Ok(PlyScalarType::Float64),
        _ => Err("Unsupported PLY property type".to_string()),
    }
}

fn parse_ascii_vertices(
    text: &str,
    header: &PlyHeader,
    indices: &SplatPropertyIndices,
) -> Result<SplatGeo, String> {
    let mut splats = SplatGeo::with_len(header.vertex_count);
    let mut read = 0usize;
    for line in text.lines() {
        if read >= header.vertex_count {
            break;
        }
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

        fill_splat_from_values(&mut splats, read, &values, indices);
        read += 1;
    }

    if read < header.vertex_count {
        return Err("Unexpected end of PLY vertex data".to_string());
    }

    splats.validate()?;
    Ok(splats)
}

fn parse_binary_vertices(
    data: &[u8],
    header: &PlyHeader,
    indices: &SplatPropertyIndices,
    little_endian: bool,
) -> Result<SplatGeo, String> {
    let mut splats = SplatGeo::with_len(header.vertex_count);
    let mut values = vec![0.0f32; header.vertex_properties.len()];
    let mut cursor = 0usize;

    for read in 0..header.vertex_count {
        for (idx, prop) in header.vertex_properties.iter().enumerate() {
            let size = prop.data_type.size();
            let end = cursor + size;
            if end > data.len() {
                return Err("Unexpected end of binary PLY data".to_string());
            }
            values[idx] = read_scalar(&data[cursor..end], prop.data_type, little_endian)?;
            cursor = end;
        }
        fill_splat_from_values(&mut splats, read, &values, indices);
    }

    splats.validate()?;
    Ok(splats)
}

fn read_scalar(data: &[u8], data_type: PlyScalarType, little_endian: bool) -> Result<f32, String> {
    let value = match data_type {
        PlyScalarType::Int8 => data
            .first()
            .copied()
            .ok_or_else(|| "Invalid PLY data".to_string())? as i8 as f32,
        PlyScalarType::Uint8 => data
            .first()
            .copied()
            .ok_or_else(|| "Invalid PLY data".to_string())? as f32,
        PlyScalarType::Int16 => {
            if data.len() < 2 {
                return Err("Invalid PLY data".to_string());
            }
            let mut bytes = [0u8; 2];
            bytes.copy_from_slice(&data[..2]);
            if little_endian {
                i16::from_le_bytes(bytes) as f32
            } else {
                i16::from_be_bytes(bytes) as f32
            }
        }
        PlyScalarType::Uint16 => {
            if data.len() < 2 {
                return Err("Invalid PLY data".to_string());
            }
            let mut bytes = [0u8; 2];
            bytes.copy_from_slice(&data[..2]);
            if little_endian {
                u16::from_le_bytes(bytes) as f32
            } else {
                u16::from_be_bytes(bytes) as f32
            }
        }
        PlyScalarType::Int32 => {
            if data.len() < 4 {
                return Err("Invalid PLY data".to_string());
            }
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&data[..4]);
            if little_endian {
                i32::from_le_bytes(bytes) as f32
            } else {
                i32::from_be_bytes(bytes) as f32
            }
        }
        PlyScalarType::Uint32 => {
            if data.len() < 4 {
                return Err("Invalid PLY data".to_string());
            }
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&data[..4]);
            if little_endian {
                u32::from_le_bytes(bytes) as f32
            } else {
                u32::from_be_bytes(bytes) as f32
            }
        }
        PlyScalarType::Float32 => {
            if data.len() < 4 {
                return Err("Invalid PLY data".to_string());
            }
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&data[..4]);
            if little_endian {
                f32::from_le_bytes(bytes)
            } else {
                f32::from_be_bytes(bytes)
            }
        }
        PlyScalarType::Float64 => {
            if data.len() < 8 {
                return Err("Invalid PLY data".to_string());
            }
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&data[..8]);
            if little_endian {
                f64::from_le_bytes(bytes) as f32
            } else {
                f64::from_be_bytes(bytes) as f32
            }
        }
    };
    Ok(value)
}

fn fill_splat_from_values(
    splats: &mut SplatGeo,
    read: usize,
    values: &[f32],
    indices: &SplatPropertyIndices,
) {
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
    } else if let (Some(r), Some(g), Some(b)) = (indices.color[0], indices.color[1], indices.color[2]) {
        let mut color = [values[r], values[g], values[b]];
        if color.iter().any(|v| *v > 1.5) {
            color = [color[0] / 255.0, color[1] / 255.0, color[2] / 255.0];
        }
        splats.sh0[read] = color;
    }
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
    fn from_properties(properties: &[PlyProperty]) -> Self {
        let mut indices = SplatPropertyIndices::default();
        for (idx, prop) in properties.iter().enumerate() {
            match prop.name.as_str() {
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
    use super::parse_splat_ply_bytes;

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

        let splats = parse_splat_ply_bytes(data.as_bytes()).expect("parse");
        assert_eq!(splats.len(), 2);
        assert!((splats.opacity[0] - 0.5).abs() < 1.0e-6);
        assert_eq!(splats.sh0[1], [0.4, 0.5, 0.6]);
    }

    #[test]
    fn parse_binary_ply_positions_and_opacity() {
        let header = "\
ply
format binary_little_endian 1.0
element vertex 1
property float x
property float y
property float z
property float opacity
end_header
";
        let mut data = Vec::from(header.as_bytes());
        data.extend_from_slice(&1.0f32.to_le_bytes());
        data.extend_from_slice(&2.0f32.to_le_bytes());
        data.extend_from_slice(&3.0f32.to_le_bytes());
        data.extend_from_slice(&0.25f32.to_le_bytes());

        let splats = parse_splat_ply_bytes(&data).expect("parse");
        assert_eq!(splats.len(), 1);
        assert_eq!(splats.positions[0], [1.0, 2.0, 3.0]);
        assert!((splats.opacity[0] - 0.25).abs() < 1.0e-6);
    }
}
