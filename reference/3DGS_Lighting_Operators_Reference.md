# 3D Gaussian Splat Lighting Operators
## Reference documentation for a Rust-based procedural 3DGS editing/processing application

**Scope:** Standard 3D Gaussian Splatting splats (μ, q, s, α, SH RGB).  
**Goal:** Provide implementation-ready operators for **delighting** (neutralizing baked lighting) and **relighting / cross-scene integration** (matching lighting to a target splat scene), plus the supporting math and engineering needed in a Houdini-like node graph.

---

## 0. What “lighting editing” means for standard 3DGS

A standard 3DGS splat stores **outgoing radiance as a function of view direction**:

```math
\mathbf{C}(\mathbf{d}) = \sum_{\ell=0}^{L}\sum_{m=-\ell}^{\ell} \mathbf{k}_{\ell m}\, Y_{\ell m}(\mathbf{d})
```

- `\mathbf{k}_{\ell m} ∈ ℝ^3` are RGB SH coefficients stored per splat.
- `Y_{\ell m}` are **real** spherical harmonics (common “graphics SH” basis).
- Typical `L=3` → 16 coefficients per color channel.

**Important:** This representation is *not* “albedo + lighting”. It is already the **baked appearance** under the capture illumination, plus whatever view-dependent effects the model learned. Any “delight” and “relight” operator on standard 3DGS is therefore an **approximate, pragmatic** decomposition.

The key to practical tools is to pick targets that are:
- stable (no explosions when coefficients are small),
- controllable (user parameters behave predictably),
- composable (operators chain cleanly in a procedural graph).

---

## 1. Conventions and canonical SH basis

### 1.1 Real SH ordering (recommended internal layout)

Use a single canonical ordering internally (convert on import/export):

For each `ℓ`, enumerate `m=-ℓ…ℓ`, and pack by increasing `ℓ`:

```
idx(ℓ,m) = ℓ*ℓ + (m + ℓ)
```

So for `L=3`:

| idx | (ℓ,m) |
|---:|:---:|
| 0 | (0,0) |
| 1 | (1,-1) |
| 2 | (1,0) |
| 3 | (1,1) |
| 4 | (2,-2) |
| 5 | (2,-1) |
| 6 | (2,0) |
| 7 | (2,1) |
| 8 | (2,2) |
| 9 | (3,-3) |
| 10 | (3,-2) |
| 11 | (3,-1) |
| 12 | (3,0) |
| 13 | (3,1) |
| 14 | (3,2) |
| 15 | (3,3) |

This matches common 3DGS implementations for *real SH*.

### 1.2 The real SH basis used by many 3DGS renderers (L≤2)

Let direction `d = (x,y,z)` be a **unit vector**.

Constants (standard “graphics SH”):

```text
Y00  = 0.28209479177387814

Y1-1 = 0.4886025119029199 * y
Y10  = 0.4886025119029199 * z
Y11  = 0.4886025119029199 * x

Y2-2 = 1.0925484305920792 * x*y
Y2-1 = 1.0925484305920792 * y*z
Y20  = 0.31539156525252005 * (3*z*z - 1)
Y21  = 1.0925484305920792 * x*z
Y22  = 0.5462742152960396 * (x*x - y*y)
```

> Many 3DGS evaluators use exactly these basis functions. If your renderer uses a different normalization or ordering, **keep your internal canonical layout** and add conversion functions.

### 1.3 DC term interpretation and conversion

If the shader evaluates:

```math
\mathbf{C}(\mathbf{d}) = \sum_i \mathbf{k}_i Y_i(\mathbf{d})
```

Then a constant color `K` should be stored as:

```math
\mathbf{k}_0 = \mathbf{K} / Y_{00}
```

and all higher coefficients zero.

So:

- **To extract the average (direction-independent) color**:
  ```math
  \mathbf{C}_{avg} \approx \mathbf{k}_0\, Y_{00}
  ```
- **To set a direction-independent color**:
  ```math
  \mathbf{k}_0 = \mathbf{C}\_{avg}/Y_{00},\quad \mathbf{k}_{i>0}=0
  ```

