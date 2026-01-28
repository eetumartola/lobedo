use crate::attributes::AttributeDomain;
use crate::graph::{NodeDefinition, NodeParams};
use crate::geometry::{merge_splats, Geometry};
use crate::mesh::Mesh;
use crate::nodes;
use crate::parallel;
use crate::param_spec::ParamSpec;
use crate::splat::SplatGeo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinNodeKind {
    Box,
    Grid,
    Sphere,
    Tube,
    Circle,
    Curve,
    Sweep,
    File,
    ReadSplats,
    WriteSplats,
    GltfOutput,
    BooleanSdf,
    BooleanGeo,
    Delete,
    Prune,
    Regularize,
    SplatLod,
    SplatToMesh,
    SplatDeform,
    SplatDelight,
    SplatIntegrate,
    SplatHeal,
    SplatOutlier,
    SplatCluster,
    SplatMerge,
    VolumeFromGeometry,
    VolumeFromSplats,
    VolumeCombine,
    VolumeBlur,
    VolumeToMesh,
    Group,
    GroupExpand,
    Transform,
    Fuse,
    Ffd,
    CopyTransform,
    Merge,
    CopyToPoints,
    Scatter,
    Normal,
    PolyFrame,
    Color,
    Noise,
    ErosionNoise,
    Smooth,
    Resample,
    UvTexture,
    UvUnwrap,
    UvView,
    Material,
    Ray,
    AttributeNoise,
    AttributePromote,
    AttributeExpand,
    AttributeFromFeature,
    AttributeFromVolume,
    AttributeTransfer,
    AttributeMath,
    Wrangle,
    ObjOutput,
    Output,
}

impl BuiltinNodeKind {
    pub fn id(self) -> &'static str {
        node_spec(self).id
    }
}

