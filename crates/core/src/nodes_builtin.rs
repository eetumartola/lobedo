use crate::attributes::AttributeDomain;
use crate::graph::{NodeDefinition, NodeParams};
use crate::geometry::{merge_splats, Geometry};
use crate::mesh::Mesh;
use crate::nodes;
use crate::splat::SplatGeo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinNodeKind {
    Box,
    Grid,
    Sphere,
    Tube,
    File,
    ReadSplats,
    WriteSplats,
    Delete,
    Prune,
    Regularize,
    SplatLod,
    Group,
    Transform,
    CopyTransform,
    Merge,
    CopyToPoints,
    Scatter,
    Normal,
    Color,
    Noise,
    Smooth,
    Ray,
    AttributeNoise,
    AttributeFromFeature,
    AttributeTransfer,
    AttributeMath,
    Wrangle,
    ObjOutput,
    Output,
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

fn mesh_error_write_splats(_params: &NodeParams, _inputs: &[Mesh]) -> Result<Mesh, String> {
    Err("Splat Write expects splat geometry, not meshes".to_string())
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
        kind: BuiltinNodeKind::Group,
        name: nodes::group::NAME,
        aliases: &[],
        definition: nodes::group::definition,
        default_params: nodes::group::default_params,
        compute_mesh: nodes::group::compute,
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
        kind: BuiltinNodeKind::Smooth,
        name: nodes::smooth::NAME,
        aliases: &[],
        definition: nodes::smooth::definition,
        default_params: nodes::smooth::default_params,
        compute_mesh: nodes::smooth::compute,
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
        kind: BuiltinNodeKind::AttributeFromFeature,
        name: nodes::attribute_from_feature::NAME,
        aliases: &[],
        definition: nodes::attribute_from_feature::definition,
        default_params: nodes::attribute_from_feature::default_params,
        compute_mesh: nodes::attribute_from_feature::compute,
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
        input_policy: InputPolicy::RequireAll,
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
        BuiltinNodeKind::File => Ok(Geometry::with_mesh(nodes::file::compute(params, &[])?)),
        BuiltinNodeKind::ReadSplats => {
            Ok(Geometry::with_splats(nodes::read_splats::compute(params)?))
        }
        BuiltinNodeKind::WriteSplats => apply_write_splats(params, inputs),
        BuiltinNodeKind::Delete => apply_delete(params, inputs),
        BuiltinNodeKind::Prune => apply_prune(params, inputs),
        BuiltinNodeKind::Regularize => apply_regularize(params, inputs),
        BuiltinNodeKind::SplatLod => apply_splat_lod(params, inputs),
        BuiltinNodeKind::Group => apply_group(params, inputs),
        BuiltinNodeKind::Transform => apply_transform(params, inputs),
        BuiltinNodeKind::CopyTransform => apply_copy_transform(params, inputs),
        BuiltinNodeKind::Ray => nodes::ray::apply_to_geometry(params, inputs),
        BuiltinNodeKind::Normal
        | BuiltinNodeKind::Scatter
        | BuiltinNodeKind::Color
        | BuiltinNodeKind::Noise
        | BuiltinNodeKind::Smooth
        | BuiltinNodeKind::AttributeNoise
        | BuiltinNodeKind::AttributeFromFeature
        | BuiltinNodeKind::AttributeMath
        | BuiltinNodeKind::Wrangle => apply_mesh_unary(kind, params, inputs),
        BuiltinNodeKind::AttributeTransfer => apply_attribute_transfer(params, inputs),
        BuiltinNodeKind::CopyToPoints => apply_copy_to_points(params, inputs),
        BuiltinNodeKind::Merge => merge_geometry(inputs),
        BuiltinNodeKind::ObjOutput => apply_obj_output(params, inputs),
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
    let mut meshes = Vec::with_capacity(input.meshes.len());
    for mesh in input.meshes.iter() {
        meshes.push(compute_mesh_node(kind, params, std::slice::from_ref(mesh))?);
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
            BuiltinNodeKind::Smooth => {
                nodes::smooth::apply_to_splats(params, &mut splat)?;
            }
            BuiltinNodeKind::AttributeNoise => {
                nodes::attribute_noise::apply_to_splats(params, &mut splat)?;
            }
            BuiltinNodeKind::AttributeFromFeature => {
                nodes::attribute_from_feature::apply_to_splats(params, &mut splat)?;
            }
            BuiltinNodeKind::AttributeMath => {
                nodes::attribute_math::apply_to_splats(params, &mut splat)?;
            }
            BuiltinNodeKind::Wrangle => {
                nodes::wrangle::apply_to_splats(params, &mut splat)?;
            }
            _ => {}
        }
        splats.push(splat);
    }

    Ok(Geometry { meshes, splats })
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

    let mut meshes = Vec::with_capacity(input.meshes.len());
    for mesh in &input.meshes {
        meshes.push(nodes::delete::compute(params, std::slice::from_ref(mesh))?);
    }

    let mut splats = Vec::with_capacity(input.splats.len());
    for splat in &input.splats {
        splats.push(filter_splats(params, splat));
    }

    Ok(Geometry { meshes, splats })
}