---

## 2. Data model in a Rust procedural engine

### 2.1 Per-splat attributes (standard 3DGS)

- `mu: Vec3` center
- `q: Quat` unit quaternion (orientation)
- `s: Vec3` axis stddev (or log-scale)
- `alpha: f32` opacity/weight
- `sh: [Vec3; (L+1)^2]` RGB SH coefficients (typically L=3)

### 2.2 Storage layout (SoA strongly recommended)

For performance, store as **structure-of-arrays**:

```rust
struct SplatsSoA {
    mu_x: Vec<f32>, mu_y: Vec<f32>, mu_z: Vec<f32>,
    q_w: Vec<f32>, q_x: Vec<f32>, q_y: Vec<f32>, q_z: Vec<f32>,
    s_x: Vec<f32>, s_y: Vec<f32>, s_z: Vec<f32>,
    alpha: Vec<f32>,
    // SH: packed as [coeff][channel][splat] or [splat][coeff][channel] depending on access pattern
    sh_r: Vec<f32>, // length = N * M
    sh_g: Vec<f32>, // length = N * M
    sh_b: Vec<f32>, // length = N * M
    // optional:
    n_x: Vec<f32>, n_y: Vec<f32>, n_z: Vec<f32>, // derived normals cache
}
```

Where `M = (L+1)^2`.

### 2.3 Node graph evaluation model (practical)

- Nodes are **pure transforms** on splat streams.
- Prefer **lazy evaluation** + caching:
  - Recompute only when upstream inputs or parameters change.
  - Cache expensive intermediate results (e.g., normals, environment SH probes).
- Most lighting operators are `O(N*M)` and trivially parallel.

---

## 3. Supporting operator: geometric transforms must include SH rotation

If a user rotates an object, the object’s baked directional appearance must rotate too. Otherwise you get the classic artifact: geometry rotates but highlights / gradients remain fixed in world/view space.

### 3.1 Geometry transform

For an object transform `T(x) = A x + t` (affine):

```math
\mu' = A\mu + t
```

Covariance transform (if you rebuild Σ):

```math
\Sigma' = A\,\Sigma\,A^T
```

If you store quaternion + scales, for rigid rotation `R`:

- `q' = q_R ⊗ q`
- scale unchanged if no scaling

### 3.2 SH rotation (core)

You want a new SH function:

```math
C'(\mathbf{d}) = C(R^T\,\mathbf{d})
```

For each band `ℓ`, SH coefficients transform via a band-specific matrix `D^ℓ(R)`:

```math
\mathbf{k'}_\ell = D^\ell(R)\,\mathbf{k}_\ell
```

- `ℓ=0`: invariant (`D^0 = [1]`)
- `ℓ=1`: rotates like a vector (with basis-order caveats)
- `ℓ=2,3`: requires a 5×5 or 7×7 rotation matrix (real SH)

### 3.3 Practical implementation strategy for a procedural app

#### Strategy A: Exact analytic band rotation (fastest per-splat; hardest to implement)
- Implement real-SH `D^ℓ(R)` (via Wigner D matrices + real-basis conversion).
- Best if you want perfect behavior and will reuse in many nodes.

#### Strategy B: **Numerical rotation matrix build once per object transform** (recommended)
If all splats share the same object rotation, build an `M×M` SH rotation matrix once and apply it to every splat.

**Idea:** Fit the rotated SH coefficients by least squares from samples on the sphere.

1. Choose `K ≥ M` sample directions `{d_i}` on the unit sphere (e.g., Fibonacci sphere).
2. Build basis matrix `B` where `B[i,j] = Y_j(d_i)` (K×M).
3. Precompute pseudoinverse `P = (BᵀB)⁻¹ Bᵀ` (M×K). This is constant for your sample set.
4. For rotation `R`:
   - Build `B_R` where `B_R[i,j] = Y_j(Rᵀ d_i)`.
   - Then `D(R) = P · B_R` (M×M).
5. Rotate any coefficient vector `k` by `k' = D(R) k`.

