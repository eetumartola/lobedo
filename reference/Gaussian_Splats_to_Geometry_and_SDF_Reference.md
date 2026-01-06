# Gaussian Splats → Geometry & SDF
## Implementation reference for procedural 3D applications (Rust / WebAssembly)

**Version:** 1.0  
**Date:** 2026-01-06  

This document is a **developer-oriented reference** for converting a 3D Gaussian Splatting (3DGS) model into:

- a **polygonal surface mesh** (watertight or open), and/or  
- a **signed distance field (SDF)** volume (dense grid, chunked grid, or on-demand queries)

It focuses on **methods, algorithms, math, and code** suitable for an interactive “procedural node” workflow (seconds, not overnight) and for implementations that can be written **from scratch in Rust** and cross-compiled to **WebAssembly**.

> Notes:
> - Gaussian splats are optimized for *photometric* quality; geometry extraction must compensate for sparse/irregular density.
> - “Best” approach depends heavily on whether you have camera poses, whether splats are surface-aligned, and whether you need watertight output.

---

## 0. Quick decision guide

### Inputs you might have

| You have… | You can do… | Recommended pipeline |
|---|---|---|
| Only splats (μ, q, s, α, maybe SH) | Mesh/SDF from density or from oriented points | **Density grid → SDF → Marching Cubes** (baseline) + optional point-based enhancements |
| Splats + reliable surface normals (or surface-aligned splats) | Higher quality mesh | **Oriented points → (Screened) Poisson** (desktop) or **MLS / implicit blend → MC** (WASM-friendly) |
| Splats + training camera poses | Best geometry consistency | **Depth/median-depth render → TSDF → Mesh** |
| Need unbounded scene extraction | Adaptive topology | **Adaptive tetrahedral extraction with binary-search level sets** |

### Output intent

| Output intent | Prefer |
|---|---|
| **Watertight mesh** | TSDF → MC, or Poisson, or occupancy-based MC |
| **Open surfaces preserved** | Point-based meshing; TSDF with “visibility” constraints; or post-trim (remove low-support regions) |
| **SDF for booleans / remeshing** | Mesh → SDF (robust sign), or occupancy → SDF if closed |
| **Fast interactive previews** | Low-res chunked SDF + MC, then refine regionally |

---

## 1. Data model and math primitives

A Gaussian splat `g` is defined by:

- mean / center: \(\mu \in \mathbb{R}^3\)
- orientation: quaternion \(q\) (unit, WXYZ)
- axis scales (standard deviations): \(\sigma = (\sigma_x, \sigma_y, \sigma_z)\)  
  (often stored in log-space)
- opacity/weight: \(\alpha \in [0,1]\)

A convenient factorization is:

\[
\Sigma = R\,\mathrm{diag}(\sigma_x^2,\sigma_y^2,\sigma_z^2)\,R^T
\]

with \(R\in SO(3)\) from quaternion \(q\).

### 1.1 Gaussian value and Mahalanobis distance

For a query point \(x\):

\[
G(x) = \alpha\,\exp\Big(-\tfrac{1}{2}(x-\mu)^T\Sigma^{-1}(x-\mu)\Big)
\]

Define the squared Mahalanobis distance:

\[
m^2(x) = (x-\mu)^T\Sigma^{-1}(x-\mu)
\]

#### Fast evaluation without building \(\Sigma^{-1}\)

Because \(\Sigma = R D R^T\) with \(D=\mathrm{diag}(\sigma^2)\), we have:

\[
\Sigma^{-1} = R\,\mathrm{diag}(\sigma_x^{-2},\sigma_y^{-2},\sigma_z^{-2})\,R^T
\]

Let \(u = R^T (x-\mu)\). Then:

\[
m^2 = (u_x/\sigma_x)^2 + (u_y/\sigma_y)^2 + (u_z/\sigma_z)^2
\]

This is the preferred inner-loop formula.

### 1.2 Density field from a splat set

Given splats \(\{g_i\}\), define a scalar “density” field:

\[
D(x) = \sum_i G_i(x)
\]

This field is used for:

- iso-surface extraction (mesh) via \(D(x)=\lambda\)
- occupancy masking \(D(x)>\lambda\)
- gradients (normals), and sometimes approximate distance (surface-aligned case)

### 1.3 Analytic gradient (for normals and refinement)