fn apply_prune(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };

    let meshes = input.meshes.clone();
    let mut splats = Vec::with_capacity(input.splats.len());
    for splat in &input.splats {
        splats.push(nodes::prune::apply_to_splats(params, splat));
    }

    Ok(Geometry { meshes, splats })
}

fn apply_regularize(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };

    let meshes = input.meshes.clone();
    let mut splats = Vec::with_capacity(input.splats.len());
    for splat in &input.splats {
        splats.push(nodes::regularize::apply_to_splats(params, splat));
    }

    Ok(Geometry { meshes, splats })
}

fn apply_splat_lod(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };

    let meshes = input.meshes.clone();
    let mut splats = Vec::with_capacity(input.splats.len());
    for splat in &input.splats {
        splats.push(nodes::splat_lod::apply_to_splats(params, splat));
    }

    Ok(Geometry { meshes, splats })
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

    let mut meshes = Vec::with_capacity(input.meshes.len());
    for mesh in &input.meshes {
        meshes.push(nodes::group::compute(params, std::slice::from_ref(mesh))?);
    }

    let mut splats = Vec::with_capacity(input.splats.len());
    for splat in &input.splats {
        let mut splat = splat.clone();
        nodes::group::apply_to_splats(params, &mut splat)?;
        splats.push(splat);
    }

    Ok(Geometry { meshes, splats })
}

fn apply_transform(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };

    let matrix = nodes::transform::transform_matrix(params);

    let mut meshes = Vec::with_capacity(input.meshes.len());
    for mesh in &input.meshes {
        let mut mesh = mesh.clone();
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

    Ok(Geometry { meshes, splats })
}

fn apply_copy_transform(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };
    let matrices = nodes::copy_transform::transform_matrices(params);
    if matrices.is_empty() {
        return Ok(Geometry::default());
    }

    let mut meshes = Vec::with_capacity(input.meshes.len());
    for mesh in &input.meshes {
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

    Ok(Geometry { meshes, splats })
}

fn apply_copy_to_points(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let mut output = Geometry::default();
    if let Some(input) = inputs.first() {
        output.splats = input.splats.clone();
    }

    let source = inputs
        .first()
        .and_then(|geo| geo.merged_mesh());
    let template = inputs
        .get(1)
        .and_then(|geo| geo.merged_mesh());

    if let (Some(source), Some(template)) = (source, template) {
        let mesh = nodes::copy_to_points::compute(params, &[source, template])?;
        output.meshes.push(mesh);
    }

    Ok(output)
}

fn apply_obj_output(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let mut output = Geometry::default();
    if let Some(input) = inputs.first() {
        output.splats = input.splats.clone();
        if let Some(mesh) = input.merged_mesh() {
            let mesh = nodes::obj_output::compute(params, &[mesh])?;
            output.meshes.push(mesh);
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
