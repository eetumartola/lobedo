use glam::{Mat4, Vec3};

#[derive(Clone, Copy)]
pub(super) struct SortedSplat {
    pub(super) depth: f32,
    pub(super) position: [f32; 3],
    pub(super) color: [f32; 3],
    pub(super) opacity: f32,
    pub(super) scale: [f32; 3],
    pub(super) rotation: [f32; 4],
}

pub(super) fn sort_splats_by_depth(
    positions: &[[f32; 3]],
    colors: &[[f32; 3]],
    opacity: &[f32],
    scales: &[[f32; 3]],
    rotations: &[[f32; 4]],
    camera_pos: Vec3,
    forward: Vec3,
) -> Vec<SortedSplat> {
    let mut entries = Vec::with_capacity(positions.len());
    for (idx, position) in positions.iter().enumerate() {
        let pos = Vec3::from(*position);
        let depth = (pos - camera_pos).dot(forward);
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

    entries.sort_by(|a, b| b.depth.partial_cmp(&a.depth).unwrap_or(std::cmp::Ordering::Equal));
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