For a single Gaussian:

\[
\nabla G(x) = -G(x)\,\Sigma^{-1}(x-\mu)
\]

Therefore:

\[
\nabla D(x) = \sum_i \nabla G_i(x)
\]

A normal estimate at point \(x\) is:

\[
n(x) = -\frac{\nabla D(x)}{\|\nabla D(x)\|+\varepsilon}
\]

This is generally better than finite differences if you can afford summing candidate Gaussians.

---

## 2. Core acceleration strategy: “3σ cutoff” + spatial index

### 2.1 Support truncation

A Gaussian’s contribution drops rapidly. Use an \(n_\sigma\) cutoff:

- if \(m^2 > n_\sigma^2\), skip evaluation  
- common: \(n_\sigma = 3\) (≈1% of peak), sometimes 4 for smoother fields

Also clamp the exponent for stability:

- clamp \(m^2\) to \([0, m^2_{max}]\) where \(m^2_{max}\approx 50\) avoids `exp()` underflow issues.

### 2.2 Spatial hash grid (recommended baseline)

Use a uniform 3D hash grid mapping cell → list of splat indices.

- choose cell size `h` such that each cell contains a small number of Gaussians  
  (often `h ≈ median(3σ_max)`; see below)
- insert each Gaussian into all cells overlapped by its **AABB** at cutoff \(n_\sigma\)

#### Axis-aligned extent from covariance

A conservative per-axis cutoff radius is:

\[
r_k = n_\sigma\,\sqrt{\Sigma_{kk}} \quad (k\in\{x,y,z\})
\]

This is cheap (requires \(\Sigma\) or its diagonal). It is conservative even when the Gaussian is rotated.

### 2.3 Two evaluation patterns

**(A) Gather (voxel-centric):** for each voxel, query hash cell → evaluate candidates  
**(B) Scatter (splat-centric):** for each Gaussian, iterate voxels in its AABB and add contribution

For building a dense grid, **scatter is usually faster** because most voxels are empty and each Gaussian touches a limited region.

---

## 3. Pipeline A — Dense density grid → SDF → Mesh (CPU-friendly, WASM-friendly)

This is the most implementable “from scratch” approach and works without camera data.

### 3.1 Choose grid bounds and resolution

User-defined controls:

- **voxel size** `dx` (preferred), or
- **resolution** `(Nx, Ny, Nz)` over a bounding box.

Compute an AABB over splat centers, then expand by a margin:

\[
\text{bounds} = [\min_i \mu_i - m,\; \max_i \mu_i + m]
\]

A simple margin choice:

- `m = n_sigma * max_sigma` where `max_sigma` is max over splats of max axis scale.

### 3.2 Build density grid (scatter)

Algorithm outline:

1. allocate `density[Nx*Ny*Nz] = 0` (float32 is fine; float64 for accumulation if needed)
2. for each splat:
   - compute cutoff AABB in voxel coordinates
   - for each voxel in AABB:
     - compute `m2` and add `α * exp(-0.5*m2)` if within cutoff

#### Rust-like code skeleton (scatter)

