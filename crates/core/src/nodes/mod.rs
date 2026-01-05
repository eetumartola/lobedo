pub mod attribute_math;
pub mod attribute_from_feature;
pub mod attribute_noise;
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
pub mod read_splats;
pub mod regularize;
pub mod write_splats;
pub mod delete;
pub mod scatter;
pub mod sphere;
pub mod transform;
pub mod tube;
pub mod wrangle;

use crate::graph::{PinDefinition, PinType};
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
