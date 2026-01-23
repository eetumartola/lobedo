mod attributes;
mod eval;
mod color;
mod gradient;
mod curve;
mod geometry;
mod geometry_eval;
mod gltf_io;
mod groups;
mod graph;
mod mesh;
mod mesh_primitives;
mod mesh_eval;
mod material;
mod noise;
mod nodes;
mod nodes_builtin;
mod node_help;
mod node_help_io;
mod node_help_splats;
mod node_help_volumes;
mod parallel;
mod param_spec;
mod param_templates;
mod progress;
mod project;
mod scene;
mod splat;
mod splat_ply;
mod splat_eval;
mod volume;
mod volume_sampling;
mod wrangle;

pub use attributes::{
    AttributeDomain, AttributeError, AttributeInfo, AttributeRef, AttributeStorage, AttributeType,
    MeshAttributes,
};
pub use assets::{is_url, load_bytes, store_bytes, url_revision};
pub use eval::{
    collect_dirty_nodes, collect_dirty_nodes_full, evaluate_from, evaluate_from_with,
    node_dirty,
    DirtyNodeReport, DirtyReason, EvalCacheStats, EvalError, EvalNodeReport, EvalReport,
    EvalState,
};
pub use progress::{report_progress, ProgressEvent, ProgressSink};
pub use color::{lerp_oklab, linear_srgb_to_oklab, oklab_to_linear_srgb};
pub use gradient::{parse_color_gradient, ColorGradient, ColorStop};
pub use curve::{encode_curve_points, parse_curve_points, sample_catmull_rom, Curve};
pub use geometry::{merge_splats, Geometry};
pub use graph::{
    Graph, GraphError, Link, LinkId, Node, NodeDefinition, NodeId, NodeParams, ParamValue, Pin,
    PinDefinition, PinId, PinKind, PinType,
};
pub use mesh::{make_box, make_grid, make_tube, Aabb, Mesh};
pub use material::{Material, MaterialLibrary};
pub use mesh_eval::{evaluate_mesh_graph, MeshEvalResult, MeshEvalState};
pub use param_spec::{ParamKind, ParamOption, ParamPathKind, ParamRange, ParamSpec, ParamWidget};
pub use geometry_eval::{
    evaluate_geometry_graph, evaluate_geometry_graph_with_progress, GeometryEvalResult,
    GeometryEvalState,
};
pub use nodes_builtin::{
    builtin_definitions, builtin_kind_from_id, builtin_kind_from_name, compute_geometry_node,
    compute_mesh_node, compute_splat_node, default_params, node_definition, node_specs,
    menu_group, param_specs, param_specs_for_kind_id, param_specs_for_name, BuiltinNodeKind,
    NodeSpec,
};
pub use node_help::{help_summary, node_help_page, node_help_page_for_kind, NodeHelpPage};
pub use nodes::obj_output::write_obj;
pub use nodes::splat_merge::build_skirt_preview_mesh;
pub use gltf_io::write_gltf;
pub use project::{
    CameraSettings, GraphNote, PanelSettings, Project, ProjectSettings, RenderDebugSettings,
    ShadingMode, SplatShadingMode, PROJECT_VERSION,
};
pub use scene::{
    scene_mesh_from_mesh, scene_snapshot_from_geometry, scene_snapshot_from_mesh,
    scene_snapshot_from_splats, SceneCurve, SceneDrawable, SceneMesh, SceneSnapshot, SceneSplats,
    SceneVolume, SceneVolumeKind, SceneMaterial,
};
pub use splat::{save_splat_ply_with_format, SplatGeo, SplatSaveFormat};
pub use splat_eval::{evaluate_splat_graph, SplatEvalResult, SplatEvalState};
pub use volume::{Volume, VolumeKind};
mod assets;