pub fn builtin_kind_from_id(id: &str) -> Option<BuiltinNodeKind> {
    node_specs()
        .iter()
        .find_map(|spec| (spec.id == id).then_some(spec.kind))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputPolicy {
    None,
    RequireAll,
    RequireAtLeast(usize),
}

pub struct NodeSpec {
    pub kind: BuiltinNodeKind,
    pub id: &'static str,
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub definition: fn() -> NodeDefinition,
    pub default_params: fn() -> NodeParams,
    pub param_specs: fn() -> Vec<ParamSpec>,
    pub compute_mesh: fn(&NodeParams, &[Mesh]) -> Result<Mesh, String>,
    pub compute_geometry: fn(&NodeParams, &[Geometry]) -> Result<Geometry, String>,
    pub compute_splat: fn(&NodeParams, &[SplatGeo]) -> Result<SplatGeo, String>,
    pub menu_group: Option<&'static str>,
    pub input_policy: InputPolicy,
}

fn mesh_error_read_splats(_params: &NodeParams, _inputs: &[Mesh]) -> Result<Mesh, String> {
    Err("Splat Read outputs splat geometry, not meshes".to_string())
}

fn mesh_error_curve(_params: &NodeParams, _inputs: &[Mesh]) -> Result<Mesh, String> {
    Err("Curve outputs curve geometry, not meshes".to_string())
}

fn mesh_error_volume_blur(_params: &NodeParams, _inputs: &[Mesh]) -> Result<Mesh, String> {
    Err("Volume Blur requires volume input, not meshes".to_string())
}

fn mesh_error_sweep(_params: &NodeParams, _inputs: &[Mesh]) -> Result<Mesh, String> {
    Err("Sweep requires curve/mesh geometry inputs".to_string())
}

fn mesh_error_write_splats(_params: &NodeParams, _inputs: &[Mesh]) -> Result<Mesh, String> {
    Err("Splat Write expects splat geometry, not meshes".to_string())
}

fn mesh_error_splat_to_mesh(_params: &NodeParams, _inputs: &[Mesh]) -> Result<Mesh, String> {
    Err("Splat to Mesh expects splat geometry, not meshes".to_string())
}

fn mesh_error_volume_from_geo(_params: &NodeParams, _inputs: &[Mesh]) -> Result<Mesh, String> {
    Err("Volume from Geometry outputs volume primitives, not meshes".to_string())
}

fn mesh_error_volume_from_splats(_params: &NodeParams, _inputs: &[Mesh]) -> Result<Mesh, String> {
    Err("Volume from Splats outputs volume primitives, not meshes".to_string())
}

fn mesh_error_volume_combine(_params: &NodeParams, _inputs: &[Mesh]) -> Result<Mesh, String> {
    Err("Volume Combine outputs volume primitives, not meshes".to_string())
}

fn mesh_error_volume_to_mesh(_params: &NodeParams, _inputs: &[Mesh]) -> Result<Mesh, String> {
    Err("Volume to Mesh expects volume geometry, not meshes".to_string())
}

fn mesh_error_attribute_from_volume(_params: &NodeParams, _inputs: &[Mesh]) -> Result<Mesh, String> {
    Err("Attribute from Volume requires volume input, not meshes".to_string())
}

static NODE_SPECS: &[NodeSpec] = &[
    NodeSpec {
        kind: BuiltinNodeKind::Box,
        id: "builtin:box",
        name: nodes::box_node::NAME,
        aliases: &[],
        definition: nodes::box_node::definition,
        default_params: nodes::box_node::default_params,
        param_specs: nodes::box_node::param_specs,
        compute_mesh: nodes::box_node::compute,
        compute_geometry: compute_geometry_box,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::None,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Grid,
        id: "builtin:grid",
        name: nodes::grid::NAME,
        aliases: &[],
        definition: nodes::grid::definition,
        default_params: nodes::grid::default_params,
        param_specs: nodes::grid::param_specs,
        compute_mesh: nodes::grid::compute,
        compute_geometry: compute_geometry_grid,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::None,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Sphere,
        id: "builtin:sphere",
        name: nodes::sphere::NAME,
        aliases: &[],
        definition: nodes::sphere::definition,
        default_params: nodes::sphere::default_params,
        param_specs: nodes::sphere::param_specs,
        compute_mesh: nodes::sphere::compute,
        compute_geometry: compute_geometry_sphere,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::None,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Tube,
        id: "builtin:tube",
        name: nodes::tube::NAME,
        aliases: &[],
        definition: nodes::tube::definition,
        default_params: nodes::tube::default_params,
        param_specs: nodes::tube::param_specs,
        compute_mesh: nodes::tube::compute,
        compute_geometry: compute_geometry_tube,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::None,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Circle,
        id: "builtin:circle",
        name: nodes::circle::NAME,
        aliases: &[],
        definition: nodes::circle::definition,
        default_params: nodes::circle::default_params,
        param_specs: nodes::circle::param_specs,
        compute_mesh: nodes::circle::compute,
        compute_geometry: compute_geometry_circle,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::None,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Curve,
        id: "builtin:curve",
        name: nodes::curve::NAME,
        aliases: &[],
        definition: nodes::curve::definition,
        default_params: nodes::curve::default_params,
        param_specs: nodes::curve::param_specs,
        compute_mesh: mesh_error_curve,
        compute_geometry: compute_geometry_curve,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::None,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Sweep,
        id: "builtin:sweep",
        name: nodes::sweep::NAME,
        aliases: &[],
        definition: nodes::sweep::definition,
        default_params: nodes::sweep::default_params,
        param_specs: nodes::sweep::param_specs,
        compute_mesh: mesh_error_sweep,
        compute_geometry: nodes::sweep::apply_to_geometry,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::File,
        id: "builtin:file",
        name: nodes::file::NAME,
        aliases: &[],
        definition: nodes::file::definition,
        default_params: nodes::file::default_params,
        param_specs: nodes::file::param_specs,
        compute_mesh: nodes::file::compute,
        compute_geometry: compute_geometry_file,
        compute_splat: splat_error_not_output,
        menu_group: Some("IO"),
        input_policy: InputPolicy::None,
    },
    NodeSpec {
        kind: BuiltinNodeKind::ReadSplats,
        id: "builtin:read_splats",
        name: nodes::read_splats::NAME,
        aliases: &[nodes::read_splats::LEGACY_NAME],
        definition: nodes::read_splats::definition,
        default_params: nodes::read_splats::default_params,
        param_specs: nodes::read_splats::param_specs,
        compute_mesh: mesh_error_read_splats,
        compute_geometry: compute_geometry_read_splats,
        compute_splat: compute_splat_read_splats,
        menu_group: Some("IO"),
        input_policy: InputPolicy::None,
    },
    NodeSpec {
        kind: BuiltinNodeKind::WriteSplats,
        id: "builtin:write_splats",
        name: nodes::write_splats::NAME,
        aliases: &[nodes::write_splats::LEGACY_NAME],
        definition: nodes::write_splats::definition,
        default_params: nodes::write_splats::default_params,
        param_specs: nodes::write_splats::param_specs,
        compute_mesh: mesh_error_write_splats,
        compute_geometry: apply_write_splats,
        compute_splat: splat_error_not_output,
        menu_group: Some("IO"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::GltfOutput,
        id: "builtin:gltf_output",
        name: nodes::gltf_output::NAME,
        aliases: &[],
        definition: nodes::gltf_output::definition,
        default_params: nodes::gltf_output::default_params,
        param_specs: nodes::gltf_output::param_specs,
        compute_mesh: nodes::gltf_output::compute,
        compute_geometry: apply_obj_output,
        compute_splat: splat_error_not_output,
        menu_group: Some("IO"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::BooleanSdf,
        id: "builtin:boolean_sdf",
        name: nodes::boolean::NAME,
        aliases: &["Boolean"],
        definition: nodes::boolean::definition,
        default_params: nodes::boolean::default_params,
        param_specs: nodes::boolean::param_specs,
        compute_mesh: nodes::boolean::compute,
        compute_geometry: nodes::boolean::apply_to_geometry,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::BooleanGeo,
        id: "builtin:boolean_geo",
        name: nodes::boolean_geo::NAME,
        aliases: &[],
        definition: nodes::boolean_geo::definition,
        default_params: nodes::boolean_geo::default_params,
        param_specs: nodes::boolean_geo::param_specs,
        compute_mesh: nodes::boolean_geo::compute,
        compute_geometry: nodes::boolean_geo::apply_to_geometry,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Delete,
        id: "builtin:delete",
        name: nodes::delete::NAME,
        aliases: &[],
        definition: nodes::delete::definition,
        default_params: nodes::delete::default_params,
        param_specs: nodes::delete::param_specs,
        compute_mesh: nodes::delete::compute,
        compute_geometry: apply_delete,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Prune,
        id: "builtin:prune",
        name: nodes::prune::NAME,
        aliases: &[nodes::prune::LEGACY_NAME],
        definition: nodes::prune::definition,
        default_params: nodes::prune::default_params,
        param_specs: nodes::prune::param_specs,
        compute_mesh: nodes::prune::compute,
        compute_geometry: apply_prune,
        compute_splat: splat_error_not_output,
        menu_group: Some("Splat"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Regularize,
        id: "builtin:regularize",
        name: nodes::regularize::NAME,
        aliases: &[nodes::regularize::LEGACY_NAME],
        definition: nodes::regularize::definition,
        default_params: nodes::regularize::default_params,
        param_specs: nodes::regularize::param_specs,
        compute_mesh: nodes::regularize::compute,
        compute_geometry: apply_regularize,
        compute_splat: splat_error_not_output,
        menu_group: Some("Splat"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::SplatLod,
        id: "builtin:splat_lod",
        name: nodes::splat_lod::NAME,
        aliases: &[],
        definition: nodes::splat_lod::definition,
        default_params: nodes::splat_lod::default_params,
        param_specs: nodes::splat_lod::param_specs,
        compute_mesh: nodes::splat_lod::compute,
        compute_geometry: apply_splat_lod,
        compute_splat: splat_error_not_output,
        menu_group: Some("Splat"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::SplatToMesh,
        id: "builtin:splat_to_mesh",
        name: nodes::splat_to_mesh::NAME,
        aliases: &[],
        definition: nodes::splat_to_mesh::definition,
        default_params: nodes::splat_to_mesh::default_params,
        param_specs: nodes::splat_to_mesh::param_specs,
        compute_mesh: mesh_error_splat_to_mesh,
        compute_geometry: nodes::splat_to_mesh::apply_to_geometry,
        compute_splat: splat_error_not_output,
        menu_group: Some("Splat"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::SplatDeform,
        id: "builtin:splat_deform",
        name: nodes::splat_deform::NAME,
        aliases: &[],
        definition: nodes::splat_deform::definition,
        default_params: nodes::splat_deform::default_params,
        param_specs: nodes::splat_deform::param_specs,
        compute_mesh: nodes::splat_deform::compute,
        compute_geometry: nodes::splat_deform::apply_to_geometry,
        compute_splat: splat_error_not_output,
        menu_group: Some("Splat"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::SplatDelight,
        id: "builtin:splat_delight",
        name: nodes::splat_delight::NAME,
        aliases: &[],
        definition: nodes::splat_delight::definition,
        default_params: nodes::splat_delight::default_params,
        param_specs: nodes::splat_delight::param_specs,
        compute_mesh: nodes::splat_delight::compute,
        compute_geometry: apply_splat_delight,
        compute_splat: splat_error_not_output,
        menu_group: Some("Splat"),
        input_policy: InputPolicy::RequireAtLeast(1),
    },
    NodeSpec {
        kind: BuiltinNodeKind::SplatIntegrate,
        id: "builtin:splat_integrate",
        name: nodes::splat_integrate::NAME,
        aliases: &[],
        definition: nodes::splat_integrate::definition,
        default_params: nodes::splat_integrate::default_params,
        param_specs: nodes::splat_integrate::param_specs,
        compute_mesh: nodes::splat_integrate::compute,
        compute_geometry: nodes::splat_integrate::apply_to_geometry,
        compute_splat: splat_error_not_output,
        menu_group: Some("Splat"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::SplatHeal,
        id: "builtin:splat_heal",
        name: nodes::splat_heal::NAME,
        aliases: &[],
        definition: nodes::splat_heal::definition,
        default_params: nodes::splat_heal::default_params,
        param_specs: nodes::splat_heal::param_specs,
        compute_mesh: nodes::splat_heal::compute,
        compute_geometry: apply_splat_heal,
        compute_splat: splat_error_not_output,
        menu_group: Some("Splat"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::SplatOutlier,
        id: "builtin:splat_outlier",
        name: nodes::splat_outlier::NAME,
        aliases: &[],
        definition: nodes::splat_outlier::definition,
        default_params: nodes::splat_outlier::default_params,
        param_specs: nodes::splat_outlier::param_specs,
        compute_mesh: nodes::splat_outlier::compute,
        compute_geometry: apply_splat_outlier,
        compute_splat: splat_error_not_output,
        menu_group: Some("Splat"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::SplatCluster,
        id: "builtin:splat_cluster",
        name: nodes::splat_cluster::NAME,
        aliases: &[],
        definition: nodes::splat_cluster::definition,
        default_params: nodes::splat_cluster::default_params,
        param_specs: nodes::splat_cluster::param_specs,
        compute_mesh: nodes::splat_cluster::compute,
        compute_geometry: apply_splat_cluster,
        compute_splat: splat_error_not_output,
        menu_group: Some("Splat"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::SplatMerge,
        id: "builtin:splat_merge",
        name: nodes::splat_merge::NAME,
        aliases: &[],
        definition: nodes::splat_merge::definition,
        default_params: nodes::splat_merge::default_params,
        param_specs: nodes::splat_merge::param_specs,
        compute_mesh: nodes::splat_merge::compute,
        compute_geometry: nodes::splat_merge::apply_to_geometry,
        compute_splat: splat_error_not_output,
        menu_group: Some("Splat"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::VolumeFromGeometry,
        id: "builtin:volume_from_geometry",
        name: nodes::volume_from_geo::NAME,
        aliases: &[],
        definition: nodes::volume_from_geo::definition,
        default_params: nodes::volume_from_geo::default_params,
        param_specs: nodes::volume_from_geo::param_specs,
        compute_mesh: mesh_error_volume_from_geo,
        compute_geometry: nodes::volume_from_geo::apply_to_geometry,
        compute_splat: splat_error_not_output,
        menu_group: Some("Volume"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::VolumeFromSplats,
        id: "builtin:volume_from_splats",
        name: nodes::volume_from_splats::NAME,
        aliases: &[],
        definition: nodes::volume_from_splats::definition,
        default_params: nodes::volume_from_splats::default_params,
        param_specs: nodes::volume_from_splats::param_specs,
        compute_mesh: mesh_error_volume_from_splats,
        compute_geometry: nodes::volume_from_splats::apply_to_geometry,
        compute_splat: splat_error_not_output,
        menu_group: Some("Volume"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::VolumeCombine,
        id: "builtin:volume_combine",
        name: nodes::volume_combine::NAME,
        aliases: &[],
        definition: nodes::volume_combine::definition,
        default_params: nodes::volume_combine::default_params,
        param_specs: nodes::volume_combine::param_specs,
        compute_mesh: mesh_error_volume_combine,
        compute_geometry: nodes::volume_combine::apply_to_geometry,
        compute_splat: splat_error_not_output,
        menu_group: Some("Volume"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::VolumeBlur,
        id: "builtin:volume_blur",
        name: nodes::volume_blur::NAME,
        aliases: &[],
        definition: nodes::volume_blur::definition,
        default_params: nodes::volume_blur::default_params,
        param_specs: nodes::volume_blur::param_specs,
        compute_mesh: mesh_error_volume_blur,
        compute_geometry: nodes::volume_blur::apply_to_geometry,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::RequireAtLeast(1),
    },
    NodeSpec {
        kind: BuiltinNodeKind::VolumeToMesh,
        id: "builtin:volume_to_mesh",
        name: nodes::volume_to_mesh::NAME,
        aliases: &[],
        definition: nodes::volume_to_mesh::definition,
        default_params: nodes::volume_to_mesh::default_params,
        param_specs: nodes::volume_to_mesh::param_specs,
        compute_mesh: mesh_error_volume_to_mesh,
        compute_geometry: nodes::volume_to_mesh::apply_to_geometry,
        compute_splat: splat_error_not_output,
        menu_group: Some("Volume"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Group,
        id: "builtin:group",
        name: nodes::group::NAME,
        aliases: &[],
        definition: nodes::group::definition,
        default_params: nodes::group::default_params,
        param_specs: nodes::group::param_specs,
        compute_mesh: nodes::group::compute,
        compute_geometry: apply_group,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::GroupExpand,
        id: "builtin:group_expand",
        name: nodes::group_expand::NAME,
        aliases: &[],
        definition: nodes::group_expand::definition,
        default_params: nodes::group_expand::default_params,
        param_specs: nodes::group_expand::param_specs,
        compute_mesh: nodes::group_expand::compute,
        compute_geometry: apply_group_expand,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Transform,
        id: "builtin:transform",
        name: nodes::transform::NAME,
        aliases: &[],
        definition: nodes::transform::definition,
        default_params: nodes::transform::default_params,
        param_specs: nodes::transform::param_specs,
        compute_mesh: nodes::transform::compute,
        compute_geometry: apply_transform,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Fuse,
        id: "builtin:fuse",
        name: nodes::fuse::NAME,
        aliases: &[],
        definition: nodes::fuse::definition,
        default_params: nodes::fuse::default_params,
        param_specs: nodes::fuse::param_specs,
        compute_mesh: nodes::fuse::compute,
        compute_geometry: nodes::fuse::apply_to_geometry,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Ffd,
        id: "builtin:ffd",
        name: nodes::ffd::NAME,
        aliases: &[],
        definition: nodes::ffd::definition,
        default_params: nodes::ffd::default_params,
        param_specs: nodes::ffd::param_specs,
        compute_mesh: nodes::ffd::compute,
        compute_geometry: nodes::ffd::apply_to_geometry,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::RequireAtLeast(1),
    },
    NodeSpec {
        kind: BuiltinNodeKind::CopyTransform,
        id: "builtin:copy_transform",
        name: nodes::copy_transform::NAME,
        aliases: &[],
        definition: nodes::copy_transform::definition,
        default_params: nodes::copy_transform::default_params,
        param_specs: nodes::copy_transform::param_specs,
        compute_mesh: nodes::copy_transform::compute,
        compute_geometry: apply_copy_transform,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Merge,
        id: "builtin:merge",
        name: nodes::merge::NAME,
        aliases: &[],
        definition: nodes::merge::definition,
        default_params: nodes::merge::default_params,
        param_specs: nodes::merge::param_specs,
        compute_mesh: nodes::merge::compute,
        compute_geometry: compute_geometry_merge,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::RequireAtLeast(1),
    },
    NodeSpec {
        kind: BuiltinNodeKind::CopyToPoints,
        id: "builtin:copy_to_points",
        name: nodes::copy_to_points::NAME,
        aliases: &[],
        definition: nodes::copy_to_points::definition,
        default_params: nodes::copy_to_points::default_params,
        param_specs: nodes::copy_to_points::param_specs,
        compute_mesh: nodes::copy_to_points::compute,
        compute_geometry: apply_copy_to_points,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Scatter,
        id: "builtin:scatter",
        name: nodes::scatter::NAME,
        aliases: &[],
        definition: nodes::scatter::definition,
        default_params: nodes::scatter::default_params,
        param_specs: nodes::scatter::param_specs,
        compute_mesh: nodes::scatter::compute,
        compute_geometry: nodes::scatter::apply_to_geometry,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Normal,
        id: "builtin:normal",
        name: nodes::normal::NAME,
        aliases: &[],
        definition: nodes::normal::definition,
        default_params: nodes::normal::default_params,
        param_specs: nodes::normal::param_specs,
        compute_mesh: nodes::normal::compute,
        compute_geometry: compute_geometry_normal,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::PolyFrame,
        id: "builtin:polyframe",
        name: nodes::polyframe::NAME,
        aliases: &[],
        definition: nodes::polyframe::definition,
        default_params: nodes::polyframe::default_params,
        param_specs: nodes::polyframe::param_specs,
        compute_mesh: nodes::polyframe::compute,
        compute_geometry: nodes::polyframe::apply_to_geometry,
        compute_splat: splat_error_not_output,
        menu_group: Some("Attribute"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Color,
        id: "builtin:color",
        name: nodes::color::NAME,
        aliases: &[],
        definition: nodes::color::definition,
        default_params: nodes::color::default_params,
        param_specs: nodes::color::param_specs,
        compute_mesh: nodes::color::compute,
        compute_geometry: compute_geometry_color,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Noise,
        id: "builtin:noise",
        name: nodes::noise::NAME,
        aliases: &[],
        definition: nodes::noise::definition,
        default_params: nodes::noise::default_params,
        param_specs: nodes::noise::param_specs,
        compute_mesh: nodes::noise::compute,
        compute_geometry: compute_geometry_noise,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::ErosionNoise,
        id: "builtin:erosion_noise",
        name: nodes::erosion_noise::NAME,
        aliases: &[],
        definition: nodes::erosion_noise::definition,
        default_params: nodes::erosion_noise::default_params,
        param_specs: nodes::erosion_noise::param_specs,
        compute_mesh: nodes::erosion_noise::compute,
        compute_geometry: compute_geometry_erosion_noise,
        compute_splat: splat_error_not_output,
        menu_group: Some("Attribute"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Smooth,
        id: "builtin:smooth",
        name: nodes::smooth::NAME,
        aliases: &[],
        definition: nodes::smooth::definition,
        default_params: nodes::smooth::default_params,
        param_specs: nodes::smooth::param_specs,
        compute_mesh: nodes::smooth::compute,
        compute_geometry: compute_geometry_smooth,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Resample,
        id: "builtin:resample",
        name: nodes::resample::NAME,
        aliases: &[],
        definition: nodes::resample::definition,
        default_params: nodes::resample::default_params,
        param_specs: nodes::resample::param_specs,
        compute_mesh: nodes::resample::compute,
        compute_geometry: nodes::resample::apply_to_geometry,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::UvTexture,
        id: "builtin:uv_texture",
        name: nodes::uv_texture::NAME,
        aliases: &[],
        definition: nodes::uv_texture::definition,
        default_params: nodes::uv_texture::default_params,
        param_specs: nodes::uv_texture::param_specs,
        compute_mesh: nodes::uv_texture::compute,
        compute_geometry: compute_geometry_uv_texture,
        compute_splat: splat_error_not_output,
        menu_group: Some("Attribute"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::UvUnwrap,
        id: "builtin:uv_unwrap",
        name: nodes::uv_unwrap::NAME,
        aliases: &[],
        definition: nodes::uv_unwrap::definition,
        default_params: nodes::uv_unwrap::default_params,
        param_specs: nodes::uv_unwrap::param_specs,
        compute_mesh: nodes::uv_unwrap::compute,
        compute_geometry: compute_geometry_uv_unwrap,
        compute_splat: splat_error_not_output,
        menu_group: Some("Attribute"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::UvView,
        id: "builtin:uv_view",
        name: nodes::uv_view::NAME,
        aliases: &[],
        definition: nodes::uv_view::definition,
        default_params: nodes::uv_view::default_params,
        param_specs: nodes::uv_view::param_specs,
        compute_mesh: nodes::uv_view::compute,
        compute_geometry: compute_geometry_uv_view,
        compute_splat: splat_error_not_output,
        menu_group: Some("Attribute"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Material,
        id: "builtin:material",
        name: nodes::material::NAME,
        aliases: &[],
        definition: nodes::material::definition,
        default_params: nodes::material::default_params,
        param_specs: nodes::material::param_specs,
        compute_mesh: nodes::material::compute,
        compute_geometry: nodes::material::apply_to_geometry,
        compute_splat: splat_error_not_output,
        menu_group: Some("Materials"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Ray,
        id: "builtin:ray",
        name: nodes::ray::NAME,
        aliases: &[],
        definition: nodes::ray::definition,
        default_params: nodes::ray::default_params,
        param_specs: nodes::ray::param_specs,
        compute_mesh: nodes::ray::compute,
        compute_geometry: nodes::ray::apply_to_geometry,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::AttributeNoise,
        id: "builtin:attribute_noise",
        name: nodes::attribute_noise::NAME,
        aliases: &[],
        definition: nodes::attribute_noise::definition,
        default_params: nodes::attribute_noise::default_params,
        param_specs: nodes::attribute_noise::param_specs,
        compute_mesh: nodes::attribute_noise::compute,
        compute_geometry: compute_geometry_attribute_noise,
        compute_splat: splat_error_not_output,
        menu_group: Some("Attribute"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::AttributePromote,
        id: "builtin:attribute_promote",
        name: nodes::attribute_promote::NAME,
        aliases: &[],
        definition: nodes::attribute_promote::definition,
        default_params: nodes::attribute_promote::default_params,
        param_specs: nodes::attribute_promote::param_specs,
        compute_mesh: nodes::attribute_promote::compute,
        compute_geometry: compute_geometry_attribute_promote,
        compute_splat: splat_error_not_output,
        menu_group: Some("Attribute"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::AttributeExpand,
        id: "builtin:attribute_expand",
        name: nodes::attribute_expand::NAME,
        aliases: &[],
        definition: nodes::attribute_expand::definition,
        default_params: nodes::attribute_expand::default_params,
        param_specs: nodes::attribute_expand::param_specs,
        compute_mesh: nodes::attribute_expand::compute,
        compute_geometry: compute_geometry_attribute_expand,
        compute_splat: splat_error_not_output,
        menu_group: Some("Attribute"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::AttributeFromFeature,
        id: "builtin:attribute_from_feature",
        name: nodes::attribute_from_feature::NAME,
        aliases: &[],
        definition: nodes::attribute_from_feature::definition,
        default_params: nodes::attribute_from_feature::default_params,
        param_specs: nodes::attribute_from_feature::param_specs,
        compute_mesh: nodes::attribute_from_feature::compute,
        compute_geometry: compute_geometry_attribute_from_feature,
        compute_splat: splat_error_not_output,
        menu_group: Some("Attribute"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::AttributeFromVolume,
        id: "builtin:attribute_from_volume",
        name: nodes::attribute_from_volume::NAME,
        aliases: &[],
        definition: nodes::attribute_from_volume::definition,
        default_params: nodes::attribute_from_volume::default_params,
        param_specs: nodes::attribute_from_volume::param_specs,
        compute_mesh: mesh_error_attribute_from_volume,
        compute_geometry: nodes::attribute_from_volume::apply_to_geometry,
        compute_splat: splat_error_not_output,
        menu_group: Some("Attribute"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::AttributeTransfer,
        id: "builtin:attribute_transfer",
        name: nodes::attribute_transfer::NAME,
        aliases: &[],
        definition: nodes::attribute_transfer::definition,
        default_params: nodes::attribute_transfer::default_params,
        param_specs: nodes::attribute_transfer::param_specs,
        compute_mesh: nodes::attribute_transfer::compute,
        compute_geometry: apply_attribute_transfer,
        compute_splat: splat_error_not_output,
        menu_group: Some("Attribute"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::AttributeMath,
        id: "builtin:attribute_math",
        name: nodes::attribute_math::NAME,
        aliases: &[],
        definition: nodes::attribute_math::definition,
        default_params: nodes::attribute_math::default_params,
        param_specs: nodes::attribute_math::param_specs,
        compute_mesh: nodes::attribute_math::compute,
        compute_geometry: compute_geometry_attribute_math,
        compute_splat: splat_error_not_output,
        menu_group: Some("Attribute"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Wrangle,
        id: "builtin:wrangle",
        name: nodes::wrangle::NAME,
        aliases: &[],
        definition: nodes::wrangle::definition,
        default_params: nodes::wrangle::default_params,
        param_specs: nodes::wrangle::param_specs,
        compute_mesh: nodes::wrangle::compute,
        compute_geometry: nodes::wrangle::apply_to_geometry,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::RequireAtLeast(1),
    },
    NodeSpec {
        kind: BuiltinNodeKind::ObjOutput,
        id: "builtin:obj_output",
        name: nodes::obj_output::NAME,
        aliases: &[],
        definition: nodes::obj_output::definition,
        default_params: nodes::obj_output::default_params,
        param_specs: nodes::obj_output::param_specs,
        compute_mesh: nodes::obj_output::compute,
        compute_geometry: apply_obj_output,
        compute_splat: splat_error_not_output,
        menu_group: Some("IO"),
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Output,
        id: "builtin:output",
        name: nodes::output::NAME,
        aliases: &[],
        definition: nodes::output::definition,
        default_params: nodes::output::default_params,
        param_specs: nodes::output::param_specs,
        compute_mesh: nodes::output::compute,
        compute_geometry: compute_geometry_output,
        compute_splat: splat_error_not_output,
        menu_group: None,
        input_policy: InputPolicy::RequireAll,
    },
];

pub fn node_specs() -> &'static [NodeSpec] {
    NODE_SPECS
}

pub fn menu_group(kind: BuiltinNodeKind) -> Option<&'static str> {
    node_spec(kind).menu_group
}

fn node_spec(kind: BuiltinNodeKind) -> &'static NodeSpec {
    NODE_SPECS
        .iter()
        .find(|spec| spec.kind == kind)
        .unwrap_or_else(|| panic!("missing node spec for {:?}", kind))
}

pub fn input_policy(kind: BuiltinNodeKind) -> InputPolicy {
    node_spec(kind).input_policy
}

impl BuiltinNodeKind {
    pub fn name(self) -> &'static str {
        node_spec(self).name
    }
}

#[allow(clippy::manual_contains)]
pub fn builtin_kind_from_name(name: &str) -> Option<BuiltinNodeKind> {
    node_specs().iter().find_map(|spec| {
        if spec.name == name || spec.aliases.iter().any(|alias| *alias == name) {
            Some(spec.kind)
        } else {
            None
        }
    })
}

pub fn builtin_definitions() -> Vec<NodeDefinition> {
    node_specs()
        .iter()
        .map(|spec| (spec.definition)())
        .collect()
}

pub fn node_definition(kind: BuiltinNodeKind) -> NodeDefinition {
    (node_spec(kind).definition)()
}

pub fn default_params(kind: BuiltinNodeKind) -> NodeParams {
    (node_spec(kind).default_params)()
}

pub fn param_specs(kind: BuiltinNodeKind) -> Vec<ParamSpec> {
    (node_spec(kind).param_specs)()
}

pub fn param_specs_for_name(name: &str) -> Vec<ParamSpec> {
    builtin_kind_from_name(name)
        .map(param_specs)
        .unwrap_or_default()
}

pub fn param_specs_for_kind_id(kind_id: &str) -> Vec<ParamSpec> {
    builtin_kind_from_id(kind_id)
        .map(param_specs)
        .unwrap_or_default()
}

pub fn compute_mesh_node(
    kind: BuiltinNodeKind,
    params: &NodeParams,
    inputs: &[Mesh],
) -> Result<Mesh, String> {
    (node_spec(kind).compute_mesh)(params, inputs)
}

pub fn compute_geometry_node(
    kind: BuiltinNodeKind,
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    (node_spec(kind).compute_geometry)(params, inputs)
}

fn compute_geometry_box(params: &NodeParams, _inputs: &[Geometry]) -> Result<Geometry, String> {
    Ok(Geometry::with_mesh(nodes::box_node::compute(params, &[])?))
}

fn compute_geometry_grid(params: &NodeParams, _inputs: &[Geometry]) -> Result<Geometry, String> {
    Ok(Geometry::with_mesh(nodes::grid::compute(params, &[])?))
}

fn compute_geometry_sphere(params: &NodeParams, _inputs: &[Geometry]) -> Result<Geometry, String> {
    Ok(Geometry::with_mesh(nodes::sphere::compute(params, &[])?))
}

fn compute_geometry_tube(params: &NodeParams, _inputs: &[Geometry]) -> Result<Geometry, String> {
    Ok(Geometry::with_mesh(nodes::tube::compute(params, &[])?))
}

fn compute_geometry_circle(params: &NodeParams, _inputs: &[Geometry]) -> Result<Geometry, String> {
    nodes::circle::apply_to_geometry(params)
}

fn compute_geometry_curve(params: &NodeParams, _inputs: &[Geometry]) -> Result<Geometry, String> {
    let output = nodes::curve::compute(params)?;
    Ok(Geometry::with_curve(output.points, output.closed))
}

fn compute_geometry_file(params: &NodeParams, _inputs: &[Geometry]) -> Result<Geometry, String> {
    Ok(Geometry::with_mesh(nodes::file::compute(params, &[])?))
}

fn compute_geometry_read_splats(
    params: &NodeParams,
    _inputs: &[Geometry],
) -> Result<Geometry, String> {
    Ok(Geometry::with_splats(nodes::read_splats::compute(params)?))
}

fn compute_geometry_merge(_params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    merge_geometry(inputs)
}

fn compute_geometry_output(_params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    Ok(inputs.first().cloned().unwrap_or_default())
}

fn compute_geometry_normal(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    apply_mesh_unary(BuiltinNodeKind::Normal, params, inputs)
}

fn compute_geometry_color(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    apply_mesh_unary(BuiltinNodeKind::Color, params, inputs)
}

fn compute_geometry_noise(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    apply_mesh_unary(BuiltinNodeKind::Noise, params, inputs)
}

fn compute_geometry_erosion_noise(
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    apply_mesh_unary(BuiltinNodeKind::ErosionNoise, params, inputs)
}

fn compute_geometry_smooth(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    apply_mesh_unary(BuiltinNodeKind::Smooth, params, inputs)
}

fn compute_geometry_uv_texture(
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    apply_mesh_unary(BuiltinNodeKind::UvTexture, params, inputs)
}

fn compute_geometry_uv_unwrap(
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    apply_mesh_unary(BuiltinNodeKind::UvUnwrap, params, inputs)
}

fn compute_geometry_uv_view(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    apply_mesh_unary(BuiltinNodeKind::UvView, params, inputs)
}

fn compute_geometry_attribute_noise(
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    apply_mesh_unary(BuiltinNodeKind::AttributeNoise, params, inputs)
}

fn compute_geometry_attribute_promote(
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    apply_mesh_unary(BuiltinNodeKind::AttributePromote, params, inputs)
}

fn compute_geometry_attribute_expand(
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    apply_mesh_unary(BuiltinNodeKind::AttributeExpand, params, inputs)
}

fn compute_geometry_attribute_from_feature(
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    apply_mesh_unary(BuiltinNodeKind::AttributeFromFeature, params, inputs)
}

fn compute_geometry_attribute_math(
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    apply_mesh_unary(BuiltinNodeKind::AttributeMath, params, inputs)
}

pub fn compute_splat_node(
    kind: BuiltinNodeKind,
    params: &NodeParams,
    inputs: &[SplatGeo],
) -> Result<SplatGeo, String> {
    (node_spec(kind).compute_splat)(params, inputs)
}

fn compute_splat_read_splats(
    params: &NodeParams,
    _inputs: &[SplatGeo],
) -> Result<SplatGeo, String> {
    nodes::read_splats::compute(params)
}

fn splat_error_not_output(
    _params: &NodeParams,
    _inputs: &[SplatGeo],
) -> Result<SplatGeo, String> {
    Err("Node does not produce splats".to_string())
}

fn apply_mesh_unary(
    kind: BuiltinNodeKind,
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };
    let mut meshes = Vec::new();
    if let Some(mesh) = input.merged_mesh() {
        meshes.push(compute_mesh_node(kind, params, std::slice::from_ref(&mesh))?);
    }

    let mut splats: Vec<SplatGeo> = (0..input.splats.len())
        .map(|_| SplatGeo::default())
        .collect();
    let input_splats = input.splats.as_slice();
    parallel::try_for_each_indexed_mut(&mut splats, |idx, slot| {
        let mut splat = input_splats[idx].clone();
        match kind {
            BuiltinNodeKind::Color => {
                nodes::color::apply_to_splats(params, &mut splat)?;
            }
            BuiltinNodeKind::Noise => {
                nodes::noise::apply_to_splats(params, &mut splat)?;
            }
            BuiltinNodeKind::ErosionNoise => {
                nodes::erosion_noise::apply_to_splats(params, &mut splat)?;
            }
            BuiltinNodeKind::Smooth => {
                nodes::smooth::apply_to_splats(params, &mut splat)?;
            }
            BuiltinNodeKind::AttributeNoise => {
                nodes::attribute_noise::apply_to_splats(params, &mut splat)?;
            }
            BuiltinNodeKind::AttributePromote => {
                nodes::attribute_promote::apply_to_splats(params, &mut splat)?;
            }
            BuiltinNodeKind::AttributeExpand => {
                nodes::attribute_expand::apply_to_splats(params, &mut splat)?;
            }
            BuiltinNodeKind::AttributeFromFeature => {
                nodes::attribute_from_feature::apply_to_splats(params, &mut splat)?;
            }
            BuiltinNodeKind::AttributeMath => {
                nodes::attribute_math::apply_to_splats(params, &mut splat)?;
            }
            BuiltinNodeKind::Wrangle => {
                nodes::wrangle::apply_to_splats(params, &mut splat, None, None, None)?;
            }
            _ => {}
        }
        *slot = splat;
        Ok::<(), String>(())
    })?;

    let curves = if meshes.is_empty() {
        Vec::new()
    } else {
        input.curves.clone()
    };

    Ok(Geometry {
        meshes,
        splats,
        curves,
        volumes: input.volumes.clone(),
        materials: input.materials.clone(),
    })
}

fn apply_splat_only<F>(
    params: &NodeParams,
    inputs: &[Geometry],
    op: F,
) -> Result<Geometry, String>
where
    F: Fn(&NodeParams, &SplatGeo) -> Result<SplatGeo, String> + Sync + Send,
{
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };

    let mut meshes = Vec::new();
    if let Some(mesh) = input.merged_mesh() {
        meshes.push(mesh);
    }

    let mut splats: Vec<SplatGeo> = (0..input.splats.len())
        .map(|_| SplatGeo::default())
        .collect();
    let input_splats = input.splats.as_slice();
    parallel::try_for_each_indexed_mut(&mut splats, |idx, slot| {
        *slot = op(params, &input_splats[idx])?;
        Ok::<(), String>(())
    })?;

    let curves = if meshes.is_empty() {
        Vec::new()
    } else {
        input.curves.clone()
    };

    Ok(Geometry {
        meshes,
        splats,
        curves,
        volumes: input.volumes.clone(),
        materials: input.materials.clone(),
    })
}

fn apply_attribute_transfer(
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    nodes::attribute_transfer::apply_to_geometry(params, inputs)
}

fn apply_delete(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };

    let mut meshes = Vec::new();
    let mut curves = Vec::new();
    if let Some(mesh) = input.merged_mesh() {
        let result = nodes::delete::compute_with_mapping(params, &[mesh])?;
        meshes.push(result.mesh);
        for curve in &input.curves {
            if let Some(remapped) = curve.remap_indices(&result.point_mapping) {
                curves.push(remapped);
            }
        }
    }

    let mut splats: Vec<SplatGeo> = (0..input.splats.len())
        .map(|_| SplatGeo::default())
        .collect();
    let input_splats = input.splats.as_slice();
    parallel::for_each_indexed_mut(&mut splats, |idx, slot| {
        *slot = filter_splats(params, &input_splats[idx]);
    });

    Ok(Geometry {
        meshes,
        splats,
        curves,
        volumes: input.volumes.clone(),
        materials: input.materials.clone(),
    })
}

fn apply_prune(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    apply_splat_only(params, inputs, |params, splat| {
        Ok(nodes::prune::apply_to_splats(params, splat))
    })
}

fn apply_regularize(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    apply_splat_only(params, inputs, |params, splat| {
        Ok(nodes::regularize::apply_to_splats(params, splat))
    })
}

fn apply_splat_lod(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    apply_splat_only(params, inputs, |params, splat| {
        Ok(nodes::splat_lod::apply_to_splats(params, splat))
    })
}

fn apply_splat_heal(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    nodes::splat_heal::apply_to_geometry(params, inputs)
}

fn apply_splat_outlier(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    apply_splat_only(params, inputs, |params, splat| {
        Ok(nodes::splat_outlier::apply_to_splats(params, splat))
    })
}

fn apply_splat_cluster(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    apply_splat_only(params, inputs, |params, splat| {
        nodes::splat_cluster::apply_to_splats(params, splat)
    })
}

fn apply_splat_delight(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    nodes::splat_delight::apply_to_geometry(params, inputs)
}

fn filter_splats(params: &NodeParams, splats: &SplatGeo) -> SplatGeo {
    let shape = params.get_string("shape", "box");
    let invert = params.get_bool("invert", false);
    let group_mask = nodes::group_utils::splat_group_mask(splats, params, AttributeDomain::Point);

    let mut kept = Vec::new();
    for (idx, position) in splats.positions.iter().enumerate() {
        let inside = crate::nodes::delete::is_inside(params, shape, glam::Vec3::from(*position));
        let mut keep = if invert { inside } else { !inside };
        if let Some(mask) = &group_mask {
            if !mask.get(idx).copied().unwrap_or(false) {
                keep = true;
            }
        }
        if keep {
            kept.push(idx);
        }
    }

    splats.filter_by_indices(&kept)
}

fn apply_group(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };

    let mut meshes = Vec::new();
    if let Some(mesh) = input.merged_mesh() {
        meshes.push(nodes::group::compute(params, std::slice::from_ref(&mesh))?);
    }

    let mut splats: Vec<SplatGeo> = (0..input.splats.len())
        .map(|_| SplatGeo::default())
        .collect();
    let input_splats = input.splats.as_slice();
    parallel::try_for_each_indexed_mut(&mut splats, |idx, slot| {
        let mut splat = input_splats[idx].clone();
        nodes::group::apply_to_splats(params, &mut splat)?;
        *slot = splat;
        Ok::<(), String>(())
    })?;

    let curves = if meshes.is_empty() { Vec::new() } else { input.curves.clone() };
    Ok(Geometry {
        meshes,
        splats,
        curves,
        volumes: input.volumes.clone(),
        materials: input.materials.clone(),
    })
}

fn apply_group_expand(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };

    let mut meshes = Vec::new();
    if let Some(mesh) = input.merged_mesh() {
        meshes.push(nodes::group_expand::compute(params, std::slice::from_ref(&mesh))?);
    }

    let mut splats: Vec<SplatGeo> = (0..input.splats.len())
        .map(|_| SplatGeo::default())
        .collect();
    let input_splats = input.splats.as_slice();
    parallel::try_for_each_indexed_mut(&mut splats, |idx, slot| {
        let mut splat = input_splats[idx].clone();
        nodes::group_expand::apply_to_splats(params, &mut splat)?;
        *slot = splat;
        Ok::<(), String>(())
    })?;

    let curves = if meshes.is_empty() { Vec::new() } else { input.curves.clone() };
    Ok(Geometry {
        meshes,
        splats,
        curves,
        volumes: input.volumes.clone(),
        materials: input.materials.clone(),
    })
}

fn apply_transform(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };

    let matrix = nodes::transform::transform_matrix(params);

    let mut meshes = Vec::new();
    if let Some(mut mesh) = input.merged_mesh() {
        nodes::transform::apply_to_mesh(params, &mut mesh, matrix);
        meshes.push(mesh);
    }

    let mut splats: Vec<SplatGeo> = (0..input.splats.len())
        .map(|_| SplatGeo::default())
        .collect();
    let input_splats = input.splats.as_slice();
    parallel::for_each_indexed_mut(&mut splats, |idx, slot| {
        let mut splat = input_splats[idx].clone();
        if let Some(mask) =
            nodes::group_utils::splat_group_mask(&splat, params, AttributeDomain::Point)
        {
            splat.transform_masked(matrix, &mask);
        } else {
            splat.transform(matrix);
        }
        *slot = splat;
    });

    let mut volumes = input.volumes.clone();
    parallel::for_each_indexed_mut(&mut volumes, |_idx, volume| {
        volume.transform = matrix * volume.transform;
    });

    let curves = if meshes.is_empty() { Vec::new() } else { input.curves.clone() };
    Ok(Geometry {
        meshes,
        splats,
        curves,
        volumes,
        materials: input.materials.clone(),
    })
}

fn apply_copy_transform(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };
    let matrices = nodes::copy_transform::transform_matrices(params);
    if matrices.is_empty() {
        return Ok(Geometry::default());
    }

    let mut meshes = Vec::new();
    let base_mesh = input.merged_mesh();
    let base_point_count = base_mesh.as_ref().map(|mesh| mesh.positions.len() as u32).unwrap_or(0);
    if let Some(mesh) = base_mesh {
        let mut copies: Vec<Mesh> = (0..matrices.len()).map(|_| Mesh::default()).collect();
        parallel::for_each_indexed_mut(&mut copies, |idx, slot| {
            let mut copy = mesh.clone();
            copy.transform(matrices[idx]);
            *slot = copy;
        });
        meshes.push(Mesh::merge(&copies));
    }

    let mut splats: Vec<SplatGeo> = (0..input.splats.len())
        .map(|_| SplatGeo::default())
        .collect();
    let input_splats = input.splats.as_slice();
    parallel::for_each_indexed_mut(&mut splats, |idx, slot| {
        let splat = &input_splats[idx];
        let mut copies = Vec::with_capacity(matrices.len());
        for matrix in &matrices {
            let mut copy = splat.clone();
            copy.transform(*matrix);
            copies.push(copy);
        }
        *slot = merge_splats(&copies);
    });

    let mut volumes = Vec::new();
    for volume in &input.volumes {
        for matrix in &matrices {
            let mut copy = volume.clone();
            copy.transform = *matrix * copy.transform;
            volumes.push(copy);
        }
    }

    let mut curves = Vec::new();
    if base_point_count > 0 {
        for curve in &input.curves {
            for (copy_idx, _) in matrices.iter().enumerate() {
                let mut copy = curve.clone();
                copy.offset_indices(base_point_count * copy_idx as u32);
                curves.push(copy);
            }
        }
    }

    Ok(Geometry {
        meshes,
        splats,
        curves,
        volumes,
        materials: input.materials.clone(),
    })
}

fn apply_copy_to_points(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let mut output = Geometry::default();
    if let Some(input) = inputs.first() {
        output.curves = Vec::new();
        output.volumes = input.volumes.clone();
        output.materials = input.materials.clone();
    }

    let source_mesh = inputs.first().and_then(|geo| geo.merged_mesh());
    let source_splats = inputs.first().and_then(|geo| geo.merged_splats());

    let template_mesh = inputs
        .get(1)
        .and_then(|geo| geo.merged_mesh())
        .filter(|mesh| !mesh.positions.is_empty());
    let template_splats = inputs
        .get(1)
        .and_then(|geo| geo.merged_splats())
        .filter(|splats| !splats.positions.is_empty());

    if let Some(source) = source_mesh.as_ref() {
        if let Some(template) = template_mesh.as_ref() {
            let mesh =
                nodes::copy_to_points::compute(params, &[source.clone(), template.clone()])?;
            output.meshes.push(mesh);
        } else if let Some(template) = template_splats.as_ref() {
            let mesh = nodes::copy_to_points::compute_mesh_from_splats(params, source, template)?;
            output.meshes.push(mesh);
        }
    }

    if let Some(source) = source_splats.as_ref() {
        if let Some(template) = template_mesh.as_ref() {
            output
                .splats
                .push(nodes::copy_to_points::compute_splats_from_mesh(
                    params,
                    source,
                    template,
                )?);
        } else if let Some(template) = template_splats.as_ref() {
            output
                .splats
                .push(nodes::copy_to_points::compute_splats_from_splats(
                    params,
                    source,
                    template,
                )?);
        } else if let Some(input) = inputs.first() {
            output.splats = input.splats.clone();
        }
    }

    Ok(output)
}

fn apply_obj_output(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let mut output = Geometry::default();
    if let Some(input) = inputs.first() {
        output.splats = input.splats.clone();
        output.volumes = input.volumes.clone();
        if let Some(mesh) = input.merged_mesh() {
            let mesh = nodes::obj_output::compute(params, &[mesh])?;
            output.meshes.push(mesh);
        }
        if !output.meshes.is_empty() {
            output.curves = input.curves.clone();
        }
    }
    Ok(output)
}

fn apply_write_splats(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };
    let Some(splats) = input.merged_splats() else {
        return Err("Splat Write requires splat geometry".to_string());
    };
    nodes::write_splats::compute(params, &splats)?;
    Ok(input.clone())
}

fn merge_geometry(inputs: &[Geometry]) -> Result<Geometry, String> {
    if inputs.is_empty() {
        return Ok(Geometry::default());
    }
    let mut output = Geometry::default();
    for input in inputs {
        output.append(input.clone());
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashSet};

    use super::*;
    use crate::mesh::make_box;

    #[test]
    fn transform_applies_scale() {
        let params = NodeParams {
            values: BTreeMap::from([("scale".to_string(), crate::graph::ParamValue::Vec3([2.0, 2.0, 2.0]))]),
        };
        let input = make_box([1.0, 1.0, 1.0]);
        let mesh = compute_mesh_node(BuiltinNodeKind::Transform, &params, &[input]).unwrap();
        let bounds = mesh.bounds().expect("bounds");
        assert!((bounds.max[0] - 1.0).abs() < 0.01);
    }

    #[test]
    fn merge_combines_meshes() {
        let a = make_box([1.0, 1.0, 1.0]);
        let b = make_box([2.0, 2.0, 2.0]);
        let mesh =
            compute_mesh_node(BuiltinNodeKind::Merge, &NodeParams::default(), &[a, b]).unwrap();
        assert!(mesh.positions.len() >= 16);
    }

    #[test]
    fn scatter_produces_points() {
        let params = NodeParams {
            values: BTreeMap::from([
                ("count".to_string(), crate::graph::ParamValue::Int(12)),
                ("seed".to_string(), crate::graph::ParamValue::Int(3)),
            ]),
        };
        let input = make_box([1.0, 1.0, 1.0]);
        let mesh = compute_mesh_node(BuiltinNodeKind::Scatter, &params, &[input]).unwrap();
        assert_eq!(mesh.positions.len(), 12);
        assert!(mesh.indices.is_empty());
        assert_eq!(mesh.normals.as_ref().map(|n| n.len()), Some(12));
    }

    #[test]
    fn normal_recomputes_normals() {
        let mut input = make_box([1.0, 1.0, 1.0]);
        input.normals = None;
        let mesh =
            compute_mesh_node(BuiltinNodeKind::Normal, &NodeParams::default(), &[input]).unwrap();
        assert!(mesh.normals.is_some());
    }

    #[test]
    fn node_specs_cover_definitions() {
        assert_eq!(NODE_SPECS.len(), builtin_definitions().len());
    }

    #[test]
    fn node_spec_ids_are_unique() {
        let mut ids = HashSet::new();
        for spec in node_specs() {
            assert!(ids.insert(spec.id));
        }
    }
}











