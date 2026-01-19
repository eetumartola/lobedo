use std::path::PathBuf;
use std::sync::atomic::AtomicU8;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use eframe::egui;
use lobedo_core::{GeometryEvalState, Project};
use render::{RenderScene, ViewportRenderer};
use tracing_subscriber::filter::LevelFilter;

use crate::node_graph;

mod eval;
mod io;
mod logging;
mod node_info;
mod spreadsheet;
mod ui_central;
mod ui_info_panels;
mod ui_inputs;
mod ui_side_panels;
mod ui_top_bar;
mod ui_preferences;
mod ui;
mod undo;
mod viewport;
mod viewport_tools;
mod wrangle_help;

pub(crate) use logging::ConsoleBuffer;

use logging::level_filter_to_u8;
use node_info::NodeInfoPanel;
use spreadsheet::SpreadsheetMode;
use undo::{UndoSnapshot, UndoStack};
use viewport_tools::ViewportToolState;
use wrangle_help::WrangleHelpPanel;

pub(crate) struct LobedoApp {
    project: Project,
    project_path: Option<PathBuf>,
    console: ConsoleBuffer,
    log_level: LevelFilter,
    log_level_state: Arc<AtomicU8>,
    viewport_renderer: Option<ViewportRenderer>,
    pending_scene: Option<RenderScene>,
    last_scene: Option<RenderScene>,
    eval_state: GeometryEvalState,
    last_eval_report: Option<lobedo_core::EvalReport>,
    last_eval_ms: Option<f32>,
    eval_dirty: bool,
    last_param_change: Option<Instant>,
    eval_job: Option<eval::EvalJob>,
    node_graph: node_graph::NodeGraphState,
    last_display_state: DisplayState,
    last_node_graph_rect: Option<egui::Rect>,
    last_viewport_rect: Option<egui::Rect>,
    pause_viewport: bool,
    viewport_tools: ViewportToolState,
    last_selected_node: Option<lobedo_core::NodeId>,
    last_selection_key: Option<(lobedo_core::NodeId, u64)>,
    last_preview_key: Option<(lobedo_core::NodeId, u64)>,
    last_template_mesh: Option<lobedo_core::Mesh>,
    show_preferences: bool,
    info_panel: Option<NodeInfoPanel>,
    held_info_panel: Option<NodeInfoPanel>,
    wrangle_help_panel: Option<WrangleHelpPanel>,
    undo_stack: UndoStack,
    pending_undo: Option<UndoSnapshot>,
    spreadsheet_mode: SpreadsheetMode,
    spreadsheet_domain: lobedo_core::AttributeDomain,
    fit_nodes_on_load: bool,
    last_window_title: String,
    last_url_revision: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DisplayState {
    Ok,
    Missing,
}

pub(crate) fn setup_tracing() -> (ConsoleBuffer, Arc<AtomicU8>) {
    logging::setup_tracing()
}

impl LobedoApp {
    pub(crate) fn new(console: ConsoleBuffer, log_level_state: Arc<AtomicU8>) -> Self {
        Self {
            project: Project::default(),
            project_path: None,
            console,
            log_level: LevelFilter::INFO,
            log_level_state,
            viewport_renderer: None,
            pending_scene: None,
            last_scene: None,
            eval_state: GeometryEvalState::new(),
            last_eval_report: None,
            last_eval_ms: None,
            eval_dirty: false,
            last_param_change: None,
            eval_job: None,
            node_graph: node_graph::NodeGraphState::default(),
            last_display_state: DisplayState::Ok,
            last_node_graph_rect: None,
            last_viewport_rect: None,
            pause_viewport: false,
            viewport_tools: ViewportToolState::default(),
            last_selected_node: None,
            last_selection_key: None,
            last_preview_key: None,
            last_template_mesh: None,
            show_preferences: false,
            info_panel: None,
            held_info_panel: None,
            wrangle_help_panel: None,
            undo_stack: UndoStack::new(),
            pending_undo: None,
            spreadsheet_mode: SpreadsheetMode::Mesh,
            spreadsheet_domain: lobedo_core::AttributeDomain::Point,
            fit_nodes_on_load: false,
            last_window_title: String::new(),
            last_url_revision: lobedo_core::url_revision(),
        }
    }

    fn set_log_level(&mut self, new_level: LevelFilter) {
        if new_level == self.log_level {
            return;
        }

        self.log_level_state.store(
            level_filter_to_u8(new_level),
            std::sync::atomic::Ordering::Relaxed,
        );
        self.log_level = new_level;
    }

    fn snapshot_undo(&self) -> UndoSnapshot {
        self.undo_stack.snapshot(
            &self.project.graph,
            &self.node_graph,
            &self.project.settings.graph_notes,
            self.project.settings.next_note_id,
        )
    }

    fn queue_undo_snapshot(&mut self, snapshot: UndoSnapshot, pointer_down: bool) {
        if pointer_down {
            if self.pending_undo.is_none() {
                self.pending_undo = Some(snapshot);
            }
        } else {
            self.undo_stack.push(snapshot);
        }
    }

    fn flush_pending_undo(&mut self) {
        if let Some(snapshot) = self.pending_undo.take() {
            self.undo_stack.push(snapshot);
        }
    }

    fn restore_snapshot(&mut self, snapshot: UndoSnapshot) {
        self.project.graph = snapshot.graph;
        self.project.settings.graph_notes = snapshot.graph_notes;
        self.project.settings.next_note_id = snapshot.next_note_id;
        self.node_graph
            .restore_layout(&self.project.graph, &snapshot.layout);
        self.last_selected_node = snapshot.layout.selected;
        self.last_selection_key = None;
        self.last_preview_key = None;
        self.last_template_mesh = None;
        self.pending_scene = None;
        self.last_scene = None;
        self.last_eval_report = None;
        self.eval_dirty = true;
        self.last_param_change = None;
        self.info_panel = None;
    }

    fn update_window_title(&mut self, ctx: &egui::Context) {
        let title = match self.project_path.as_ref() {
            Some(path) => {
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "project.json".to_string());
                format!("Lobedo - {}", name)
            }
            None => "Lobedo - Unsaved".to_string(),
        };
        if title != self.last_window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_window_title = title;
        }
    }

    fn try_undo(&mut self) {
        self.pending_undo = None;
        let current = self.snapshot_undo();
        if let Some(snapshot) = self.undo_stack.undo(current) {
            self.restore_snapshot(snapshot);
        }
    }

    fn try_redo(&mut self) {
        self.pending_undo = None;
        let current = self.snapshot_undo();
        if let Some(snapshot) = self.undo_stack.redo(current) {
            self.restore_snapshot(snapshot);
        }
    }
}
