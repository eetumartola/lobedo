use serde::{Deserialize, Serialize};

use crate::graph::Graph;
use crate::nodes;

pub const PROJECT_VERSION: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub version: u32,
    pub settings: ProjectSettings,
    pub graph: Graph,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            version: PROJECT_VERSION,
            settings: ProjectSettings::default(),
            graph: Graph::default(),
        }
    }
}

impl Project {
    pub fn migrate_to_latest(&mut self) {
        if self.version < 2 {
            self.graph.migrate_geometry_pins();
            self.version = 2;
        }
        if self.version < 3 {
            self.graph.ensure_node_kind_ids();
            self.version = 3;
        }
        self.graph.rebuild_link_index();
        self.graph
            .rename_nodes(nodes::read_splats::LEGACY_NAME, nodes::read_splats::NAME);
        self.graph
            .rename_nodes(nodes::write_splats::LEGACY_NAME, nodes::write_splats::NAME);
        self.graph
            .rename_nodes(nodes::prune::LEGACY_NAME, nodes::prune::NAME);
        self.graph
            .rename_nodes(nodes::regularize::LEGACY_NAME, nodes::regularize::NAME);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectSettings {
    pub viewport_split: f32,
    pub viewport_sheet_split: f32,
    pub node_params_split: f32,
    pub panels: PanelSettings,
    pub camera: CameraSettings,
    pub render_debug: RenderDebugSettings,
    pub graph_notes: Vec<GraphNote>,
    pub next_note_id: u64,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            viewport_split: 0.5,
            viewport_sheet_split: 0.75,
            node_params_split: 0.25,
            panels: PanelSettings::default(),
            camera: CameraSettings::default(),
            render_debug: RenderDebugSettings::default(),
            graph_notes: Vec::new(),
            next_note_id: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNote {
    pub id: u64,
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelSettings {
    pub show_inspector: bool,
    pub show_spreadsheet: bool,
    pub show_debug: bool,
    pub show_console: bool,
}

impl Default for PanelSettings {
    fn default() -> Self {
        Self {
            show_inspector: true,
            show_spreadsheet: false,
            show_debug: false,
            show_console: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraSettings {
    pub target: [f32; 3],
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            target: [0.0, 0.0, 0.0],
            distance: 5.0,
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShadingMode {
    Lit,
    Normals,
    Depth,
    SplatOpacity,
    SplatScale,
    SplatOverdraw,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SplatShadingMode {
    ColorOnly,
    FullSh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RenderDebugSettings {
    pub show_grid: bool,
    pub show_axes: bool,
    pub show_normals: bool,
    pub show_bounds: bool,
    pub normal_length: f32,
    pub show_stats: bool,
    pub show_points: bool,
    pub show_splats: bool,
    pub point_size: f32,
    pub key_shadows: bool,
    pub shading_mode: ShadingMode,
    pub depth_near: f32,
    pub depth_far: f32,
    pub splat_debug_min: f32,
    pub splat_debug_max: f32,
    pub splat_shading_mode: SplatShadingMode,
    pub splat_depth_prepass: bool,
    pub splat_tile_binning: bool,
    pub splat_tile_size: u32,
    pub splat_tile_threshold: u32,
    pub splat_rebuild_fps_enabled: bool,
    pub splat_rebuild_fps: f32,
    pub splat_frustum_cull: bool,
}

impl Default for RenderDebugSettings {
    fn default() -> Self {
        Self {
            show_grid: true,
            show_axes: true,
            show_normals: false,
            show_bounds: false,
            normal_length: 0.3,
            show_stats: true,
            show_points: false,
            show_splats: true,
            point_size: 4.0,
            key_shadows: false,
            shading_mode: ShadingMode::Lit,
            depth_near: 0.5,
            depth_far: 20.0,
            splat_debug_min: 0.0,
            splat_debug_max: 1.0,
            splat_shading_mode: SplatShadingMode::FullSh,
            splat_depth_prepass: false,
            splat_tile_binning: false,
            splat_tile_size: 160,
            splat_tile_threshold: 50_000,
            splat_rebuild_fps_enabled: false,
            splat_rebuild_fps: 15.0,
            splat_frustum_cull: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes_builtin::{node_definition, BuiltinNodeKind};

    #[test]
    fn migrate_rebuilds_link_index_and_kind_ids() {
        let mut project = Project::default();
        project.version = 1;

        let a = project
            .graph
            .add_node(node_definition(BuiltinNodeKind::Box));
        let b = project
            .graph
            .add_node(node_definition(BuiltinNodeKind::Merge));
        let from = project.graph.node(a).unwrap().outputs[0];
        let to = project.graph.node(b).unwrap().inputs[0];
        project.graph.add_link(from, to).unwrap();

        let data = serde_json::to_vec(&project).expect("serialize project");
        let mut loaded: Project =
            serde_json::from_slice(&data).expect("deserialize project");
        assert!(loaded.graph.input_node(b, 0).is_none());

        loaded.migrate_to_latest();

        assert_eq!(loaded.version, PROJECT_VERSION);
        assert_eq!(loaded.graph.input_node(b, 0), Some(a));
        let node = loaded.graph.node(a).expect("node");
        assert!(!node.kind_id.is_empty());
    }
}
