use crate::assets;
use crate::splat::{SplatGeo, SplatLoadMode};

#[cfg(not(target_arch = "wasm32"))]
use std::io::Read;

#[cfg(not(target_arch = "wasm32"))]
use flate2::read::GzDecoder;

const SPZ_MAGIC: u32 = 0x5053474e;
const COLOR_SCALE: f32 = 0.15;
#[allow(clippy::excessive_precision)]
const SH_C0: f32 = 0.28209479177387814;
#[allow(clippy::excessive_precision)]
const SQRT1_2: f32 = 0.7071067811865475;
const MAX_POINTS: usize = 10_000_000;

#[derive(Clone, Copy)]
struct CoordinateConverter {
    flip_p: [f32; 3],
    flip_q: [f32; 3],
    flip_sh: [f32; 15],
}

#[derive(Clone, Copy)]
enum CoordinateSystem {
    Rub,
    Ruf,
}

fn coordinate_converter(from: CoordinateSystem, to: CoordinateSystem) -> CoordinateConverter {
    let (x_match, y_match, z_match) = axes_match(from, to);
    let x = if x_match { 1.0 } else { -1.0 };
    let y = if y_match { 1.0 } else { -1.0 };
    let z = if z_match { 1.0 } else { -1.0 };
    CoordinateConverter {
        flip_p: [x, y, z],
        flip_q: [y * z, x * z, x * y],
        flip_sh: [
            y,         // 0
            z,         // 1
            x,         // 2
            x * y,     // 3
            y * z,     // 4
            1.0,       // 5
            x * z,     // 6
            1.0,       // 7
            y,         // 8
            x * y * z, // 9
            y,         // 10
            z,         // 11
            x,         // 12
            z,         // 13
            x,         // 14
        ],
    }
}

fn axes_match(from: CoordinateSystem, to: CoordinateSystem) -> (bool, bool, bool) {
    let (fx, fy, fz) = axis_bits(from);
    let (tx, ty, tz) = axis_bits(to);
    (fx == tx, fy == ty, fz == tz)
}

fn axis_bits(system: CoordinateSystem) -> (bool, bool, bool) {
    match system {
        CoordinateSystem::Rub => (true, true, false),
        CoordinateSystem::Ruf => (true, true, true),
    }
}

#[derive(Clone, Copy)]
struct SpzHeader {
    version: u32,
    num_points: usize,
    sh_degree: u8,
    fractional_bits: u8,
    _flags: u8,
}

#[allow(dead_code)]
pub fn load_splat_spz(path: &str) -> Result<SplatGeo, String> {
    load_splat_spz_with_mode(path, SplatLoadMode::Full)
}

pub fn load_splat_spz_with_mode(path: &str, mode: SplatLoadMode) -> Result<SplatGeo, String> {
    if let Some(data) = assets::load_bytes(path) {
        return parse_splat_spz_bytes_with_mode(&data, mode);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if assets::is_url(path) {
            Err(format!("Failed to download URL: {path}"))
        } else {
            let data = std::fs::read(path).map_err(|err| err.to_string())?;
            parse_splat_spz_bytes_with_mode(&data, mode)
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        if assets::is_url(path) {
            Err("Splat URL is downloading; retrying shortly.".to_string())
        } else {
            Err("Splat Read is not supported in web builds without a picked file".to_string())
        }
    }
}

fn parse_splat_spz_bytes_with_mode(data: &[u8], mode: SplatLoadMode) -> Result<SplatGeo, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (data, mode);
        return Err("SPZ decode is not supported in web builds.".to_string());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let decoded = decompress_gzip(data)?;
        let mut offset = 0usize;
        let header = parse_header(&decoded, &mut offset)?;
        if header.num_points > MAX_POINTS {
            return Err(format!(
                "SPZ contains too many points ({})",
                header.num_points
            ));
        }
        if header.sh_degree > 3 {
            return Err(format!("Unsupported SPZ SH degree: {}", header.sh_degree));
        }

        let sh_dim = dim_for_degree(header.sh_degree);
        let uses_float16 = header.version == 1;
        let uses_smallest_three = header.version >= 3;

        let position_bytes = header.num_points * 3 * if uses_float16 { 2 } else { 3 };
        let alpha_bytes = header.num_points;
        let color_bytes = header.num_points * 3;
        let scale_bytes = header.num_points * 3;
        let rotation_bytes = header.num_points * if uses_smallest_three { 4 } else { 3 };
        let sh_bytes = header.num_points * sh_dim * 3;

        let total =
            position_bytes + alpha_bytes + color_bytes + scale_bytes + rotation_bytes + sh_bytes;
        if decoded.len().saturating_sub(offset) < total {
            return Err("SPZ data is truncated".to_string());
        }

        let positions = take_slice(&decoded, &mut offset, position_bytes)?;
        let alphas = take_slice(&decoded, &mut offset, alpha_bytes)?;
        let colors = take_slice(&decoded, &mut offset, color_bytes)?;
        let scales = take_slice(&decoded, &mut offset, scale_bytes)?;
        let rotations = take_slice(&decoded, &mut offset, rotation_bytes)?;
        let sh = take_slice(&decoded, &mut offset, sh_bytes)?;

        let sh_coeffs = if matches!(mode, SplatLoadMode::Full) {
            sh_dim
        } else {
            0
        };
        let mut splats = SplatGeo::with_len_and_sh(header.num_points, sh_coeffs);

        let converter = coordinate_converter(CoordinateSystem::Rub, CoordinateSystem::Ruf);
        decode_positions(
            &mut splats,
            positions,
            uses_float16,
            header.fractional_bits,
            converter,
        )?;
        decode_scales(&mut splats, scales);
        decode_rotations(&mut splats, rotations, uses_smallest_three, converter)?;
        decode_opacity(&mut splats, alphas);
        decode_colors(&mut splats, colors, sh_coeffs == 0);
        if sh_coeffs > 0 && sh_dim > 0 {
            decode_sh(&mut splats, sh, sh_dim, converter)?;
        }

        splats.normalize_on_load();
        splats.validate()?;
        Ok(splats)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = GzDecoder::new(data);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|err| format!("SPZ gzip decode failed: {err}"))?;
    Ok(out)
}

