use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseType {
    Value,
    Perlin,
}

impl NoiseType {
    pub fn from_int(value: i32) -> Self {
        match value {
            1 => NoiseType::Perlin,
            _ => NoiseType::Value,
        }
    }
}

pub fn fbm_noise(
    p: Vec3,
    seed: u32,
    noise_type: NoiseType,
    octaves: u32,
    lacunarity: f32,
    gain: f32,
) -> f32 {
    let mut value = 0.0;
    let mut amp = 1.0;
    let mut freq = 1.0;
    let octaves = octaves.max(1);
    for i in 0..octaves {
        let offset_seed = seed.wrapping_add(i * 1013);
        let n = match noise_type {
            NoiseType::Value => value_noise(p * freq, offset_seed),
            NoiseType::Perlin => perlin_noise(p * freq, offset_seed),
        };
        value += n * amp;
        amp *= gain;
        freq *= lacunarity;
    }
    value
}

pub fn value_noise(p: Vec3, seed: u32) -> f32 {
    let base = p.floor();
    let frac = p - base;
    let f = smooth(frac);

    let x0 = base.x as i32;
    let y0 = base.y as i32;
    let z0 = base.z as i32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;
    let z1 = z0 + 1;

    let c000 = hash_f32(x0, y0, z0, seed);
    let c100 = hash_f32(x1, y0, z0, seed);
    let c010 = hash_f32(x0, y1, z0, seed);
    let c110 = hash_f32(x1, y1, z0, seed);
    let c001 = hash_f32(x0, y0, z1, seed);
    let c101 = hash_f32(x1, y0, z1, seed);
    let c011 = hash_f32(x0, y1, z1, seed);
    let c111 = hash_f32(x1, y1, z1, seed);

    let x00 = lerp(c000, c100, f.x);
    let x10 = lerp(c010, c110, f.x);
    let x01 = lerp(c001, c101, f.x);
    let x11 = lerp(c011, c111, f.x);
    let y0 = lerp(x00, x10, f.y);
    let y1 = lerp(x01, x11, f.y);
    lerp(y0, y1, f.z) * 2.0 - 1.0
}

pub fn perlin_noise(p: Vec3, seed: u32) -> f32 {
    let base = p.floor();
    let frac = p - base;
    let f = fade(frac);

    let x0 = base.x as i32;
    let y0 = base.y as i32;
    let z0 = base.z as i32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;
    let z1 = z0 + 1;

    let g000 = gradient(x0, y0, z0, seed).dot(frac - Vec3::new(0.0, 0.0, 0.0));
    let g100 = gradient(x1, y0, z0, seed).dot(frac - Vec3::new(1.0, 0.0, 0.0));
    let g010 = gradient(x0, y1, z0, seed).dot(frac - Vec3::new(0.0, 1.0, 0.0));
    let g110 = gradient(x1, y1, z0, seed).dot(frac - Vec3::new(1.0, 1.0, 0.0));
    let g001 = gradient(x0, y0, z1, seed).dot(frac - Vec3::new(0.0, 0.0, 1.0));
    let g101 = gradient(x1, y0, z1, seed).dot(frac - Vec3::new(1.0, 0.0, 1.0));
    let g011 = gradient(x0, y1, z1, seed).dot(frac - Vec3::new(0.0, 1.0, 1.0));
    let g111 = gradient(x1, y1, z1, seed).dot(frac - Vec3::new(1.0, 1.0, 1.0));

    let x00 = lerp(g000, g100, f.x);
    let x10 = lerp(g010, g110, f.x);
    let x01 = lerp(g001, g101, f.x);
    let x11 = lerp(g011, g111, f.x);
    let y0 = lerp(x00, x10, f.y);
    let y1 = lerp(x01, x11, f.y);
    lerp(y0, y1, f.z)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn smooth(v: Vec3) -> Vec3 {
    v * v * (Vec3::splat(3.0) - 2.0 * v)
}

fn fade(v: Vec3) -> Vec3 {
    Vec3::new(fade_component(v.x), fade_component(v.y), fade_component(v.z))
}

fn fade_component(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn gradient(x: i32, y: i32, z: i32, seed: u32) -> Vec3 {
    let h = hash_u32(x, y, z, seed);
    let hx = ((h & 0x3ff) as f32 / 511.5) - 1.0;
    let hy = (((h >> 10) & 0x3ff) as f32 / 511.5) - 1.0;
    let hz = (((h >> 20) & 0x3ff) as f32 / 511.5) - 1.0;
    let v = Vec3::new(hx, hy, hz);
    if v.length_squared() > 0.0 {
        v.normalize()
    } else {
        Vec3::Y
    }
}

fn hash_f32(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let h = hash_u32(x, y, z, seed);
    (h as f32) / (u32::MAX as f32)
}

fn hash_u32(x: i32, y: i32, z: i32, seed: u32) -> u32 {
    let mut h = x as u32;
    h ^= (y as u32).wrapping_mul(374761393);
    h = h.rotate_left(13);
    h ^= (z as u32).wrapping_mul(668265263);
    h = h.rotate_left(17);
    h ^= seed.wrapping_mul(2246822519);
    h = h.wrapping_mul(3266489917);
    h ^ (h >> 16)
}
