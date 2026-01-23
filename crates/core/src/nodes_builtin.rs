use crate::attributes::AttributeDomain;
use crate::graph::{NodeDefinition, NodeParams};
use crate::geometry::{merge_splats, Geometry};
use crate::mesh::Mesh;
use crate::nodes;
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
        match self {
            BuiltinNodeKind::Box => "builtin:box",
            BuiltinNodeKind::Grid => "builtin:grid",
            BuiltinNodeKind::Sphere => "builtin:sphere",
            BuiltinNodeKind::Tube => "builtin:tube",
            BuiltinNodeKind::Circle => "builtin:circle",
            BuiltinNodeKind::Curve => "builtin:curve",
            BuiltinNodeKind::Sweep => "builtin:sweep",
            BuiltinNodeKind::File => "builtin:file",
            BuiltinNodeKind::ReadSplats => "builtin:read_splats",
            BuiltinNodeKind::WriteSplats => "builtin:write_splats",
            BuiltinNodeKind::GltfOutput => "builtin:gltf_output",
            BuiltinNodeKind::BooleanSdf => "builtin:boolean_sdf",
            BuiltinNodeKind::BooleanGeo => "builtin:boolean_geo",
            BuiltinNodeKind::Delete => "builtin:delete",
            BuiltinNodeKind::Prune => "builtin:prune",
            BuiltinNodeKind::Regularize => "builtin:regularize",
            BuiltinNodeKind::SplatLod => "builtin:splat_lod",
            BuiltinNodeKind::SplatToMesh => "builtin:splat_to_mesh",
            BuiltinNodeKind::SplatDeform => "builtin:splat_deform",
            BuiltinNodeKind::SplatDelight => "builtin:splat_delight",
            BuiltinNodeKind::SplatIntegrate => "builtin:splat_integrate",
            BuiltinNodeKind::SplatHeal => "builtin:splat_heal",
            BuiltinNodeKind::SplatOutlier => "builtin:splat_outlier",
            BuiltinNodeKind::SplatCluster => "builtin:splat_cluster",
            BuiltinNodeKind::SplatMerge => "builtin:splat_merge",
            BuiltinNodeKind::VolumeFromGeometry => "builtin:volume_from_geometry",
            BuiltinNodeKind::VolumeCombine => "builtin:volume_combine",
            BuiltinNodeKind::VolumeBlur => "builtin:volume_blur",
            BuiltinNodeKind::VolumeToMesh => "builtin:volume_to_mesh",
            BuiltinNodeKind::Group => "builtin:group",
            BuiltinNodeKind::GroupExpand => "builtin:group_expand",
            BuiltinNodeKind::Transform => "builtin:transform",
            BuiltinNodeKind::Fuse => "builtin:fuse",
            BuiltinNodeKind::Ffd => "builtin:ffd",
            BuiltinNodeKind::CopyTransform => "builtin:copy_transform",
            BuiltinNodeKind::Merge => "builtin:merge",
            BuiltinNodeKind::CopyToPoints => "builtin:copy_to_points",
            BuiltinNodeKind::Scatter => "builtin:scatter",
            BuiltinNodeKind::Normal => "builtin:normal",
            BuiltinNodeKind::PolyFrame => "builtin:polyframe",
            BuiltinNodeKind::Color => "builtin:color",
            BuiltinNodeKind::Noise => "builtin:noise",
            BuiltinNodeKind::ErosionNoise => "builtin:erosion_noise",
            BuiltinNodeKind::Smooth => "builtin:smooth",
            BuiltinNodeKind::Resample => "builtin:resample",
            BuiltinNodeKind::UvTexture => "builtin:uv_texture",
            BuiltinNodeKind::UvUnwrap => "builtin:uv_unwrap",
            BuiltinNodeKind::UvView => "builtin:uv_view",
            BuiltinNodeKind::Material => "builtin:material",
            BuiltinNodeKind::Ray => "builtin:ray",
            BuiltinNodeKind::AttributeNoise => "builtin:attribute_noise",
            BuiltinNodeKind::AttributePromote => "builtin:attribute_promote",
            BuiltinNodeKind::AttributeExpand => "builtin:attribute_expand",
            BuiltinNodeKind::AttributeFromFeature => "builtin:attribute_from_feature",
            BuiltinNodeKind::AttributeFromVolume => "builtin:attribute_from_volume",
            BuiltinNodeKind::AttributeTransfer => "builtin:attribute_transfer",
            BuiltinNodeKind::AttributeMath => "builtin:attribute_math",
            BuiltinNodeKind::Wrangle => "builtin:wrangle",
            BuiltinNodeKind::ObjOutput => "builtin:obj_output",
            BuiltinNodeKind::Output => "builtin:output",
        }
    }
}

