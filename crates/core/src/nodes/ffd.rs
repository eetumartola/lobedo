use std::cmp::Ordering;
use std::collections::BTreeMap;

use glam::{Mat3, Vec3};

use crate::attributes::AttributeDomain;
use crate::curve::parse_curve_points;
use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::mesh::Mesh;
use crate::nodes::{
    geometry_in,
    geometry_out,
    group_utils::{mask_has_any, mesh_group_mask, splat_group_mask},
    recompute_mesh_normals,
    require_mesh_input,
};
use crate::parallel;
use crate::splat::SplatGeo;

pub const NAME: &str = "FFD";

const DEFAULT_RES: i32 = 2;
const DEFAULT_PADDING: f32 = 0.0;
const MIN_AXIS_SIZE: f32 = 1.0e-6;
const JACOBIAN_EPS_SCALE: f32 = 1.0e-3;

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        name: NAME.to_string(),
        category: "Operators".to_string(),
        inputs: vec![geometry_in("in"), geometry_in("lattice")],
        outputs: vec![geometry_out("out")],
    }
}

pub fn default_params() -> NodeParams {
    NodeParams {
        values: BTreeMap::from([
            ("group".to_string(), ParamValue::String(String::new())),
            ("group_type".to_string(), ParamValue::Int(0)),
            ("res_x".to_string(), ParamValue::Int(DEFAULT_RES)),
            ("res_y".to_string(), ParamValue::Int(DEFAULT_RES)),
            ("res_z".to_string(), ParamValue::Int(DEFAULT_RES)),
            ("lattice_points".to_string(), ParamValue::String(String::new())),
            ("use_input_bounds".to_string(), ParamValue::Bool(true)),
            ("center".to_string(), ParamValue::Vec3([0.0, 0.0, 0.0])),
            ("size".to_string(), ParamValue::Vec3([1.0, 1.0, 1.0])),
            ("padding".to_string(), ParamValue::Float(DEFAULT_PADDING)),
            ("extrapolate".to_string(), ParamValue::Bool(false)),
        ]),
    }
}

pub fn compute(params: &NodeParams, inputs: &[Mesh]) -> Result<Mesh, String> {
    let mut mesh = require_mesh_input(inputs, 0, "FFD requires a mesh input")?;
    let lattice_input = inputs.get(1);
    let lattice = build_lattice_from_mesh(params, &mesh, lattice_input)?;
    apply_to_mesh(params, &mut mesh, &lattice);
    Ok(mesh)
}

pub fn apply_to_geometry(
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    let Some(source) = inputs.first() else {
        return Ok(Geometry::default());
    };
    let lattice_geo = inputs.get(1);

    let merged_mesh = source.merged_mesh();
    let merged_splats = source.merged_splats();

    let lattice = build_lattice(
        params,
        merged_mesh.as_ref(),
        merged_splats.as_ref(),
        lattice_geo,
    )?;

    let mut meshes = Vec::new();
    if let Some(mut mesh) = merged_mesh {
        apply_to_mesh(params, &mut mesh, &lattice);
        meshes.push(mesh);
    }

    let mut splats = Vec::with_capacity(source.splats.len());
    for splat in &source.splats {
        let mut splat = splat.clone();
        apply_to_splats(params, &mut splat, &lattice);
        splats.push(splat);
    }

    let curves = if meshes.is_empty() {
        Vec::new()
    } else {
        source.curves.clone()
    };

    Ok(Geometry {
        meshes,
        splats,
        curves,
        volumes: source.volumes.clone(),
        materials: source.materials.clone(),
    })
}

fn apply_to_mesh(params: &NodeParams, mesh: &mut Mesh, lattice: &Lattice) {
    if mesh.positions.is_empty() {
        return;
    }
    let mask = mesh_group_mask(mesh, params, AttributeDomain::Point);
    if !mask_has_any(mask.as_deref()) {
        return;
    }
    let extrapolate = params.get_bool("extrapolate", false);
    let mask_ref = mask.as_deref();
    parallel::for_each_indexed_mut(&mut mesh.positions, |idx, slot| {
        if mask_ref
            .as_ref()
            .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
        {
            return;
        }
        let pos = Vec3::from(*slot);
        if !pos.is_finite() {
            return;
        }
        let deformed = lattice.eval_position(pos, extrapolate);
        *slot = deformed.to_array();
    });
    recompute_mesh_normals(mesh);
}

