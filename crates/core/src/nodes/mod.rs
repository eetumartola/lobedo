pub mod attribute_math;
pub mod attribute_utils;
pub mod attribute_from_feature;
pub mod attribute_noise;
pub mod attribute_transfer;
pub mod box_node;
pub mod color;
pub mod copy_to_points;
pub mod copy_transform;
pub mod file;
pub mod group;
pub mod grid;
pub mod group_utils;
pub mod merge;
pub mod noise;
pub mod normal;
pub mod obj_output;
pub mod output;
pub mod prune;
pub mod ray;
pub mod read_splats;
pub mod regularize;
pub mod splat_to_mesh;
pub mod splat_deform;
pub mod splat_lod;
pub mod splat_utils;
pub mod write_splats;
pub mod delete;
pub mod scatter;
pub mod sphere;
pub mod smooth;
pub mod transform;
pub mod tube;
pub mod wrangle;

use std::collections::BTreeMap;

use crate::graph::{ParamValue, PinDefinition, PinType};
use crate::mesh::Mesh;

pub fn geometry_in(name: &str) -> PinDefinition {
    PinDefinition {
        name: name.to_string(),
        pin_type: PinType::Geometry,
    }
}

pub fn geometry_out(name: &str) -> PinDefinition {
    PinDefinition {
        name: name.to_string(),
        pin_type: PinType::Geometry,
    }
}

pub fn require_mesh_input(
    inputs: &[Mesh],
    index: usize,
    message: &str,
) -> Result<Mesh, String> {
    inputs
        .get(index)
        .cloned()
        .ok_or_else(|| message.to_string())
}

pub fn selection_shape_params() -> BTreeMap<String, ParamValue> {
    BTreeMap::from([
        ("shape".to_string(), ParamValue::String("box".to_string())),
        ("invert".to_string(), ParamValue::Bool(false)),
        ("center".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0])),
        ("size".to_string(), ParamValue::Vec3([1.0, 1.0, 1.0])),
        ("radius".to_string(), ParamValue::Float(1.0)),
        (
            "plane_origin".to_string(),
            ParamValue::Vec3([0.0, 0.0, 0.0]),
        ),
        (
            "plane_normal".to_string(),
            ParamValue::Vec3([0.0, 1.0, 0.0]),
        ),
    ])
}