fn parse_header(data: &[u8], offset: &mut usize) -> Result<SpzHeader, String> {
    let magic = read_u32_le(data, offset)?;
    if magic != SPZ_MAGIC {
        return Err("SPZ header magic mismatch".to_string());
    }
    let version = read_u32_le(data, offset)?;
    if !(1..=3).contains(&version) {
        return Err(format!("Unsupported SPZ version: {version}"));
    }
    let num_points = read_u32_le(data, offset)? as usize;
    let sh_degree = read_u8(data, offset)?;
    let fractional_bits = read_u8(data, offset)?;
    let flags = read_u8(data, offset)?;
    let _reserved = read_u8(data, offset)?;

    Ok(SpzHeader {
        version,
        num_points,
        sh_degree,
        fractional_bits,
        _flags: flags,
    })
}

fn read_u32_le(data: &[u8], offset: &mut usize) -> Result<u32, String> {
    if data.len().saturating_sub(*offset) < 4 {
        return Err("Unexpected end of SPZ data".to_string());
    }
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&data[*offset..*offset + 4]);
    *offset += 4;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u8(data: &[u8], offset: &mut usize) -> Result<u8, String> {
    if *offset >= data.len() {
        return Err("Unexpected end of SPZ data".to_string());
    }
    let value = data[*offset];
    *offset += 1;
    Ok(value)
}

fn take_slice<'a>(data: &'a [u8], offset: &mut usize, len: usize) -> Result<&'a [u8], String> {
    let end = offset.saturating_add(len);
    if end > data.len() {
        return Err("Unexpected end of SPZ data".to_string());
    }
    let slice = &data[*offset..end];
    *offset = end;
    Ok(slice)
}

fn dim_for_degree(degree: u8) -> usize {
    match degree {
        0 => 0,
        1 => 3,
        2 => 8,
        3 => 15,
        _ => 0,
    }
}

fn decode_positions(
    splats: &mut SplatGeo,
    data: &[u8],
    uses_float16: bool,
    fractional_bits: u8,
    converter: CoordinateConverter,
) -> Result<(), String> {
    let mut idx = 0usize;
    if uses_float16 {
        for point in 0..splats.len() {
            for axis in 0..3 {
                if idx + 2 > data.len() {
                    return Err("SPZ position data truncated".to_string());
                }
                let value = half_to_f32(u16::from_le_bytes([data[idx], data[idx + 1]]));
                splats.positions[point][axis] = value * converter.flip_p[axis];
                idx += 2;
            }
        }
    } else {
        let scale = 1.0f32 / (1u32 << fractional_bits) as f32;
        for point in 0..splats.len() {
            for axis in 0..3 {
                if idx + 3 > data.len() {
                    return Err("SPZ position data truncated".to_string());
                }
                let mut fixed = data[idx] as i32;
                fixed |= (data[idx + 1] as i32) << 8;
                fixed |= (data[idx + 2] as i32) << 16;
                if (fixed & 0x800000) != 0 {
                    fixed |= 0xff000000u32 as i32;
                }
                let value = fixed as f32 * scale;
                splats.positions[point][axis] = value * converter.flip_p[axis];
                idx += 3;
            }
        }
    }
    Ok(())
}

fn decode_scales(splats: &mut SplatGeo, data: &[u8]) {
    let mut idx = 0usize;
    for point in 0..splats.len() {
        for axis in 0..3 {
            if idx >= data.len() {
                return;
            }
            splats.scales[point][axis] = data[idx] as f32 / 16.0 - 10.0;
            idx += 1;
        }
    }
}