```rust
#[derive(Clone, Copy)]
struct Splat {
    mu: [f32; 3],
    // rotation matrix columns (world axes in local frame) or store quat and compute R^T*(x-mu) fast
    r_t: [[f32; 3]; 3],     // R^T
    sigma: [f32; 3],        // stddev along local axes
    alpha: f32,
}

#[inline]
fn eval_m2(s: &Splat, x: [f32; 3]) -> f32 {
    let dx = [x[0]-s.mu[0], x[1]-s.mu[1], x[2]-s.mu[2]];
    // u = R^T * dx
    let u = [
        s.r_t[0][0]*dx[0] + s.r_t[0][1]*dx[1] + s.r_t[0][2]*dx[2],
        s.r_t[1][0]*dx[0] + s.r_t[1][1]*dx[1] + s.r_t[1][2]*dx[2],
        s.r_t[2][0]*dx[0] + s.r_t[2][1]*dx[1] + s.r_t[2][2]*dx[2],
    ];
    let sx = u[0] / s.sigma[0];
    let sy = u[1] / s.sigma[1];
    let sz = u[2] / s.sigma[2];
    sx*sx + sy*sy + sz*sz
}

fn rasterize_density(
    splats: &[Splat],
    bounds_min: [f32; 3],
    bounds_max: [f32; 3],
    nx: usize, ny: usize, nz: usize,
    n_sigma: f32,
) -> Vec<f32> {
    let mut grid = vec![0.0f32; nx*ny*nz];
    let dx = [
        (bounds_max[0]-bounds_min[0]) / nx as f32,
        (bounds_max[1]-bounds_min[1]) / ny as f32,
        (bounds_max[2]-bounds_min[2]) / nz as f32,
    ];

    for s in splats {
        // Conservative per-axis radius: r_k = n_sigma * sqrt(Sigma_kk)
        // If you don't store Sigma_kk, approximate using max axis sigma:
        let r = n_sigma * s.sigma[0].max(s.sigma[1]).max(s.sigma[2]);

        let min = [
            ((s.mu[0]-r - bounds_min[0]) / dx[0]).floor() as isize,
            ((s.mu[1]-r - bounds_min[1]) / dx[1]).floor() as isize,
            ((s.mu[2]-r - bounds_min[2]) / dx[2]).floor() as isize,
        ];
        let max = [
            ((s.mu[0]+r - bounds_min[0]) / dx[0]).ceil() as isize,
            ((s.mu[1]+r - bounds_min[1]) / dx[1]).ceil() as isize,
            ((s.mu[2]+r - bounds_min[2]) / dx[2]).ceil() as isize,
        ];

        let ix0 = min[0].clamp(0, nx as isize - 1) as usize;
        let iy0 = min[1].clamp(0, ny as isize - 1) as usize;
        let iz0 = min[2].clamp(0, nz as isize - 1) as usize;
        let ix1 = max[0].clamp(0, nx as isize - 1) as usize;
        let iy1 = max[1].clamp(0, ny as isize - 1) as usize;
        let iz1 = max[2].clamp(0, nz as isize - 1) as usize;

        for iz in iz0..=iz1 {
            let z = bounds_min[2] + (iz as f32 + 0.5) * dx[2];
            for iy in iy0..=iy1 {
                let y = bounds_min[1] + (iy as f32 + 0.5) * dx[1];
                for ix in ix0..=ix1 {
                    let x = bounds_min[0] + (ix as f32 + 0.5) * dx[0];

                    let m2 = eval_m2(s, [x,y,z]);
                    if m2 > n_sigma*n_sigma { continue; }
                    let m2 = m2.min(50.0);
                    let w = (-(0.5*m2) as f32).exp(); // f32 exp
                    let idx = ix + nx*(iy + ny*iz);
                    grid[idx] += s.alpha * w;
                }
            }
        }
    }
    grid
}
```

> Performance notes:
> - Use `rayon` to parallelize over splats **and** accumulate into per-chunk grids (avoid atomics).
> - For WebAssembly (no threads), chunk the volume (e.g. 64³ bricks) and process sequentially with progress callbacks.

### 3.3 Density → occupancy mask

Pick an iso threshold \(\lambda\) (see threshold section below). Define:

\[
B(x) = [D(x) > \lambda]
\]

### 3.4 Occupancy → signed distance (SDF)

A robust production approach is:

\[
\text{SDF}(x) = d_{outside}(x) - d_{inside}(x)
\]

where `d_inside` is the distance to the nearest **inside voxel**, and `d_outside` is distance to nearest **outside voxel**.
This can be computed using two Euclidean distance transforms (EDT):

- `d_inside = EDT(B)`
- `d_outside = EDT(!B)`

Then:

- inside points (B=true) yield negative values because `d_outside` is small and `d_inside` is zero
- outside points yield positive values

#### Implementing EDT (practical options)

1. **Exact squared EDT in O(N)** per dimension (Felzenszwalb & Huttenlocher style).  
   This is the best “from scratch” CPU option.

2. **Fast marching method (FMM)** (solves \(|\nabla d|=1\)). Good but more complex.

3. **Approximate chamfer distance** (very simple), then optional smoothing.

For interactive use: implement (1) once and reuse everywhere.

---

## 4. Pipeline B — Oriented points → implicit surface → mesh (better quality without cameras)

If splats are near surfaces, you can treat splats as a noisy “oriented surfel cloud”.

### 4.1 Get point + normal per splat

Two common normal estimators:

