use eframe::egui;
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

    pub(super) fn handle_viewport_input(&mut self, response: &egui::Response) {
        if !response.hovered() {
            return;
        }

        let camera = &mut self.project.settings.camera;
        let orbit_speed = 0.01;
        let pan_speed = 0.0025 * camera.distance.max(0.1);
        let zoom_speed = 0.1;

        if response.dragged_by(egui::PointerButton::Primary) {
            let delta = response.drag_motion();
            camera.yaw += delta.x * orbit_speed;
            camera.pitch = (camera.pitch + delta.y * orbit_speed).clamp(-1.54, 1.54);
        }

        if response.dragged_by(egui::PointerButton::Middle) {
            let delta = response.drag_motion();
            camera.target[0] -= delta.x * pan_speed;
            camera.target[1] += delta.y * pan_speed;
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
