use std::collections::BTreeMap;

use glam::{Quat, Vec3};

use crate::attributes::{AttributeDomain, AttributeStorage};
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    attribute_utils::{domain_from_params, existing_float_attr_mesh, existing_float_attr_splats},
    geometry_in,
    geometry_out,
    group_utils::{mesh_group_mask, splat_group_mask},
    require_mesh_input,
};
use crate::splat::SplatGeo;

pub const NAME: &str = "Attribute from Feature";

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
            ("feature".to_string(), ParamValue::Int(0)),
            ("attr".to_string(), ParamValue::String(String::new())),
            ("domain".to_string(), ParamValue::Int(0)),
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut input =
        require_mesh_input(inputs, 0, "Attribute from Feature requires a mesh input")?;
    apply_to_mesh(params, &mut input)?;
    Ok(input)
}

pub(crate) fn apply_to_splats(params: &NodeParams, splats: &mut SplatGeo) -> Result<(), String> {
    let feature = params.get_int("feature", 0).clamp(0, 1);
    let domain = domain_from_params(params);
    let attr_name = target_attr_name(params, feature);
    let count = splats.attribute_domain_len(domain);
    if count == 0 && domain != AttributeDomain::Detail {
        return Ok(());
    }
    let mask = splat_group_mask(splats, params, domain);
    if let Some(mask) = &mask {
        if !mask.iter().any(|value| *value) {
            return Ok(());
        }
    }

    match feature {
        0 => apply_area_splats(splats, domain, &attr_name, mask.as_deref())?,
        _ => apply_gradient_splats(splats, domain, &attr_name, mask.as_deref())?,
    }
    Ok(())
}

fn apply_to_mesh(params: &NodeParams, mesh: &mut Mesh) -> Result<(), String> {
    let feature = params.get_int("feature", 0).clamp(0, 1);
    let domain = domain_from_params(params);
    let attr_name = target_attr_name(params, feature);
    let count = mesh.attribute_domain_len(domain);
    if count == 0 && domain != AttributeDomain::Detail {
        return Ok(());
    }

    let mask = mesh_group_mask(mesh, params, domain);
    if let Some(mask) = &mask {
        if !mask.iter().any(|value| *value) {
            return Ok(());
        }
    }

    match feature {
        0 => apply_area_mesh(mesh, domain, &attr_name, mask.as_deref())?,
        _ => apply_gradient_mesh(mesh, domain, &attr_name, mask.as_deref())?,
    }
    Ok(())
}

fn target_attr_name(params: &NodeParams, feature: i32) -> String {
    let name = params.get_string("attr", "");
    if name.trim().is_empty() {
        return match feature {
            1 => "gradient".to_string(),
            _ => "area".to_string(),
        };
    }
    name.to_string()
}

fn apply_area_mesh(
    mesh: &mut Mesh,
    domain: AttributeDomain,
    attr: &str,
    mask: Option<&[bool]>,
) -> Result<(), String> {
    let areas = primitive_areas(mesh)?;
    let count = mesh.attribute_domain_len(domain);
    let mut values = existing_float_attr_mesh(mesh, domain, attr, count);

    match domain {
        AttributeDomain::Point => {
            let mut accum = vec![0.0f32; mesh.positions.len()];
            for (prim_index, tri) in mesh.indices.chunks_exact(3).enumerate() {
                let area = areas.get(prim_index).copied().unwrap_or(0.0);
                if area <= 0.0 {
                    continue;
                }
                let share = area / 3.0;
                for &idx in tri {
                    if let Some(slot) = accum.get_mut(idx as usize) {
                        *slot += share;
                    }
                }
            }
            for (idx, value) in accum.iter().enumerate() {
                if mask
                    .as_ref()
                    .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
                {
                    continue;
                }
                if let Some(slot) = values.get_mut(idx) {
                    *slot = *value;
                }
            }
        }
        AttributeDomain::Vertex => {
            for (prim_index, _tri) in mesh.indices.chunks_exact(3).enumerate() {
                let area = areas.get(prim_index).copied().unwrap_or(0.0);
                let share = area / 3.0;
                let base = prim_index * 3;
                for corner in 0..3 {
                    let idx = base + corner;
                    if mask
                        .as_ref()
                        .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
                    {
                        continue;
                    }
                    if let Some(slot) = values.get_mut(idx) {
                        *slot = share;
                    }
                }
            }
        }
        AttributeDomain::Primitive => {
            for (idx, value) in areas.iter().enumerate() {
                if mask
                    .as_ref()
                    .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
                {
                    continue;
                }
                if let Some(slot) = values.get_mut(idx) {
                    *slot = *value;
                }
            }
        }
        AttributeDomain::Detail => {
            let total: f32 = areas.iter().sum();
            if mask
                .as_ref()
                .is_some_and(|mask| !mask.first().copied().unwrap_or(false))
            {
                return Ok(());
            }
            if values.is_empty() {
                values.push(total);
            } else {
                values[0] = total;
            }
        }
    }

    mesh.set_attribute(domain, attr, AttributeStorage::Float(values))
        .map_err(|err| format!("Attribute from Feature error: {:?}", err))?;
    Ok(())
}