**(A) Covariance normal:** smallest-eigenvector of \(\Sigma\)  
- cheap if you already have eigen decomposition or principal axes
- assumes splats are surface-aligned

**(B) Density-gradient normal:** \(n(x) = -\nabla D(x)/\|\nabla D(x)\|\)  
- more correct, but requires neighbor queries + accumulation

### 4.2 Build an implicit function from oriented points

You need an implicit surface \(F(x)=0\) that you can sample on a grid.

Two WASM-friendly choices:

#### (1) Smooth-min of ellipsoid “distances” (cheap, robust)

Define a per-splat ellipsoid level-set (Mahalanobis “radius”):

\[
d_i(x) = \sqrt{(x-\mu_i)^T\Sigma_i^{-1}(x-\mu_i)} - r_0
\]

where \(r_0\) is a chosen shell radius (often 1–2).

Blend with smooth-min:

\[
d_{blend}(x) = -k\,\log\sum_i \exp\big(-d_i(x)/k\big)
\]

- `k` controls blend sharpness (smaller = closer to hard min)

Then mesh `d_blend(x)=0` with marching cubes.
This tends to give a “union-of-ellipsoids” surface; it is not always the true object surface but is controllable and stable.

#### (2) MLS (moving least squares) signed distance (higher quality)

Use kNN points around x and fit a plane (or quadratic) using normals, then estimate signed distance to that local plane. More complex but can preserve open surfaces if sign is handled carefully.

---

## 5. Pipeline C — Depth / median-depth render → TSDF → mesh (best when cameras exist)

If you have camera poses from training/capture:

1. render a depth-like surface from each view:
   - “median depth”: depth where accumulated opacity reaches 0.5 is often robust
2. integrate into a TSDF volume:
   - update voxels along rays with truncated signed distance
3. mesh TSDF with marching cubes

This tends to produce **watertight** surfaces and clean geometry, because TSDF fusion aggregates multi-view constraints.

Implementation note: fully replicating high-quality splat rendering in CPU is non-trivial; a simplified ray-marcher through candidate Gaussians can work for interactive.

---

## 6. Adaptive tetrahedral extraction (advanced, unbounded scenes)

Uniform grids struggle when density is near-zero almost everywhere. An alternative is to build an **adaptive tetrahedral grid** from Gaussian centers and extract a level set via **marching tetrahedra**.

Key improvement: for nonlinear opacity/density fields, use **binary search on edges** to find the threshold crossing instead of linear interpolation.

High-level steps:

1. Build tetrahedralization of points (Gaussian centers + boundary points)
2. Evaluate field at vertices
3. For each edge that crosses the iso threshold:
   - binary search `t ∈ [0,1]` on `p(t)= (1-t)a + tb`
   - output intersection points
4. Triangulate per tetra configuration

This is powerful but depends on a robust 3D Delaunay tetrahedralization implementation.

---

## 7. Choosing thresholds and parameters

### 7.1 Iso threshold λ for density

Practical starting ranges:

- object-centric: λ ≈ 0.3–0.5
- indoor scenes: λ ≈ 0.4–0.5
- unbounded/outdoor: λ ≈ 0.5–0.6

**Adaptive heuristic (no cameras):**
- sample D(μ) for “confident” splats (α above cutoff)
- set λ to a low percentile (e.g. 5th percentile) to capture surface support while rejecting noise.

### 7.2 n_sigma cutoff

- default: `n_sigma = 3`
- if field is too discontinuous: try 4 (slower, smoother)
- if performance is too slow: try 2.5 (risk holes)

### 7.3 Resolution selection (interactive)

A useful UI pattern: user selects **voxel size** `dx`, not Nx³.

- choose dx relative to median Gaussian size:
  - `dx ≈ median(sigma_min)` for high detail
  - `dx ≈ 2–4× median(sigma_min)` for preview

### 7.4 Open vs closed output

- If you need a true SDF sign but geometry is open: prefer **mesh-mediated sign** or output **unsigned distance**.
- If you just need a “soft volume” for booleans: treat density threshold mask as “inside” (produces closed surfaces).

---

## 8. Post-processing: make output usable

### 8.1 Clean floaters / tiny components

Connected component filtering:
- compute triangle component areas
- remove components below threshold percentage of total area

### 8.2 Smoothing without shrink: Taubin smoothing

