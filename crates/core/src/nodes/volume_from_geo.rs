use std::collections::BTreeMap;

use glam::Vec3;

use crate::geometry::Geometry;
use crate::graph::{NodeDefinition, NodeParams, ParamValue};
use crate::nodes::{geometry_in, geometry_out};
use crate::volume::{Volume, VolumeKind};

pub const NAME: &str = "Volume from Geometry";

const DEFAULT_MAX_DIM: i32 = 32;
const DEFAULT_PADDING: f32 = 0.1;
const DEFAULT_DENSITY_SCALE: f32 = 1.0;
const DEFAULT_SDF_BAND: f32 = 0.2;
const MAX_GRID_POINTS: u64 = 32_000_000;

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
            ("mode".to_string(), ParamValue::String("density".to_string())),
            ("max_dim".to_string(), ParamValue::Int(DEFAULT_MAX_DIM)),
            ("padding".to_string(), ParamValue::Float(DEFAULT_PADDING)),
            (
                "density_scale".to_string(),
                ParamValue::Float(DEFAULT_DENSITY_SCALE),
            ),
            ("sdf_band".to_string(), ParamValue::Float(DEFAULT_SDF_BAND)),
        ]),
    }
}

pub fn apply_to_geometry(
    params: &NodeParams,
    inputs: &[Geometry],
) -> Result<Geometry, String> {
    let Some(input) = inputs.first() else {
        return Ok(Geometry::default());
    };

    let mode = params.get_string("mode", "density").to_lowercase();
    let kind = if mode.contains("sdf") {
        VolumeKind::Sdf
    } else {
        VolumeKind::Density
    };
    let max_dim = params.get_int("max_dim", DEFAULT_MAX_DIM).max(1) as u32;
    let padding = params.get_float("padding", DEFAULT_PADDING).max(0.0);
    let density_scale = params
        .get_float("density_scale", DEFAULT_DENSITY_SCALE)
        .max(0.0);

    let gathered = gather_geometry(input)?;
    let bounds_min = gathered.min;
    let bounds_max = gathered.max;
    let triangles = gathered.triangles;
    let splat_spheres = gathered.splat_spheres;
    let points = gathered.points;

    let mut min = bounds_min - Vec3::splat(padding);
    let mut max = bounds_max + Vec3::splat(padding);
    if (max - min).length_squared() < 1.0e-8 {
        max += Vec3::splat(1.0e-3);
        min -= Vec3::splat(1.0e-3);
    }

    let size = (max - min).max(Vec3::splat(1.0e-6));
    let max_axis = size.x.max(size.y.max(size.z)).max(1.0e-6);
    let voxel_size = (max_axis / max_dim as f32).max(1.0e-6);
    let dims = dims_from_size(size, voxel_size);
    let sdf_band = params
        .get_float("sdf_band", DEFAULT_SDF_BAND)
        .max(voxel_size * 2.0)
        .max(1.0e-6);

    let total = dims[0] as u64 * dims[1] as u64 * dims[2] as u64;
    if total == 0 || total > MAX_GRID_POINTS {
        return Err(format!(
            "Volume grid too large ({} voxels, max {})",
            total, MAX_GRID_POINTS
        ));
    }

    let mut values = vec![0.0f32; total as usize];
    let has_tris = !triangles.is_empty();
    let use_points = !has_tris;
    let mut idx = 0usize;
    for z in 0..dims[2] {
        let zf = min.z + (z as f32 + 0.5) * voxel_size;
        for y in 0..dims[1] {
            let yf = min.y + (y as f32 + 0.5) * voxel_size;
            for x in 0..dims[0] {
                let xf = min.x + (x as f32 + 0.5) * voxel_size;
                let pos = Vec3::new(xf, yf, zf);
                let mut unsigned_dist = f32::INFINITY;
                let mut signed_dist = f32::INFINITY;
                if has_tris {
                    for tri in &triangles {
                        let approx = pos.distance(tri.center) - tri.radius;
                        if approx > unsigned_dist {
                            continue;
                        }
                        let d = distance_to_triangle(pos, tri);
                        if d < unsigned_dist {
                            unsigned_dist = d;
                            if unsigned_dist <= 1.0e-6 {
                                break;
                            }
                        }
                    }
                }
                for (center, radius) in &splat_spheres {
                    let d = pos.distance(*center) - *radius;
                    if d.abs() < unsigned_dist {
                        unsigned_dist = d.abs();
                    }
                    if d < signed_dist {
                        signed_dist = d;
                    }
                }
                if use_points {
                    for point in &points {
                        let d = pos.distance(*point);
                        if d < unsigned_dist {
                            unsigned_dist = d;
                        }
                    }
                }
                if has_tris {
                    let inside = is_inside_mesh(pos, &triangles);
                    let signed_mesh = if inside { -unsigned_dist } else { unsigned_dist };
                    if signed_dist.is_infinite() {
                        signed_dist = signed_mesh;
                    } else if signed_mesh < signed_dist {
                        signed_dist = signed_mesh;
                    }
                }
                if !signed_dist.is_finite() {
                    signed_dist = unsigned_dist;
                }
                if !signed_dist.is_finite() {
                    signed_dist = 0.0;
                }

                let value = match kind {
                    VolumeKind::Density => {
                        let half = (voxel_size * 0.5).max(1.0e-6);
                        let t = ((half - signed_dist) / (2.0 * half)).clamp(0.0, 1.0);
                        let smooth = t * t * (3.0 - 2.0 * t);
                        smooth * density_scale
                    }
                    VolumeKind::Sdf => {
                        signed_dist
                    }
                };
                values[idx] = value;
                idx += 1;
            }
        }
    }

    let mut volume = Volume::new(kind, min.to_array(), dims, voxel_size, values);
    volume.density_scale = density_scale;
    volume.sdf_band = sdf_band;

    Ok(Geometry::with_volume(volume))
}

