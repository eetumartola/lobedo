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

impl BuiltinNodeKind {
    pub fn name(self) -> &'static str {
        match self {
            BuiltinNodeKind::Box => nodes::box_node::NAME,
            BuiltinNodeKind::Grid => nodes::grid::NAME,
            BuiltinNodeKind::Sphere => nodes::sphere::NAME,
            BuiltinNodeKind::Tube => nodes::tube::NAME,
            BuiltinNodeKind::File => nodes::file::NAME,
            BuiltinNodeKind::ReadSplats => nodes::read_splats::NAME,
            BuiltinNodeKind::WriteSplats => nodes::write_splats::NAME,
            BuiltinNodeKind::Delete => nodes::delete::NAME,
            BuiltinNodeKind::Prune => nodes::prune::NAME,
            BuiltinNodeKind::Regularize => nodes::regularize::NAME,
            BuiltinNodeKind::Group => nodes::group::NAME,
            BuiltinNodeKind::Transform => nodes::transform::NAME,
            BuiltinNodeKind::CopyTransform => nodes::copy_transform::NAME,
            BuiltinNodeKind::Merge => nodes::merge::NAME,
            BuiltinNodeKind::CopyToPoints => nodes::copy_to_points::NAME,
            BuiltinNodeKind::Scatter => nodes::scatter::NAME,
            BuiltinNodeKind::Normal => nodes::normal::NAME,
            BuiltinNodeKind::Color => nodes::color::NAME,
            BuiltinNodeKind::Noise => nodes::noise::NAME,
            BuiltinNodeKind::Smooth => nodes::smooth::NAME,
            BuiltinNodeKind::Ray => nodes::ray::NAME,
            BuiltinNodeKind::AttributeNoise => nodes::attribute_noise::NAME,
            BuiltinNodeKind::AttributeFromFeature => nodes::attribute_from_feature::NAME,
            BuiltinNodeKind::AttributeTransfer => nodes::attribute_transfer::NAME,
            BuiltinNodeKind::AttributeMath => nodes::attribute_math::NAME,
            BuiltinNodeKind::Wrangle => nodes::wrangle::NAME,
            BuiltinNodeKind::ObjOutput => nodes::obj_output::NAME,
            BuiltinNodeKind::Output => nodes::output::NAME,
        }
    }
}

pub fn builtin_kind_from_name(name: &str) -> Option<BuiltinNodeKind> {
    match name {
        nodes::box_node::NAME => Some(BuiltinNodeKind::Box),
        nodes::grid::NAME => Some(BuiltinNodeKind::Grid),
        nodes::sphere::NAME => Some(BuiltinNodeKind::Sphere),
        nodes::tube::NAME => Some(BuiltinNodeKind::Tube),
        nodes::file::NAME => Some(BuiltinNodeKind::File),
        nodes::read_splats::NAME | nodes::read_splats::LEGACY_NAME => {
            Some(BuiltinNodeKind::ReadSplats)
        }
        nodes::write_splats::NAME | nodes::write_splats::LEGACY_NAME => {
            Some(BuiltinNodeKind::WriteSplats)
        }
        nodes::delete::NAME => Some(BuiltinNodeKind::Delete),
        nodes::prune::NAME | nodes::prune::LEGACY_NAME => Some(BuiltinNodeKind::Prune),
        nodes::regularize::NAME | nodes::regularize::LEGACY_NAME => {
            Some(BuiltinNodeKind::Regularize)
        }
        nodes::group::NAME => Some(BuiltinNodeKind::Group),
        nodes::transform::NAME => Some(BuiltinNodeKind::Transform),
        nodes::copy_transform::NAME => Some(BuiltinNodeKind::CopyTransform),
        nodes::merge::NAME => Some(BuiltinNodeKind::Merge),
        nodes::copy_to_points::NAME => Some(BuiltinNodeKind::CopyToPoints),
        nodes::scatter::NAME => Some(BuiltinNodeKind::Scatter),
        nodes::normal::NAME => Some(BuiltinNodeKind::Normal),
        nodes::color::NAME => Some(BuiltinNodeKind::Color),
        nodes::noise::NAME => Some(BuiltinNodeKind::Noise),
        nodes::smooth::NAME => Some(BuiltinNodeKind::Smooth),
        nodes::ray::NAME => Some(BuiltinNodeKind::Ray),
        nodes::attribute_noise::NAME => Some(BuiltinNodeKind::AttributeNoise),
        nodes::attribute_from_feature::NAME => Some(BuiltinNodeKind::AttributeFromFeature),
        nodes::attribute_transfer::NAME => Some(BuiltinNodeKind::AttributeTransfer),
        nodes::attribute_math::NAME => Some(BuiltinNodeKind::AttributeMath),
        nodes::wrangle::NAME => Some(BuiltinNodeKind::Wrangle),
        nodes::obj_output::NAME => Some(BuiltinNodeKind::ObjOutput),
        nodes::output::NAME => Some(BuiltinNodeKind::Output),
        _ => None,
    }
}