pub fn builtin_kind_from_id(id: &str) -> Option<BuiltinNodeKind> {
    node_specs()
        .iter()
        .find_map(|spec| (spec.kind.id() == id).then_some(spec.kind))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputPolicy {
    None,
    RequireAll,
    RequireAtLeast(usize),
}

pub struct NodeSpec {
    pub kind: BuiltinNodeKind,
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub definition: fn() -> NodeDefinition,
    pub default_params: fn() -> NodeParams,
    pub compute_mesh: fn(&NodeParams, &[Mesh]) -> Result<Mesh, String>,
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
        name: nodes::box_node::NAME,
        aliases: &[],
        definition: nodes::box_node::definition,
        default_params: nodes::box_node::default_params,
        compute_mesh: nodes::box_node::compute,
        input_policy: InputPolicy::None,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Grid,
        name: nodes::grid::NAME,
        aliases: &[],
        definition: nodes::grid::definition,
        default_params: nodes::grid::default_params,
        compute_mesh: nodes::grid::compute,
        input_policy: InputPolicy::None,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Sphere,
        name: nodes::sphere::NAME,
        aliases: &[],
        definition: nodes::sphere::definition,
        default_params: nodes::sphere::default_params,
        compute_mesh: nodes::sphere::compute,
        input_policy: InputPolicy::None,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Tube,
        name: nodes::tube::NAME,
        aliases: &[],
        definition: nodes::tube::definition,
        default_params: nodes::tube::default_params,
        compute_mesh: nodes::tube::compute,
        input_policy: InputPolicy::None,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Circle,
        name: nodes::circle::NAME,
        aliases: &[],
        definition: nodes::circle::definition,
        default_params: nodes::circle::default_params,
        compute_mesh: nodes::circle::compute,
        input_policy: InputPolicy::None,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Curve,
        name: nodes::curve::NAME,
        aliases: &[],
        definition: nodes::curve::definition,
        default_params: nodes::curve::default_params,
        compute_mesh: mesh_error_curve,
        input_policy: InputPolicy::None,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Sweep,
        name: nodes::sweep::NAME,
        aliases: &[],
        definition: nodes::sweep::definition,
        default_params: nodes::sweep::default_params,
        compute_mesh: mesh_error_sweep,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::File,
        name: nodes::file::NAME,
        aliases: &[],
        definition: nodes::file::definition,
        default_params: nodes::file::default_params,
        compute_mesh: nodes::file::compute,
        input_policy: InputPolicy::None,
    },
    NodeSpec {
        kind: BuiltinNodeKind::ReadSplats,
        name: nodes::read_splats::NAME,
        aliases: &[nodes::read_splats::LEGACY_NAME],
        definition: nodes::read_splats::definition,
        default_params: nodes::read_splats::default_params,
        compute_mesh: mesh_error_read_splats,
        input_policy: InputPolicy::None,
    },
    NodeSpec {
        kind: BuiltinNodeKind::WriteSplats,
        name: nodes::write_splats::NAME,
        aliases: &[nodes::write_splats::LEGACY_NAME],
        definition: nodes::write_splats::definition,
        default_params: nodes::write_splats::default_params,
        compute_mesh: mesh_error_write_splats,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::GltfOutput,
        name: nodes::gltf_output::NAME,
        aliases: &[],
        definition: nodes::gltf_output::definition,
        default_params: nodes::gltf_output::default_params,
        compute_mesh: nodes::gltf_output::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::BooleanSdf,
        name: nodes::boolean::NAME,
        aliases: &["Boolean"],
        definition: nodes::boolean::definition,
        default_params: nodes::boolean::default_params,
        compute_mesh: nodes::boolean::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::BooleanGeo,
        name: nodes::boolean_geo::NAME,
        aliases: &[],
        definition: nodes::boolean_geo::definition,
        default_params: nodes::boolean_geo::default_params,
        compute_mesh: nodes::boolean_geo::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Delete,
        name: nodes::delete::NAME,
        aliases: &[],
        definition: nodes::delete::definition,
        default_params: nodes::delete::default_params,
        compute_mesh: nodes::delete::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Prune,
        name: nodes::prune::NAME,
        aliases: &[nodes::prune::LEGACY_NAME],
        definition: nodes::prune::definition,
        default_params: nodes::prune::default_params,
        compute_mesh: nodes::prune::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Regularize,
        name: nodes::regularize::NAME,
        aliases: &[nodes::regularize::LEGACY_NAME],
        definition: nodes::regularize::definition,
        default_params: nodes::regularize::default_params,
        compute_mesh: nodes::regularize::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::SplatLod,
        name: nodes::splat_lod::NAME,
        aliases: &[],
        definition: nodes::splat_lod::definition,
        default_params: nodes::splat_lod::default_params,
        compute_mesh: nodes::splat_lod::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::SplatToMesh,
        name: nodes::splat_to_mesh::NAME,
        aliases: &[],
        definition: nodes::splat_to_mesh::definition,
        default_params: nodes::splat_to_mesh::default_params,
        compute_mesh: mesh_error_splat_to_mesh,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::SplatDeform,
        name: nodes::splat_deform::NAME,
        aliases: &[],
        definition: nodes::splat_deform::definition,
        default_params: nodes::splat_deform::default_params,
        compute_mesh: nodes::splat_deform::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::SplatDelight,
        name: nodes::splat_delight::NAME,
        aliases: &[],
        definition: nodes::splat_delight::definition,
        default_params: nodes::splat_delight::default_params,
        compute_mesh: nodes::splat_delight::compute,
        input_policy: InputPolicy::RequireAtLeast(1),
    },
    NodeSpec {
        kind: BuiltinNodeKind::SplatIntegrate,
        name: nodes::splat_integrate::NAME,
        aliases: &[],
        definition: nodes::splat_integrate::definition,
        default_params: nodes::splat_integrate::default_params,
        compute_mesh: nodes::splat_integrate::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::SplatHeal,
        name: nodes::splat_heal::NAME,
        aliases: &[],
        definition: nodes::splat_heal::definition,
        default_params: nodes::splat_heal::default_params,
        compute_mesh: nodes::splat_heal::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::SplatOutlier,
        name: nodes::splat_outlier::NAME,
        aliases: &[],
        definition: nodes::splat_outlier::definition,
        default_params: nodes::splat_outlier::default_params,
        compute_mesh: nodes::splat_outlier::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::SplatCluster,
        name: nodes::splat_cluster::NAME,
        aliases: &[],
        definition: nodes::splat_cluster::definition,
        default_params: nodes::splat_cluster::default_params,
        compute_mesh: nodes::splat_cluster::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::SplatMerge,
        name: nodes::splat_merge::NAME,
        aliases: &[],
        definition: nodes::splat_merge::definition,
        default_params: nodes::splat_merge::default_params,
        compute_mesh: nodes::splat_merge::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::VolumeFromGeometry,
        name: nodes::volume_from_geo::NAME,
        aliases: &[],
        definition: nodes::volume_from_geo::definition,
        default_params: nodes::volume_from_geo::default_params,
        compute_mesh: mesh_error_volume_from_geo,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::VolumeCombine,
        name: nodes::volume_combine::NAME,
        aliases: &[],
        definition: nodes::volume_combine::definition,
        default_params: nodes::volume_combine::default_params,
        compute_mesh: mesh_error_volume_combine,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::VolumeBlur,
        name: nodes::volume_blur::NAME,
        aliases: &[],
        definition: nodes::volume_blur::definition,
        default_params: nodes::volume_blur::default_params,
        compute_mesh: mesh_error_volume_blur,
        input_policy: InputPolicy::RequireAtLeast(1),
    },
    NodeSpec {
        kind: BuiltinNodeKind::VolumeToMesh,
        name: nodes::volume_to_mesh::NAME,
        aliases: &[],
        definition: nodes::volume_to_mesh::definition,
        default_params: nodes::volume_to_mesh::default_params,
        compute_mesh: mesh_error_volume_to_mesh,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Group,
        name: nodes::group::NAME,
        aliases: &[],
        definition: nodes::group::definition,
        default_params: nodes::group::default_params,
        compute_mesh: nodes::group::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::GroupExpand,
        name: nodes::group_expand::NAME,
        aliases: &[],
        definition: nodes::group_expand::definition,
        default_params: nodes::group_expand::default_params,
        compute_mesh: nodes::group_expand::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Transform,
        name: nodes::transform::NAME,
        aliases: &[],
        definition: nodes::transform::definition,
        default_params: nodes::transform::default_params,
        compute_mesh: nodes::transform::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Fuse,
        name: nodes::fuse::NAME,
        aliases: &[],
        definition: nodes::fuse::definition,
        default_params: nodes::fuse::default_params,
        compute_mesh: nodes::fuse::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Ffd,
        name: nodes::ffd::NAME,
        aliases: &[],
        definition: nodes::ffd::definition,
        default_params: nodes::ffd::default_params,
        compute_mesh: nodes::ffd::compute,
        input_policy: InputPolicy::RequireAtLeast(1),
    },
    NodeSpec {
        kind: BuiltinNodeKind::CopyTransform,
        name: nodes::copy_transform::NAME,
        aliases: &[],
        definition: nodes::copy_transform::definition,
        default_params: nodes::copy_transform::default_params,
        compute_mesh: nodes::copy_transform::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Merge,
        name: nodes::merge::NAME,
        aliases: &[],
        definition: nodes::merge::definition,
        default_params: nodes::merge::default_params,
        compute_mesh: nodes::merge::compute,
        input_policy: InputPolicy::RequireAtLeast(1),
    },
    NodeSpec {
        kind: BuiltinNodeKind::CopyToPoints,
        name: nodes::copy_to_points::NAME,
        aliases: &[],
        definition: nodes::copy_to_points::definition,
        default_params: nodes::copy_to_points::default_params,
        compute_mesh: nodes::copy_to_points::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Scatter,
        name: nodes::scatter::NAME,
        aliases: &[],
        definition: nodes::scatter::definition,
        default_params: nodes::scatter::default_params,
        compute_mesh: nodes::scatter::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Normal,
        name: nodes::normal::NAME,
        aliases: &[],
        definition: nodes::normal::definition,
        default_params: nodes::normal::default_params,
        compute_mesh: nodes::normal::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::PolyFrame,
        name: nodes::polyframe::NAME,
        aliases: &[],
        definition: nodes::polyframe::definition,
        default_params: nodes::polyframe::default_params,
        compute_mesh: nodes::polyframe::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Color,
        name: nodes::color::NAME,
        aliases: &[],
        definition: nodes::color::definition,
        default_params: nodes::color::default_params,
        compute_mesh: nodes::color::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Noise,
        name: nodes::noise::NAME,
        aliases: &[],
        definition: nodes::noise::definition,
        default_params: nodes::noise::default_params,
        compute_mesh: nodes::noise::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::ErosionNoise,
        name: nodes::erosion_noise::NAME,
        aliases: &[],
        definition: nodes::erosion_noise::definition,
        default_params: nodes::erosion_noise::default_params,
        compute_mesh: nodes::erosion_noise::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Smooth,
        name: nodes::smooth::NAME,
        aliases: &[],
        definition: nodes::smooth::definition,
        default_params: nodes::smooth::default_params,
        compute_mesh: nodes::smooth::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Resample,
        name: nodes::resample::NAME,
        aliases: &[],
        definition: nodes::resample::definition,
        default_params: nodes::resample::default_params,
        compute_mesh: nodes::resample::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::UvTexture,
        name: nodes::uv_texture::NAME,
        aliases: &[],
        definition: nodes::uv_texture::definition,
        default_params: nodes::uv_texture::default_params,
        compute_mesh: nodes::uv_texture::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::UvUnwrap,
        name: nodes::uv_unwrap::NAME,
        aliases: &[],
        definition: nodes::uv_unwrap::definition,
        default_params: nodes::uv_unwrap::default_params,
        compute_mesh: nodes::uv_unwrap::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::UvView,
        name: nodes::uv_view::NAME,
        aliases: &[],
        definition: nodes::uv_view::definition,
        default_params: nodes::uv_view::default_params,
        compute_mesh: nodes::uv_view::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Material,
        name: nodes::material::NAME,
        aliases: &[],
        definition: nodes::material::definition,
        default_params: nodes::material::default_params,
        compute_mesh: nodes::material::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Ray,
        name: nodes::ray::NAME,
        aliases: &[],
        definition: nodes::ray::definition,
        default_params: nodes::ray::default_params,
        compute_mesh: nodes::ray::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::AttributeNoise,
        name: nodes::attribute_noise::NAME,
        aliases: &[],
        definition: nodes::attribute_noise::definition,
        default_params: nodes::attribute_noise::default_params,
        compute_mesh: nodes::attribute_noise::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::AttributePromote,
        name: nodes::attribute_promote::NAME,
        aliases: &[],
        definition: nodes::attribute_promote::definition,
        default_params: nodes::attribute_promote::default_params,
        compute_mesh: nodes::attribute_promote::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::AttributeExpand,
        name: nodes::attribute_expand::NAME,
        aliases: &[],
        definition: nodes::attribute_expand::definition,
        default_params: nodes::attribute_expand::default_params,
        compute_mesh: nodes::attribute_expand::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::AttributeFromFeature,
        name: nodes::attribute_from_feature::NAME,
        aliases: &[],
        definition: nodes::attribute_from_feature::definition,
        default_params: nodes::attribute_from_feature::default_params,
        compute_mesh: nodes::attribute_from_feature::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::AttributeFromVolume,
        name: nodes::attribute_from_volume::NAME,
        aliases: &[],
        definition: nodes::attribute_from_volume::definition,
        default_params: nodes::attribute_from_volume::default_params,
        compute_mesh: mesh_error_attribute_from_volume,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::AttributeTransfer,
        name: nodes::attribute_transfer::NAME,
        aliases: &[],
        definition: nodes::attribute_transfer::definition,
        default_params: nodes::attribute_transfer::default_params,
        compute_mesh: nodes::attribute_transfer::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::AttributeMath,
        name: nodes::attribute_math::NAME,
        aliases: &[],
        definition: nodes::attribute_math::definition,
        default_params: nodes::attribute_math::default_params,
        compute_mesh: nodes::attribute_math::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Wrangle,
        name: nodes::wrangle::NAME,
        aliases: &[],
        definition: nodes::wrangle::definition,
        default_params: nodes::wrangle::default_params,
        compute_mesh: nodes::wrangle::compute,
        input_policy: InputPolicy::RequireAtLeast(1),
    },
    NodeSpec {
        kind: BuiltinNodeKind::ObjOutput,
        name: nodes::obj_output::NAME,
        aliases: &[],
        definition: nodes::obj_output::definition,
        default_params: nodes::obj_output::default_params,
        compute_mesh: nodes::obj_output::compute,
        input_policy: InputPolicy::RequireAll,
    },
    NodeSpec {
        kind: BuiltinNodeKind::Output,
        name: nodes::output::NAME,
        aliases: &[],
        definition: nodes::output::definition,
        default_params: nodes::output::default_params,
        compute_mesh: nodes::output::compute,
        input_policy: InputPolicy::RequireAll,
    },
];

