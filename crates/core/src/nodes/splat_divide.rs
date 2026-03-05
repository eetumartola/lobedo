use std::collections::{BTreeMap, BTreeSet};

use crate::attributes::{AttributeDomain, AttributeRef};
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::nodes::{geometry_in, geometry_out};
use crate::param_spec::ParamSpec;
use crate::splat::SplatGeo;

pub const NAME: &str = "Splat Divide";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Splat".to_string(),
        inputs: vec![geometry_in("in")],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([(
            "attr".to_string(),
            ParamValue::String("segment_id".to_string()),
        )]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![ParamSpec::string("attr", "Attribute").with_help("Attribute to split by (int/float).")]
}

pub fn apply_to_geometry(params: &NodeParams, inputs: &[Geometry]) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };

    let attr_name = params.get_string("attr", "segment_id").trim().to_string();
    if attr_name.is_empty() {
        return Err("Splat Divide requires an attribute name".to_string());
    }

    let mut out_splats: Vec<SplatGeo> = Vec::new();
    for splat in &input.splats {
        let attr = splat
            .attribute(AttributeDomain::Point, &attr_name)
            .or_else(|| splat.attribute(AttributeDomain::Primitive, &attr_name));
        let Some(attr) = attr else {
            out_splats.push(splat.clone());
            continue;
        };
        let count = splat.len();

        let mut buckets: BTreeMap<i32, Vec<usize>> = BTreeMap::new();
        match attr {
            AttributeRef::Int(values) => {
                if values.len() != count {
                    return Err(format!(
                        "Splat Divide attribute '{attr_name}' has invalid length"
                    ));
                }
                for (idx, value) in values.iter().enumerate() {
                    buckets.entry(*value).or_default().push(idx);
                }
            }
            AttributeRef::Float(values) => {
                if values.len() != count {
                    return Err(format!(
                        "Splat Divide attribute '{attr_name}' has invalid length"
                    ));
                }
                for (idx, value) in values.iter().enumerate() {
                    let id = value.round() as i32;
                    buckets.entry(id).or_default().push(idx);
                }
            }
            _ => {
                return Err(format!(
                    "Splat Divide attribute '{attr_name}' must be int or float"
                ));
            }
        }

        if buckets.is_empty() {
            out_splats.push(splat.clone());
            continue;
        }

        let mut keys = BTreeSet::new();
        keys.extend(buckets.keys().copied());
        for key in keys {
            let indices = buckets.get(&key).cloned().unwrap_or_default();
            if indices.is_empty() {
                continue;
            }
            out_splats.push(splat.filter_by_indices(&indices));
        }
    }

    Ok(Geometry {
        meshes: input.meshes.clone(),
        splats: out_splats,
        curves: input.curves.clone(),
        volumes: input.volumes.clone(),
        materials: input.materials.clone(),
    })
}
