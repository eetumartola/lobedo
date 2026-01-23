use std::collections::BTreeMap;

use glam::Vec3;

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    attribute_utils::{
        domain_from_params, existing_float_attr_mesh, existing_float_attr_splats,
        existing_vec2_attr_mesh, existing_vec2_attr_splats, existing_vec3_attr_mesh,
        existing_vec3_attr_splats, mesh_sample_position, splat_sample_position,
    },
    geometry_in,
    geometry_out,
    group_utils::{mask_has_any, mesh_group_mask, splat_group_mask},
    recompute_mesh_normals,
    require_mesh_input,
};
use crate::noise::{fractal_noise, FractalSettings, FractalType, NoiseType};
use crate::param_spec::ParamSpec;
use crate::parallel;
use crate::splat::SplatGeo;

pub const NAME: &str = "Attribute Noise";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![geometry_in("in")],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            ("attr".to_string(), ParamValue::String("P".to_string())),
            ("domain".to_string(), ParamValue::Int(0)),
            ("data_type".to_string(), ParamValue::Int(2)),
            ("noise_type".to_string(), ParamValue::Int(0)),
            ("fractal_type".to_string(), ParamValue::Int(1)),
            ("octaves".to_string(), ParamValue::Int(3)),
            ("lacunarity".to_string(), ParamValue::Float(2.0)),
            ("roughness".to_string(), ParamValue::Float(0.5)),
            ("amplitude".to_string(), ParamValue::Float(0.5)),
            ("frequency".to_string(), ParamValue::Float(1.0)),
            ("offset".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0])),
            ("seed".to_string(), ParamValue::Int(1)),
            ("flow_rotation".to_string(), ParamValue::Float(0.0)),
            ("distortion".to_string(), ParamValue::Float(0.0)),
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn param_specs() -> Vec<ParamSpec> {
    vec![
        ParamSpec::string("attr", "Attribute").with_help("Attribute name to write."),
        ParamSpec::int_enum(
            "domain",
            "Domain",
            vec![
                (0, "Point"),
                (1, "Vertex"),
                (2, "Primitive"),
                (3, "Detail"),
            ],
        )
        .with_help("Attribute domain to write."),
        ParamSpec::int_enum(
            "data_type",
            "Data Type",
            vec![(0, "Float"), (1, "Vec2"), (2, "Vec3")],
        )
        .with_help("Attribute data type."),
        ParamSpec::int_enum(
            "noise_type",
            "Noise Type",
            vec![
                (0, "Fast"),
                (1, "Sparse Convolution"),
                (2, "Alligator"),
                (3, "Perlin"),
                (4, "Perlin Flow"),
                (5, "Simplex"),
                (6, "Worley F1"),
                (7, "Worley F2-F1"),
                (8, "Manhattan F1"),
                (9, "Manhattan F2-F1"),
                (10, "Chebyshev F1"),
                (11, "Chebyshev F2-F1"),
                (12, "Perlin Cloud"),
                (13, "Simplex Cloud"),
            ],
        )
        .with_help("Noise basis (Fast/Perlin/Simplex/Worley/etc)."),
        ParamSpec::int_enum(
            "fractal_type",
            "Fractal Type",
            vec![(0, "None"), (1, "Standard"), (2, "Terrain"), (3, "Hybrid")],
        )
        .with_help("Fractal mode (None/Standard/Terrain/Hybrid)."),
        ParamSpec::int_slider("octaves", "Octaves", 1, 8)
            .with_help("Number of fractal octaves."),
        ParamSpec::float_slider("lacunarity", "Lacunarity", 1.0, 4.0)
            .with_help("Frequency multiplier per octave."),
        ParamSpec::float_slider("roughness", "Roughness", 0.0, 1.0)
            .with_help("Amplitude multiplier per octave."),
        ParamSpec::float_slider("amplitude", "Amplitude", -10.0, 10.0)
            .with_help("Noise amplitude."),
        ParamSpec::float_slider("frequency", "Frequency", 0.0, 10.0)
            .with_help("Noise frequency."),
        ParamSpec::vec3("offset", "Offset").with_help("Noise space offset."),
        ParamSpec::int_slider("seed", "Seed", 0, 100).with_help("Noise seed."),
        ParamSpec::float_slider("flow_rotation", "Flow Rotation", 0.0, 360.0)
            .with_help("Perlin Flow rotation (degrees)."),
        ParamSpec::float_slider("distortion", "Distortion", 0.0, 10.0)
            .with_help("Cloud noise distortion amount."),
        ParamSpec::string("group", "Group").with_help("Restrict to a group."),
        ParamSpec::int_enum(
            "group_type",
            "Group Type",
            vec![
                (0, "Auto"),
                (1, "Vertex"),
                (2, "Point"),
                (3, "Primitive"),
            ],
        )
        .with_help("Group domain to use."),
    ]
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut input = require_mesh_input(inputs, 0, "Attribute Noise requires a mesh input")?;
    apply_to_mesh(params, &mut input)?;
    Ok(input)
}