pub fn node_specs() -> &'static [NodeSpec] {
    NODE_SPECS
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
    match kind {
        BuiltinNodeKind::Box => nodes::box_node::param_specs(),
        BuiltinNodeKind::Grid => nodes::grid::param_specs(),
        BuiltinNodeKind::Sphere => nodes::sphere::param_specs(),
        BuiltinNodeKind::Tube => nodes::tube::param_specs(),
        BuiltinNodeKind::Circle => nodes::circle::param_specs(),
        BuiltinNodeKind::Curve => nodes::curve::param_specs(),
        BuiltinNodeKind::Sweep => nodes::sweep::param_specs(),
        BuiltinNodeKind::File => nodes::file::param_specs(),
        BuiltinNodeKind::ReadSplats => nodes::read_splats::param_specs(),
        BuiltinNodeKind::WriteSplats => nodes::write_splats::param_specs(),
        BuiltinNodeKind::GltfOutput => nodes::gltf_output::param_specs(),
        BuiltinNodeKind::BooleanSdf => nodes::boolean::param_specs(),
        BuiltinNodeKind::BooleanGeo => nodes::boolean_geo::param_specs(),
        BuiltinNodeKind::Delete => nodes::delete::param_specs(),
        BuiltinNodeKind::Prune => nodes::prune::param_specs(),
        BuiltinNodeKind::Regularize => nodes::regularize::param_specs(),
        BuiltinNodeKind::SplatLod => nodes::splat_lod::param_specs(),
        BuiltinNodeKind::SplatToMesh => nodes::splat_to_mesh::param_specs(),
        BuiltinNodeKind::SplatDeform => nodes::splat_deform::param_specs(),
        BuiltinNodeKind::SplatDelight => nodes::splat_delight::param_specs(),
        BuiltinNodeKind::SplatIntegrate => nodes::splat_integrate::param_specs(),
        BuiltinNodeKind::SplatHeal => nodes::splat_heal::param_specs(),
        BuiltinNodeKind::SplatOutlier => nodes::splat_outlier::param_specs(),
        BuiltinNodeKind::SplatCluster => nodes::splat_cluster::param_specs(),
        BuiltinNodeKind::SplatMerge => nodes::splat_merge::param_specs(),
        BuiltinNodeKind::Group => nodes::group::param_specs(),
        BuiltinNodeKind::GroupExpand => nodes::group_expand::param_specs(),
        BuiltinNodeKind::Material => nodes::material::param_specs(),
        BuiltinNodeKind::VolumeFromGeometry => nodes::volume_from_geo::param_specs(),
        BuiltinNodeKind::VolumeCombine => nodes::volume_combine::param_specs(),
        BuiltinNodeKind::VolumeBlur => nodes::volume_blur::param_specs(),
        BuiltinNodeKind::VolumeToMesh => nodes::volume_to_mesh::param_specs(),
        BuiltinNodeKind::Transform => nodes::transform::param_specs(),
        BuiltinNodeKind::CopyTransform => nodes::copy_transform::param_specs(),
        BuiltinNodeKind::Fuse => nodes::fuse::param_specs(),
        BuiltinNodeKind::Ffd => nodes::ffd::param_specs(),
        BuiltinNodeKind::CopyToPoints => nodes::copy_to_points::param_specs(),
        BuiltinNodeKind::PolyFrame => nodes::polyframe::param_specs(),
        BuiltinNodeKind::Normal => nodes::normal::param_specs(),
        BuiltinNodeKind::AttributeNoise => nodes::attribute_noise::param_specs(),
        BuiltinNodeKind::AttributePromote => nodes::attribute_promote::param_specs(),
        BuiltinNodeKind::AttributeExpand => nodes::attribute_expand::param_specs(),
        BuiltinNodeKind::AttributeFromFeature => nodes::attribute_from_feature::param_specs(),
        BuiltinNodeKind::AttributeFromVolume => nodes::attribute_from_volume::param_specs(),
        BuiltinNodeKind::AttributeMath => nodes::attribute_math::param_specs(),
        BuiltinNodeKind::AttributeTransfer => nodes::attribute_transfer::param_specs(),
        BuiltinNodeKind::Color => nodes::color::param_specs(),
        BuiltinNodeKind::Noise => nodes::noise::param_specs(),
        BuiltinNodeKind::ErosionNoise => nodes::erosion_noise::param_specs(),
        BuiltinNodeKind::Ray => nodes::ray::param_specs(),
        BuiltinNodeKind::UvTexture => nodes::uv_texture::param_specs(),
        BuiltinNodeKind::UvUnwrap => nodes::uv_unwrap::param_specs(),
        BuiltinNodeKind::Smooth => nodes::smooth::param_specs(),
        BuiltinNodeKind::Scatter => nodes::scatter::param_specs(),
        BuiltinNodeKind::Resample => nodes::resample::param_specs(),
        BuiltinNodeKind::Wrangle => nodes::wrangle::param_specs(),
        BuiltinNodeKind::ObjOutput => nodes::obj_output::param_specs(),
        _ => Vec::new(),
    }
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
    match kind {
        BuiltinNodeKind::Box => Ok(Geometry::with_mesh(nodes::box_node::compute(params, &[])?)),
        BuiltinNodeKind::Grid => Ok(Geometry::with_mesh(nodes::grid::compute(params, &[])?)),
        BuiltinNodeKind::Sphere => Ok(Geometry::with_mesh(nodes::sphere::compute(params, &[])?)),
        BuiltinNodeKind::Tube => Ok(Geometry::with_mesh(nodes::tube::compute(params, &[])?)),
        BuiltinNodeKind::Circle => nodes::circle::apply_to_geometry(params),
        BuiltinNodeKind::Curve => {
            let output = nodes::curve::compute(params)?;
            Ok(Geometry::with_curve(output.points, output.closed))
        }
        BuiltinNodeKind::Sweep => nodes::sweep::apply_to_geometry(params, inputs),
        BuiltinNodeKind::File => Ok(Geometry::with_mesh(nodes::file::compute(params, &[])?)),
        BuiltinNodeKind::ReadSplats => {
            Ok(Geometry::with_splats(nodes::read_splats::compute(params)?))
        }
        BuiltinNodeKind::WriteSplats => apply_write_splats(params, inputs),
        BuiltinNodeKind::BooleanSdf => nodes::boolean::apply_to_geometry(params, inputs),
        BuiltinNodeKind::BooleanGeo => nodes::boolean_geo::apply_to_geometry(params, inputs),
        BuiltinNodeKind::Delete => apply_delete(params, inputs),
        BuiltinNodeKind::Prune => apply_prune(params, inputs),
        BuiltinNodeKind::Regularize => apply_regularize(params, inputs),
        BuiltinNodeKind::SplatLod => apply_splat_lod(params, inputs),
        BuiltinNodeKind::SplatToMesh => nodes::splat_to_mesh::apply_to_geometry(params, inputs),
        BuiltinNodeKind::SplatDeform => nodes::splat_deform::apply_to_geometry(params, inputs),
        BuiltinNodeKind::SplatDelight => apply_splat_delight(params, inputs),
        BuiltinNodeKind::SplatIntegrate => nodes::splat_integrate::apply_to_geometry(params, inputs),
        BuiltinNodeKind::SplatHeal => apply_splat_heal(params, inputs),
        BuiltinNodeKind::SplatOutlier => apply_splat_outlier(params, inputs),
        BuiltinNodeKind::SplatCluster => apply_splat_cluster(params, inputs),
        BuiltinNodeKind::SplatMerge => nodes::splat_merge::apply_to_geometry(params, inputs),
        BuiltinNodeKind::VolumeFromGeometry => {
            nodes::volume_from_geo::apply_to_geometry(params, inputs)
        }
        BuiltinNodeKind::VolumeCombine => nodes::volume_combine::apply_to_geometry(params, inputs),
        BuiltinNodeKind::VolumeBlur => nodes::volume_blur::apply_to_geometry(params, inputs),
        BuiltinNodeKind::VolumeToMesh => nodes::volume_to_mesh::apply_to_geometry(params, inputs),
        BuiltinNodeKind::Group => apply_group(params, inputs),
        BuiltinNodeKind::GroupExpand => apply_group_expand(params, inputs),
        BuiltinNodeKind::Transform => apply_transform(params, inputs),
        BuiltinNodeKind::Fuse => nodes::fuse::apply_to_geometry(params, inputs),
        BuiltinNodeKind::Ffd => nodes::ffd::apply_to_geometry(params, inputs),
        BuiltinNodeKind::CopyTransform => apply_copy_transform(params, inputs),
        BuiltinNodeKind::Ray => nodes::ray::apply_to_geometry(params, inputs),
        BuiltinNodeKind::Material => nodes::material::apply_to_geometry(params, inputs),
        BuiltinNodeKind::Scatter => nodes::scatter::apply_to_geometry(params, inputs),
        BuiltinNodeKind::Resample => nodes::resample::apply_to_geometry(params, inputs),
        BuiltinNodeKind::PolyFrame => nodes::polyframe::apply_to_geometry(params, inputs),
        BuiltinNodeKind::Normal
        | BuiltinNodeKind::Color
        | BuiltinNodeKind::Noise
        | BuiltinNodeKind::ErosionNoise
        | BuiltinNodeKind::Smooth
        | BuiltinNodeKind::UvTexture
        | BuiltinNodeKind::UvUnwrap
        | BuiltinNodeKind::UvView
        | BuiltinNodeKind::AttributeNoise
        | BuiltinNodeKind::AttributePromote
        | BuiltinNodeKind::AttributeExpand
        | BuiltinNodeKind::AttributeFromFeature
        | BuiltinNodeKind::AttributeMath => apply_mesh_unary(kind, params, inputs),
        BuiltinNodeKind::Wrangle => nodes::wrangle::apply_to_geometry(params, inputs),
        BuiltinNodeKind::AttributeTransfer => apply_attribute_transfer(params, inputs),
        BuiltinNodeKind::AttributeFromVolume => {
            nodes::attribute_from_volume::apply_to_geometry(params, inputs)
        }
        BuiltinNodeKind::CopyToPoints => apply_copy_to_points(params, inputs),
        BuiltinNodeKind::Merge => merge_geometry(inputs),
        BuiltinNodeKind::ObjOutput => apply_obj_output(params, inputs),
        BuiltinNodeKind::GltfOutput => apply_obj_output(params, inputs),
        BuiltinNodeKind::Output => Ok(inputs.first().cloned().unwrap_or_default()),
    }
}