fn decode_rotations(
    splats: &mut SplatGeo,
    data: &[u8],
    uses_smallest_three: bool,
    converter: CoordinateConverter,
) -> Result<(), String> {
    let mut idx = 0usize;
    for point in 0..splats.len() {
        if uses_smallest_three {
            if idx + 4 > data.len() {
                return Err("SPZ rotation data truncated".to_string());
            }
            let rotation = unpack_quaternion_smallest_three(&data[idx..idx + 4], converter.flip_q);
            splats.rotations[point] = rotation;
            idx += 4;
        } else {
            if idx + 3 > data.len() {
                return Err("SPZ rotation data truncated".to_string());
            }
            let rotation = unpack_quaternion_first_three(&data[idx..idx + 3], converter.flip_q);
            splats.rotations[point] = rotation;
            idx += 3;
        }
    }
    Ok(())
}

fn decode_opacity(splats: &mut SplatGeo, data: &[u8]) {
    for (idx, value) in data.iter().enumerate().take(splats.len()) {
        let alpha = (*value as f32 / 255.0).clamp(1.0e-6, 1.0 - 1.0e-6);
        splats.opacity[idx] = logit(alpha);
    }
}

fn decode_colors(splats: &mut SplatGeo, data: &[u8], to_rgb: bool) {
    let mut idx = 0usize;
    for point in 0..splats.len() {
        if idx + 3 > data.len() {
            return;
        }
        let r = ((data[idx] as f32 / 255.0) - 0.5) / COLOR_SCALE;
        let g = ((data[idx + 1] as f32 / 255.0) - 0.5) / COLOR_SCALE;
        let b = ((data[idx + 2] as f32 / 255.0) - 0.5) / COLOR_SCALE;
        idx += 3;
        if to_rgb {
            splats.sh0[point] = [r * SH_C0 + 0.5, g * SH_C0 + 0.5, b * SH_C0 + 0.5];
        } else {
            splats.sh0[point] = [r, g, b];
        }
    }
}

fn decode_sh(
    splats: &mut SplatGeo,
    data: &[u8],
    sh_dim: usize,
    converter: CoordinateConverter,
) -> Result<(), String> {
    let expected = splats.len() * sh_dim * 3;
    if data.len() < expected {
        return Err("SPZ spherical harmonics data truncated".to_string());
    }
    let mut idx = 0usize;
    for point in 0..splats.len() {
        let base = point * sh_dim;
        for coeff in 0..sh_dim {
            let r = unquantize_sh(data[idx]);
            let g = unquantize_sh(data[idx + 1]);
            let b = unquantize_sh(data[idx + 2]);
            let flip = converter.flip_sh[coeff];
            splats.sh_rest[base + coeff] = [r * flip, g * flip, b * flip];
            idx += 3;
        }
    }
    Ok(())
}

fn unpack_quaternion_first_three(bytes: &[u8], flip_q: [f32; 3]) -> [f32; 4] {
    let x = (bytes[0] as f32 / 127.5) - 1.0;
    let y = (bytes[1] as f32 / 127.5) - 1.0;
    let z = (bytes[2] as f32 / 127.5) - 1.0;
    let x = x * flip_q[0];
    let y = y * flip_q[1];
    let z = z * flip_q[2];
    let w = (1.0 - (x * x + y * y + z * z)).max(0.0).sqrt();
    [w, x, y, z]
}

fn unpack_quaternion_smallest_three(bytes: &[u8], flip_q: [f32; 3]) -> [f32; 4] {
    let mut comp = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let mask = (1u32 << 9) - 1;
    let i_largest = (comp >> 30) as usize;
    let mut rotation = [0.0f32; 4];
    let mut sum = 0.0f32;
    for i in (0..4).rev() {
        if i == i_largest {
            continue;
        }
        let mag = comp & mask;
        let negbit = (comp >> 9) & 1;
        comp >>= 10;
        let mut value = SQRT1_2 * (mag as f32) / (mask as f32);
        if negbit == 1 {
            value = -value;
        }
        rotation[i] = value;
        sum += value * value;
    }
    rotation[i_largest] = (1.0 - sum).max(0.0).sqrt();
    rotation[0] *= flip_q[0];
    rotation[1] *= flip_q[1];
    rotation[2] *= flip_q[2];
    [rotation[3], rotation[0], rotation[1], rotation[2]]
}

fn unquantize_sh(value: u8) -> f32 {
    (value as f32 - 128.0) / 128.0
}

fn logit(value: f32) -> f32 {
    (value / (1.0 - value)).ln()
}

fn half_to_f32(bits: u16) -> f32 {
    let sign = ((bits >> 15) & 0x1) as u32;
    let exp = ((bits >> 10) & 0x1f) as i32;
    let frac = (bits & 0x3ff) as u32;
    let f_bits = if exp == 0 {
        if frac == 0 {
            sign << 31
        } else {
            let mut exp_val = -14;
            let mut frac_val = frac;
            while (frac_val & 0x400) == 0 {
                frac_val <<= 1;
                exp_val -= 1;
            }
            frac_val &= 0x3ff;
            let exp_bits = (exp_val + 127) as u32;
            (sign << 31) | (exp_bits << 23) | (frac_val << 13)
        }
    } else if exp == 0x1f {
        (sign << 31) | 0x7f800000 | (frac << 13)
    } else {
        let exp_bits = (exp + 112) as u32;
        (sign << 31) | (exp_bits << 23) | (frac << 13)
    };
    f32::from_bits(f_bits)
}
