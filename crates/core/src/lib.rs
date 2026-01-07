mod attributes;
mod eval;
mod gradient;
mod curve;
mod geometry;
mod geometry_eval;
mod groups;
mod graph;
mod mesh;
mod mesh_primitives;
mod mesh_eval;
mod material;
mod noise;
mod nodes;
mod nodes_builtin;
mod project;
mod scene;
mod splat;
mod splat_ply;
mod splat_eval;
mod wrangle;

pub use attributes::{
    AttributeDomain, AttributeError, AttributeInfo, AttributeRef, AttributeStorage, AttributeType,
    MeshAttributes,
};
pub use assets::{load_bytes, store_bytes};
pub use eval::{
    evaluate_from, evaluate_from_with, DirtyNodeReport, DirtyReason, EvalCacheStats, EvalError,
    EvalNodeReport, EvalReport, EvalState,
};
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
pub use geometry_eval::{evaluate_geometry_graph, GeometryEvalResult, GeometryEvalState};
pub use nodes_builtin::{
    builtin_definitions, builtin_kind_from_name, compute_geometry_node, compute_mesh_node,
    compute_splat_node, default_params, node_definition, node_specs, BuiltinNodeKind, NodeSpec,
};
pub use nodes::obj_output::write_obj;
pub use project::{
    CameraSettings, PanelSettings, Project, ProjectSettings, RenderDebugSettings, ShadingMode,
    SplatShadingMode, PROJECT_VERSION,
};
pub use scene::{SceneCurve, SceneDrawable, SceneMesh, SceneSnapshot, SceneSplats};
pub use splat::{save_splat_ply_with_format, SplatGeo, SplatSaveFormat};
pub use splat_eval::{evaluate_splat_graph, SplatEvalResult, SplatEvalState};
mod assets;