This avoids deriving Wigner D algebra, and is robust for L≤3.

**Cost:** per rotation, build `B_R` in `O(K*M)` and `D` in `O(M*K*M)`; with `M=16`, `K=32` this is tiny. Then per splat, apply `k' = D k` (`16×16` multiply per channel) which is fast.

#### Rust-like sketch: building D(R)

```rust
fn build_sh_rotation_matrix_l3(
    sample_dirs: &[[f32;3]],     // K
    pinv: &Vec<f32>,             // MxK row-major
    rot: [[f32;3];3],            // R
) -> Vec<f32> {                  // MxM row-major
    let k = sample_dirs.len();
    let m = 16usize;

    // B_R: KxM
    let mut b = vec![0.0f32; k*m];
    for (i, d) in sample_dirs.iter().enumerate() {
        // d' = R^T d
        let dp = [
            rot[0][0]*d[0] + rot[1][0]*d[1] + rot[2][0]*d[2],
            rot[0][1]*d[0] + rot[1][1]*d[1] + rot[2][1]*d[2],
            rot[0][2]*d[0] + rot[1][2]*d[1] + rot[2][2]*d[2],
        ];
        let y = eval_real_sh_l3(dp); // returns [f32;16] in your canonical order
        for j in 0..m { b[i*m + j] = y[j]; }
    }

    // D = P (MxK) * B_R (KxM) => MxM
    let mut dmat = vec![0.0f32; m*m];
    for r in 0..m {
        for c in 0..m {
            let mut sum = 0.0f32;
            for i in 0..k {
                sum += pinv[r*k + i] * b[i*m + c];
            }
            dmat[r*m + c] = sum;
        }
    }
    dmat
}
```

> Tip: For pure rigid-body transforms in your app, `D(R)` changes only when the object rotation changes—cache it per node instance.

---

## 4. Supporting operator: normal estimation (needed for physically-motivated delight/relight)

Standard 3DGS does not store normals. For lighting operators that need shading, estimate a normal per splat.

### 4.1 Fast normal from covariance “shortest axis”

If splats are surface-aligned surfels, they tend to be **flattened** along the surface, so the smallest axis approximates the surface normal.

1. Find `i = argmin(sx, sy, sz)` in **local ellipsoid axes**.
2. Convert quaternion to rotation matrix `R`.
3. The world-space axis `R[:,i]` is the normal direction (sign ambiguous).

```rust
fn estimate_normal_shortest_axis(q: [f32;4], s: [f32;3]) -> [f32;3] {
    let i = if s[0] <= s[1] && s[0] <= s[2] { 0 } else if s[1] <= s[2] { 1 } else { 2 };
    let r = quat_to_mat3(q); // columns are the rotated basis
    let mut n = [r[0][i], r[1][i], r[2][i]];
    // normalize just in case:
    n = normalize3(n);
    n
}
```

### 4.2 Normal smoothing and sign disambiguation (recommended)

Because the shortest axis gives a **line**, not an oriented vector, neighboring splats may flip signs.

Procedure:
1. Build a neighbor query structure (hash grid or KD-tree on μ).
2. For each splat, gather kNN.
3. Flip neighbor normals if `dot(n_i, n_j) < 0`.
4. Average and normalize.

Optionally orient consistently:
- If you have a known “outside” point (object centroid and assume outward), enforce `dot(n, μ - centroid) > 0`.
- If you have camera poses, orient toward camera that most strongly observed the splat.

---

## 5. Environment lighting representation and acquisition

Your lighting ops need a representation of “lighting conditions”:

- `EnvSH`: spherical harmonics coefficients of an environment / probe (RGB per coefficient)
- Possibly spatially varying probes: `EnvSH(x)`.

### 5.1 Projecting an environment map to SH

**Goal:** Convert cubemap/equirectangular HDR into SH coefficients `L[i]` (i=0..M-1, RGB).

Discrete projection:

```math
\mathbf{L}_j \approx \sum_{p} \mathbf{E}(\omega_p)\, Y_j(\omega_p)\, \Delta\Omega_p
```

