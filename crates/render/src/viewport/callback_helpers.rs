use glam::{Mat3, Mat4, Vec3};

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
#[derive(Clone, Copy)]
pub(super) struct SortedSplat {
    pub(super) depth: f32,
    pub(super) position: [f32; 3],
    pub(super) color: [f32; 3],
    pub(super) opacity: f32,
    pub(super) scale: [f32; 3],
    pub(super) rotation: [f32; 4],
}

#[allow(clippy::too_many_arguments)]
pub(super) fn sort_splats_by_depth(
    positions: &[[f32; 3]],
    colors: &[[f32; 3]],
    opacity: &[f32],
    scales: &[[f32; 3]],
    rotations: &[[f32; 4]],
    camera_pos: Vec3,
    forward: Vec3,
    depth_transform: Mat3,
) -> Vec<SortedSplat> {
    let mut entries = Vec::with_capacity(positions.len());
    for (idx, position) in positions.iter().enumerate() {
        let pos = Vec3::from(*position);
        let pos_depth = depth_transform * pos;
        let depth = (pos_depth - camera_pos).dot(forward);
        entries.push(SortedSplat {
            depth,
            position: *position,
            color: colors.get(idx).copied().unwrap_or([1.0, 1.0, 1.0]),
            opacity: opacity.get(idx).copied().unwrap_or(1.0),
            scale: scales.get(idx).copied().unwrap_or([1.0, 1.0, 1.0]),
            rotation: rotations
                .get(idx)
                .copied()
                .unwrap_or([0.0, 0.0, 0.0, 1.0]),
        });
    }

    #[cfg(target_arch = "wasm32")]
    {
        // Web fallback: keep input order to avoid heavy CPU sorting.
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        entries.sort_by(|a, b| {
            b.depth
                .partial_cmp(&a.depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    entries
}

pub(super) fn light_view_projection(bounds: ([f32; 3], [f32; 3]), key_dir: Vec3) -> Mat4 {
    let min = Vec3::from(bounds.0);
    let max = Vec3::from(bounds.1);
    let center = (min + max) * 0.5;
    let extent = (max - min) * 0.5;
    let radius = extent.length().max(0.5);

    let dir = if key_dir.length_squared() > 0.0001 {
        key_dir.normalize()
    } else {
        Vec3::new(0.6, 1.0, 0.2).normalize()
    };
    let light_pos = center + dir * radius * 4.0;
    let mut up = Vec3::Y;
    if dir.abs().dot(up) > 0.95 {
        up = Vec3::Z;
    }
    let mut right = dir.cross(up);
    if right.length_squared() < 0.0001 {
        right = dir.cross(Vec3::X);
    }
    right = right.normalize_or_zero();
    up = right.cross(dir).normalize_or_zero();
    let view = Mat4::look_at_rh(light_pos, center, up);
    let corners = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(max.x, max.y, max.z),
    ];
    let mut min_ls = Vec3::splat(f32::INFINITY);
    let mut max_ls = Vec3::splat(f32::NEG_INFINITY);
    for corner in corners {
        let ls = (view * corner.extend(1.0)).truncate();
        min_ls = min_ls.min(ls);
        max_ls = max_ls.max(ls);
    }
    let xy_pad = radius * 0.05;
    let z_pad = radius * 0.1;
    min_ls.x -= xy_pad;
    min_ls.y -= xy_pad;
    max_ls.x += xy_pad;
    max_ls.y += xy_pad;
    let near = (-max_ls.z - z_pad).max(0.01);
    let far = (-min_ls.z + z_pad).max(near + 0.01);
    let ortho = Mat4::orthographic_rh(min_ls.x, max_ls.x, min_ls.y, max_ls.y, near, far);
    ortho * view
}

#[allow(clippy::excessive_precision)]
const SH_C0: f32 = 0.2820948;
#[allow(clippy::excessive_precision)]
const SH_C1: f32 = 0.4886025119029199;
#[allow(clippy::excessive_precision)]
const SH_C2: [f32; 5] = [
    1.0925484305920792,
    1.0925484305920792,
    0.31539156525252005,
    1.0925484305920792,
    0.5462742152960396,
];
#[allow(clippy::excessive_precision)]
const SH_C3: [f32; 7] = [
    0.5900435899266435,
    2.890611442640554,
    0.4570457994644658,
    0.3731763325901154,
    0.4570457994644658,
    1.445305721320277,
    0.5900435899266435,
];

fn sh_basis_l1(dir: Vec3) -> [f32; 3] {
    let x = dir.x;
    let y = dir.y;
    let z = dir.z;
    [-SH_C1 * y, SH_C1 * z, -SH_C1 * x]
}

fn sh_basis_l2(dir: Vec3) -> [f32; 5] {
    let x = dir.x;
    let y = dir.y;
    let z = dir.z;
    [
        SH_C2[0] * x * y,
        SH_C2[1] * y * z,
        SH_C2[2] * (3.0 * z * z - 1.0),
        SH_C2[3] * x * z,
        SH_C2[4] * (x * x - y * y),
    ]
}

fn sh_basis_l3(dir: Vec3) -> [f32; 7] {
    let x = dir.x;
    let y = dir.y;
    let z = dir.z;
    [
        SH_C3[0] * y * (3.0 * x * x - y * y),
        SH_C3[1] * x * y * z,
        SH_C3[2] * y * (5.0 * z * z - 1.0),
        SH_C3[3] * z * (5.0 * z * z - 3.0),
        SH_C3[4] * x * (5.0 * z * z - 1.0),
        SH_C3[5] * z * (x * x - y * y),
        SH_C3[6] * x * (x * x - 3.0 * y * y),
    ]
}

pub(super) fn splat_color_from_sh(
    sh0: [f32; 3],
    sh_rest: &[[f32; 3]],
    sh_coeffs: usize,
    sh0_is_coeff: bool,
    full_sh: bool,
    view_dir: Vec3,
) -> [f32; 3] {
    let mut color = if sh0_is_coeff {
        Vec3::from(sh0) * SH_C0
    } else {
        Vec3::from(sh0)
    };

    let mut dir = view_dir;
    if dir.length_squared() < 1.0e-6 {
        dir = Vec3::Z;
    } else {
        dir = dir.normalize();
    }

    let coeff_count = sh_coeffs.min(sh_rest.len());
    if full_sh && sh0_is_coeff {
        let mut index = 0usize;
        if coeff_count >= 3 {
            let basis = sh_basis_l1(dir);
            for i in 0..3 {
                color += Vec3::from(sh_rest[index + i]) * basis[i];
            }
            index += 3;
        }
        if coeff_count >= 8 {
            let basis = sh_basis_l2(dir);
            for i in 0..5 {
                color += Vec3::from(sh_rest[index + i]) * basis[i];
            }
            index += 5;
        }
        if coeff_count >= 15 {
            let basis = sh_basis_l3(dir);
            for i in 0..7 {
                color += Vec3::from(sh_rest[index + i]) * basis[i];
            }
        }
    }

    if sh0_is_coeff {
        color += Vec3::splat(0.5);
    }
    color = color.clamp(Vec3::ZERO, Vec3::ONE);
    color.to_array()
}
