use eframe::egui;
use glam::Vec3;
use render::{CameraState, ViewportRenderer};

use super::LobedoApp;

impl LobedoApp {
    pub(super) fn sync_wgpu_renderer(&mut self, frame: &eframe::Frame) {
        let Some(render_state) = frame.wgpu_render_state() else {
            return;
        };

        if self.viewport_renderer.is_none() {
            self.viewport_renderer = Some(ViewportRenderer::new(render_state.target_format));
        }

        if let (Some(renderer), Some(scene)) = (&self.viewport_renderer, self.pending_scene.take())
        {
            renderer.set_scene(scene);
        }
    }

    pub(super) fn handle_viewport_input(&mut self, response: &egui::Response, rect: egui::Rect) {
        if self.handle_viewport_tools_input(response, rect) {
            return;
        }
        if !response.hovered() {
            return;
        }

        let camera = &mut self.project.settings.camera;
        let orbit_speed = 0.01;
        let pan_speed = 0.0025 * camera.distance.max(0.1);
        let zoom_speed = 0.1;

        let (alt_down, primary_down, keys_down, dt) = response.ctx.input(|i| {
            (
                i.modifiers.alt,
                i.pointer.primary_down(),
                i.keys_down.clone(),
                i.stable_dt.max(0.0),
            )
        });
        if primary_down && !alt_down && !response.ctx.wants_keyboard_input() {
            let w = keys_down.contains(&egui::Key::W);
            let a = keys_down.contains(&egui::Key::A);
            let s = keys_down.contains(&egui::Key::S);
            let d = keys_down.contains(&egui::Key::D);
            let q = keys_down.contains(&egui::Key::Q);
            let e = keys_down.contains(&egui::Key::E);
            if w || a || s || d || q || e {
                let pitch = camera.pitch.clamp(-1.54, 1.54);
                let yaw = camera.yaw;
                let cos_pitch = pitch.cos();
                let sin_pitch = pitch.sin();
                let cos_yaw = yaw.cos();
                let sin_yaw = yaw.sin();
                let dir = [
                    cos_pitch * cos_yaw,
                    sin_pitch,
                    cos_pitch * sin_yaw,
                ];
                let forward = normalize([-dir[0], -dir[1], -dir[2]]);
                let world_up = [0.0f32, 1.0f32, 0.0f32];
                let right = normalize(cross(forward, world_up));
                let step = camera.distance.max(0.1) * 0.6 * dt;
                let mut delta = [0.0f32, 0.0f32, 0.0f32];
                if w {
                    delta[0] += forward[0] * step;
                    delta[1] += forward[1] * step;
                    delta[2] += forward[2] * step;
                }
                if s {
                    delta[0] -= forward[0] * step;
                    delta[1] -= forward[1] * step;
                    delta[2] -= forward[2] * step;
                }
                if d {
                    delta[0] += right[0] * step;
                    delta[1] += right[1] * step;
                    delta[2] += right[2] * step;
                }
                if a {
                    delta[0] -= right[0] * step;
                    delta[1] -= right[1] * step;
                    delta[2] -= right[2] * step;
                }
                if e {
                    delta[0] += world_up[0] * step;
                    delta[1] += world_up[1] * step;
                    delta[2] += world_up[2] * step;
                }
                if q {
                    delta[0] -= world_up[0] * step;
                    delta[1] -= world_up[1] * step;
                    delta[2] -= world_up[2] * step;
                }
                if delta[0] != 0.0 || delta[1] != 0.0 || delta[2] != 0.0 {
                    camera.target[0] += delta[0];
                    camera.target[1] += delta[1];
                    camera.target[2] += delta[2];
                }
            }
        }

        if response.dragged_by(egui::PointerButton::Primary) {
            let delta = response.drag_motion();
            if alt_down {
                camera.yaw += delta.x * orbit_speed;
                camera.pitch = (camera.pitch + delta.y * orbit_speed).clamp(-1.54, 1.54);
            } else {
                let pitch = camera.pitch.clamp(-1.54, 1.54);
                let yaw = camera.yaw;
                let cos_pitch = pitch.cos();
                let sin_pitch = pitch.sin();
                let cos_yaw = yaw.cos();
                let sin_yaw = yaw.sin();
                let dir_old = [
                    cos_pitch * cos_yaw,
                    sin_pitch,
                    cos_pitch * sin_yaw,
                ];
                let camera_pos = [
                    camera.target[0] + dir_old[0] * camera.distance,
                    camera.target[1] + dir_old[1] * camera.distance,
                    camera.target[2] + dir_old[2] * camera.distance,
                ];

                let new_yaw = yaw + delta.x * orbit_speed;
                let new_pitch = (pitch + delta.y * orbit_speed).clamp(-1.54, 1.54);
                let cos_pitch = new_pitch.cos();
                let sin_pitch = new_pitch.sin();
                let cos_yaw = new_yaw.cos();
                let sin_yaw = new_yaw.sin();
                let dir_new = [
                    cos_pitch * cos_yaw,
                    sin_pitch,
                    cos_pitch * sin_yaw,
                ];
                camera.yaw = new_yaw;
                camera.pitch = new_pitch;
                camera.target[0] = camera_pos[0] - dir_new[0] * camera.distance;
                camera.target[1] = camera_pos[1] - dir_new[1] * camera.distance;
                camera.target[2] = camera_pos[2] - dir_new[2] * camera.distance;
            }
        }

        if response.dragged_by(egui::PointerButton::Middle) {
            let delta = response.drag_motion();
            let pitch = camera.pitch.clamp(-1.54, 1.54);
            let yaw = camera.yaw;
            let cos_pitch = pitch.cos();
            let sin_pitch = pitch.sin();
            let cos_yaw = yaw.cos();
            let sin_yaw = yaw.sin();

            let dir = [
                cos_pitch * cos_yaw,
                sin_pitch,
                cos_pitch * sin_yaw,
            ];
            let forward = [-dir[0], -dir[1], -dir[2]];
            let world_up = [0.0f32, 1.0f32, 0.0f32];
            let right = normalize(cross(forward, world_up));
            let up = normalize(cross(right, forward));
            let pan_x = -delta.x * pan_speed;
            let pan_y = delta.y * pan_speed;
            camera.target[0] += right[0] * pan_x + up[0] * pan_y;
            camera.target[1] += right[1] * pan_x + up[1] * pan_y;
            camera.target[2] += right[2] * pan_x + up[2] * pan_y;
        }

        if response.dragged_by(egui::PointerButton::Secondary) {
            let delta = response.drag_motion();
            if delta.y.abs() > 0.0 {
                let zoom_delta = -delta.y * 3.0;
                let zoom = 1.0 - (zoom_delta * zoom_speed / 100.0);
                camera.distance = (camera.distance * zoom).clamp(0.1, 1000.0);
            }
        }

        let scroll_delta = response.ctx.input(|i| i.raw_scroll_delta.y);
        if scroll_delta.abs() > 0.0 {
            let zoom = 1.0 - (scroll_delta * zoom_speed / 100.0);
            camera.distance = (camera.distance * zoom).clamp(0.1, 1000.0);
        }
    }