For a cubemap texel at normalized face coords `(u,v)`, a standard solid-angle weight is:

```math
\Delta\Omega \propto \frac{1}{(1+u^2+v^2)^{3/2}}
```

Implementation tips:
- Precompute for each texel: direction `ω` and weight `ΔΩ`.
- Accumulate in `f64` then store as `f32`.
- Normalize by `\sum ΔΩ` so energy is consistent across resolutions.

### 5.2 Sampling environment from a 3DGS scene (probe-from-scene)

When you need “lighting of target scene at insertion point”:

- Render a **cubemap** from that point using your existing splat renderer (6 cameras with 90° FOV).
- Convert that cubemap to SH.

**Caveats (pragmatic):**
- This is not “true incident radiance” (there is occlusion and local geometry), but it is often adequate for harmonization.
- For stability, blur the cubemap (or keep only L≤2) before projection.

### 5.3 Spatial probes

Expose an operator that returns `EnvSH` at a position:

- `ProbeEnvSH(scene, position, cubemap_res, sh_order, blur_sigma, exposure_mode) -> EnvSH`

Cache these results—probing is much more expensive than coefficient scaling.

---

## 6. Delighting operator (remove/neutralize baked lighting)

Delighting produces splats that are visually closer to “intrinsic color” and are easier to match to new scenes.

### 6.1 What you can realistically achieve

With standard 3DGS you can reliably:
- remove most **directional gradients** and view-dependent sparkle
- reduce color cast/exposure mismatches
- create a stable “neutral” object for insertion

You cannot reliably:
- recover missing detail from deep baked shadows
- reconstruct true BRDF/specular behavior

### 6.2 Delight node API (recommended)

**Node:** `Delight3DGS`

**Inputs:**
- `Splats` (standard 3DGS)
- optional `EnvSH_source` (lighting probe for the source)
- optional `Normals` (if not provided, compute)

**Parameters:**
- `mode`:
  1. `Band0Only` (fast flatten)
  2. `SHRatioToNeutral` (ratio scaling)
  3. `IrradianceDivide` (normals + diffuse irradiance)
- `neutral_env`:
  - `UniformWhite` (only L00)
  - `CustomEnvSH`
- `eps`, `ratio_clamp_min/max`
- `dampen_high_bands` (0..1)
- `output_sh_order` (0..3)

**Outputs:**
- modified `Splats` (still standard 3DGS SH)

### 6.3 Mode A — Band0Only (fastest)

- Keep `k0`, zero `k_{i>0}`.
- Optional: mild local smoothing of `k0` over neighbors to reduce baked gradients.

This is “de-specularize + flatten”, not true delighting.

### 6.4 Mode B — SHRatioToNeutral (global coefficient transfer)

Treat the object’s SH as if it were “reflectance × lighting” in SH domain and transfer to a neutral lighting.

Let `L_src[i]` be the source lighting SH, and `L_neu[i]` neutral lighting SH.

For each coefficient (per channel):
```math
k'_i = k_i \cdot \mathrm{clamp}\!\left(\frac{L_{neu,i} + \varepsilon}{L_{src,i} + \varepsilon},\, t_{min}, t_{max}\right)
```

Practical settings:
- `ε = 1e-3 * max(|L_src|)` per channel
- clamp `t ∈ [0.1, 10]` (or tighter for stability)
- optionally dampen higher bands after scaling:
  ```math
  k'_{i>0} \leftarrow (1-\beta)k'_{i>0}
  ```

This is the most “procedural” delight: stable, fast, and reversible.

### 6.5 Mode C — IrradianceDivide (normals + diffuse irradiance)

This produces a more meaningful “albedo-like” DC term.

1. Extract baked average color:
   ```math
   C_{avg} \approx k_0\,Y_{00}
   ```
2. Compute diffuse irradiance `E(n)` from `EnvSH_source` and estimated normal `n`.
3. Set albedo:
   ```math
   \rho = \frac{C_{avg}}{E(n) + \varepsilon}
   ```