fn apply_to_splats(params: &NodeParams, splats: &mut SplatGeo, lattice: &Lattice) {
    if splats.positions.is_empty() {
        return;
    }
    let mask = splat_group_mask(splats, params, AttributeDomain::Point);
    if !mask_has_any(mask.as_deref()) {
        return;
    }
    let extrapolate = params.get_bool("extrapolate", false);
    let eps = lattice.jacobian_epsilon();
    let mut normals_storage = splats
        .attributes
        .remove(AttributeDomain::Point, "N");

    for idx in 0..splats.positions.len() {
        if mask
            .as_ref()
            .is_some_and(|mask| !mask.get(idx).copied().unwrap_or(false))
        {
            continue;
        }
        let pos = Vec3::from(splats.positions[idx]);
        if !pos.is_finite() {
            continue;
        }
        let (deformed, linear) = lattice.eval_with_jacobian(pos, extrapolate, eps);
        splats.positions[idx] = deformed.to_array();
        splats.apply_linear_deform(idx, linear);
        if let Some(crate::attributes::AttributeStorage::Vec3(normals)) =
            normals_storage.as_mut()
        {
            if let Some(slot) = normals.get_mut(idx) {
                *slot = transform_normal(*slot, linear);
            }
        }
    }
    if let Some(storage) = normals_storage {
        splats
            .attributes
            .map_mut(AttributeDomain::Point)
            .insert("N".to_string(), storage);
    }
}

fn transform_normal(normal: [f32; 3], linear: Mat3) -> [f32; 3] {
    let det = linear.determinant();
    if !det.is_finite() || det.abs() < 1.0e-8 {
        return normal;
    }
    let normal_matrix = linear.inverse().transpose();
    let v = normal_matrix * Vec3::from(normal);
    let len = v.length();
    if len > 0.0 {
        (v / len).to_array()
    } else {
        [0.0, 1.0, 0.0]
    }
}

fn build_lattice_from_mesh(
    params: &NodeParams,
    source: &Mesh,
    lattice_input: Option<&Mesh>,
) -> Result<Lattice, String> {
    let lattice_positions = lattice_input.map(|mesh| mesh.positions.clone());
    build_lattice_from_positions(
        params,
        Some(source),
        None,
        lattice_positions.as_deref(),
    )
}

fn build_lattice(
    params: &NodeParams,
    mesh: Option<&Mesh>,
    splats: Option<&SplatGeo>,
    lattice_geo: Option<&Geometry>,
) -> Result<Lattice, String> {
    let lattice_positions = lattice_geo.and_then(extract_lattice_positions);
    build_lattice_from_positions(
        params,
        mesh,
        splats,
        lattice_positions.as_deref(),
    )
}

fn build_lattice_from_positions(
    params: &NodeParams,
    mesh: Option<&Mesh>,
    splats: Option<&SplatGeo>,
    lattice_positions: Option<&[[f32; 3]]>,
) -> Result<Lattice, String> {
    let res_x = params.get_int("res_x", DEFAULT_RES).max(2) as usize;
    let res_y = params.get_int("res_y", DEFAULT_RES).max(2) as usize;
    let res_z = params.get_int("res_z", DEFAULT_RES).max(2) as usize;
    let total = res_x * res_y * res_z;

    let mut control = lattice_points_from_params(params, total);
    if control.is_none() {
        if let Some(positions) = lattice_positions {
            if !positions.is_empty() {
                if positions.len() != total {
                    return Err(format!(
                        "FFD expects {} lattice points but got {}",
                        total,
                        positions.len()
                    ));
                }
                let mut points: Vec<Vec3> =
                    positions.iter().map(|p| Vec3::from(*p)).collect();
                let (bounds_min, bounds_max) = lattice_bounds_from_params(
                    params,
                    mesh,
                    splats,
                    positions,
                )
                .unwrap_or((Vec3::ZERO, Vec3::ONE));
                sort_lattice_points(&mut points, bounds_min, bounds_max);
                control = Some(points);
            }
        }
    }

    let use_input_bounds = params.get_bool("use_input_bounds", true);
    let padding = params.get_float("padding", DEFAULT_PADDING).max(0.0);
    let mut bounds = if use_input_bounds {
        geometry_bounds(mesh, splats).or_else(|| lattice_positions.and_then(bounds_from_positions))
    } else {
        None
    };
    if bounds.is_none() {
        bounds = Some(bounds_from_params(params));
    }
    let (mut min, mut max) = bounds.unwrap_or((Vec3::ZERO, Vec3::ONE));
    if padding > 0.0 {
        let pad = Vec3::splat(padding);
        min -= pad;
        max += pad;
    }

    if control.is_none() {
        control = Some(default_lattice_points(res_x, res_y, res_z, min, max));
    }

    let control = control.unwrap_or_default();
    Ok(Lattice::new(res_x, res_y, res_z, min, max, control))
}