Alternate Laplacian steps with \(\lambda>0\) and \(\mu<0\) (prevents shrink):

- typical: `λ=0.5`, `μ=-0.53`, 10–30 iterations.

### 8.3 Mesh decimation

Quadric error metrics (QEM) is the standard; target 50–90% reduction for viewport-friendly meshes.

### 8.4 Recompute normals

- For triangle mesh: angle-weighted vertex normals
- For SDF-derived mesh: optionally use SDF gradient for consistent normals

---

## 9. Validation and debug views (high leverage in a new app)

Add debug outputs:

- slice views of density and SDF (XY/XZ/YZ)
- histogram of D values and automatic λ suggestion
- heatmap of anisotropy and sigma values
- mesh component stats (count, area, bbox)

Unit tests:

- gaussian eval sanity: `D(mu)` increases with alpha, decreases with sigma
- gradient check: numeric finite diff vs analytic gradient
- SDF sign check: boundary voxels near surface ~0

---

## 10. Practical build plan (recommended)

1. Implement **fast Gaussian evaluation** (Mahalanobis in local frame)
2. Implement **scatter density rasterization** to a user-defined grid
3. Implement **EDT-based SDF** and **marching cubes**
4. Add **hash grid** to accelerate density sampling / gradient normals
5. Add **post-processing** (component filtering, Taubin, decimation)
6. Add optional pipelines:
   - **smooth-min ellipsoid implicit** (fast union surface)
   - **TSDF fusion** (if cameras exist)
   - **adaptive tetrahedral** extraction (advanced)

---

## Appendix A — Notes on “why marching cubes fails” on raw 3DGS

In raw 3DGS, many Gaussians become extremely small (to capture texture), and the density field can be near zero almost everywhere except tiny neighborhoods. A uniform voxel grid then either:

- misses the narrow bands unless resolution is extremely high, or
- produces bumpy “ellipsoid bump” artifacts at high resolution

Workarounds:

- smooth the field (higher n_sigma, blur density grid)
- downsample points and use point-based surface reconstruction
- use adaptive grids/tetrahedral extraction with binary searched intersections

---

## Appendix B — Minimal spatial hash grid sketch (Rust)

```rust
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct CellKey(i32, i32, i32);

fn cell_key(p: [f32;3], origin: [f32;3], h: f32) -> CellKey {
    CellKey(
        ((p[0]-origin[0]) / h).floor() as i32,
        ((p[1]-origin[1]) / h).floor() as i32,
        ((p[2]-origin[2]) / h).floor() as i32,
    )
}

struct HashGrid {
    h: f32,
    origin: [f32;3],
    buckets: HashMap<CellKey, Vec<usize>>,
}

impl HashGrid {
    fn new(h: f32, origin: [f32;3]) -> Self {
        Self { h, origin, buckets: HashMap::new() }
    }

    fn insert_splat_aabb(&mut self, aabb_min: [f32;3], aabb_max: [f32;3], splat_idx: usize) {
        let k0 = cell_key(aabb_min, self.origin, self.h);
        let k1 = cell_key(aabb_max, self.origin, self.h);
        for kz in k0.2..=k1.2 {
            for ky in k0.1..=k1.1 {
                for kx in k0.0..=k1.0 {
                    self.buckets.entry(CellKey(kx,ky,kz)).or_default().push(splat_idx);
                }
            }
        }
    }

    fn query(&self, p: [f32;3]) -> impl Iterator<Item=&usize> {
        let k = cell_key(p, self.origin, self.h);
        self.buckets.get(&k).into_iter().flat_map(|v| v.iter())
    }
}
```

---

## Appendix C — “Mesh → SDF” (most robust sign)

If you already have a mesh, the most robust SDF generation is:

1. compute unsigned distance to mesh triangles
2. compute sign via winding number or ray parity
3. combine

This is usually the best route if you need a high-quality SDF for boolean ops.

---

## Appendix D — Glossary

- **Density field**: \(D(x)=\sum_i G_i(x)\)
- **Occupancy mask**: \(B(x)=[D(x)>\lambda]\)
- **SDF**: signed distance to surface (negative inside, positive outside)
- **TSDF**: truncated SDF accumulated from depth observations
- **EDT**: Euclidean distance transform (grid distance to nearest boundary)

