# Geometric Joining of Two 3D Gaussian Splat Models
## Smooth blends, skirts, and SDF-like merges for a Rust procedural 3DGS editing app

This document is a **self-contained implementation reference** for *geometrically* joining two standard 3D Gaussian Splatting (3DGS) models **A** and **B** after you’ve already solved **photometric** consistency (delighting/relighting/exposure matching).

The goal is to create a **continuous transition** across the interface — like a **smooth union** of two SDFs — but producing an output that remains a **3DGS splat set** (or optionally mesh/SDF sidecars).

---

## Contents
- [1. Problem statement and mental model](#1-problem-statement-and-mental-model)
- [2. Core fields: density, occupancy, SDF](#2-core-fields-density-occupancy-sdf)
- [3. Approach options](#3-approach-options)
  - [3.1 Feathered partition-of-unity blend in splat domain](#31-feathered-partition-of-unity-blend-in-splat-domain)
  - [3.2 Local SDF smooth-union + seam resplat (recommended)](#32-local-sdf-smooth-union--seam-resplat-recommended)
  - [3.3 Full implicit fusion + full resplat (robust, heavier)](#33-full-implicit-fusion--full-resplat-robust-heavier)
  - [3.4 “Skirt” generation without SDF (fast bridging splats)](#34-skirt-generation-without-sdf-fast-bridging-splats)
  - [3.5 Optimization-based seam solve (advanced)](#35-optimization-based-seam-solve-advanced)
- [4. Converting SDF/mesh back to splats](#4-converting-sdfmesh-back-to-splats)
- [5. Practical node designs](#5-practical-node-designs)
- [6. Implementation details in Rust](#6-implementation-details-in-rust)
- [7. Pitfalls and quality checks](#7-pitfalls-and-quality-checks)
- [Appendix A. Smooth min / smooth union functions](#appendix-a-smooth-min--smooth-union-functions)
- [Appendix B. Minimal Rust-like pseudocode snippets](#appendix-b-minimal-rust-like-pseudocode-snippets)

---

## 1. Problem statement and mental model

### What does it mean to “join splat models geometrically”?
A standard 3DGS model renders by alpha compositing many anisotropic Gaussians. It does **not** store an explicit surface, so seams show up as:

- **gaps**: density/opacity too low in between models (a “crack”)
- **double walls**: overlap yields over-dense/over-opaque regions (a “ridge”)
- **mismatched thickness**: one model is “fatter” (larger σ/α) than the other at the interface

A geometric join operator should create a **single coherent density/surface band**.

### What “SDF smooth merge” implies
Smooth union of SDFs connects surfaces within a blend radius `k`. The analog for splats is:

- build or approximate an **implicit surface** from each model
- combine them with **smooth union**
- re-express the result back as splats (or add splats in the seam band)

---

## 2. Core fields: density, occupancy, SDF

### 2.1 Splat density field
Given splats with centers `μ_i`, covariances `Σ_i`, opacity/weight `α_i`:

```math
D(x) = \sum_i \alpha_i \exp\left( -\frac{1}{2}(x-\mu_i)^T \Sigma_i^{-1}(x-\mu_i) \right)
```

You can use `D(x)` as a scalar field for iso-surface extraction: `D(x) = λ`.

### 2.2 Occupancy from density
Define a binary inside/outside mask:

```math
B(x) = [D(x) > \lambda]
```

### 2.3 Signed distance field (SDF)
An SDF is a scalar field `s(x)`:

- `s(x) < 0` inside
- `s(x) = 0` on surface
- `s(x) > 0` outside
- `|∇s| ≈ 1`

A robust grid SDF can be computed from `B(x)` via two Euclidean distance transforms (EDT):

- `d_in = EDT(B)`
- `d_out = EDT(!B)`
- `s = d_out - d_in` (scaled by voxel size)

---

## 3. Approach options

### Summary table

| Option | Output | Best for | Pros | Cons |
|---|---|---|---|---|
| 3.1 Feathered blend | splats | overlap-only transitions | extremely fast; pure attribute ops | won’t fill gaps; not true geometric join |
| 3.2 Local SDF smooth union + resplat | splats (+ optional mesh) | “SDF-like join”, interactive | highest quality per cost; resolves gaps+overlap | needs SDF/MC + resplat implementation |
| 3.3 Full implicit fusion + full resplat | splats | robust global union | clean, coherent global surface | heavier; may lose high-frequency 3DGS appearance |
| 3.4 Skirt bridging splats | splats | quick “seam fill” | fast; no grids | more heuristic; can produce lumps |
| 3.5 Optimization seam solve | splats | best quality, researchy | can preserve appearance + geometry | complex; needs iterative solve |

---

## 3.1 Feathered partition-of-unity blend in splat domain

This is the simplest **procedural** join: you fade out splats from A as you approach B and vice versa, using weights that sum to 1:

```math
D_{\text{blend}}(x) = w_A(x) D_A(x) + w_B(x) D_B(x), \quad w_A+w_B=1
```

### Key idea
You don’t change geometry; you change **which model contributes** near the interface so you don’t get double density.

### Weight functions
You need a “distance to interface” scalar `t(x)`.

Practical choices:

1. **SDF difference** (best if you already have SDFs):
   - `δ(x) = s_A(x) - s_B(x)`
   - interface is `δ=0`
   - weights via smoothstep:
     ```math
     w_A(x)=\mathrm{smoothstep}(-k, k, -\delta(x)),\quad w_B=1-w_A
     ```

2. **Nearest-neighbor distance** (fastest; no SDF):
   - for each splat center `μ` in A, compute `d = dist(μ, B)` using kNN
   - `w_A(μ)=smoothstep(0, k, d)` (far from B → keep A)
   - similarly for B

### Apply weights to splats
Approximate `w_A(x)` at splat centers and scale each splat:

- `α_i ← α_i * w(μ_i)`
- SH coefficients `k_i ← k_i * w(μ_i)` (preserves premultiplied behavior)

> This is an approximation: scaling at the center scales the whole Gaussian uniformly. In practice it works well when weights vary slowly over the splat support.

### Pros/cons
- ✅ Great when models **overlap** and you only need to remove the ridge.
- ❌ Won’t fill a **gap**. For gaps, add skirts or resplat.

---

## 3.2 Local SDF smooth union + seam resplat (recommended)

This is the closest to “smooth merge of SDFs” while staying interactive.

### Overview
1. Define a **seam region** around the interface.
2. Build an SDF for A and B **only in that region** (chunked).
3. Compute a **smooth union SDF** `sU`.
4. Extract seam geometry (mesh or point samples) from `sU=0`.
5. Convert seam geometry back into **new splats**.
6. Replace/feather the original splats inside the seam band.

This produces an actual geometric bridge, and can connect small gaps within the blend radius.

---

### 3.2.1 Define the seam region

You want a bounding box (or a set of bricks) that covers where A and B can interact.

Practical seam AABB:

1. Compute AABB of A, AABB of B.
2. Compute intersection AABB (may be empty).
3. Expand by a margin `m = blend_radius + 3*max_sigma`.

If the AABBs don’t overlap, define a seam box around the closest points between sets:
- pick a subset of splat centers from A, find nearest in B
- take pairs with distance ≤ `d_max`
- build AABB of those pairs + margin

---

### 3.2.2 Compute SDFs in the seam region

Use user-defined voxel size `dx` (preferred).

Steps per model (A and B) in seam region:

1. Rasterize density `D` to a grid (scatter splats into voxels).
2. Threshold to occupancy `B=[D>λ]`.
3. EDT to compute SDF `s`.

> You only need a **narrow band** around the surface for marching cubes. But a full EDT in the seam bricks is often acceptable for interactive use if the region is small.

---

### 3.2.3 Smooth union in SDF domain

Given `sA(x)` and `sB(x)`:

- hard union SDF: `sU = min(sA, sB)`
- smooth union SDF: `sU = smin(sA, sB, k)` where `k` is blend radius (in world units)

Recommended smooth-min (polynomial) is stable and branch-light:

```math
h = \mathrm{clamp}\left(0.5 + 0.5\frac{s_B - s_A}{k}, 0, 1\right)
```

```math
s_{\text{smooth}} = \mathrm{lerp}(s_B, s_A, h) - k\,h(1-h)
```

- `k` controls the radius of smoothing. Larger `k` creates thicker transition.

---

### 3.2.4 Extract a seam surface

Run marching cubes on `sU` in the seam region to extract `sU=0`.  
This yields a mesh patch that bridges A and B.

**Open vs closed output**
- If you want a **closed** join: ensure both masks are “solid” (inside/outside well-defined) and run MC in a region that includes the connection.
- If you want to **preserve open surfaces**: treat SDF as *unsigned distance* in the join band and avoid forcing a global inside/outside, or only generate “skirt” splats without claiming sign.

---

### 3.2.5 Convert seam surface to splats (“resplat”)

You want to generate a splat set `S_seam` that approximates the seam geometry and blends appearance.

High-level approach:
1. Sample points on the seam mesh (Poisson disk recommended).
2. For each sample, set a splat center `μ = p`, normal `n` from mesh.
3. Choose Gaussian covariance with smallest axis aligned to `n`.
4. Compute color / SH from blended source contributions.
5. Set `α` so the seam is neither too thin nor too opaque.

See [Section 4](#4-converting-sdfmesh-back-to-splats) for details.

---

### 3.2.6 Replace/feather original splats

Within the seam band (distance to seam surface < `k`), fade the original splats and inject seam splats:

- compute `d = |sU(μ)|` for each original splat center
- `w_keep = smoothstep(k, 0, d)` (keep outside, fade inside band)
- scale `α` and SH by `w_keep`

Then append seam splats.

This avoids double walls and makes the seam splats “take over” the join region.

---

## 3.3 Full implicit fusion + full resplat (robust, heavier)

If you need a very coherent global merge (e.g., you’re authoring a final asset):

1. Build SDF volumes for A and B over a global bounding box (or adaptive grid).
2. Smooth union: `sU = smin(sA,sB,k)`.
3. Option A: extract final mesh and ship mesh+splats (splats as beauty, mesh for collisions).
4. Option B: resplat the entire fused surface/volume.

This gives the cleanest geometric result, but you often lose the “original 3DGS micro-appearance” (high-frequency radiance baked into splats), unless you keep and blend appearance carefully.

---

## 3.4 Skirt generation without SDF (fast bridging splats)

If you want something “like a skirt” but don’t want grids:

### 3.4.1 Identify boundary splats near the other model

For each splat in A:
- find nearest splat in B: `(j, dist)`
- if `dist < skirt_max`, mark as boundary pair

Optionally also require that the splat is on the “outer layer”:
- check local neighbor density in A (few neighbors within radius) to avoid interior points

### 3.4.2 Generate bridging splats along the segment

For a pair `(μA, μB)` with distance `d`:

Choose number of steps:
```math
n = \left\lceil \frac{d}{\Delta} \right\rceil,\quad \Delta \approx 2\,\sigma_{\text{target}}
```

For each `t = 1..n-1`, create a new splat:
```math
\mu(t) = (1-\tau)\mu_A + \tau\mu_B,\quad \tau = \frac{t}{n}
```

Set covariance:
- axis 0 (normal-ish) can align to `(μB-μA)` or to local surface normal if available
- tangential σ increases to cover cracks:
  - `σ_t = lerp(σA_t, σB_t, τ) + skirt_thickness`
- normal σ small:
  - `σ_n = min(σA_n, σB_n)`

Set opacity:
- `α = skirt_alpha * smoothstep(0, 1, 1-|2τ-1|)` (peak in middle)

Set color:
- blend DC color from A and B (and optionally keep higher bands damped):
  - `C = lerp(CA, CB, τ)`
  - store as SH DC

This creates a “bridge strip” of splats.

**Pros:** very fast, no marching cubes, works well for small gaps.  
**Cons:** heuristic; can produce lumpy density if many pairs overlap.

---

## 3.5 Optimization-based seam solve (advanced)

Treat seam splats as variables and solve a small optimization:

- Define a target implicit field `F_target(x)` (e.g., smooth union SDF or density).
- Variables are parameters of seam splats: `(μ, Σ, α, color)`.
- Minimize:
```math
\min_{\theta} \sum_{x \in \mathcal{X}} \|F_\theta(x) - F_{target}(x)\|^2 + \lambda R(\theta)
```

Where:
- `F_θ(x)` is density from seam splats (or seam+kept originals)
- `R(θ)` regularizes sizes, prevents splats from exploding, encourages coverage

This can be done in a **small seam region** with a few thousand samples `x`.

If you want this without autodiff:
- optimize only `α` and DC colors (linear least squares)
- keep positions fixed on the seam surface

---

## 4. Converting SDF/mesh back to splats

A reliable “resplat seam” implementation is the heart of the SDF join workflow.

### 4.1 Sampling the seam surface

Use **Poisson disk sampling** on the mesh with radius `r`:

- choose `r ≈ 1.5 * dx` (voxel size) for even coverage
- or `r ≈ 2 * median(σ_min)` for splat-scale coverage

Outputs:
- sample points `p_i`
- normals `n_i` (triangle normal or vertex normal)

### 4.2 Constructing a Gaussian covariance from a surface sample

Pick a local frame `(t1, t2, n)`:

- `n` is mesh normal
- `t1` can be any unit vector orthogonal to `n`
- `t2 = n × t1`

Choose standard deviations:
- `σ_n = normal_thickness` (small)
- `σ_t = tangent_radius` (larger)

Typical defaults:
- `σ_n = 0.25 * dx`
- `σ_t = 0.75 * dx` (or 1–2× based on desired softness)

Construct covariance in world:

```math
R = [t_1\; t_2\; n], \quad S = \mathrm{diag}(\sigma_t, \sigma_t, \sigma_n)
```

```math
\Sigma = R\,S^2\,R^T
```

Store as quaternion+scales:
- quaternion from `R`
- scales = `(σ_t, σ_t, σ_n)` (or log-scales if your format uses that)

### 4.3 Choosing opacity α

You want the seam band to contribute like a thin surface layer.

A practical heuristic:
- choose target peak density at surface ≈ `λ`
- set `α` to match average density contribution of neighbors

Simpler interactive default:
- `α = seam_alpha` (user knob)
- then post-normalize using a local density check:
  - evaluate density at sample point with the seam splat only:
    - `D_self(μ)=α`
  - scale α so that `D_total(μ)` hits `λ` on average.

### 4.4 Assigning appearance (color / SH) for seam splats

Most stable:
- set only DC term (`l=0`) and zero higher bands:
  - seam will be matte and won’t carry baked specular artifacts across the join

Compute seam color by blending A and B:

1. Compute a blend weight from SDFs:
   ```math
   w = \mathrm{smoothstep}(-k, k, s_B(p) - s_A(p))
   ```
   - `w≈0` closer to A, `w≈1` closer to B

2. Evaluate source colors:
   - fastest: use DC color from nearest splat / kNN average
   - better: evaluate SH at a canonical direction (e.g., viewer-independent approximation)
     - often just use DC anyway for seam

3. Set seam DC:
   ```math
   C = (1-w) C_A + w C_B
   ```
   ```math
   k_0 = C / Y_{00},\quad k_{i>0}=0
   ```

---

## 5. Practical node designs

### Node: `SplatJoinFeather`
Fast overlap blending.

**Inputs:** splats A, splats B  
**Params:** `blend_radius`, `distance_mode={kNN,SDF}`, `clamp_alpha`, `preserve_bands`  
**Output:** merged splats with per-splat weights, no new splats

---

### Node: `SplatJoinSDFSmoothUnion`
Recommended “SDF-like join”.

**Inputs:** splats A, splats B  
**Params:** `dx`, `lambda`, `smooth_k`, `seam_margin`, `poisson_r`, `sigma_n`, `sigma_t`, `seam_alpha`, `replace_band`  
**Outputs:** merged splats (+ optional seam mesh, seam SDF)

Pipeline (internal):
1. Build seam region
2. Compute `sA`, `sB`
3. Smooth union `sU`
4. Marching cubes on `sU=0` (optional output mesh)
5. Poisson sample mesh → seam splats
6. Fade original splats near seam and inject seam splats

---

### Node: `SplatSkirtBridge`
Fast “bridge splats” (no grids).

**Inputs:** splats A, splats B  
**Params:** `skirt_max_dist`, `step`, `sigma_n`, `sigma_t_add`, `alpha_profile`, `color_mode`  
**Output:** A+B+bridge splats (+ optional fade weights)

---

### Node: `SplatReplaceBand`
Utility.

**Inputs:** splats, SDF field (or seam surface)  
**Params:** `band_width`, `falloff`, `mode={fade,delete}`  
**Output:** modified splats

---

## 6. Implementation details in Rust

### 6.1 Data representation notes
- Keep splats in **SoA** for bandwidth and SIMD.
- Keep seam computations in **bricks** (e.g., 64³).
- For `dx` user control, compute Nx,Ny,Nz from seam bounds.

### 6.2 Acceleration structures
- kNN distances: use a hash grid (cell size ~ `2*median_sigma`) or KD-tree.
- SDF queries (for weights): store seam SDF grid and sample via trilinear interpolation.

### 6.3 Parallelism
- CPU: `rayon` over bricks / splats.
- WASM: no threads (usually) → chunked loops + progress callbacks.

---

## 7. Pitfalls and quality checks

### 7.1 Alpha compositing can hide geometric problems
Because splat rendering is order-dependent, seams can look “okay” at one view and wrong at another. Always check:

- grazing angles
- inside/outside views (if applicable)
- depth peeling / debug “density” render

### 7.2 Double density ridge
If you add seam splats but don’t fade originals, you’ll likely get a ridge.

Fix: always implement a **replace band**:
- fade originals near seam
- inject seam splats

### 7.3 Choosing λ and dx
- too high λ → seam mesh shrinks, gaps appear
- too low λ → seam mesh balloons, overlaps

Add UI:
- histogram of `D(x)` in seam
- preview of `sU=0` iso-surface

### 7.4 Maintaining coherence with lighting ops
If you delighted/relit A into B, seam splats should use the same appearance domain:
- if your insertion pipeline is “albedo-like + relight”, use **DC-only seam splats**.

---

## Appendix A. Smooth min / smooth union functions

### A.1 Polynomial smooth min (recommended)
```math
h=\mathrm{clamp}\left(0.5 + 0.5\frac{b-a}{k},0,1\right)
```

```math
\mathrm{smin}(a,b,k)=\mathrm{lerp}(b,a,h)-k\,h(1-h)
```

- `k` in world units.
- `k→0` approaches hard min.

### A.2 Log-sum-exp smooth min
```math
\mathrm{smin}(a,b,k) = -\frac{1}{k}\ln\left(e^{-ka}+e^{-kb}\right)
```

- smoother but uses `exp/log` (slower).

---

## Appendix B. Minimal Rust-like pseudocode snippets

### B.1 Smooth min

```rust
#[inline]
fn smin_poly(a: f32, b: f32, k: f32) -> f32 {
    if k <= 0.0 { return a.min(b); }
    let h = (0.5 + 0.5*(b - a)/k).clamp(0.0, 1.0);
    (b*(1.0 - h) + a*h) - k*h*(1.0 - h)
}
```

### B.2 Feather weights from SDF difference

```rust
#[inline]
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t*t*(3.0 - 2.0*t)
}

fn weights_from_sdf_diff(sdf_a: f32, sdf_b: f32, k: f32) -> (f32, f32) {
    let delta = sdf_a - sdf_b;
    let w_a = smoothstep(-k, k, -delta);
    (w_a, 1.0 - w_a)
}
```

### B.3 Fade original splats near seam SDF

```rust
fn fade_splats_near_seam(
    alpha: &mut [f32],
    sh_r: &mut [f32], sh_g: &mut [f32], sh_b: &mut [f32],
    sdf_u_at_mu: &[f32], // |sU(mu)| or sU(mu) depending on your scheme
    band: f32,
    n: usize, m: usize,
) {
    for i in 0..n {
        let d = sdf_u_at_mu[i].abs();
        let keep = smoothstep(band, 0.0, d); // 1 outside band, 0 at seam
        alpha[i] *= keep;
        for j in 0..m {
            let idx = i*m + j;
            sh_r[idx] *= keep;
            sh_g[idx] *= keep;
            sh_b[idx] *= keep;
        }
    }
}
```

### B.4 Constructing a seam splat from mesh point + normal

```rust
struct SeamSplat {
    mu: [f32;3],
    q: [f32;4],
    s: [f32;3],
    alpha: f32,
    sh_r: [f32;16],
    sh_g: [f32;16],
    sh_b: [f32;16],
}

fn make_seam_splat(p: [f32;3], n: [f32;3], color: [f32;3], sigma_t: f32, sigma_n: f32, alpha: f32) -> SeamSplat {
    let n = normalize3(n);
    let t1 = orthonormal_tangent(n);
    let t2 = cross3(n, t1);

    let r = mat3_from_cols(t1, t2, n);
    let q = quat_from_mat3(r);

    // Store scales (stddev)
    let s = [sigma_t, sigma_t, sigma_n];

    // DC-only SH
    let y00 = 0.28209479177387814_f32;
    let k0 = [color[0]/y00, color[1]/y00, color[2]/y00];
    let mut sh_r = [0.0f32;16];
    let mut sh_g = [0.0f32;16];
    let mut sh_b = [0.0f32;16];
    sh_r[0] = k0[0]; sh_g[0] = k0[1]; sh_b[0] = k0[2];

    SeamSplat { mu: p, q, s, alpha, sh_r, sh_g, sh_b }
}
```

---

## Recommended default for production

If you want one “best default” node that behaves like SDF smooth merge:

- implement **Local SDF smooth union + seam resplat**
- provide a fast preview mode:
  - lower seam resolution
  - larger `sigma_t` (so fewer seam splats)
- then a refine mode:
  - smaller `dx`
  - smaller `sigma_n`, smaller `poisson_r`
  - optionally compute normals from the seam mesh for better consistency

This matches the “Houdini node” workflow: fast interactive preview + refine.