pub(crate) fn apply_to_splats(params: &NodeParams, splats: &mut SplatGeo) -> Result<(), String> {
    let attr = params.get_string("attr", "P");
    let domain = domain_from_params(params);
    let data_type = params.get_int("data_type", 2).clamp(0, 2);
    let noise_type = NoiseType::from_int(params.get_int("noise_type", 0));
    let fractal_type = FractalType::from_int(params.get_int("fractal_type", 1));
    let octaves = params.get_int("octaves", 3).max(1) as u32;
    let lacunarity = params.get_float("lacunarity", 2.0);
    let roughness = params.get_float("roughness", 0.5);
    let amplitude = params.get_float("amplitude", 0.5);
    let frequency = params.get_float("frequency", 1.0);
    let offset = Vec3::from(params.get_vec3("offset", [0.0, 0.0, 0.0]));
    let seed = params.get_int("seed", 1) as u32;
    let flow_rotation = params.get_float("flow_rotation", 0.0);
    let distortion = params.get_float("distortion", 0.0);
    let fractal = FractalSettings {
        octaves,
        lacunarity,
        roughness,
    };

    let count = splats.attribute_domain_len(domain);
    if count == 0 && domain != AttributeDomain::Detail {
        return Ok(());
    }

    let mask = splat_group_mask(splats, params, domain);
    let mask_ref = mask.as_deref();
    if !mask_has_any(mask.as_deref()) {
        return Ok(());
    }

    match data_type {
        0 => {
            let mut values = existing_float_attr_splats(splats, domain, attr, count);
            parallel::for_each_indexed_mut(&mut values, |idx, value| {
                if mask_ref
                    .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
                {
                    return;
                }
                let p = splat_sample_position(splats, domain, idx) * frequency + offset;
                let n = fractal_noise(
                    p,
                    seed,
                    noise_type,
                    fractal_type,
                    fractal,
                    flow_rotation,
                    distortion,
                );
                *value += n * amplitude;
            });
            splats
                .set_attribute(domain, attr, AttributeStorage::Float(values))
                .map_err(|err| format!("Attribute Noise error: {:?}", err))?;
        }
        1 => {
            let mut values = existing_vec2_attr_splats(splats, domain, attr, count);
            let offsets = [Vec3::new(12.7, 45.3, 19.1), Vec3::new(31.9, 7.2, 58.4)];
            parallel::for_each_indexed_mut(&mut values, |idx, value| {
                if mask_ref
                    .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
                {
                    return;
                }
                let p = splat_sample_position(splats, domain, idx) * frequency + offset;
                let n0 = fractal_noise(
                    p + offsets[0],
                    seed,
                    noise_type,
                    fractal_type,
                    fractal,
                    flow_rotation,
                    distortion,
                );
                let n1 = fractal_noise(
                    p + offsets[1],
                    seed.wrapping_add(7),
                    noise_type,
                    fractal_type,
                    fractal,
                    flow_rotation,
                    distortion,
                );
                value[0] += n0 * amplitude;
                value[1] += n1 * amplitude;
            });
            splats
                .set_attribute(domain, attr, AttributeStorage::Vec2(values))
                .map_err(|err| format!("Attribute Noise error: {:?}", err))?;
        }
        _ => {
            let mut values = existing_vec3_attr_splats(splats, domain, attr, count);
            let offsets = [
                Vec3::new(12.7, 45.3, 19.1),
                Vec3::new(31.9, 7.2, 58.4),
                Vec3::new(23.1, 91.7, 3.7),
            ];
            parallel::for_each_indexed_mut(&mut values, |idx, value| {
                if mask_ref
                    .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
                {
                    return;
                }
                let p = splat_sample_position(splats, domain, idx) * frequency + offset;
                let n0 = fractal_noise(
                    p + offsets[0],
                    seed,
                    noise_type,
                    fractal_type,
                    fractal,
                    flow_rotation,
                    distortion,
                );
                let n1 = fractal_noise(
                    p + offsets[1],
                    seed.wrapping_add(7),
                    noise_type,
                    fractal_type,
                    fractal,
                    flow_rotation,
                    distortion,
                );
                let n2 = fractal_noise(
                    p + offsets[2],
                    seed.wrapping_add(13),
                    noise_type,
                    fractal_type,
                    fractal,
                    flow_rotation,
                    distortion,
                );
                value[0] += n0 * amplitude;
                value[1] += n1 * amplitude;
                value[2] += n2 * amplitude;
            });
            splats
                .set_attribute(domain, attr, AttributeStorage::Vec3(values))
                .map_err(|err| format!("Attribute Noise error: {:?}", err))?;
        }
    }

    Ok(())
}