pub fn compute_splat_node(
    kind: BuiltinNodeKind,
    params: &NodeParams,
    _inputs: &[SplatGeo],
) -> Result<SplatGeo, String> {
    match kind {
        BuiltinNodeKind::ReadSplats => nodes::read_splats::compute(params),
        _ => Err("Node does not produce splats".to_string()),
    }
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

    let mut splats = Vec::with_capacity(input.splats.len());
    for splat in &input.splats {
        let mut splat = splat.clone();
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
        splats.push(splat);
    }

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
    mut op: F,
) -> Result<Geometry, String>
where
    F: FnMut(&NodeParams, &SplatGeo) -> Result<SplatGeo, String>,
{
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };

    let mut meshes = Vec::new();
    if let Some(mesh) = input.merged_mesh() {
        meshes.push(mesh);
    }

    let mut splats = Vec::with_capacity(input.splats.len());
    for splat in &input.splats {
        splats.push(op(params, splat)?);
    }

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

    let mut splats = Vec::with_capacity(input.splats.len());
    for splat in &input.splats {
        splats.push(filter_splats(params, splat));
    }

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

    let mut splats = Vec::with_capacity(input.splats.len());
    for splat in &input.splats {
        let mut splat = splat.clone();
        nodes::group::apply_to_splats(params, &mut splat)?;
        splats.push(splat);
    }

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

    let mut splats = Vec::with_capacity(input.splats.len());
    for splat in &input.splats {
        let mut splat = splat.clone();
        nodes::group_expand::apply_to_splats(params, &mut splat)?;
        splats.push(splat);
    }

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

    let mut splats = Vec::with_capacity(input.splats.len());
    for splat in &input.splats {
        let mut splat = splat.clone();
        if let Some(mask) =
            nodes::group_utils::splat_group_mask(&splat, params, AttributeDomain::Point)
        {
            splat.transform_masked(matrix, &mask);
        } else {
            splat.transform(matrix);
        }
        splats.push(splat);
    }

    let mut volumes = Vec::with_capacity(input.volumes.len());
    for volume in &input.volumes {
        let mut volume = volume.clone();
        volume.transform = matrix * volume.transform;
        volumes.push(volume);
    }

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
        let mut copies = Vec::with_capacity(matrices.len());
        for matrix in &matrices {
            let mut copy = mesh.clone();
            copy.transform(*matrix);
            copies.push(copy);
        }
        meshes.push(Mesh::merge(&copies));
    }

    let mut splats = Vec::with_capacity(input.splats.len());
    for splat in &input.splats {
        let mut copies = Vec::with_capacity(matrices.len());
        for matrix in &matrices {
            let mut copy = splat.clone();
            copy.transform(*matrix);
            copies.push(copy);
        }
        splats.push(merge_splats(&copies));
    }

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
    use std::collections::BTreeMap;

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
}