fn extract_lattice_positions(geo: &Geometry) -> Option<Vec<[f32; 3]>> {
    if let Some(mesh) = geo.merged_mesh() {
        if !mesh.positions.is_empty() {
            return Some(mesh.positions);
        }
    }
    if let Some(splats) = geo.merged_splats() {
        if !splats.positions.is_empty() {
            return Some(splats.positions);
        }
    }
    None
}

fn lattice_points_from_params(params: &NodeParams, total: usize) -> Option<Vec<Vec3>> {
    let raw = params.get_string("lattice_points", "");
    let points = parse_curve_points(raw);
    if points.is_empty() || points.len() != total {
        return None;
    }
    Some(points.into_iter().map(Vec3::from).collect())
}

fn lattice_bounds_from_params(
    params: &NodeParams,
    mesh: Option<&Mesh>,
    splats: Option<&SplatGeo>,
    lattice_positions: &[[f32; 3]],
) -> Option<(Vec3, Vec3)> {
    let use_input_bounds = params.get_bool("use_input_bounds", true);
    if use_input_bounds {
        geometry_bounds(mesh, splats)
    } else {
        bounds_from_positions(lattice_positions).or(Some(bounds_from_params(params)))
    }
}

fn geometry_bounds(mesh: Option<&Mesh>, splats: Option<&SplatGeo>) -> Option<(Vec3, Vec3)> {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    let mut any = false;
    if let Some(mesh) = mesh {
        for p in &mesh.positions {
            let v = Vec3::from(*p);
            if !v.is_finite() {
                continue;
            }
            min = min.min(v);
            max = max.max(v);
            any = true;
        }
    }
    if let Some(splats) = splats {
        for p in &splats.positions {
            let v = Vec3::from(*p);
            if !v.is_finite() {
                continue;
            }
            min = min.min(v);
            max = max.max(v);
            any = true;
        }
    }
    if any {
        Some((min, max))
    } else {
        None
    }
}

fn bounds_from_positions(positions: &[[f32; 3]]) -> Option<(Vec3, Vec3)> {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    let mut any = false;
    for p in positions {
        let v = Vec3::from(*p);
        if !v.is_finite() {
            continue;
        }
        min = min.min(v);
        max = max.max(v);
        any = true;
    }
    if any {
        Some((min, max))
    } else {
        None
    }
}

fn bounds_from_params(params: &NodeParams) -> (Vec3, Vec3) {
    let center = Vec3::from(params.get_vec3("center", [0.0, 0.0, 0.0]));
    let size = Vec3::from(params.get_vec3("size", [1.0, 1.0, 1.0])).abs();
    let half = size * 0.5;
    (center - half, center + half)
}

fn default_lattice_points(
    res_x: usize,
    res_y: usize,
    res_z: usize,
    min: Vec3,
    max: Vec3,
) -> Vec<Vec3> {
    let mut points = Vec::with_capacity(res_x * res_y * res_z);
    let size = (max - min).max(Vec3::splat(MIN_AXIS_SIZE));
    for z in 0..res_z {
        let tz = if res_z > 1 {
            z as f32 / (res_z - 1) as f32
        } else {
            0.0
        };
        let pz = min.z + size.z * tz;
        for y in 0..res_y {
            let ty = if res_y > 1 {
                y as f32 / (res_y - 1) as f32
            } else {
                0.0
            };
            let py = min.y + size.y * ty;
            for x in 0..res_x {
                let tx = if res_x > 1 {
                    x as f32 / (res_x - 1) as f32
                } else {
                    0.0
                };
                let px = min.x + size.x * tx;
                points.push(Vec3::new(px, py, pz));
            }
        }
    }
    points
}

fn sort_lattice_points(points: &mut [Vec3], min: Vec3, max: Vec3) {
    let size = (max - min).max(Vec3::splat(MIN_AXIS_SIZE));
    points.sort_by(|a, b| {
        let na = (*a - min) / size;
        let nb = (*b - min) / size;
        let ord = na
            .z
            .partial_cmp(&nb.z)
            .unwrap_or(Ordering::Equal);
        if ord != Ordering::Equal {
            return ord;
        }
        let ord = na
            .y
            .partial_cmp(&nb.y)
            .unwrap_or(Ordering::Equal);
        if ord != Ordering::Equal {
            return ord;
        }
        na.x
            .partial_cmp(&nb.x)
            .unwrap_or(Ordering::Equal)
    });
}

struct Lattice {
    res_x: usize,
    res_y: usize,
    res_z: usize,
    min: Vec3,
    size: Vec3,
    control: Vec<Vec3>,
    binom_x: Vec<f32>,
    binom_y: Vec<f32>,
    binom_z: Vec<f32>,
}