struct Triangle {
    a: Vec3,
    b: Vec3,
    c: Vec3,
    center: Vec3,
    radius: f32,
}

struct GatheredGeometry {
    min: Vec3,
    max: Vec3,
    triangles: Vec<Triangle>,
    splat_spheres: Vec<(Vec3, f32)>,
    points: Vec<Vec3>,
}

fn gather_geometry(input: &Geometry) -> Result<GatheredGeometry, String> {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    let mut found = false;
    let mut triangles = Vec::new();
    let mut points = Vec::new();

    for mesh in &input.meshes {
        if let Some(bounds) = mesh.bounds() {
            min = min.min(Vec3::from(bounds.min));
            max = max.max(Vec3::from(bounds.max));
            found = true;
        } else if !mesh.positions.is_empty() {
            for pos in &mesh.positions {
                let v = Vec3::from(*pos);
                min = min.min(v);
                max = max.max(v);
            }
            found = true;
        }
        if mesh.indices.len() >= 3 {
            for tri in mesh.indices.chunks_exact(3) {
                let a = Vec3::from(mesh.positions[tri[0] as usize]);
                let b = Vec3::from(mesh.positions[tri[1] as usize]);
                let c = Vec3::from(mesh.positions[tri[2] as usize]);
                let center = (a + b + c) / 3.0;
                let radius = center
                    .distance(a)
                    .max(center.distance(b))
                    .max(center.distance(c));
                triangles.push(Triangle {
                    a,
                    b,
                    c,
                    center,
                    radius,
                });
            }
        } else {
            points.extend(mesh.positions.iter().copied().map(Vec3::from));
        }
    }

    let mut splat_spheres = Vec::new();
    for splat in &input.splats {
        for (idx, position) in splat.positions.iter().enumerate() {
            let center = Vec3::from(*position);
            let radius = splat_radius(splat.scales.get(idx).copied());
            min = min.min(center - Vec3::splat(radius));
            max = max.max(center + Vec3::splat(radius));
            found = true;
            splat_spheres.push((center, radius));
        }
    }

    if !found {
        return Err("Volume from Geometry requires input geometry".to_string());
    }

    if triangles.is_empty() && points.is_empty() && splat_spheres.is_empty() {
        return Err("Volume from Geometry found no points or surfaces".to_string());
    }

    Ok(GatheredGeometry {
        min,
        max,
        triangles,
        splat_spheres,
        points,
    })
}

