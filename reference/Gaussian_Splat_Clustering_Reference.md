# Gaussian Splat Clustering (3DGS)
## Reference documentation for clustering, segmentation, and LOD in a procedural 3D Gaussian Splat editing/processing app (Rust/WASM-friendly)

This document is a **self-contained implementation reference** for clustering **standard 3D Gaussian Splatting (3DGS)** models: grouping splats into clusters for editing, cleanup, compression, streaming, and multi-resolution (LOD) representations.

It focuses on:
- **What to cluster** (features and derived quantities)
- **How to measure distance/similarity** between splats (metrics that respect anisotropic Gaussians)
- **Clustering algorithms** that work well for splats (DBSCAN/connected components/grid/hierarchy/k-means/GMM)
- **How to aggregate** a cluster into a “super-splat” (moment matching, covariance, SH averaging)
- **Practical node designs** for Houdini-like workflows
- **Rust implementation sketches** with attention to interactive performance and cross-compiling to WebAssembly

---

## Contents
- [1. Why cluster splats?](#1-why-cluster-splats)
- [2. Splat parameterization and derived attributes](#2-splat-parameterization-and-derived-attributes)
- [3. Feature spaces for clustering](#3-feature-spaces-for-clustering)
- [4. Distances/similarities between splats](#4-distancessimilarities-between-splats)
- [5. Core clustering approaches](#5-core-clustering-approaches)
  - [5.1 Grid / voxel clustering](#51-grid--voxel-clustering)
  - [5.2 Radius graph + connected components](#52-radius-graph--connected-components)
  - [5.3 DBSCAN (recommended general-purpose)](#53-dbscan-recommended-general-purpose)
  - [5.4 HDBSCAN (variable density)](#54-hdbscan-variable-density)
  - [5.5 k-means / mini-batch k-means](#55-k-means--mini-batch-k-means)
  - [5.6 Agglomerative / hierarchical clustering](#56-agglomerative--hierarchical-clustering)
  - [5.7 Gaussian Mixture Models (EM)](#57-gaussian-mixture-models-em)
  - [5.8 Graph partitioning / spectral / SLIC-like “super-splats”](#58-graph-partitioning--spectral--slic-like-super-splats)
- [6. Cluster summaries and “super-splat” aggregation](#6-cluster-summaries-and-super-splat-aggregation)
- [7. Building LOD hierarchies by clustering](#7-building-lod-hierarchies-by-clustering)
- [8. Clustering for editing workflows](#8-clustering-for-editing-workflows)
- [9. Performance engineering notes (Rust/WASM)](#9-performance-engineering-notes-rustwasm)
- [10. Validation and debugging](#10-validation-and-debugging)
- [Appendix A: Moment-matching derivations](#appendix-a-moment-matching-derivations)
- [Appendix B: Rust-like pseudocode snippets](#appendix-b-rust-like-pseudocode-snippets)

---

## 1. Why cluster splats?

Clustering splats is useful in multiple parts of a procedural 3DGS toolchain:

### 1.1 Editing and selection
- Create **groups** (like “superpixels” but in 3D) so users can select/edit at a higher level.
- Identify connected components (separate objects or fragments).
- Create masks for operations like delete, transform, recolor, delight/relight, hole-fill, etc.

### 1.2 Cleanup and quality
- Detect and remove **floaters** and sparse outliers (noise clusters).
- Detect **thin crack regions** (clusters of boundary splats) for healing.
- Identify duplicated/overlapping regions for pruning.

### 1.3 Performance and streaming
- Partition splats into **tiles/chunks** for:
  - multi-threading
  - frustum culling
  - occlusion culling (cluster-level depth bounds)
  - out-of-core streaming

### 1.4 Compression and LOD
- Build coarse representations by aggregating clusters into “super-splats”:
  - fewer primitives
  - stable far-field render
  - faster CPU/GPU evaluation and sorting

---

## 2. Splat parameterization and derived attributes

Assume each splat has:

- center: `μ ∈ ℝ³`
- rotation: quaternion `q` → rotation matrix `R`
- axis stddev (scale): `s = (sₓ,sᵧ,s_z)` (often stored in log-space)
- opacity/weight: `α ∈ (0,1)`
- appearance: SH coefficients per color channel:
  ```math
  \mathbf{C}(\mathbf{d}) = \sum_{i=0}^{M-1} \mathbf{k}_i Y_i(\mathbf{d})
  ```
  where `M = (L+1)^2`, usually `L=3` → `M=16`.

### 2.1 Covariance matrix
Many splat systems implicitly represent a covariance:

```math
\Sigma = R \, \mathrm{diag}(s_x^2, s_y^2, s_z^2) \, R^T
```

### 2.2 Useful derived attributes for clustering
- **Local normal** `n`: often the rotated axis of the **smallest** scale (surfel assumption)
- **Volume proxy** `vol = det(Σ)^{1/2} = sₓ sᵧ s_z`
- **Anisotropy**:
  ```math
  a = \frac{\max(s)}{\min(s)+\epsilon}
  ```
- **DC color** (view-independent approximation):
  - if SH uses `Y00 = 0.28209479`, then `C_dc ≈ k0 * Y00`
- **SH energy** by band (useful to detect “sparkly” regions):
  ```math
  E_\ell = \sum_{m=-\ell}^{\ell} \|\mathbf{k}_{\ell m}\|^2
  ```

---

## 3. Feature spaces for clustering

Clustering depends heavily on feature choice. In a procedural app, expose **multiple “feature presets”** depending on the task.

### 3.1 Spatial clustering (geometry-only)
Use when grouping by proximity / connectedness.

Feature vector:
```text
f = [x, y, z]
```

Recommended normalization:
- scale positions by scene scale or median neighbor distance, so thresholds are intuitive.

### 3.2 Surface-aware clustering (geometry + orientation)
Use when you want clusters aligned to surfaces and not bridging across close-but-opposing surfaces.

```text
f = [x, y, z, w_n * n_x, w_n * n_y, w_n * n_z]
```

where `w_n` is a weight controlling how much normal alignment matters.

### 3.3 Appearance-aware clustering (for “material groups”)
Use DC color and optionally SH band energy.

```text
f = [x,y,z, w_c * r, w_c * g, w_c * b]
```

Or:
```text
f = [x,y,z, w_c * C_dc, w_e * E_1, w_e * E_2, ...]
```

### 3.4 Scale-aware clustering (for LOD, pruning duplicates)
Include log-scales:

```text
f = [x,y,z, w_s * log(sx), w_s * log(sy), w_s * log(sz)]
```

### 3.5 Practical advice: whiten + clamp
- **Whiten** features (divide by robust stddev / MAD) so one dimension doesn’t dominate.
- **Clamp** extreme SH/opacity to avoid letting outliers form their own clusters due to a single huge coefficient.

---

## 4. Distances/similarities between splats

### 4.1 Euclidean distance between centers
Fastest and most common:
```math
d_{pos}(i,j) = \|\mu_i - \mu_j\|_2
```

### 4.2 Ellipsoid-aware overlap distance (Mahalanobis-like)
Use one splat’s covariance to measure distance:
```math
d_{mah}(i,j) = \sqrt{ (\mu_i-\mu_j)^T \Sigma_i^{-1} (\mu_i-\mu_j) }
```

For symmetry, use:
```math
d_{symmah}(i,j) = \sqrt{ (\mu_i-\mu_j)^T \left(\frac{\Sigma_i^{-1}+\Sigma_j^{-1}}{2}\right) (\mu_i-\mu_j) }
```

Useful for “connected components of overlapping splats”.

### 4.3 2-Wasserstein distance between Gaussians (geometry metric)
Treat each splat as a Gaussian distribution `N(μ, Σ)`. Then the squared 2-Wasserstein distance:

```math
W_2^2(\mathcal{N}_1,\mathcal{N}_2)=\|\mu_1-\mu_2\|^2 + \mathrm{Tr}\!\left(\Sigma_1+\Sigma_2-2(\Sigma_1^{1/2}\Sigma_2\Sigma_1^{1/2})^{1/2}\right)
```

Pros:
- respects anisotropy and size
- stable for comparing “shape+position”

Cons:
- requires matrix square roots (eigendecomposition), more expensive

Practical compromise:
- compute `W_2` only for candidate neighbors from a spatial index.

### 4.4 Appearance distance (DC color or SH)
DC-only:
```math
d_{col}(i,j)=\|\mathbf{C}_{dc,i}-\mathbf{C}_{dc,j}\|
```

SH distance:
```math
d_{sh}(i,j)=\sqrt{\sum_{c=0}^{M-1}\|\mathbf{k}_{c,i}-\mathbf{k}_{c,j}\|^2}
```

Often you want a weighted sum:
```math
d(i,j)= w_p d_{pos}+ w_n d_{nrm}+ w_c d_{col}+ w_s d_{scale}
```

Where:
- `d_nrm = arccos(clamp(dot(n_i,n_j),-1,1))` or simply `1 - dot(n_i,n_j)`.

### 4.5 Choosing thresholds: make them scale-invariant
Derive base scale from the model itself:
- `spacing = median(1NN distance)`
- then:
  - DBSCAN `eps = 1.5 * spacing` (start point)
  - adjacency `r_adj = 2.0 * spacing`

---

## 5. Core clustering approaches

### 5.1 Grid / voxel clustering

**Use case:** very fast chunking, streaming, LOD pre-binning.

Method:
- pick a cell size `h` (often 2–8× median spacing)
- map each splat to integer cell coordinates:
  ```math
  c = \left(\lfloor x/h \rfloor,\lfloor y/h \rfloor,\lfloor z/h \rfloor\right)
  ```
- cluster id = hash(c)

Pros:
- O(N) time
- stable and deterministic
- great for parallel processing

Cons:
- not adaptive to density
- boundaries can cut through objects

**Refinement:** Within each cell, you can run a second-stage clustering (e.g., connected components) for better segmentation.

---

### 5.2 Radius graph + connected components

**Use case:** identify connected pieces of geometry; detect islands; split into components.

Method:
1. Build a neighbor graph: connect i↔j if `d_pos(i,j) < r_adj`
2. Compute connected components via union-find / BFS.

Pros:
- simple, deterministic
- good for “fragment separation”

Cons:
- sensitive to `r_adj`
- can wrongly connect nearby but disjoint surfaces (use normals/scale filters)

**Filters to reduce false connections:**
- require normal similarity: `dot(n_i,n_j) > cos(θ_max)` (e.g., θ_max=45°)
- require ellipsoid overlap: `d_symmah < τ`

---

### 5.3 DBSCAN (recommended general-purpose)

DBSCAN clusters points by density.

Parameters:
- `eps`: neighborhood radius
- `min_pts`: minimum neighbors for a core point

Algorithm (informal):
- a point with ≥ `min_pts` neighbors within `eps` is a **core**
- connected cores form clusters
- points near cores but not cores are **border**
- others are **noise** (great for floater removal)

Pros:
- automatically finds number of clusters
- handles outliers naturally
- good for variable shapes

Cons:
- struggles with strongly varying densities (HDBSCAN helps)

**Practical for splats:**
- Run DBSCAN in a feature space that includes normals and/or scale to prevent bridging across layers.

Typical starting parameters:
- `eps = 1.5 * spacing`
- `min_pts = 8..24` depending on density

---

### 5.4 HDBSCAN (variable density)

HDBSCAN is a hierarchical DBSCAN variant that handles varying densities better.
It is more complex but excellent for real scan data where density changes.

Implementation is heavier; consider:
- using a library for desktop builds
- or using DBSCAN with adaptive `eps` from local `r_k` for WASM

---

### 5.5 k-means / mini-batch k-means

**Use case:** forcing a fixed number of clusters (e.g., GPU tiles, workload partitioning).

Pros:
- fast and simple
- easy to implement with mini-batches

Cons:
- assumes roughly spherical clusters in feature space
- requires choosing `k`

If you use k-means for spatial chunking:
- run on `[x,y,z]`
- then optionally split clusters that exceed max AABB size

---

### 5.6 Agglomerative / hierarchical clustering

**Use case:** LOD hierarchy, progressive merging.

Common pattern for splats:
- start with fine splats
- repeatedly merge nearest neighbors/clusters under an error threshold

Pros:
- produces a tree (useful for LOD)
- can enforce cluster size constraints

Cons:
- naive O(N²); must use spatial acceleration and/or restrict merges locally

---

### 5.7 Gaussian Mixture Models (EM)

Treat the splat centers (or densities) as samples from a mixture model.
GMM clustering is more statistically grounded but heavier.

Pros:
- soft assignments (a splat can belong partially to clusters)
- can model anisotropic clusters

Cons:
- iterative EM; must pick `K`
- may not match surface structure unless features are chosen well

Practical use in a procedural app:
- rarely used as a default; keep as an “advanced” node option.

---

### 5.8 Graph partitioning / spectral / SLIC-like “super-splats”

If you want “super-splats” analogous to superpixels:
- build a kNN graph with edge weights based on combined similarity (pos+normal+color)
- partition with:
  - spectral clustering (expensive)
  - region-growing from seeds (cheap)
  - SLIC-style iterative assignment (very practical)

**SLIC-like 3D super-splats (recommended for editing UX):**
- choose K seeds in space
- iterate:
  - assign splats to nearest seed in combined space:
    ```math
    d = \|x-x_s\|^2 + m^2 \|c-c_s\|^2 + n^2(1-\mathbf{n}\cdot\mathbf{n}_s)
    ```
  - recompute seed features as cluster means
- stop after ~5–15 iterations

This yields stable, compact “patches” on surfaces.

---

## 6. Cluster summaries and “super-splat” aggregation

Many workflows require reducing a cluster to a representative primitive (for LOD, preview, compression).

### 6.1 Weighted moment matching: cluster → one Gaussian

Treat each splat as a Gaussian `N(μ_i, Σ_i)` with weight `w_i`.

Common weights:
- `w_i = α_i` (opacity weight)
- or `w_i = α_i * vol_i` (mass proxy)
- or `w_i = 1` (uniform)

Define total weight:
```math
W=\sum_i w_i
```

**Mean:**
```math
\mu = \frac{1}{W}\sum_i w_i \mu_i
```

**Covariance (mixture moment):**
```math
\Sigma = \frac{1}{W}\sum_i w_i \left[\Sigma_i + (\mu_i-\mu)(\mu_i-\mu)^T\right]
```

This produces an SPD covariance if inputs are SPD and weights are positive.

Then convert `Σ` back to `(q,s)` by eigendecomposition:
- eigenvectors → rotation `R`
- eigenvalues → `s = sqrt(λ)`

> This gives a geometric “super-splat” approximating the cluster’s spatial distribution.

### 6.2 Aggregating opacity α
Opacity in 3DGS is not strictly additive. For LOD/proxy use, pragmatic options:

1. **Weight-preserving** (for density-like use):
   ```math
   \alpha = \mathrm{clamp}\left(\frac{1}{W}\sum_i w_i \alpha_i, 0, 1\right)
   ```

2. **“Union” style** (treat as independent coverage):
   ```math
   \alpha = 1 - \prod_i (1-\alpha_i)^{\gamma_i}
   ```
   where `γ_i` can be normalized weights. This can saturate fast; use only when cluster size is small.

3. **Calibrate by rendering error** (best, but heavier):
   - choose α so that cluster proxy matches average transmittance in a few test directions.

### 6.3 Aggregating SH coefficients / color
For premultiplied behavior, weight SH coefficients by a mass/opacity term:

Let `k_i[c]` be coefficient c (per channel). Then:
```math
k[c] = \frac{\sum_i w_i k_i[c]}{\sum_i w_i}
```

If the cluster spans strongly varying view-dependent appearance, you can:
- store only DC for the proxy,
- or compute band energies and damp higher bands.

### 6.4 Cluster statistics (for debugging and constraints)
Compute:
- cluster AABB / bounding sphere
- point count
- average spacing
- normal variance
- color variance
- SH band energies

Use these for:
- rejecting unstable proxies
- selecting algorithm parameters automatically

---

## 7. Building LOD hierarchies by clustering

### 7.1 Simple and effective: octree / grid-based LOD
1. Choose base cell size `h0` (near-field).
2. Build grid clusters at `h0`.
3. Aggregate each cluster → super-splat.
4. Increase cell size by factor 2 and repeat.

This yields LOD levels `L0 (fine) → L1 → L2 → ...`.

Pros:
- deterministic
- fast O(N) per level
- easy streaming

Cons:
- can blur thin objects if `h` gets too large; mitigate by splitting clusters with high anisotropy or high normal variance.

### 7.2 Error-driven hierarchical merging (higher quality)
Build a graph where each splat can merge with neighbors; iteratively merge the best pair:

Define a merge cost:
```math
E(i,j) = w_p \|\mu_i-\mu_j\|^2 + w_n (1-\mathbf{n}_i\cdot\mathbf{n}_j) + w_c \|\mathbf{C}_i-\mathbf{C}_j\|^2
```

Merge pairs with smallest E under constraints:
- max cluster radius
- max normal variance
- max color variance

This produces better LOD for structured geometry, but needs a priority queue and neighbor updates.

### 7.3 View-dependent / screen-space LOD selection
In a renderer, choose which level to draw based on projected footprint:

If a splat has covariance Σ, projected size depends on camera.
A pragmatic LOD metric:
- approximate projected radius from `max(s)` and depth:
  ```math
  r_px \approx \frac{f \cdot \max(s)}{z}
  ```
Then select a level where `r_px` is within a target band.

---

## 8. Clustering for editing workflows

### 8.1 “Select islands” (connected components)
- radius graph + connected components
- optionally treat “touching” surfaces as separate by normal threshold

### 8.2 “Material clusters” (appearance segmentation)
- SLIC-like clustering in (position, normal, DC color)
- output stable groups for:
  - recolor, delight/relight, roughness hacks (band dampening), etc.

### 8.3 “Noise clusters” (floaters/outliers)
- DBSCAN on positions only (or pos+alpha)
- delete clusters with:
  - very low point count
  - low local density
  - inconsistent normals

### 8.4 “Processing tiles” (parallel compute nodes)
- grid clustering to create tiles
- within each tile you can:
  - run local operations (filtering, hole filling, normal smoothing)
  - then merge back

---

## 9. Performance engineering notes (Rust/WASM)

### 9.1 Use SoA storage
Clustering is bandwidth-bound. Structure-of-arrays improves SIMD and cache behavior.

### 9.2 Spatial index choices
- For CPU desktop: kd-tree (`kiddo`) is easy and fast.
- For dynamic updates: hash grid is often faster than kd-tree for radius queries.
- For WASM: prefer hash grid (simpler, fewer allocations), or use a small kd-tree crate if stable.

### 9.3 Parallelism
- Desktop: use `rayon` to parallelize neighbor queries + labeling.
- WASM: often single-threaded; process in chunks with progress callbacks.

### 9.4 Avoid heavy matrix ops in inner loops
- If you need Wasserstein or matrix square roots, only compute them for candidate neighbors (after pruning by bounding radius).
- Keep most clustering in cheap metrics (pos+normal+color).

---

## 10. Validation and debugging

### 10.1 Debug outputs
- visualize clusters by random color per cluster
- show cluster bounding boxes
- show cluster-level stats histogram (sizes, variances)
- show “noise points” / outliers

### 10.2 Unit tests
- moment matching covariance remains SPD (eigenvalues > 0)
- invariance: clustering stable under rigid transforms if using transform-invariant features
- DBSCAN: synthetic datasets with known clusters
- connected components: verify against known graph components

---

## Appendix A: Moment-matching derivations

Given a mixture distribution:
```math
p(x) = \sum_i \pi_i \mathcal{N}(x|\mu_i,\Sigma_i)
```
where `π_i = w_i / W`.

Mean:
```math
\mathbb{E}[x]=\sum_i \pi_i \mu_i
```

Second moment:
```math
\mathbb{E}[xx^T]=\sum_i \pi_i (\Sigma_i + \mu_i\mu_i^T)
```

Covariance:
```math
\Sigma = \mathbb{E}[xx^T] - \mu\mu^T
      = \sum_i \pi_i \Sigma_i + \sum_i \pi_i (\mu_i-\mu)(\mu_i-\mu)^T
```

---

## Appendix B: Rust-like pseudocode snippets

### B.1 Hash grid for radius queries

```rust
use std::collections::HashMap;

#[derive(Clone, Copy)]
struct I3(i32, i32, i32);

fn cell_of(p: [f32;3], h: f32) -> I3 {
    I3((p[0]/h).floor() as i32, (p[1]/h).floor() as i32, (p[2]/h).floor() as i32)
}

fn build_grid(points: &[[f32;3]], h: f32) -> HashMap<I3, Vec<usize>> {
    let mut grid: HashMap<I3, Vec<usize>> = HashMap::new();
    for (i, &p) in points.iter().enumerate() {
        grid.entry(cell_of(p, h)).or_default().push(i);
    }
    grid
}

fn neighbors_in_radius(
    i: usize,
    points: &[[f32;3]],
    grid: &HashMap<I3, Vec<usize>>,
    h: f32,
    r: f32,
) -> Vec<usize> {
    let p = points[i];
    let I3(cx, cy, cz) = cell_of(p, h);
    let mut out = Vec::new();
    let r2 = r*r;

    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let key = I3(cx+dx, cy+dy, cz+dz);
                if let Some(list) = grid.get(&key) {
                    for &j in list {
                        if j == i { continue; }
                        let q = points[j];
                        let dx = q[0]-p[0]; let dy = q[1]-p[1]; let dz = q[2]-p[2];
                        if dx*dx + dy*dy + dz*dz <= r2 {
                            out.push(j);
                        }
                    }
                }
            }
        }
    }
    out
}
```

### B.2 DBSCAN skeleton (using radius queries)

```rust
// labels: -1 = noise, -2 = unvisited, >=0 cluster id
fn dbscan(points: &[[f32;3]], eps: f32, min_pts: usize) -> Vec<i32> {
    let n = points.len();
    let h = eps; // cell size
    let grid = build_grid(points, h);
    let mut labels = vec![-2i32; n];
    let mut cluster_id = 0i32;

    for i in 0..n {
        if labels[i] != -2 { continue; }
        let neigh = neighbors_in_radius(i, points, &grid, h, eps);
        if neigh.len() + 1 < min_pts {
            labels[i] = -1;
            continue;
        }
        // start new cluster
        labels[i] = cluster_id;
        let mut queue = neigh;

        while let Some(p) = queue.pop() {
            if labels[p] == -1 { labels[p] = cluster_id; } // border becomes member
            if labels[p] != -2 { continue; }               // already processed
            labels[p] = cluster_id;

            let neigh_p = neighbors_in_radius(p, points, &grid, h, eps);
            if neigh_p.len() + 1 >= min_pts {
                // expand
                queue.extend(neigh_p);
            }
        }

        cluster_id += 1;
    }

    labels
}
```

### B.3 Moment-matching aggregation

```rust
use nalgebra::Matrix3;

fn moment_match_gaussian(
    mus: &[[f32;3]],
    sigmas: &[Matrix3<f32>],
    weights: &[f32],
    indices: &[usize],
) -> ([f32;3], Matrix3<f32>) {
    let mut wsum = 0.0f32;
    let mut mu = [0.0f32;3];
    for &i in indices {
        let w = weights[i].max(0.0);
        wsum += w;
        mu[0] += w * mus[i][0];
        mu[1] += w * mus[i][1];
        mu[2] += w * mus[i][2];
    }
    let inv = 1.0 / wsum.max(1e-12);
    mu[0] *= inv; mu[1] *= inv; mu[2] *= inv;

    let mut sigma = Matrix3::<f32>::zeros();
    for &i in indices {
        let w = weights[i].max(0.0) * inv;
        let dx = mus[i][0]-mu[0];
        let dy = mus[i][1]-mu[1];
        let dz = mus[i][2]-mu[2];
        let outer = Matrix3::new(
            dx*dx, dx*dy, dx*dz,
            dy*dx, dy*dy, dy*dz,
            dz*dx, dz*dy, dz*dz,
        );
        sigma += sigmas[i]*w + outer*w;
    }
    (mu, sigma)
}
```

---

## Appendix C: Useful heuristics and defaults

### C.1 Estimating “spacing”
- compute nearest-neighbor distances `d1[i]`
- `spacing = median(d1)`

### C.2 Default cluster presets
- **Tiles for parallel processing:** grid cell `h = 8 * spacing`
- **Connected components:** adjacency radius `r_adj = 2 * spacing`, with normal threshold 45°
- **DBSCAN for outliers:** `eps = 1.5 * spacing`, `min_pts = 12`
- **Super-splats for selection:** SLIC-like, seed count `K = N / 500` (rough start), 10 iterations

### C.3 When to use “ellipsoid-aware” distances
Use Mahalanobis/Wasserstein only when:
- you need to ensure clusters respect splat scale/orientation
- you’re aggregating for LOD and want stable results
Otherwise, stick to fast pos/normal/color metrics.

---

## Suggested node set for a procedural 3DGS app

- `ClusterGrid` (fast spatial binning)
- `ClusterDBSCAN` (density clustering + outlier labeling)
- `ClusterConnectedComponents` (islands)
- `ClusterSuperSplats` (SLIC-like surface patches)
- `AggregateClustersToSplats` (moment-matching super-splats)
- `BuildLODHierarchy` (grid or merge-based)
- `SelectCluster`, `DeleteClusters`, `TransformClusters`, `AttributeReducePerCluster`

These nodes compose into workflows like:
- outlier removal → connected components → group selection → LOD build → export