pub fn builtin_definitions() -> Vec<NodeDefinition> {
    vec![
        node_definition(BuiltinNodeKind::Box),
        node_definition(BuiltinNodeKind::Grid),
        node_definition(BuiltinNodeKind::Sphere),
        node_definition(BuiltinNodeKind::Tube),
        node_definition(BuiltinNodeKind::File),
        node_definition(BuiltinNodeKind::ReadSplats),
        node_definition(BuiltinNodeKind::WriteSplats),
        node_definition(BuiltinNodeKind::Delete),
        node_definition(BuiltinNodeKind::Prune),
        node_definition(BuiltinNodeKind::Regularize),
        node_definition(BuiltinNodeKind::Group),
        node_definition(BuiltinNodeKind::Transform),
        node_definition(BuiltinNodeKind::CopyTransform),
        node_definition(BuiltinNodeKind::Merge),
        node_definition(BuiltinNodeKind::CopyToPoints),
        node_definition(BuiltinNodeKind::Scatter),
        node_definition(BuiltinNodeKind::Normal),
        node_definition(BuiltinNodeKind::Color),
        node_definition(BuiltinNodeKind::Noise),
        node_definition(BuiltinNodeKind::Smooth),
        node_definition(BuiltinNodeKind::Ray),
        node_definition(BuiltinNodeKind::AttributeNoise),
        node_definition(BuiltinNodeKind::AttributeFromFeature),
        node_definition(BuiltinNodeKind::AttributeTransfer),
        node_definition(BuiltinNodeKind::AttributeMath),
        node_definition(BuiltinNodeKind::Wrangle),
        node_definition(BuiltinNodeKind::ObjOutput),
        node_definition(BuiltinNodeKind::Output),
    ]
}

pub fn node_definition(kind: BuiltinNodeKind) -> NodeDefinition {
    match kind {
        BuiltinNodeKind::Box => nodes::box_node::definition(),
        BuiltinNodeKind::Grid => nodes::grid::definition(),
        BuiltinNodeKind::Sphere => nodes::sphere::definition(),
        BuiltinNodeKind::Tube => nodes::tube::definition(),
        BuiltinNodeKind::File => nodes::file::definition(),
        BuiltinNodeKind::ReadSplats => nodes::read_splats::definition(),
        BuiltinNodeKind::WriteSplats => nodes::write_splats::definition(),
        BuiltinNodeKind::Delete => nodes::delete::definition(),
        BuiltinNodeKind::Prune => nodes::prune::definition(),
        BuiltinNodeKind::Regularize => nodes::regularize::definition(),
        BuiltinNodeKind::Group => nodes::group::definition(),
        BuiltinNodeKind::Transform => nodes::transform::definition(),
        BuiltinNodeKind::CopyTransform => nodes::copy_transform::definition(),
        BuiltinNodeKind::Merge => nodes::merge::definition(),
        BuiltinNodeKind::CopyToPoints => nodes::copy_to_points::definition(),
        BuiltinNodeKind::Scatter => nodes::scatter::definition(),
        BuiltinNodeKind::Normal => nodes::normal::definition(),
        BuiltinNodeKind::Color => nodes::color::definition(),
        BuiltinNodeKind::Noise => nodes::noise::definition(),
        BuiltinNodeKind::Smooth => nodes::smooth::definition(),
        BuiltinNodeKind::Ray => nodes::ray::definition(),
        BuiltinNodeKind::AttributeNoise => nodes::attribute_noise::definition(),
        BuiltinNodeKind::AttributeFromFeature => nodes::attribute_from_feature::definition(),
        BuiltinNodeKind::AttributeTransfer => nodes::attribute_transfer::definition(),
        BuiltinNodeKind::AttributeMath => nodes::attribute_math::definition(),
        BuiltinNodeKind::Wrangle => nodes::wrangle::definition(),
        BuiltinNodeKind::ObjOutput => nodes::obj_output::definition(),
        BuiltinNodeKind::Output => nodes::output::definition(),
    }
}

