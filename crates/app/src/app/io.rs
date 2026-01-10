use std::io;
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
use rfd::FileDialog;

use lobedo_core::Project;
#[cfg(not(target_arch = "wasm32"))]
use lobedo_core::{
    evaluate_geometry_graph, save_splat_ply_with_format, write_gltf, write_obj, SplatSaveFormat,
};

use super::LobedoApp;
use crate::node_graph::WriteRequest;
#[cfg(not(target_arch = "wasm32"))]
use crate::node_graph::WriteRequestKind;

const DEFAULT_GRAPH_PATH: &str = "graphs/default.json";
const DEFAULT_GRAPH_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../graphs/default.json"));

impl LobedoApp {
    pub(super) fn new_project(&mut self) {
        self.project = Project::default();
        self.project_path = None;
        self.node_graph.reset();
        self.fit_nodes_on_load = false;
        self.undo_stack.clear();
        self.pending_undo = None;
        self.eval_dirty = true;
        self.pending_scene = None;
        self.last_scene = None;
        self.last_selection_key = None;
        tracing::info!("new project created");
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn save_project_to(&self, path: &Path) -> io::Result<()> {
        let data = serde_json::to_vec_pretty(&self.project).map_err(io::Error::other)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    #[allow(dead_code)]
    pub(super) fn save_project_to(&self, _path: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "File save is not available in web builds",
        ))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn load_project_from(&mut self, path: &Path) -> io::Result<()> {
        let data = std::fs::read(path)?;
        self.load_project_from_bytes(&data, Some(path.to_path_buf()))
    }

    #[cfg(target_arch = "wasm32")]
    #[allow(dead_code)]
    pub(super) fn load_project_from(&mut self, _path: &Path) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "File load is not available in web builds",
        ))
    }

    pub(crate) fn try_load_default_graph(&mut self) {
        if cfg!(target_arch = "wasm32") {
            if let Err(err) = self.load_project_from_bytes(DEFAULT_GRAPH_JSON.as_bytes(), None) {
                tracing::error!("failed to load compiled default graph: {}", err);
            } else {
                tracing::info!("compiled default graph loaded");
            }
            return;
        }

        let path = Path::new(DEFAULT_GRAPH_PATH);
        if path.exists() {
            match self.load_project_from(path) {
                Ok(()) => {
                    tracing::info!("default graph loaded");
                }
                Err(err) => {
                    tracing::error!("failed to load default graph: {}", err);
                }
            }
            return;
        }

        match self.load_project_from_bytes(DEFAULT_GRAPH_JSON.as_bytes(), None) {
            Ok(()) => tracing::info!("compiled default graph loaded"),
            Err(err) => tracing::error!("failed to load compiled default graph: {}", err),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn handle_write_request(&mut self, _request: WriteRequest) {
        tracing::warn!("Writing is not available in web builds.");
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn handle_write_request(&mut self, request: WriteRequest) {
        let Some(node) = self.project.graph.node(request.node_id) else {
            tracing::warn!("Write failed: missing node");
            return;
        };
        let path = node.params.get_string("path", "");
        if path.trim().is_empty() {
            tracing::warn!("Write failed: output path is empty");
            return;
        }
        let result =
            evaluate_geometry_graph(&self.project.graph, request.node_id, &mut self.eval_state);
        let geometry = match result {
            Ok(result) => result.output,
            Err(err) => {
                tracing::warn!("Write failed: eval error {:?}", err);
                return;
            }
        };
        let Some(geometry) = geometry else {
            tracing::warn!("Write failed: no output geometry");
            return;
        };
        match request.kind {
            WriteRequestKind::Obj => {
                let Some(mesh) = geometry.merged_mesh() else {
                    tracing::warn!("Write failed: no mesh output");
                    return;
                };
                if let Err(err) = write_obj(path, &mesh) {
                    tracing::warn!("OBJ write failed: {}", err);
                } else {
                    tracing::info!("OBJ written to {}", path);
                }
            }
            WriteRequestKind::Gltf => {
                let Some(mesh) = geometry.merged_mesh() else {
                    tracing::warn!("Write failed: no mesh output");
                    return;
                };
                if let Err(err) = write_gltf(path, &mesh) {
                    tracing::warn!("GLTF write failed: {}", err);
                } else {
                    tracing::info!("GLTF written to {}", path);
                }
            }
            WriteRequestKind::Splat => {
                let Some(splats) = geometry.merged_splats() else {
                    tracing::warn!("Write failed: no splat output");
                    return;
                };
                let format = match node.params.get_int("format", 0) {
                    1 => SplatSaveFormat::Ascii,
                    _ => SplatSaveFormat::BinaryLittle,
                };
                if let Err(err) = save_splat_ply_with_format(path, &splats, format) {
                    tracing::warn!("PLY write failed: {}", err);
                } else {
                    tracing::info!("PLY written to {}", path);
                }
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn open_project_dialog(&mut self) {
        if let Some(path) = FileDialog::new()
            .add_filter("Lobedo Project", &["json"])
            .pick_file()
        {
            match self.load_project_from(&path) {
                Ok(()) => {
                    self.project_path = Some(path);
                    tracing::info!("project loaded");
                }
                Err(err) => {
                    tracing::error!("failed to load project: {}", err);
                }
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn save_project_dialog(&mut self) {
        if let Some(path) = FileDialog::new()
            .add_filter("Lobedo Project", &["json"])
            .set_file_name("project.json")
            .save_file()
        {
            match self.save_project_to(&path) {
                Ok(()) => {
                    self.project_path = Some(path);
                    tracing::info!("project saved");
                }
                Err(err) => {
                    tracing::error!("failed to save project: {}", err);
                }
            }
        }
    }

    fn load_project_from_bytes(
        &mut self,
        data: &[u8],
        path: Option<std::path::PathBuf>,
    ) -> io::Result<()> {
        let mut project: Project = serde_json::from_slice(data)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        project.migrate_to_latest();
        self.project = project;
        self.project_path = path;
        self.node_graph
            .restore_layout_from_graph(&self.project.graph);
        self.fit_nodes_on_load = true;
        self.undo_stack.clear();
        self.pending_undo = None;
        self.eval_dirty = true;
        self.pending_scene = None;
        self.last_scene = None;
        self.last_selection_key = None;
        Ok(())
    }
}