4. Output splat with DC set to `ρ` and (optionally) zero or damp higher bands.

#### Efficient L2 irradiance evaluation (9-coeff SH)

Using the common closed-form constants (quadratic in `n=(x,y,z)`), with `L` being the **lighting SH** coefficients in canonical order `[L00, L1-1, L10, L11, L2-2, L2-1, L20, L21, L22]`:

```text
c1 = 0.429043
c2 = 0.511664
c3 = 0.743125
c4 = 0.886227
c5 = 0.247708
```

Then:

```math
E(n) = c4 L00
     + 2c2 (L11 x + L1-1 y + L10 z)
     + c1 L22 (x^2 - y^2)
     + 2c1 (L2-2 xy + L21 xz + L2-1 yz)
     + c3 L20 z^2 - c5 L20
```

Implementation returns RGB irradiance.

#### Rust-like function

```rust
fn irradiance_from_env_sh_l2(n: [f32;3], l: &[[f32;3];9]) -> [f32;3] {
    let x = n[0]; let y = n[1]; let z = n[2];
    let c1 = 0.429043f32;
    let c2 = 0.511664f32;
    let c3 = 0.743125f32;
    let c4 = 0.886227f32;
    let c5 = 0.247708f32;

    // Helper: scale + add RGB
    let mut e = [0.0f32;3];
    let mut add = |a: f32, v: [f32;3]| { e[0]+=a*v[0]; e[1]+=a*v[1]; e[2]+=a*v[2]; };

    // Index mapping in canonical order:
    // 0:L00 1:L1-1 2:L10 3:L11 4:L2-2 5:L2-1 6:L20 7:L21 8:L22

    add(c4, l[0]);

    add(2.0*c2*x, l[3]); // L11
    add(2.0*c2*y, l[1]); // L1-1
    add(2.0*c2*z, l[2]); // L10

    add(c1*(x*x - y*y), l[8]);         // L22
    add(2.0*c1*x*y, l[4]);             // L2-2
    add(2.0*c1*y*z, l[5]);             // L2-1
    add(2.0*c1*x*z, l[7]);             // L21

    add(c3*z*z - c5, l[6]);            // L20 term

    // Clamp to non-negative (diffuse irradiance cannot be negative)
    e[0] = e[0].max(0.0);
    e[1] = e[1].max(0.0);
    e[2] = e[2].max(0.0);
    e
}
```

---

## 7. Relighting operator (match lighting to a target scene / probe)

Relighting updates splats so they appear under target illumination.

### 7.1 Relight node API (recommended)

**Node:** `Relight3DGS`

**Inputs:**
- `Splats` (either original or delighted)
- `EnvSH_source` (optional; needed for direct transfer)
- `EnvSH_target` (required)

**Parameters:**
- `mode`:
  1. `SHRatioTransfer` (fast cross-scene transfer)
  2. `DiffuseBakeFromAlbedo` (requires normals + albedo)
  3. `Hybrid` (diffuse DC + scaled higher bands)
- `eps`, `ratio_clamp_min/max`
- `high_band_gain` (0..1)
- `preserve_chroma` (optional color-preservation heuristic)

**Outputs:**
- modified `Splats` (standard 3DGS)

### 7.2 Mode A — SHRatioTransfer (cross-scene transfer)

If the object is still under its original lighting, apply:

```math
k'_i = k_i \cdot \mathrm{clamp}\!\left(\frac{L_{tgt,i} + \varepsilon}{L_{src,i} + \varepsilon}\right)
```

- Fastest relight-integration tool.
- Works best when:
  - lighting differences are mostly low-frequency
  - object is mostly diffuse / rough
- Does not explicitly use normals; shadows won’t be physically correct.

### 7.3 Mode B — DiffuseBakeFromAlbedo (physically motivated diffuse)

Assume you already have (or can estimate) an albedo-like color `ρ` per splat.

- Compute irradiance `E_tgt(n)` from target env SH and splat normal.
- Set the splat’s outgoing radiance to diffuse-only:
  ```math
  C = \rho \cdot E_{tgt}(n)
  ```
