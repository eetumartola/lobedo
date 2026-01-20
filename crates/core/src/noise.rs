use glam::{Mat3, Vec3};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseType {
    Fast,
    SparseConvolution,
    Alligator,
    Perlin,
    PerlinFlow,
    Simplex,
    WorleyF1,
    WorleyF2MinusF1,
    ManhattanF1,
    ManhattanF2MinusF1,
    ChebyshevF1,
    ChebyshevF2MinusF1,
    PerlinCloud,
    SimplexCloud,
}

impl NoiseType {
    pub fn from_int(value: i32) -> Self {
        match value {
            1 => NoiseType::SparseConvolution,
            2 => NoiseType::Alligator,
            3 => NoiseType::Perlin,
            4 => NoiseType::PerlinFlow,
            5 => NoiseType::Simplex,
            6 => NoiseType::WorleyF1,
            7 => NoiseType::WorleyF2MinusF1,
            8 => NoiseType::ManhattanF1,
            9 => NoiseType::ManhattanF2MinusF1,
            10 => NoiseType::ChebyshevF1,
            11 => NoiseType::ChebyshevF2MinusF1,
            12 => NoiseType::PerlinCloud,
            13 => NoiseType::SimplexCloud,
            _ => NoiseType::Fast,
        }
    }

    fn frequency_scale(self) -> f32 {
        match self {
            NoiseType::Fast | NoiseType::SparseConvolution => 1.25,
            NoiseType::Alligator => 1.64,
            _ => 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FractalType {
    None,
    Standard,
    Terrain,
    Hybrid,
}

impl FractalType {
    pub fn from_int(value: i32) -> Self {
        match value {
            1 => FractalType::Standard,
            2 => FractalType::Terrain,
            3 => FractalType::Hybrid,
            _ => FractalType::None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FractalSettings {
    pub octaves: u32,
    pub lacunarity: f32,
    pub roughness: f32,
}

pub fn fractal_noise(
    p: Vec3,
    seed: u32,
    noise_type: NoiseType,
    fractal_type: FractalType,
    settings: FractalSettings,
    flow_rotation: f32,
    distortion: f32,
) -> f32 {
    let octaves = settings.octaves.max(1);
    let lacunarity = settings.lacunarity.max(0.0);
    let roughness = settings.roughness.clamp(0.0, 1.0);
    match fractal_type {
        FractalType::None => base_noise(p, seed, noise_type, flow_rotation, distortion),
        FractalType::Standard => {
            let mut value = 0.0;
            let mut amp = 1.0;
            let mut freq = 1.0;
            for i in 0..octaves {
                let offset_seed = seed.wrapping_add(i * 1013);
                let n =
                    base_noise(p * freq, offset_seed, noise_type, flow_rotation, distortion);
                value += n * amp;
                amp *= roughness;
                freq *= lacunarity;
            }
            value
        }
        FractalType::Terrain => {
            let mut value = 0.0;
            let mut amp = 1.0;
            let mut freq = 1.0;
            let mut weight = 1.0;
            for i in 0..octaves {
                let offset_seed = seed.wrapping_add(i * 1013);
                let n =
                    base_noise(p * freq, offset_seed, noise_type, flow_rotation, distortion);
                value += n * amp * weight;
                let n01 = (n * 0.5 + 0.5).clamp(0.0, 1.0);
                weight = n01;
                amp *= roughness;
                freq *= lacunarity;
            }
            value
        }
        FractalType::Hybrid => {
            let mut value = 0.0;
            let mut amp = 1.0;
            let mut freq = 1.0;
            let mut weight = 1.0;
            for i in 0..octaves {
                let offset_seed = seed.wrapping_add(i * 1013);
                let n =
                    base_noise(p * freq, offset_seed, noise_type, flow_rotation, distortion);
                value += n * amp * weight;
                let n01 = (n * 0.5 + 0.5).clamp(0.0, 1.0);
                weight = (n01 * n01).clamp(0.0, 1.0);
                amp *= roughness;
                freq *= lacunarity;
            }
            value
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
    fractal_noise(
        p,
        seed,
        noise_type,
        FractalType::Standard,
        FractalSettings {
            octaves,
            lacunarity,
            roughness: gain,
        },
        0.0,
        0.0,
    )
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

pub fn simplex_noise(p: Vec3, seed: u32) -> f32 {
    let (x, y, z) = (p.x, p.y, p.z);
    let s = (x + y + z) * (1.0 / 3.0);
    let i = (x + s).floor();
    let j = (y + s).floor();
    let k = (z + s).floor();
    let t = (i + j + k) * (1.0 / 6.0);
    let x0 = x - (i - t);
    let y0 = y - (j - t);
    let z0 = z - (k - t);

    let (i1, j1, k1, i2, j2, k2) = if x0 >= y0 {
        if y0 >= z0 {
            (1.0, 0.0, 0.0, 1.0, 1.0, 0.0)
        } else if x0 >= z0 {
            (1.0, 0.0, 0.0, 1.0, 0.0, 1.0)
        } else {
            (0.0, 0.0, 1.0, 1.0, 0.0, 1.0)
        }
    } else if y0 < z0 {
        (0.0, 0.0, 1.0, 0.0, 1.0, 1.0)
    } else if x0 < z0 {
        (0.0, 1.0, 0.0, 0.0, 1.0, 1.0)
    } else {
        (0.0, 1.0, 0.0, 1.0, 1.0, 0.0)
    };

    let x1 = x0 - i1 + 1.0 / 6.0;
    let y1 = y0 - j1 + 1.0 / 6.0;
    let z1 = z0 - k1 + 1.0 / 6.0;
    let x2 = x0 - i2 + 2.0 / 6.0;
    let y2 = y0 - j2 + 2.0 / 6.0;
    let z2 = z0 - k2 + 2.0 / 6.0;
    let x3 = x0 - 1.0 + 3.0 / 6.0;
    let y3 = y0 - 1.0 + 3.0 / 6.0;
    let z3 = z0 - 1.0 + 3.0 / 6.0;

    let i0 = i as i32;
    let j0 = j as i32;
    let k0 = k as i32;

    let mut n0 = 0.0;
    let mut n1 = 0.0;
    let mut n2 = 0.0;
    let mut n3 = 0.0;

    let t0 = 0.6 - x0 * x0 - y0 * y0 - z0 * z0;
    if t0 > 0.0 {
        let g = gradient(i0, j0, k0, seed);
        let t = t0 * t0;
        n0 = t * t * g.dot(Vec3::new(x0, y0, z0));
    }
    let t1 = 0.6 - x1 * x1 - y1 * y1 - z1 * z1;
    if t1 > 0.0 {
        let g = gradient(i0 + i1 as i32, j0 + j1 as i32, k0 + k1 as i32, seed);
        let t = t1 * t1;
        n1 = t * t * g.dot(Vec3::new(x1, y1, z1));
    }
    let t2 = 0.6 - x2 * x2 - y2 * y2 - z2 * z2;
    if t2 > 0.0 {
        let g = gradient(i0 + i2 as i32, j0 + j2 as i32, k0 + k2 as i32, seed);
        let t = t2 * t2;
        n2 = t * t * g.dot(Vec3::new(x2, y2, z2));
    }
    let t3 = 0.6 - x3 * x3 - y3 * y3 - z3 * z3;
    if t3 > 0.0 {
        let g = gradient(i0 + 1, j0 + 1, k0 + 1, seed);
        let t = t3 * t3;
        n3 = t * t * g.dot(Vec3::new(x3, y3, z3));
    }

    (n0 + n1 + n2 + n3) * 32.0
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

#[derive(Debug, Clone, Copy)]
enum DistanceMetric {
    Euclidean,
    Manhattan,
    Chebyshev,
}

#[derive(Debug, Clone, Copy)]
enum WorleyMode {
    F1,
    F2MinusF1,
}

fn base_noise(
    p: Vec3,
    seed: u32,
    noise_type: NoiseType,
    flow_rotation: f32,
    distortion: f32,
) -> f32 {
    let p = p * noise_type.frequency_scale();
    match noise_type {
        NoiseType::Fast => value_noise(p, seed),
        NoiseType::SparseConvolution => worley_noise(p, seed, DistanceMetric::Euclidean, WorleyMode::F1),
        NoiseType::Alligator => {
            let n = perlin_noise(p, seed);
            let ridge = 1.0 - n.abs();
            ridge * 2.0 - 1.0
        }
        NoiseType::Perlin => perlin_noise(p, seed),
        NoiseType::PerlinFlow => {
            let rotated = rotate_flow(p, flow_rotation);
            perlin_noise(rotated, seed)
        }
        NoiseType::Simplex => simplex_noise(p, seed),
        NoiseType::WorleyF1 => worley_noise(p, seed, DistanceMetric::Euclidean, WorleyMode::F1),
        NoiseType::WorleyF2MinusF1 => {
            worley_noise(p, seed, DistanceMetric::Euclidean, WorleyMode::F2MinusF1)
        }
        NoiseType::ManhattanF1 => worley_noise(p, seed, DistanceMetric::Manhattan, WorleyMode::F1),
        NoiseType::ManhattanF2MinusF1 => {
            worley_noise(p, seed, DistanceMetric::Manhattan, WorleyMode::F2MinusF1)
        }
        NoiseType::ChebyshevF1 => worley_noise(p, seed, DistanceMetric::Chebyshev, WorleyMode::F1),
        NoiseType::ChebyshevF2MinusF1 => {
            worley_noise(p, seed, DistanceMetric::Chebyshev, WorleyMode::F2MinusF1)
        }
        NoiseType::PerlinCloud => cloud_noise(p, seed, CloudBase::Perlin, distortion),
        NoiseType::SimplexCloud => cloud_noise(p, seed, CloudBase::Simplex, distortion),
    }
}

fn rotate_flow(p: Vec3, degrees: f32) -> Vec3 {
    if degrees.abs() <= f32::EPSILON {
        return p;
    }
    let angle = degrees.to_radians();
    Mat3::from_axis_angle(Vec3::Y, angle) * p
}

#[derive(Debug, Clone, Copy)]
enum CloudBase {
    Perlin,
    Simplex,
}

fn cloud_noise(p: Vec3, seed: u32, base: CloudBase, distortion: f32) -> f32 {
    let distortion = distortion.max(0.0);
    let base_noise = |p: Vec3, seed| match base {
        CloudBase::Perlin => perlin_noise(p, seed),
        CloudBase::Simplex => simplex_noise(p, seed),
    };
    if distortion <= 0.0 {
        return base_noise(p, seed);
    }
    let warp_offsets = [
        Vec3::new(12.7, 45.3, 19.1),
        Vec3::new(31.9, 7.2, 58.4),
        Vec3::new(23.1, 91.7, 3.7),
    ];
    let warp = Vec3::new(
        base_noise(p + warp_offsets[0], seed.wrapping_add(17)),
        base_noise(p + warp_offsets[1], seed.wrapping_add(31)),
        base_noise(p + warp_offsets[2], seed.wrapping_add(47)),
    );
    base_noise(p + warp * distortion, seed)
}

fn worley_noise(p: Vec3, seed: u32, metric: DistanceMetric, mode: WorleyMode) -> f32 {
    let (f1, f2) = worley_f1_f2(p, seed, metric);
    let max_dist = match metric {
        DistanceMetric::Euclidean => 3.5,
        DistanceMetric::Manhattan => 6.0,
        DistanceMetric::Chebyshev => 2.0,
    };
    let f1n = (f1 / max_dist).min(1.0);
    match mode {
        WorleyMode::F1 => 1.0 - f1n * 2.0,
        WorleyMode::F2MinusF1 => {
            let diff = (f2 - f1) / max_dist;
            (diff.clamp(0.0, 1.0) * 2.0) - 1.0
        }
    }
}

fn worley_f1_f2(p: Vec3, seed: u32, metric: DistanceMetric) -> (f32, f32) {
    let cell = p.floor();
    let mut f1 = f32::MAX;
    let mut f2 = f32::MAX;
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let cell_pos = cell + Vec3::new(dx as f32, dy as f32, dz as f32);
                let feature = cell_pos
                    + Vec3::new(
                        hash_f32(cell_pos.x as i32, cell_pos.y as i32, cell_pos.z as i32, seed),
                        hash_f32(
                            cell_pos.x as i32,
                            cell_pos.y as i32,
                            cell_pos.z as i32,
                            seed.wrapping_add(1013),
                        ),
                        hash_f32(
                            cell_pos.x as i32,
                            cell_pos.y as i32,
                            cell_pos.z as i32,
                            seed.wrapping_add(2029),
                        ),
                    );
                let d = distance_metric(p, feature, metric);
                if d < f1 {
                    f2 = f1;
                    f1 = d;
                } else if d < f2 {
                    f2 = d;
                }
            }
        }
    }
    (f1, f2)
}

fn distance_metric(a: Vec3, b: Vec3, metric: DistanceMetric) -> f32 {
    let d = (a - b).abs();
    match metric {
        DistanceMetric::Euclidean => d.length(),
        DistanceMetric::Manhattan => d.x + d.y + d.z,
        DistanceMetric::Chebyshev => d.x.max(d.y).max(d.z),
    }
}