    pub(super) fn camera_state(&self) -> CameraState {
        CameraState {
            target: self.project.settings.camera.target,
            distance: self.project.settings.camera.distance,
            yaw: self.project.settings.camera.yaw,
            pitch: self.project.settings.camera.pitch,
        }
    }

    pub(super) fn fit_viewport_to_scene(&mut self) {
        let Some(scene) = &self.last_scene else {
            return;
        };
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        let mut found = false;

        for drawable in &scene.drawables {
            match drawable {
                render::RenderDrawable::Mesh(mesh) => {
                    for pos in &mesh.positions {
                        for i in 0..3 {
                            min[i] = min[i].min(pos[i]);
                            max[i] = max[i].max(pos[i]);
                        }
                        found = true;
                    }
                }
                render::RenderDrawable::Splats(splats) => {
                    for pos in &splats.positions {
                        for i in 0..3 {
                            min[i] = min[i].min(pos[i]);
                            max[i] = max[i].max(pos[i]);
                        }
                        found = true;
                    }
                }
                render::RenderDrawable::Curve(curve) => {
                    for pos in &curve.points {
                        for i in 0..3 {
                            min[i] = min[i].min(pos[i]);
                            max[i] = max[i].max(pos[i]);
                        }
                        found = true;
                    }
                }
                render::RenderDrawable::Volume(volume) => {
                    let origin = Vec3::from(volume.origin);
                    let size = Vec3::new(
                        volume.dims[0] as f32,
                        volume.dims[1] as f32,
                        volume.dims[2] as f32,
                    ) * volume.voxel_size;
                    let max_corner = origin + size;
                    let corners = [
                        Vec3::new(origin.x, origin.y, origin.z),
                        Vec3::new(max_corner.x, origin.y, origin.z),
                        Vec3::new(origin.x, max_corner.y, origin.z),
                        Vec3::new(origin.x, origin.y, max_corner.z),
                        Vec3::new(max_corner.x, max_corner.y, origin.z),
                        Vec3::new(origin.x, max_corner.y, max_corner.z),
                        Vec3::new(max_corner.x, origin.y, max_corner.z),
                        Vec3::new(max_corner.x, max_corner.y, max_corner.z),
                    ];
                    let transform = volume.transform;
                    for corner in corners {
                        let pos = transform.transform_point3(corner);
                        min[0] = min[0].min(pos.x);
                        min[1] = min[1].min(pos.y);
                        min[2] = min[2].min(pos.z);
                        max[0] = max[0].max(pos.x);
                        max[1] = max[1].max(pos.y);
                        max[2] = max[2].max(pos.z);
                    }
                    found = true;
                }
            }
        }

        if !found {
            return;
        }

        let center = [
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ];
        let extent = [
            (max[0] - min[0]) * 0.5,
            (max[1] - min[1]) * 0.5,
            (max[2] - min[2]) * 0.5,
        ];
        let radius = (extent[0] * extent[0] + extent[1] * extent[1] + extent[2] * extent[2])
            .sqrt()
            .max(0.001);
        let fov_y = 45_f32.to_radians();
        let distance = (radius / (fov_y * 0.5).tan()).max(0.1) * 1.2;

        let camera = &mut self.project.settings.camera;
        camera.target = center;
        camera.distance = distance;
    }
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 1.0e-6 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        [0.0, 0.0, 0.0]
    }
}