impl Lattice {
    fn new(
        res_x: usize,
        res_y: usize,
        res_z: usize,
        min: Vec3,
        max: Vec3,
        control: Vec<Vec3>,
    ) -> Self {
        let size = (max - min).max(Vec3::splat(MIN_AXIS_SIZE));
        Self {
            res_x,
            res_y,
            res_z,
            min,
            size,
            control,
            binom_x: binomial_coeffs(res_x),
            binom_y: binomial_coeffs(res_y),
            binom_z: binomial_coeffs(res_z),
        }
    }

    fn eval_position(&self, pos: Vec3, extrapolate: bool) -> Vec3 {
        let uvw = self.param_coords(pos, extrapolate);
        self.evaluate(uvw)
    }

    fn eval_with_jacobian(&self, pos: Vec3, extrapolate: bool, eps: f32) -> (Vec3, Mat3) {
        let step = if eps > 0.0 { eps } else { 1.0e-4 };
        let base = self.eval_position(pos, extrapolate);
        let dx = self.eval_position(pos + Vec3::X * step, extrapolate);
        let dy = self.eval_position(pos + Vec3::Y * step, extrapolate);
        let dz = self.eval_position(pos + Vec3::Z * step, extrapolate);
        let col_x = (dx - base) / step;
        let col_y = (dy - base) / step;
        let col_z = (dz - base) / step;
        (base, Mat3::from_cols(col_x, col_y, col_z))
    }

    fn jacobian_epsilon(&self) -> f32 {
        let max_dim = self
            .size
            .x
            .abs()
            .max(self.size.y.abs())
            .max(self.size.z.abs());
        (max_dim * JACOBIAN_EPS_SCALE).max(1.0e-4)
    }

    fn param_coords(&self, pos: Vec3, extrapolate: bool) -> Vec3 {
        let mut u = if self.size.x.abs() > MIN_AXIS_SIZE {
            (pos.x - self.min.x) / self.size.x
        } else {
            0.5
        };
        let mut v = if self.size.y.abs() > MIN_AXIS_SIZE {
            (pos.y - self.min.y) / self.size.y
        } else {
            0.5
        };
        let mut w = if self.size.z.abs() > MIN_AXIS_SIZE {
            (pos.z - self.min.z) / self.size.z
        } else {
            0.5
        };
        if !extrapolate {
            u = u.clamp(0.0, 1.0);
            v = v.clamp(0.0, 1.0);
            w = w.clamp(0.0, 1.0);
        }
        Vec3::new(u, v, w)
    }

    fn evaluate(&self, uvw: Vec3) -> Vec3 {
        let bx = bernstein_weights(&self.binom_x, uvw.x);
        let by = bernstein_weights(&self.binom_y, uvw.y);
        let bz = bernstein_weights(&self.binom_z, uvw.z);
        let mut out = Vec3::ZERO;
        let mut idx = 0usize;
        for z in 0..self.res_z {
            let wz = bz.get(z).copied().unwrap_or(0.0);
            for y in 0..self.res_y {
                let wy = by.get(y).copied().unwrap_or(0.0);
                let wzy = wz * wy;
                for x in 0..self.res_x {
                    let wx = bx.get(x).copied().unwrap_or(0.0);
                    if let Some(cp) = self.control.get(idx) {
                        out += *cp * (wzy * wx);
                    }
                    idx += 1;
                }
            }
        }
        out
    }
}

fn binomial_coeffs(count: usize) -> Vec<f32> {
    if count <= 1 {
        return vec![1.0];
    }
    let n = (count - 1) as f32;
    let mut coeffs = vec![1.0f32; count];
    for i in 1..count - 1 {
        coeffs[i] = coeffs[i - 1] * (n - (i as f32) + 1.0) / (i as f32);
    }
    coeffs
}

fn bernstein_weights(coeffs: &[f32], t: f32) -> Vec<f32> {
    let count = coeffs.len();
    if count <= 1 {
        return vec![1.0];
    }
    let inv = 1.0 - t;
    let mut t_pows = vec![1.0f32; count];
    let mut inv_pows = vec![1.0f32; count];
    for i in 1..count {
        t_pows[i] = t_pows[i - 1] * t;
        inv_pows[i] = inv_pows[i - 1] * inv;
    }
    let deg = count - 1;
    let mut weights = vec![0.0f32; count];
    for i in 0..count {
        let inv_idx = deg - i;
        weights[i] = coeffs[i] * t_pows[i] * inv_pows[inv_idx];
    }
    weights
}
