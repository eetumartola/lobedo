use eframe::egui::{Pos2, Rect};
use glam::{Mat4, Vec3};


pub(super) fn viewport_view_proj(
    camera: render::CameraState,
    rect: Rect,
    pixels_per_point: f32,
) -> Mat4 {
    let viewport_width = (rect.width() * pixels_per_point).max(1.0);
    let viewport_height = (rect.height() * pixels_per_point).max(1.0);
    let aspect = viewport_width / viewport_height;

    let target = Vec3::from(camera.target);
    let position = camera_position(camera);

    let view = Mat4::look_at_rh(position, target, Vec3::Y);
    let projection = Mat4::perspective_rh(45_f32.to_radians(), aspect, 0.01, 10000.0);
    projection * view
}

pub(super) fn camera_position(camera: render::CameraState) -> Vec3 {
    let pitch = camera.pitch.clamp(-1.54, 1.54);
    let yaw = camera.yaw;
    let cos_pitch = pitch.cos();
    let sin_pitch = pitch.sin();
    let cos_yaw = yaw.cos();
    let sin_yaw = yaw.sin();
    let direction = Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw);
    let target = Vec3::from(camera.target);
    target + direction * camera.distance.max(0.1)
}

pub(super) fn camera_forward(camera: render::CameraState) -> Vec3 {
    let position = camera_position(camera);
    let target = Vec3::from(camera.target);
    (target - position).normalize_or_zero()
}

pub(super) fn project_world_to_screen(view_proj: Mat4, rect: Rect, world: Vec3) -> Option<Pos2> {
    let clip = view_proj * world.extend(1.0);
    if clip.w.abs() <= 1.0e-6 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    if !ndc.x.is_finite() || !ndc.y.is_finite() || !ndc.z.is_finite() {
        return None;
    }
    let x = rect.min.x + (ndc.x * 0.5 + 0.5) * rect.width();
    let y = rect.min.y + (0.5 - ndc.y * 0.5) * rect.height();
    Some(Pos2::new(x, y))
}

pub(super) fn project_world_to_screen_with_depth(
    view_proj: Mat4,
    rect: Rect,
    world: Vec3,
) -> Option<(Pos2, f32)> {
    let clip = view_proj * world.extend(1.0);
    if clip.w.abs() <= 1.0e-6 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    if !ndc.x.is_finite() || !ndc.y.is_finite() || !ndc.z.is_finite() {
        return None;
    }
    let x = rect.min.x + (ndc.x * 0.5 + 0.5) * rect.width();
    let y = rect.min.y + (0.5 - ndc.y * 0.5) * rect.height();
    Some((Pos2::new(x, y), ndc.z))
}

pub(super) fn screen_ray(
    camera: render::CameraState,
    rect: Rect,
    pixels_per_point: f32,
    pos: Pos2,
) -> Option<(Vec3, Vec3)> {
    let view_proj = viewport_view_proj(camera, rect, pixels_per_point);
    let inv = view_proj.inverse();
    let ndc_x = ((pos.x - rect.min.x) / rect.width()) * 2.0 - 1.0;
    let ndc_y = 1.0 - ((pos.y - rect.min.y) / rect.height()) * 2.0;
    let near = inv.project_point3(Vec3::new(ndc_x, ndc_y, 0.0));
    let far = inv.project_point3(Vec3::new(ndc_x, ndc_y, 1.0));
    let dir = (far - near).normalize_or_zero();
    Some((near, dir))
}

pub(super) fn raycast_plane_y(
    camera: render::CameraState,
    rect: Rect,
    pixels_per_point: f32,
    pos: Pos2,
    y: f32,
) -> Option<Vec3> {
    let (origin, dir) = screen_ray(camera, rect, pixels_per_point, pos)?;
    if dir.y.abs() <= 1.0e-6 {
        return None;
    }
    let t = (y - origin.y) / dir.y;
    if t < 0.0 {
        return None;
    }
    Some(origin + dir * t)
}

pub(super) fn raycast_plane(
    camera: render::CameraState,
    rect: Rect,
    pixels_per_point: f32,
    pos: Pos2,
    plane_origin: Vec3,
    plane_normal: Vec3,
) -> Option<Vec3> {
    let (origin, dir) = screen_ray(camera, rect, pixels_per_point, pos)?;
    let denom = plane_normal.dot(dir);
    if denom.abs() <= 1.0e-6 {
        return None;
    }
    let t = (plane_origin - origin).dot(plane_normal) / denom;
    if t < 0.0 {
        return None;
    }
    Some(origin + dir * t)
}

pub(super) fn distance_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ap = p - a;
    let ab = b - a;
    let t = (ap.dot(ab) / ab.dot(ab)).clamp(0.0, 1.0);
    let closest = a + ab * t;
    (p - closest).length()
}

pub(super) fn distance_to_polyline(p: Pos2, points: &[Pos2]) -> f32 {
    if points.len() < 2 {
        return f32::INFINITY;
    }
    let mut best = f32::INFINITY;
    for segment in points.windows(2) {
        let dist = distance_to_segment(p, segment[0], segment[1]);
        best = best.min(dist);
    }
    best
}

pub(super) fn point_in_triangle(p: Pos2, a: Pos2, b: Pos2, c: Pos2, tol: f32) -> bool {
    let ab = b - a;
    let bc = c - b;
    let ca = a - c;
    let ap = p - a;
    let bp = p - b;
    let cp = p - c;
    let cross1 = ab.x * ap.y - ab.y * ap.x;
    let cross2 = bc.x * bp.y - bc.y * bp.x;
    let cross3 = ca.x * cp.y - ca.y * cp.x;
    let has_neg = cross1 < -tol || cross2 < -tol || cross3 < -tol;
    let has_pos = cross1 > tol || cross2 > tol || cross3 > tol;
    !(has_neg && has_pos)
}

pub(super) fn distance_to_triangle_edges(p: Pos2, a: Pos2, b: Pos2, c: Pos2) -> f32 {
    let d0 = distance_to_segment(p, a, b);
    let d1 = distance_to_segment(p, b, c);
    let d2 = distance_to_segment(p, c, a);
    d0.min(d1).min(d2)
}

pub(super) fn rect_corners_in_triangle(rect: Rect, a: Pos2, b: Pos2, c: Pos2) -> bool {
    let corners = [
        rect.min,
        Pos2::new(rect.max.x, rect.min.y),
        rect.max,
        Pos2::new(rect.min.x, rect.max.y),
    ];
    corners
        .into_iter()
        .any(|corner| point_in_triangle(corner, a, b, c, 0.0))
}