- Store in DC only:
  ```math
  k_0 = C / Y00,\quad k_{i>0}=0
  ```

This yields stable, consistent insertion for diffuse objects.

### 7.4 Mode C — Hybrid (recommended default for “looks like 3DGS”)

To preserve some “3DGS sparkle” (view-dependent detail) while still harmonizing lighting:

1. Compute new DC via diffuse bake:
   - `k0_new = (ρ * E_tgt(n)) / Y00`
2. Keep higher-order terms from the source splats, but scale and damp:
   - `k_{i>0,new} = high_band_gain * k_{i>0,src} * ratio_i`
   - or simply `high_band_gain * k_{i>0,src}` if you prefer to avoid ratio noise

This often looks better than diffuse-only, while still making the object’s gross lighting match.

---

## 8. Cross-scene insertion pipeline (practical “node recipe”)

A robust insertion pipeline for object `A` into target scene `B`:

1. **Extract object splats** (mask/crop nodes).
2. **Transform object** into target coordinates:
   - update μ, q, s
   - rotate SH by object rotation `R` (build `D(R)` once)
3. **Probe lighting**:
   - `L_src = ProbeEnvSH(A_scene, object_anchor_point)`
   - `L_tgt = ProbeEnvSH(B_scene, insertion_point)`
4. **Delight**:
   - `A_delit = Delight3DGS(A, L_src, mode=IrradianceDivide or SHRatioToNeutral)`
5. **Relight**:
   - `A_relit = Relight3DGS(A_delit, L_src=Neutral, L_tgt, mode=Hybrid)`
6. **Exposure match (optional but high impact)**:
   - compute log-luminance mean for `B` region and `A_relit`:
     ```math
     \mu = \mathrm{mean}(\log(\epsilon + Y))
     ```
     scale `A_relit` by `2^{(\mu_B - \mu_A)}`
7. **Merge**:
   - concatenate splats into a single buffer
   - renderer must globally depth-sort splats **every frame**

---

## 9. Exposure and color management node (recommended)

Even with correct relighting, capture pipelines differ. Add a node:

**Node:** `ExposureMatch3DGS`

**Method (simple and effective):**
- Compute luminance `Y = 0.2126 R + 0.7152 G + 0.0722 B` from DC color.
- Use log-average luminance for stability:
  ```math
  \mu = \exp\left(\frac{1}{N}\sum_i \log(\epsilon + Y_i)\right)
  ```
- Compute scale `g = \mu_{target} / \mu_{source}`.
- Multiply all SH coefficients by `g` (or only DC).

Also consider:
- `WhiteBalanceMatch`: scale channels based on average chromaticity.

---

## 10. Stability checklist (practical defaults)

- **Normalize quaternions** before building rotation matrices.
- **Clamp** ratio transfer:
  - `eps = 1e-3 * max(|L_src|)` per channel
  - clamp ratio to `[0.1, 10]` (or `[0.25, 4]` for very stable UX)
- **Dampen higher bands** after delighting:
  - `high_band_gain ∈ [0, 0.5]` is often enough
- **Clamp irradiance** to `>=0` before dividing (diffuse irradiance cannot be negative).
- **Clamp albedo** to `[0, 2]` (or `[0, 1]` if you want conservative output).

---

## 11. Performance notes for interactive + WASM targets

- Coefficient scaling is memory-bandwidth bound; SoA layout and SIMD help.
- Probing env SH by rendering cubemaps is the expensive part:
  - cache by `(scene_id, position_quantized, params_hash)`
  - update only when upstream scene changes
- Normal smoothing: use a hash grid; allow user to turn off or reduce kNN for speed.
- SH rotation using numeric `D(R)` build:
  - build `D(R)` once per object rotation, apply to all splats → cheap
- For WebAssembly:
  - avoid huge allocations; chunk processing (N in blocks)
  - consider L2-only ops in browser for speed; keep L3 for desktop

---

## 12. Validation & debug tools (high leverage)

Add debug views/nodes:

