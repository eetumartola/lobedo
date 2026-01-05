#[cfg(not(target_arch = "wasm32"))]
use std::process;

#[cfg(not(target_arch = "wasm32"))]
use eframe::egui;

#[cfg(not(target_arch = "wasm32"))]
mod app;
#[cfg(not(target_arch = "wasm32"))]
mod headless;
#[cfg(not(target_arch = "wasm32"))]
mod node_graph;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let (console, log_level_state) = app::setup_tracing();

    tracing::info!("Lobedo starting");

    let args: Vec<String> = std::env::args().collect();
    match headless::maybe_run_headless(&args) {
        Ok(true) => return Ok(()),
        Ok(false) => {}
        Err(err) => {
            eprintln!("headless error: {err}");
            process::exit(1);
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(true)
            .with_maximized(true),
        renderer: eframe::Renderer::Wgpu,
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        "Lobedo",
        native_options,
        Box::new(|cc| {
            let mut style = (*cc.egui_ctx.style()).clone();
            let stroke_width = 0.25;
            style.visuals.window_stroke.width = stroke_width;
            style.visuals.widgets.inactive.bg_stroke.width = stroke_width;
            style.visuals.widgets.hovered.bg_stroke.width = stroke_width;
            style.visuals.widgets.active.bg_stroke.width = stroke_width;
            cc.egui_ctx.set_style(style);

            let mut app = app::LobedoApp::new(console, log_level_state);
            app.try_load_default_graph();
            Ok(Box::new(app))
        }),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {}