fn apply_gradient_mesh(
    mesh: &mut Mesh,
    domain: AttributeDomain,
    attr: &str,
    mask: Option<&[bool]>,
) -> Result<(), String> {
    let count = mesh.attribute_domain_len(domain);
    let mut values = existing_float_attr_mesh(mesh, domain, attr, count);

    match domain {
        AttributeDomain::Point => {
            if mesh.normals.is_none() && !mesh.compute_normals() {
                return Err("Attribute from Feature requires triangle mesh input".to_string());
            }
            let normals = mesh.normals.as_ref().unwrap();
            for (idx, normal) in normals.iter().enumerate() {
                if mask
                    .as_ref()
                    .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
                {
                    continue;
                }
                if let Some(slot) = values.get_mut(idx) {
                    *slot = gradient_from_normal(Vec3::from(*normal));
                }
            }
        }
        AttributeDomain::Vertex => {
            let normals = if let Some(normals) = &mesh.corner_normals {
                normals
            } else {
                let prim_normals = primitive_normals(mesh)?;
                let mut corner_normals = vec![[0.0, 1.0, 0.0]; mesh.indices.len()];
                for (prim_index, normal) in prim_normals.iter().enumerate() {
                    let base = prim_index * 3;
                    for corner in 0..3 {
                        if let Some(slot) = corner_normals.get_mut(base + corner) {
                            *slot = normal.to_array();
                        }
                    }
                }
                mesh.corner_normals = Some(corner_normals);
                mesh.corner_normals.as_ref().unwrap()
            };
            for (idx, normal) in normals.iter().enumerate() {
                if mask
                    .as_ref()
                    .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
                {
                    continue;
                }
                if let Some(slot) = values.get_mut(idx) {
                    *slot = gradient_from_normal(Vec3::from(*normal));
                }
            }
        }
        AttributeDomain::Primitive => {
            let normals = primitive_normals(mesh)?;
            for (idx, normal) in normals.iter().enumerate() {
                if mask
                    .as_ref()
                    .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
                {
                    continue;
                }
                if let Some(slot) = values.get_mut(idx) {
                    *slot = gradient_from_normal(*normal);
                }
            }
        }
        AttributeDomain::Detail => {
            let value = if mesh.indices.len().is_multiple_of(3) && !mesh.indices.is_empty() {
                let normals = primitive_normals(mesh)?;
                average_gradient(normals.iter().copied())
            } else if let Some(normals) = &mesh.normals {
                average_gradient(normals.iter().copied().map(Vec3::from))
            } else {
                0.0
            };
            if mask
                .as_ref()
                .is_some_and(|mask| !mask.first().copied().unwrap_or(false))
            {
                return Ok(());
            }
            if values.is_empty() {
                values.push(value);
            } else {
                values[0] = value;
            }
        }
    }

    mesh.set_attribute(domain, attr, AttributeStorage::Float(values))
        .map_err(|err| format!("Attribute from Feature error: {:?}", err))?;
    Ok(())
}

fn apply_area_splats(
    splats: &mut SplatGeo,
    domain: AttributeDomain,
    attr: &str,
    mask: Option<&[bool]>,
) -> Result<(), String> {
    let count = splats.attribute_domain_len(domain);
    let mut values = existing_float_attr_splats(splats, domain, attr, count);

    let use_log_scale = splats
        .scales
        .iter()
        .any(|value| value[0] < 0.0 || value[1] < 0.0 || value[2] < 0.0);
    let mut areas = Vec::with_capacity(splats.len());
    for scale in &splats.scales {
        let mut v = Vec3::from(*scale);
        if use_log_scale {
            v = Vec3::new(v.x.exp(), v.y.exp(), v.z.exp());
        }
        v = v.abs();
        let area = std::f32::consts::PI * v.x * v.y;
        areas.push(area);
    }

    match domain {
        AttributeDomain::Point | AttributeDomain::Primitive => {
            for (idx, value) in areas.iter().enumerate() {
                if mask
                    .as_ref()
                    .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
                {
                    continue;
                }
                if let Some(slot) = values.get_mut(idx) {
                    *slot = *value;
                }
            }
        }
        AttributeDomain::Detail => {
            let total: f32 = areas.iter().sum();
            if mask
                .as_ref()
                .is_some_and(|mask| !mask.first().copied().unwrap_or(false))
            {
                return Ok(());
            }
            if values.is_empty() {
                values.push(total);
            } else {
                values[0] = total;
            }
        }
        AttributeDomain::Vertex => {}
    }

    splats
        .set_attribute(domain, attr, AttributeStorage::Float(values))
        .map_err(|err| format!("Attribute from Feature error: {:?}", err))?;
    Ok(())
}