- `InspectSH`: visualize DC and band energy histograms.
- `ProbeViewer`: show probed cubemap and its SH reconstruction.
- `NormalViewer`: visualize estimated normals as colors.
- `BeforeAfter`: A/B compare of delight/relight.

Unit tests:

- SH basis sanity: reconstruct known functions (constant, linear).
- SH rotation sanity: rotate a known l=1 vector field and confirm energy preserved.
- Irradiance sanity: uniform env yields constant irradiance regardless of normal.
- Delight/relight round-trip: applying ratio to neutral then back approximates identity (within clamp losses).

---

## Appendix A — Minimal real SH evaluation stubs (L≤3)

Below are stubs you can fill in. Keep one canonical basis in the engine.

```rust
fn eval_real_sh_l2(d: [f32;3]) -> [f32;9] {
    let x=d[0]; let y=d[1]; let z=d[2];
    [
        0.28209479177387814,
        0.4886025119029199 * y,
        0.4886025119029199 * z,
        0.4886025119029199 * x,
        1.0925484305920792 * x*y,
        1.0925484305920792 * y*z,
        0.31539156525252005 * (3.0*z*z - 1.0),
        1.0925484305920792 * x*z,
        0.5462742152960396 * (x*x - y*y),
    ]
}

// L3 adds 7 more basis values (idx 9..15). Implement using your chosen real SH definition.
fn eval_real_sh_l3(_d: [f32;3]) -> [f32;16] {
    // TODO: implement L3 real SH basis in the same convention as your renderer.
    // Start by matching your renderer's eval_sh() code exactly.
    unimplemented!()
}
```

---

## Appendix B — “One-line” delight/relight core loops (SoA-friendly)

```rust
fn sh_ratio_transfer(
    sh_r: &mut [f32], sh_g: &mut [f32], sh_b: &mut [f32], // N*M
    lsrc: &[[f32;3]], ltgt: &[[f32;3]],                    // M x RGB
    eps: [f32;3], clamp_lo: f32, clamp_hi: f32,
    n: usize, m: usize,
) {
    for i in 0..n {
        for j in 0..m {
            let idx = i*m + j;
            let tr = ((ltgt[j][0] + eps[0]) / (lsrc[j][0] + eps[0])).clamp(clamp_lo, clamp_hi);
            let tg = ((ltgt[j][1] + eps[1]) / (lsrc[j][1] + eps[1])).clamp(clamp_lo, clamp_hi);
            let tb = ((ltgt[j][2] + eps[2]) / (lsrc[j][2] + eps[2])).clamp(clamp_lo, clamp_hi);
            sh_r[idx] *= tr;
            sh_g[idx] *= tg;
            sh_b[idx] *= tb;
        }
    }
}
```

---

## Appendix C — Recommended node set for lighting workflows

- `ProbeEnvSH` (scene → EnvSH at point)
- `EnvMapToSH` (HDR env → EnvSH)
- `EstimateNormals` (shortest axis)
- `SmoothNormals` (kNN + sign fix)
- `RotateSH` (object transform → apply D(R))
- `Delight3DGS` (Band0Only / SHRatioToNeutral / IrradianceDivide)
- `Relight3DGS` (SHRatioTransfer / DiffuseBake / Hybrid)
- `ExposureMatch3DGS`
- `MergeSplats` (concat only; renderer sorts globally each frame)

---

## Appendix D — Known limitations (so you can design UX around them)

- **Specular**: Standard 3DGS SH captures only low-frequency view effects; relighting can’t create sharp new highlights.
- **Shadows**: Without explicit visibility/geometry, new cast shadows are not physically correct.
- **Deep baked occlusion**: Delighting cannot reconstruct missing detail; it can only re-scale what exists.
- **Probe-from-scene is approximate**: The “environment” estimated from a scene contains scene appearance, not true incident light.

A good procedural UX acknowledges these limits:
- Provide `high_band_gain` sliders
- Provide `clamp` sliders
- Provide a “Diffuse-only bake” mode for maximum stability
- Optionally allow a proxy mesh/SDF input later for shadowing (separate operator)