fn apply_to_mesh(params: &NodeParams, mesh: &mut Mesh) -> Result<(), String> {
    let attr = params.get_string("attr", "P");
    let domain = domain_from_params(params);
    let data_type = params.get_int("data_type", 2).clamp(0, 2);
    let noise_type = NoiseType::from_int(params.get_int("noise_type", 0));
    let fractal_type = FractalType::from_int(params.get_int("fractal_type", 1));
    let octaves = params.get_int("octaves", 3).max(1) as u32;
    let lacunarity = params.get_float("lacunarity", 2.0);
    let roughness = params.get_float("roughness", 0.5);
    let amplitude = params.get_float("amplitude", 0.5);
    let frequency = params.get_float("frequency", 1.0);
    let offset = Vec3::from(params.get_vec3("offset", [0.0, 0.0, 0.0]));
    let seed = params.get_int("seed", 1) as u32;
    let flow_rotation = params.get_float("flow_rotation", 0.0);
    let distortion = params.get_float("distortion", 0.0);
    let fractal = FractalSettings {
        octaves,
        lacunarity,
        roughness,
    };

    let count = mesh.attribute_domain_len(domain);
    if count == 0 && domain != AttributeDomain::Detail {
        return Ok(());
    }

    let mask = mesh_group_mask(mesh, params, domain);
    let mask_ref = mask.as_deref();
    if !mask_has_any(mask.as_deref()) {
        return Ok(());
    }

    match data_type {
        0 => {
            let mut values = existing_float_attr_mesh(mesh, domain, attr, count);
            parallel::for_each_indexed_mut(&mut values, |idx, value| {
                if mask_ref
                    .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
                {
                    return;
                }
                let p = mesh_sample_position(mesh, domain, idx) * frequency + offset;
                let n = fractal_noise(
                    p,
                    seed,
                    noise_type,
                    fractal_type,
                    fractal,
                    flow_rotation,
                    distortion,
                );
                *value += n * amplitude;
            });
            mesh.set_attribute(domain, attr, AttributeStorage::Float(values))
                .map_err(|err| format!("Attribute Noise error: {:?}", err))?;
        }
        1 => {
            let mut values = existing_vec2_attr_mesh(mesh, domain, attr, count);
            let offsets = [Vec3::new(12.7, 45.3, 19.1), Vec3::new(31.9, 7.2, 58.4)];
            parallel::for_each_indexed_mut(&mut values, |idx, value| {
                if mask_ref
                    .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
                {
                    return;
                }
                let p = mesh_sample_position(mesh, domain, idx) * frequency + offset;
                let n0 = fractal_noise(
                    p + offsets[0],
                    seed,
                    noise_type,
                    fractal_type,
                    fractal,
                    flow_rotation,
                    distortion,
                );
                let n1 = fractal_noise(
                    p + offsets[1],
                    seed.wrapping_add(7),
                    noise_type,
                    fractal_type,
                    fractal,
                    flow_rotation,
                    distortion,
                );
                value[0] += n0 * amplitude;
                value[1] += n1 * amplitude;
            });
            mesh.set_attribute(domain, attr, AttributeStorage::Vec2(values))
                .map_err(|err| format!("Attribute Noise error: {:?}", err))?;
        }
        _ => {
            let mut values = existing_vec3_attr_mesh(mesh, domain, attr, count);
            let offsets = [
                Vec3::new(12.7, 45.3, 19.1),
                Vec3::new(31.9, 7.2, 58.4),
                Vec3::new(23.1, 91.7, 3.7),
            ];
            parallel::for_each_indexed_mut(&mut values, |idx, value| {
                if mask_ref
                    .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
                {
                    return;
                }
                let p = mesh_sample_position(mesh, domain, idx) * frequency + offset;
                let n0 = fractal_noise(
                    p + offsets[0],
                    seed,
                    noise_type,
                    fractal_type,
                    fractal,
                    flow_rotation,
                    distortion,
                );
                let n1 = fractal_noise(
                    p + offsets[1],
                    seed.wrapping_add(7),
                    noise_type,
                    fractal_type,
                    fractal,
                    flow_rotation,
                    distortion,
                );
                let n2 = fractal_noise(
                    p + offsets[2],
                    seed.wrapping_add(13),
                    noise_type,
                    fractal_type,
                    fractal,
                    flow_rotation,
                    distortion,
                );
                value[0] += n0 * amplitude;
                value[1] += n1 * amplitude;
                value[2] += n2 * amplitude;
            });
            mesh.set_attribute(domain, attr, AttributeStorage::Vec3(values))
                .map_err(|err| format!("Attribute Noise error: {:?}", err))?;
        }
    }

    if attr == "P" && domain == AttributeDomain::Point && data_type == 2 {
        recompute_mesh_normals(mesh);
    }
    Ok(())
}