fn apply_gradient_splats(
    splats: &mut SplatGeo,
    domain: AttributeDomain,
    attr: &str,
    mask: Option<&[bool]>,
) -> Result<(), String> {
    let count = splats.attribute_domain_len(domain);
    let mut values = existing_float_attr_splats(splats, domain, attr, count);

    let normals = splat_normals(splats);

    match domain {
        AttributeDomain::Point | AttributeDomain::Primitive => {
            for (idx, normal) in normals.iter().enumerate() {
                if mask
                    .as_ref()
                    .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
                {
                    continue;
                }
                if let Some(slot) = values.get_mut(idx) {
                    *slot = gradient_from_normal(*normal);
                }
            }
        }
        AttributeDomain::Detail => {
            let value = average_gradient(normals.iter().copied());
            if mask
                .as_ref()
                .is_some_and(|mask| !mask.first().copied().unwrap_or(false))
            {
                return Ok(());
            }
            if values.is_empty() {
                values.push(value);
            } else {
                values[0] = value;
            }
        }
        AttributeDomain::Vertex => {}
    }

    splats
        .set_attribute(domain, attr, AttributeStorage::Float(values))
        .map_err(|err| format!("Attribute from Feature error: {:?}", err))?;
    Ok(())
}

fn primitive_areas(mesh: &Mesh) -> Result<Vec<f32>, String> {
    if !mesh.indices.len().is_multiple_of(3) {
        return Err("Attribute from Feature requires triangle mesh input".to_string());
    }
    let tri_count = mesh.indices.len() / 3;
    let mut areas = Vec::with_capacity(tri_count);
    for tri in mesh.indices.chunks_exact(3) {
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;
        if i0 >= mesh.positions.len() || i1 >= mesh.positions.len() || i2 >= mesh.positions.len()
        {
            return Err("Attribute from Feature has invalid indices".to_string());
        }
        let p0 = Vec3::from(mesh.positions[i0]);
        let p1 = Vec3::from(mesh.positions[i1]);
        let p2 = Vec3::from(mesh.positions[i2]);
        let area = 0.5 * (p1 - p0).cross(p2 - p0).length();
        areas.push(area.max(0.0));
    }
    Ok(areas)
}

fn primitive_normals(mesh: &Mesh) -> Result<Vec<Vec3>, String> {
    if !mesh.indices.len().is_multiple_of(3) {
        return Err("Attribute from Feature requires triangle mesh input".to_string());
    }
    let tri_count = mesh.indices.len() / 3;
    let mut normals = Vec::with_capacity(tri_count);
    for tri in mesh.indices.chunks_exact(3) {
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;
        if i0 >= mesh.positions.len() || i1 >= mesh.positions.len() || i2 >= mesh.positions.len()
        {
            return Err("Attribute from Feature has invalid indices".to_string());
        }
        let p0 = Vec3::from(mesh.positions[i0]);
        let p1 = Vec3::from(mesh.positions[i1]);
        let p2 = Vec3::from(mesh.positions[i2]);
        let normal = (p1 - p0).cross(p2 - p0);
        let normal = if normal.length_squared() > 0.0 {
            normal.normalize()
        } else {
            Vec3::Y
        };
        normals.push(normal);
    }
    Ok(normals)
}

fn splat_normals(splats: &SplatGeo) -> Vec<Vec3> {
    if let Some(crate::attributes::AttributeRef::Vec3(values)) =
        splats.attribute(AttributeDomain::Point, "N")
    {
        if values.len() == splats.len() {
            return values.iter().copied().map(Vec3::from).collect();
        }
    }
    splats
        .rotations
        .iter()
        .map(|rotation| {
            let mut quat =
                Quat::from_xyzw(rotation[1], rotation[2], rotation[3], rotation[0]);
            if quat.length_squared() > 0.0 {
                quat = quat.normalize();
            } else {
                quat = Quat::IDENTITY;
            }
            quat * Vec3::Y
        })
        .collect()
}

fn gradient_from_normal(normal: Vec3) -> f32 {
    let n = normal.normalize_or_zero();
    Vec3::new(n.x, 0.0, n.z).length()
}

fn average_gradient<I>(iter: I) -> f32
where
    I: IntoIterator<Item = Vec3>,
{
    let mut sum = 0.0f32;
    let mut count = 0u32;
    for normal in iter {
        sum += gradient_from_normal(normal);
        count += 1;
    }
    if count == 0 {
        0.0
    } else {
        sum / count as f32
    }
}