fn dims_from_size(size: Vec3, voxel_size: f32) -> [u32; 3] {
    [
        (size.x / voxel_size).ceil().max(1.0) as u32,
        (size.y / voxel_size).ceil().max(1.0) as u32,
        (size.z / voxel_size).ceil().max(1.0) as u32,
    ]
}

fn distance_to_triangle(p: Vec3, tri: &Triangle) -> f32 {
    let (closest, _) = closest_point_on_triangle(p, tri.a, tri.b, tri.c);
    p.distance(closest)
}

fn is_inside_mesh(p: Vec3, triangles: &[Triangle]) -> bool {
    let dir = Vec3::X;
    let mut hits = 0usize;
    for tri in triangles {
        if let Some(t) = ray_triangle_intersect(p, dir, tri.a, tri.b, tri.c) {
            if t > 0.0 {
                hits += 1;
            }
        }
    }
    hits % 2 == 1
}

fn ray_triangle_intersect(
    origin: Vec3,
    dir: Vec3,
    a: Vec3,
    b: Vec3,
    c: Vec3,
) -> Option<f32> {
    let eps = 1.0e-6;
    let edge1 = b - a;
    let edge2 = c - a;
    let h = dir.cross(edge2);
    let det = edge1.dot(h);
    if det.abs() < eps {
        return None;
    }
    let inv_det = 1.0 / det;
    let s = origin - a;
    let u = s.dot(h) * inv_det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(edge1);
    let v = dir.dot(q) * inv_det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = edge2.dot(q) * inv_det;
    Some(t)
}

fn closest_point_on_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> (Vec3, [f32; 3]) {
    let ab = b - a;
    let ac = c - a;
    let area = ab.cross(ac).length_squared();
    if area <= 1.0e-12 {
        let mut best = a;
        let mut bary = [1.0, 0.0, 0.0];
        let mut best_dist = (p - a).length_squared();
        let dist_b = (p - b).length_squared();
        if dist_b < best_dist {
            best = b;
            bary = [0.0, 1.0, 0.0];
            best_dist = dist_b;
        }
        let dist_c = (p - c).length_squared();
        if dist_c < best_dist {
            best = c;
            bary = [0.0, 0.0, 1.0];
        }
        return (best, bary);
    }
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return (a, [1.0, 0.0, 0.0]);
    }

    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return (b, [0.0, 1.0, 0.0]);
    }

    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return (a + ab * v, [1.0 - v, v, 0.0]);
    }

    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return (c, [0.0, 0.0, 1.0]);
    }

    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return (a + ac * w, [1.0 - w, 0.0, w]);
    }

    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        let point = b + (c - b) * w;
        return (point, [0.0, 1.0 - w, w]);
    }

    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    let u = 1.0 - v - w;
    let point = a + ab * v + ac * w;
    (point, [u, v, w])
}

fn splat_radius(scale: Option<[f32; 3]>) -> f32 {
    let Some(scale) = scale else {
        return 1.0;
    };
    let clamped = Vec3::new(
        scale[0].clamp(-10.0, 10.0).exp(),
        scale[1].clamp(-10.0, 10.0).exp(),
        scale[2].clamp(-10.0, 10.0).exp(),
    );
    let radius = (clamped.x + clamped.y + clamped.z) / 3.0;
    radius.max(1.0e-4)
}