pub fn default_params(kind: BuiltinNodeKind) -> NodeParams {
    match kind {
        BuiltinNodeKind::Box => nodes::box_node::default_params(),
        BuiltinNodeKind::Grid => nodes::grid::default_params(),
        BuiltinNodeKind::Sphere => nodes::sphere::default_params(),
        BuiltinNodeKind::Tube => nodes::tube::default_params(),
        BuiltinNodeKind::File => nodes::file::default_params(),
        BuiltinNodeKind::ReadSplats => nodes::read_splats::default_params(),
        BuiltinNodeKind::WriteSplats => nodes::write_splats::default_params(),
        BuiltinNodeKind::Delete => nodes::delete::default_params(),
        BuiltinNodeKind::Prune => nodes::prune::default_params(),
        BuiltinNodeKind::Regularize => nodes::regularize::default_params(),
        BuiltinNodeKind::Group => nodes::group::default_params(),
        BuiltinNodeKind::Transform => nodes::transform::default_params(),
        BuiltinNodeKind::CopyTransform => nodes::copy_transform::default_params(),
        BuiltinNodeKind::Merge => nodes::merge::default_params(),
        BuiltinNodeKind::CopyToPoints => nodes::copy_to_points::default_params(),
        BuiltinNodeKind::Scatter => nodes::scatter::default_params(),
        BuiltinNodeKind::Normal => nodes::normal::default_params(),
        BuiltinNodeKind::Color => nodes::color::default_params(),
        BuiltinNodeKind::Noise => nodes::noise::default_params(),
        BuiltinNodeKind::Smooth => nodes::smooth::default_params(),
        BuiltinNodeKind::Ray => nodes::ray::default_params(),
        BuiltinNodeKind::AttributeNoise => nodes::attribute_noise::default_params(),
        BuiltinNodeKind::AttributeFromFeature => nodes::attribute_from_feature::default_params(),
        BuiltinNodeKind::AttributeTransfer => nodes::attribute_transfer::default_params(),
        BuiltinNodeKind::AttributeMath => nodes::attribute_math::default_params(),
        BuiltinNodeKind::Wrangle => nodes::wrangle::default_params(),
        BuiltinNodeKind::ObjOutput => nodes::obj_output::default_params(),
        BuiltinNodeKind::Output => nodes::output::default_params(),
    }
}

pub fn compute_mesh_node(
    kind: BuiltinNodeKind,
    params: &NodeParams,
    inputs: &[Mesh],
) -> Result<Mesh, String> {
    match kind {
        BuiltinNodeKind::Box => nodes::box_node::compute(params, inputs),
        BuiltinNodeKind::Grid => nodes::grid::compute(params, inputs),
        BuiltinNodeKind::Sphere => nodes::sphere::compute(params, inputs),
        BuiltinNodeKind::Tube => nodes::tube::compute(params, inputs),
        BuiltinNodeKind::File => nodes::file::compute(params, inputs),
        BuiltinNodeKind::ReadSplats => {
            Err("Splat Read outputs splat geometry, not meshes".to_string())
        }
        BuiltinNodeKind::WriteSplats => {
            Err("Splat Write expects splat geometry, not meshes".to_string())
        }
        BuiltinNodeKind::Delete => nodes::delete::compute(params, inputs),
        BuiltinNodeKind::Prune => nodes::prune::compute(params, inputs),
        BuiltinNodeKind::Regularize => nodes::regularize::compute(params, inputs),
        BuiltinNodeKind::Group => nodes::group::compute(params, inputs),
        BuiltinNodeKind::Transform => nodes::transform::compute(params, inputs),
        BuiltinNodeKind::CopyTransform => nodes::copy_transform::compute(params, inputs),
        BuiltinNodeKind::Merge => nodes::merge::compute(params, inputs),
        BuiltinNodeKind::CopyToPoints => nodes::copy_to_points::compute(params, inputs),
        BuiltinNodeKind::Scatter => nodes::scatter::compute(params, inputs),
        BuiltinNodeKind::Normal => nodes::normal::compute(params, inputs),
        BuiltinNodeKind::Color => nodes::color::compute(params, inputs),
        BuiltinNodeKind::Noise => nodes::noise::compute(params, inputs),
        BuiltinNodeKind::Smooth => nodes::smooth::compute(params, inputs),
        BuiltinNodeKind::Ray => nodes::ray::compute(params, inputs),
        BuiltinNodeKind::AttributeNoise => nodes::attribute_noise::compute(params, inputs),
        BuiltinNodeKind::AttributeFromFeature => {
            nodes::attribute_from_feature::compute(params, inputs)
        }
        BuiltinNodeKind::AttributeTransfer => nodes::attribute_transfer::compute(params, inputs),
        BuiltinNodeKind::AttributeMath => nodes::attribute_math::compute(params, inputs),
        BuiltinNodeKind::Wrangle => nodes::wrangle::compute(params, inputs),
        BuiltinNodeKind::ObjOutput => nodes::obj_output::compute(params, inputs),
        BuiltinNodeKind::Output => nodes::output::compute(params, inputs),
    }
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
