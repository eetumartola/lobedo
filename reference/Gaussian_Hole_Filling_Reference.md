# Gaussian Hole Filling for 3D Gaussian Splatting (3DGS)
## Implementation reference for a Rust-based procedural splat editing/processing app

This document describes **practical, implementation-ready methods** to “heal” **small unwanted holes** in standard 3D Gaussian Splatting (3DGS) models by **generating new splats**.

- **Target failure mode:** small disconnected gaps / cracks where a surface should be continuous (scan / training / densification failure).
- **Output:** **pure splats** (Gaussian primitives), no required mesh/SDF outputs (but intermediate mesh/SDF may be used).
- **Runtime goal:** seconds (interactive node), not real-time per-frame.
- **Design style:** Houdini-like procedural operator: deterministic, controllable, composable, debug-friendly.

---

## Table of contents
- [1. Core representation](#1-core-representation)
- [2. What is a “hole” in splat space?](#2-what-is-a-hole-in-splat-space)
- [3. Detection: finding hole regions and boundaries](#3-detection-finding-hole-regions-and-boundaries)
- [4. Choosing a filling strategy](#4-choosing-a-filling-strategy)
- [5. Strategy A: Local implicit surface (MLS/RBF) + Poisson sampling + resplat](#5-strategy-a-local-implicit-surface-mlsrbf--poisson-sampling--resplat)
- [6. Strategy B: Local voxel density → occupancy → (narrow-band) SDF → patch resplat](#6-strategy-b-local-voxel-density--occupancy--narrow-band-sdf--patch-resplat)
- [7. Strategy C: Mesh-based hole patch + triangle-to-splats](#7-strategy-c-mesh-based-hole-patch--triangle-to-splats)
- [8. Strategy D: Splat-native “edge grow” densification](#8-strategy-d-splat-native-edge-grow-densification)
- [9. Strategy E: Patch-match / copy–transform–paste (repetitive textures)](#9-strategy-e-patch-match--copytransformpaste-repetitive-textures)
- [10. Strategy F: Optional image-guided inpainting + depth unprojection](#10-strategy-f-optional-image-guided-inpainting--depth-unprojection)
- [11. How to generate new splats: geometry, covariance, opacity](#11-how-to-generate-new-splats-geometry-covariance-opacity)
- [12. How to assign appearance: color + SH coefficients](#12-how-to-assign-appearance-color--sh-coefficients)
- [13. Blending, pruning, validation](#13-blending-pruning-validation)
- [14. Node design: `HealHoles3DGS`](#14-node-design-healholes3dgs)
- [15. Rust implementation notes and library options](#15-rust-implementation-notes-and-library-options)
- [Appendix A. Log-Euclidean interpolation of covariance (SPD)](#appendix-a-log-euclidean-interpolation-of-covariance-spd)
- [Appendix B. Poisson disk sampling (Bridson) pseudocode](#appendix-b-poisson-disk-sampling-bridson-pseudocode)
- [Appendix C. Useful heuristics and defaults](#appendix-c-useful-heuristics-and-defaults)

---

## 1. Core representation

Assume standard 3DGS per-splat parameters:

- **Position:** `μ ∈ ℝ³`
- **Orientation:** `q` (unit quaternion) giving rotation `R(q)`
- **Scale / stddev:** `s = (sₓ,sᵧ,s_z)` (std deviations, often stored in log-space)
- **Opacity:** `α ∈ (0,1)`
- **Appearance:** RGB spherical harmonic coefficients per splat:
  ```math
  \mathbf{C}(\mathbf{d})=\sum_{\ell=0}^{L}\sum_{m=-\ell}^{\ell}\mathbf{k}_{\ell m}Y_{\ell m}(\mathbf{d})
  ```
  Typical `L=3` ⇒ 16 coefficients per channel.

Many operations below assume you can:
- query neighborhoods (kNN / radius)
- estimate normals
- evaluate or approximate “coverage” (density or transmittance)

---

## 2. What is a “hole” in splat space?

Unlike meshes (explicit topology), a hole in splats is a **coverage failure**:

### 2.1 Density field view (3D)
Define a scalar density/opacity proxy field (one common form):
```math
D(\mathbf{x})=\sum_i \alpha_i \exp\left(-\frac{1}{2}(\mathbf{x}-\mu_i)^T\Sigma_i^{-1}(\mathbf{x}-\mu_i)\right)
```
A “surface band” often corresponds to `D(x)` near a threshold `λ`.

A hole can be approximated as a region where:
- the surrounding surface exists (nearby boundary splats have consistent normals and density),
- but `D(x)` drops under the threshold (gap).

### 2.2 Rendering/transmittance view (2D)
In splat rendering, holes often show up as **high transmittance** `T` (rays pass through without accumulating enough opacity).

A practical definition per pixel:
- render accumulated transmittance `T(u,v)` after compositing
- “hole pixels” satisfy `T(u,v) > τ_T` (e.g. 0.3–0.7 depending on model)

This is **fast** and directly tied to what users see.

---

## 3. Detection: finding hole regions and boundaries

The best hole-filling node usually uses a **hybrid detector**:
- **2D evidence:** transmittance / alpha holes in renders
- **3D evidence:** local density + boundary statistics

### 3.1 Pre-clean (strongly recommended)
Before hole detection, remove obvious outliers (“floaters”) that can confuse density/boundary tests.

**Statistical outlier removal (SOR):**
1. For each splat, compute mean distance to k neighbors: `d_i`
2. Compute `μ_d`, `σ_d`
3. Mark outliers if `d_i > μ_d + β σ_d` (β ~ 1.0–2.0)

### 3.2 kNN density anomaly (3D)
For each splat center `μ_i`, compute `r_k(i)` = distance to k-th nearest neighbor.

Simple density proxy:
```math
\rho_i \approx r_k(i)
```
Boundary candidates often have **large** `r_k` relative to local median.

### 3.3 Boundary detection via eigenanalysis + angular gap
This is a robust way to find splats that lie on an “edge” (hole rim).

For each splat `i`:
1. Get neighborhood `N_k(i)` (positions only).
2. Compute covariance of neighbor positions:
   ```math
   C=\frac{1}{k}\sum_{j\in N_k(i)}(\mu_j-\bar{\mu})(\mu_j-\bar{\mu})^T
   ```
3. Eigen-decompose: `λ₁≥λ₂≥λ₃`, eigenvectors `v₁,v₂,v₃`.
   - `v₃` approximates local normal.
4. Project neighbor vectors onto tangent plane spanned by `v₁,v₂`.
5. Compute angles of projections and find the maximum angular gap `Δθ_max`.
6. If `Δθ_max > θ_thresh` (e.g. 90°–140°), mark as boundary candidate.

This distinguishes interior points (neighbors all around) from edge points.

### 3.4 2D hole mask + backprojection
If you have a renderer available in the app:
1. Render a set of views (existing camera set, or procedurally chosen orbit).
2. Compute a binary hole mask where `T(u,v) > τ_T`.
3. Backproject hole pixels to 3D rays and accumulate votes in space.
4. Convert vote volume into 3D hole regions; intersect with splat neighborhoods to identify boundary splats.

This reduces false positives: a region is a “hole” only if it is both:
- visually transparent in 2D
- geometrically under-dense in 3D

### 3.5 Grouping boundary splats into loops / components
Once you have boundary candidates, cluster them:

- Build adjacency graph where edges connect splats within radius `r_adj`
- Extract connected components
- For each component:
  - reject if it is too small (noise) or too large (likely open boundary)
  - optionally compute an ordered boundary loop using nearest-neighbor chaining in tangent-plane coordinates

For small-hole healing you want **smallish loops** (or narrow crack components).

---

## 4. Choosing a filling strategy

Use a method selector that chooses a strategy based on:
- hole diameter / area (small vs medium)
- local curvature (planar vs curved)
- whether the model has reliable normals / covariances
- availability of mesh/SDF utilities

**Recommended defaults for small holes:**
1. **Strategy A (MLS/RBF implicit + Poisson sampling + resplat)** for curved surfaces.
2. **Strategy B (local voxel SDF + patch resplat)** when you want robust closure and you already have voxel/SDF tooling.

For repetitive textures (grass/asphalt), **Strategy E** often looks best.

---

## 5. Strategy A: Local implicit surface (MLS/RBF) + Poisson sampling + resplat

This is a strong “seconds-scale” default: it avoids global meshing and only works in a local region around each hole.

### 5.1 Inputs
- Boundary splats `B` (hole rim)
- Support splats `S` (a ring around the rim, e.g. all splats within radius `R_support` of the rim)
- Optional: estimated normals for boundary/support

### 5.2 Local implicit surface via plane-blended MLS
A practical MLS implicit function that extrapolates a smooth surface:

For a query point `x` define:
```math
f(\mathbf{x})=\sum_{i\in S} w_i(\mathbf{x})\, (\mathbf{x}-\mu_i)\cdot \mathbf{n}_i
```
with Gaussian weights:
```math
w_i(\mathbf{x})=\exp\left(-\frac{\|\mathbf{x}-\mu_i\|^2}{2h^2}\right)
```
- `n_i` are oriented normals (see Section 11.1).
- `h` is a bandwidth ~ 2–4× local splat spacing.

Intuition: each neighbor contributes signed distance to its tangent plane; the weighted sum forms an implicit surface. The hole boundary lies on `f=0`.

**Normals from the implicit:** approximate
```math
\mathbf{n}(\mathbf{x}) = \frac{\nabla f(\mathbf{x})}{\|\nabla f(\mathbf{x})\|}
```
You can estimate ∇f numerically (finite differences) if you don’t want symbolic gradients.

### 5.3 Generate patch points (surface sampling)
We want points on `f(x)=0` spanning the hole.

A robust recipe:
1. Build a local 2D parameterization:
   - compute average normal `n̄` from boundary
   - define tangent basis `(t1,t2)` orthonormal to `n̄`
2. Project boundary points into 2D (u,v).
3. Triangulate the 2D polygon (ear clipping / constrained Delaunay).
4. Sample interior points in 2D with Poisson disk spacing.
5. Lift each sample to 3D:
   - start from planar point `x0 = c + u t1 + v t2`
   - **project to surface** by iterative root finding along `n̄`:
     ```math
     x_{k+1} = x_k - \eta f(x_k)\, n̄
     ```
     or do a few Newton-like steps with numerical gradient.

This yields 3D points on the extrapolated surface.

### 5.4 Convert patch points into new splats
For each sampled point `p`:
- compute normal `n(p)` (implicit gradient or neighbor average)
- choose covariance “pancake” aligned to the normal
- assign appearance by interpolation

See [Section 11](#11-how-to-generate-new-splats-geometry-covariance-opacity) and [Section 12](#12-how-to-assign-appearance-color--sh-coefficients).

### 5.5 Pros/cons
✅ No global grids required; works well on curved surfaces.  
✅ Very controllable for “small hole” scope.  
⚠ Requires normal orientation consistency (fix sign flips).  
⚠ If the surrounding surface is very non-smooth/noisy, MLS extrapolation can drift.

---

## 6. Strategy B: Local voxel density → occupancy → (narrow-band) SDF → patch resplat

This is your “more SDF-like” local closure. It’s robust, especially when holes are best seen as missing occupancy.

### 6.1 Local voxelization of density
Define a local AABB around the hole:
- center at hole boundary centroid `c`
- extent = boundary bbox expanded by margin `m`

Choose voxel size `dx` based on local splat spacing:
```math
dx \approx 0.5 \cdot \mathrm{median}(r_k)
```

Compute density per voxel center `x_v`:
```math
D(x_v)=\sum_{i \in S} \alpha_i \exp(-0.5 (x_v-\mu_i)^T \Sigma_i^{-1} (x_v-\mu_i))
```

To keep this fast:
- restrict sum to splats whose support intersects the brick
- approximate each splat’s influence radius as `R_i ≈ 3 * max(s_i)`

### 6.2 Occupancy and hole localization
Pick an iso threshold `λ` and define occupancy `B(x)= [D(x)>λ]`.

Now find holes in occupancy with one of:
- flood fill from outside → mark exterior empty → hole is empty connected to exterior through the surface
- morphological operations (closing) to detect small gaps
- connected components of empty voxels adjacent to surface

### 6.3 SDF computation (local)
Compute SDF from occupancy using two distance transforms:
- `d_in = EDT(B)`
- `d_out = EDT(!B)`
- `s = d_out - d_in` (scaled by `dx`)

You can restrict to a narrow band around `s=0` to keep memory/time down.

### 6.4 Close small holes
To fill small holes, you can:
- apply morphological closing to occupancy before SDF
- or compute SDF, then apply **smooth union** / smoothing operations to close thin gaps

A simple “close radius” in SDF space:
- `s_closed = s - r_close` then re-iso at 0 (equivalent to dilation in distance space)

### 6.5 Extract surface patch and resplat
Extract the surface in the local brick:
- marching cubes on `s_closed = 0`
- keep only triangles in a band around the hole area

Then sample points on that patch and generate splats (Section 11 + 12).

### 6.6 Pros/cons
✅ Very robust closure; easy to add “close radius” knob.  
✅ Compatible with SDF-based debugging and validation.  
⚠ Voxel work scales with resolution; keep region tight.  
⚠ Density evaluation can be the bottleneck without good spatial culling.

---

## 7. Strategy C: Mesh-based hole patch + triangle-to-splats

If you can reconstruct a local mesh from splat centers, mesh hole filling becomes straightforward.

### 7.1 Build a local mesh
Approaches:
- ball pivoting (good for dense, clean point clouds)
- Poisson surface reconstruction (smooth, watertight bias)

Because our goal is **small holes**, you can restrict to:
- splats in a local region around the hole boundary

### 7.2 Detect mesh holes and fill
Once you have a mesh:
- boundary edges are edges with only one adjacent triangle
- follow boundary edges to form loops
- fill each loop with a hole-filling method (fan triangulation, minimal surface, etc.)

### 7.3 Convert filled triangles to splats
Given a triangle with vertices `v0,v1,v2`:
- centroid:
  ```math
  \mu = (v0+v1+v2)/3
  ```
- normal:
  ```math
  n = \frac{(v1-v0)\times(v2-v0)}{\|(v1-v0)\times(v2-v0)\|}
  ```
- area:
  ```math
  A = \frac{1}{2}\|(v1-v0)\times(v2-v0)\|
  ```

Make a tangent frame `(u,v,n)` and choose scales:
- `σ_t = k_cov * sqrt(A)` (k_cov ~ 1.0–1.5)
- `σ_n = ε * σ_t` (ε ~ 0.01–0.1)

Generate one splat per triangle or multiple per triangle based on triangle size.

### 7.4 Pros/cons
✅ Reliable when meshing works; easy to reason about.  
✅ Mesh gives clean boundary loops and curvature estimates.  
⚠ Meshing can fail on noisy splats; may be heavier than MLS for tiny holes.  
⚠ Must be careful to avoid losing fine splat appearance (treat mesh as geometry guide only).

---

## 8. Strategy D: Splat-native edge grow densification

If holes are small cracks, you can often fill them without any mesh/grid by iteratively “growing” splats into the void.

### 8.1 Outline
1. Detect boundary splats `B` around the hole.
2. For each boundary splat `g`:
   - clone it `g'`
   - shift the clone inward by a step along an estimated “hole inward” direction
3. Slightly reduce opacity of clones initially.
4. Repeat for several rounds until coverage metrics improve.
5. Smooth/prune.

### 8.2 Inward direction estimation
For a boundary splat with normal `n`:
- determine which side is “missing” by analyzing neighbor angular gap:
  - in tangent plane, find the largest angular gap direction `t_gap`
- inward direction can be `d_in = normalize(t_gap)` (in tangent plane),
  or a blend of tangent and normal if holes are “through-surface” defects.

### 8.3 Covariance/SH update
- Keep covariance similar, but optionally interpolate covariance between nearby boundary splats across the gap.
- Copy SH DC; optionally damp higher bands (stability).

### 8.4 Pros/cons
✅ Very fast; stays entirely in splat space.  
✅ Great for thin cracks and small missing streaks.  
⚠ Can create “lumps” if you over-grow; needs pruning and density control.  
⚠ Harder to guarantee correct surface shape on curved regions vs MLS/SDF.

---

## 9. Strategy E: Patch-match / copy–transform–paste (repetitive textures)

For repetitive materials (road, grass, brick), copying an existing nearby splat patch into the hole can look better than purely geometric interpolation.

### 9.1 Steps
1. Compute a **context signature** for the hole boundary:
   - local density stats, normal histogram, DC color histogram, SH energy
2. Search nearby areas for a patch whose signature matches.
3. Compute a rigid transform aligning patch to hole rim (ICP on positions + normals).
4. Copy splats, apply transform to μ, q, Σ.
5. Rotate/adjust SH (at least DC and band-1; optionally damp higher bands).
6. Blend and prune overlaps.

### 9.2 Similarity metrics
You can keep it simple:
- compare mean/variance of DC color and normals
- compare local spacing
Or more advanced:
- compare distributions with transport-like metrics (heavier)

### 9.3 Pros/cons
✅ Best for repeated textures: preserves “texture frequency” and SH detail.  
✅ Avoids needing to infer unseen texture.  
⚠ Can copy wrong semantics if search radius is too wide.  
⚠ Needs careful blending and SH handling under rotation.

---

## 10. Strategy F: Optional image-guided inpainting + depth unprojection

This is optional and typically heavier than “seconds-scale” unless you already have GPU diffusion/depth models integrated.

Idea:
1. Render a view where the hole is visible (alpha/transmittance mask).
2. Inpaint RGB in 2D.
3. Predict/complete depth in the hole area.
4. Unproject filled pixels into 3D points.
5. Convert points to splats; optionally run a small multi-view refinement optimizing new splats only.

This can yield excellent texture completion, but for “small holes” you can usually avoid it by Strategy A/B/E.

---

## 11. How to generate new splats: geometry, covariance, opacity

### 11.1 Normal estimation (for new splats and for boundary)
If splats are surfel-like, a fast normal estimate uses the smallest axis of the Gaussian:

- find smallest scale axis in local frame
- rotate that axis by quaternion `q`

If you don’t trust splat scales/orientations, use neighbor PCA normal:
- compute covariance of neighbor positions
- normal is smallest eigenvector

**Sign disambiguation:** align normals to neighbor majority:
- if `dot(n_i, n_j)<0`, flip
- optionally orient outward using object centroid

### 11.2 Covariance construction for new splats
For a new patch point with normal `n`, build tangent frame `(t1,t2,n)` and choose:
- `σ_n` small (surface thickness)
- `σ_t` based on local spacing

Rule of thumb:
- `σ_t ≈ 0.6 * spacing`
- `σ_n ≈ 0.1 * σ_t` (or smaller for crisp surfaces)

Store as `(q,s)`:
- `q = quat_from_mat3([t1 t2 n])`
- `s = (σ_t, σ_t, σ_n)`

### 11.3 Covariance interpolation on SPD manifold (optional but nice)
When bridging between two sides of a crack, interpolating covariance with standard linear lerp can cause swelling or invalid matrices. Use Log-Euclidean interpolation:

```math
\Sigma(t)=\exp((1-t)\log\Sigma_A + t\log\Sigma_B)
```

See [Appendix A](#appendix-a-log-euclidean-interpolation-of-covariance-spd).

### 11.4 Opacity α initialization
Practical defaults:
- `α_new = median(α_neighbors)` or `max(α_neighbors)`
- apply boundary feathering (lower α near seam if needed)

If you model density with `D(x)`, you can adapt α so that `D(μ_new)` hits a target threshold `λ` on average.

---

## 12. How to assign appearance: color + SH coefficients

### 12.1 Minimal stable option: DC-only
For hole fills that must never look “sparkly” or inconsistent:
- set only SH DC coefficient (`l=0`) from neighbor DC average
- set `l>0` = 0

This produces a matte patch; often acceptable for tiny holes.

### 12.2 Interpolate full SH from neighbors
For each new splat position `p`:
1. find k nearest original splats in the support region
2. weight by Gaussian kernel:
   ```math
   w_i=\exp(-\|p-\mu_i\|^2/(2h^2))
   ```
3. for each SH coefficient index `c` and color channel:
   ```math
   k_c(p)=\frac{\sum_i w_i k_{c,i}}{\sum_i w_i}
   ```

### 12.3 SH rotation for copied/rotated patches
If you copy splats and rotate them, their view-dependent SH must rotate too (at least band-1). For band-1 (l=1), coefficients transform like a vector in the chosen basis. Higher bands require proper SH rotation (Wigner-D) or a numerical sampling approach.

A practical compromise:
- rotate DC unchanged
- rotate band-1 with object rotation
- damp band-2 and band-3 toward 0 (stability)

---

## 13. Blending, pruning, validation

### 13.1 Blending / seam feathering
To avoid ridges or over-opaque overlaps:
- compute distance of each original splat center to the filled patch (or to hole boundary)
- apply a weight `w_keep` that fades originals near the patch
- apply `w_new` that fades new splats near the boundary if necessary

### 13.2 Pruning redundancies and “floaters”
After insertion:
- remove new splats that are too close to existing splats:
  - if `dist(μ_new, nearest_old) < c * spacing` with `c ~ 0.3–0.6`, drop it
- remove isolated new splats that have too few neighbors (likely off-surface)

### 13.3 Validation loop
Run a cheap validation pass:
- render N probe views and compute remaining hole pixels (`T > τ_T`)
- stop if below a target (e.g., <0.1% of pixels in ROI)
- otherwise increase sampling density or run one more “grow” iteration

This makes the node robust and deterministic.

---

## 14. Node design: `HealHoles3DGS`

### Inputs
- `Splats` (standard 3DGS)
- optional `Selection` / ROI (mask, bbox, sphere, or “component id”)
- optional `Cameras` (for render-based hole detection; if absent, generate orbit cams)

### Outputs
- `Splats_out` (original + new)
- `NewSplatsMask` (indices or per-splat bool)
- optional debug: `HoleMaskViews`, `BoundarySplatsMask`, `PatchMesh`, `LocalSDF`

### Parameters (suggested)
**Detection**
- `detect_mode = {3D_density, 2D_transmittance, hybrid}`
- `k_neighbors`, `theta_thresh`, `density_percentile`
- `max_hole_diameter` (reject large holes)
- `remove_floaters` (SOR β)

**Fill strategy**
- `fill_mode = {MLS, voxel_SDF, mesh_patch, edge_grow, patch_match}`
- `target_spacing` (or auto from neighbors)
- `sigma_t_mult`, `sigma_n_mult`
- `alpha_mode = {neighbor_median, neighbor_max, target_density}`
- `appearance_mode = {DC_only, interpolate_SH, copy_patch_SH}`

**Post**
- `prune_threshold`
- `feather_width`
- `validate_views`, `stop_threshold`

---

## 15. Rust implementation notes and library options

### 15.1 Core crates you’ll likely want
- **Linear algebra / eigen decomposition:** `nalgebra`
- **kNN / spatial indexing:** `kiddo`, `kdtree`, `rstar` (AABB trees), or custom hash grid
- **Parallelism:** `rayon`
- **Meshing (optional):**
  - marching cubes: any Rust MC implementation or small custom MC for local bricks
  - mesh processing/hole fill: consider FFI to robust C++ libs if needed (Open3D/CGAL) but keep it optional

### 15.2 Performance structure (important)
- Store splats in **SoA** layout for bandwidth.
- For voxel/SDF strategies:
  - operate in **bricks** (e.g., 32³ or 64³)
  - cull contributing splats per brick via spatial binning
- Cache derived data:
  - normals
  - kNN distances
  - boundary clusters

### 15.3 Determinism and procedural UX
A Houdini-like tool benefits from:
- deterministic random seeds (Poisson sampling)
- stable ordering of output splats (for caching)
- extensive debug outputs

---

## Appendix A. Log-Euclidean interpolation of covariance (SPD)

For 3×3 symmetric positive definite matrices (covariances):

1. Eigen-decompose:
   ```math
   \Sigma = Q \Lambda Q^T
   ```
2. Matrix log:
   ```math
   \log\Sigma = Q \log(\Lambda) Q^T
   ```
   where `log(Λ)` applies `log` to diagonal entries.
3. Matrix exp similarly.

Interpolation:
```math
\Sigma(t)=\exp((1-t)\log\Sigma_A + t\log\Sigma_B)
```

### Rust-like sketch (nalgebra)
```rust
use nalgebra::{Matrix3, SymmetricEigen};

fn mat_log_spd(a: Matrix3<f32>) -> Matrix3<f32> {
    let eig = SymmetricEigen::new(a);
    let mut l = Matrix3::zeros();
    for i in 0..3 {
        l[(i,i)] = eig.eigenvalues[i].max(1e-12).ln();
    }
    eig.eigenvectors * l * eig.eigenvectors.transpose()
}

fn mat_exp_sym(a: Matrix3<f32>) -> Matrix3<f32> {
    let eig = SymmetricEigen::new(a);
    let mut e = Matrix3::zeros();
    for i in 0..3 {
        e[(i,i)] = eig.eigenvalues[i].exp();
    }
    eig.eigenvectors * e * eig.eigenvectors.transpose()
}

fn cov_log_euclid_lerp(a: Matrix3<f32>, b: Matrix3<f32>, t: f32) -> Matrix3<f32> {
    let la = mat_log_spd(a);
    let lb = mat_log_spd(b);
    let l = la * (1.0 - t) + lb * t;
    mat_exp_sym(l)
}
```

---

## Appendix B. Poisson disk sampling (Bridson) pseudocode

Use this to sample points on a patch with minimum distance `r`:

```text
samples = []
active = []

seed = random_point_in_domain()
samples.add(seed)
active.add(seed)

while active not empty:
  s = random_choice(active)
  found = false
  repeat k times:
    c = random_point_in_annulus(center=s, r, 2r)
    if c inside domain AND far_from_all_samples(c, r):
       samples.add(c)
       active.add(c)
       found = true
       break
  if not found:
     active.remove(s)
```

For 3D patches, sample in parameter space (u,v) then lift to surface.

---

## Appendix C. Useful heuristics and defaults

### Detection defaults
- `k_neighbors = 16..32`
- `theta_thresh = 110°`
- density anomaly: boundary if `r_k > 1.5 * median(r_k in neighborhood)`

### Fill defaults
- spacing = median 1NN distance around boundary
- `σ_t = 0.6 * spacing`
- `σ_n = 0.08 * σ_t`
- `α_new = median(α_neighbors)` and clamp to `[0.2, 0.99]`

### Safety clamps
- prune if `dist(new, old) < 0.4*spacing`
- cap new splats per hole to avoid runaway sampling

### Validation defaults
- render 8–16 probe views
- stop when hole pixels in ROI < 0.1% at `T > 0.5`

---

## Recommended “one good default” pipeline

If you want a single robust method that fits your constraints:

1. **Hybrid detection:** kNN boundary + optional transmittance mask.
2. **Local implicit MLS surface** over boundary + support ring.
3. **Poisson disk sampling** in boundary parameterization, then project to implicit.
4. Create new splats with pancake covariance aligned to implicit normals.
5. **Interpolate DC + full SH** from neighbors (or DC-only if you want maximum stability).
6. Feather/prune, validate in a few probe renders.

This achieves reliable **small-hole closure** with controllable behavior and runs in seconds for typical hole sizes.
