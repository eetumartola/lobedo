# Image → Segmented 3D Gaussian Splats  
## Full implementation reference for a node‑based 3D geometry editor

---

## 0) What you’re actually building (and what it isn’t)

You are building a **monocular 2.5D lifting pipeline** that converts a *single RGB image* into a **set of segmented 3D Gaussian splats**.

**What it is**
- A *surface‑shell reconstruction*: visible geometry only
- Semantically segmented (one object → one splat group)
- Deterministic, editor‑friendly initialization of splats
- Designed for **editing**, not final photogrammetry accuracy

**What it is not**
- Not full 3D reconstruction (no hidden backsides)
- Not topology‑correct geometry
- Not physically grounded depth everywhere
- Not multi‑view consistent without further refinement

This distinction is critical for UX, documentation, and later extension nodes.

---

## 1) High‑level pipeline (stages + outputs)

1. **Depth inference** (DepthPro preferred)
2. **Segmentation inference** (SAM / SAM2)
3. **Mask aggregation** → single Segmentation ID map
4. **Resolution & coordinate alignment**
5. **Unprojection** (pixel + depth → 3D)
6. **Normal estimation**
7. **Gaussian initialization**
8. **Grouping by segment**
9. **Serialization / internal storage**

---

## 2) Node‑graph feature layout

### A) IO + ML nodes
1. **Image Input**
2. **Depth (Monocular)**
3. **Segmentation (Auto)**

### B) Fusion + geometry nodes
4. **Align Maps**
5. **Unproject**
6. **Normals From Depth**
7. **Init Gaussians**
8. **Split / Filter by Segment**

### C) Editing nodes
- Transform Segment
- Opacity / Color / SH controls
- Delete / Isolate / Merge
- Resample / Heal
- Export

Internally, prefer **one Gaussian buffer + segment_id attribute**, not many separate objects.

---

## 3) Runtime & inference strategy (Rust‑first)

- Use **`ort` (ONNX Runtime for Rust)** as primary backend
- Prefer GPU execution providers when available
- CPU fallback must always work

Why:
- ViT‑based SAM and DepthPro are large
- ONNX Runtime has the best kernel coverage
- Easier future WASM fallback via `tract` if needed

---

## 4) Depth stage

### Requirements
- Per‑pixel depth
- Prefer **metric scale**
- Prefer estimated focal length

### DepthPro advantages
- Metric depth
- Sharp boundaries
- Predicts focal length
- Good for object‑scale correctness

### Outputs
- `DepthMapF32`
- `Intrinsics { fx, fy, cx, cy }`

If metric depth is unavailable, expose a **Scale Calibration** node.

---

## 5) Segmentation stage (SAM / SAM2)

### Execution model
- Encoder: once per image
- Decoder: many times, batched

### Auto‑segmentation via grid prompting
1. Generate grid of points
2. Run decoder in batches
3. Filter masks by quality
4. Run NMS
5. Aggregate into `SegMap`

### SegMap aggregation strategies
- **Details‑on‑top** (small masks override)
- **Big‑objects‑win** (large masks override)

Expose as a parameter.

---

## 6) Canonical coordinate alignment

Define a canonical resolution **R0** (usually original image).

All data must be sampleable in R0:
- depth: bilinear
- segmentation: nearest
- color: direct

Never mix coordinate spaces implicitly.

---

## 7) Unprojection math

For pixel `(u, v)` with depth `Z`:

```
X = (u - cx) * Z / fx
Y = (v - cy) * Z / fy
Z = Z
```

This yields camera‑space coordinates.

Store in camera space; transform later if needed.

---

## 8) Normal estimation

### Fast baseline (recommended)
Depth‑gradient method:

```
P(u,v) = unproject(u,v)
dPdx = P(u+1,v) - P(u-1,v)
dPdy = P(u,v+1) - P(u,v-1)
N = normalize(cross(dPdx, dPdy))
```

Flip orientation if needed.

### Alternative
Local PCA in 3D (slower, smoother).

---

## 9) Sampling strategy

Avoid one‑Gaussian‑per‑pixel by default.

### Recommended
- Allocate a **budget per segment**
- Use:
  1. curvature / edge‑weighted sampling
  2. farthest‑point sampling

This balances detail and coverage.

---

## 10) Gaussian parameterization (standard 3DGS)

Per Gaussian:

- Position: `μ (3)`
- Scale (log): `scale_log (3)`
- Rotation: `quat (4)`
- Opacity (logit): `1`
- SH color (degree‑3): `48`

Total: **59 floats per Gaussian**

Covariance:
```
Σ = R · diag(exp(scale_log)^2) · Rᵀ
```

---

## 11) Rotation initialization

Align local splat axis to surface normal:

```
q = rotation_between(local_z, N)
```

Handle degenerate cases (N ≈ −Z).

---

## 12) Scale initialization

### Simple footprint‑based
At depth Z:

```
wx ≈ Z / fx
wy ≈ Z / fy
sx = k * wx
sy = k * wy
sz = τ * min(sx, sy)
```

Typical:
- k = 1.2–2.0
- τ = 0.05–0.2

Produces disk‑like splats.

---

## 13) Color & opacity initialization

### Color
- Use SH DC term from RGB
- Higher SH terms = 0

### Opacity
- Start low (e.g. α = 0.1)
- Store as logit: `log(α / (1−α)) ≈ −2.197`

---

## 14) Boundary cleanup

Depth discontinuities cause flying splats.

Mitigations:
- Drop samples with large depth gradients
- Reduce opacity near segmentation boundaries
- Optional dilation + falloff masks

---

## 15) Data model & grouping

Internally:
- One `GaussianSet`
- Each Gaussian has `segment_id`

Derive:
- segment ranges
- filtered views
- split outputs if required by node graph

This keeps transforms and rendering simple.

---

## 16) Export & interoperability

Export **standard‑ish 3DGS PLY**:
- x,y,z
- scale_0..2
- rot_0..3
- opacity
- SH coefficients

Set unused SH coefficients to zero.

---

## 17) Known limitations (be explicit)

- No backsides
- Occlusion holes
- Depth errors on glass/sky
- No topology guarantees

Design later nodes:
- Inflate / Backfill
- Heal
- Merge
- Multi‑view fuse

---

## 18) Minimal “best approach” summary

1. `ort` + GPU EP
2. DepthPro ONNX (fallback depth optional)
3. SAM2 tiny/small with grid prompting
4. Canonical alignment
5. Depth → normals → splats
6. Segment‑aware sampling
7. Store one buffer + segment IDs
8. Export standard PLY

---

## 19) Implementation checklist

- [ ] ONNX sessions (Depth, SAM encoder, SAM decoder)
- [ ] Map alignment utilities
- [ ] Normal prepass
- [ ] Segment‑aware sampler
- [ ] Gaussian init
- [ ] Editor transforms
- [ ] Renderer hookup
- [ ] PLY export
