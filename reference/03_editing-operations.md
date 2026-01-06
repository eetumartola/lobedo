<a id="3-editing-operations"></a>

## 3. Editing Operations

This section defines the core edit operators your application should support and how each operator must update Gaussian parameters to remain correct. The emphasis is on operators that are composable (procedural stacks) and stable under repeated application.

<a id="3-1-selection-picking-and-grouping"></a>
### 3.1 Selection, picking, and grouping

Editing starts with selecting splats. Practical selection modes:
• Screen-space rectangle/lasso selection using projected ellipses.
• Ray picking (click): choose the splat that maximizes contribution along the camera ray.
• Volume selection: box/sphere/frustum selection in 3D.
• Attribute queries: select by opacity, scale, semantic tag, cluster ID, etc.

<a id="ray-gaussian-picking-analytic-best-t-along-the-ray"></a>
#### Ray–Gaussian picking (analytic best-t along the ray):

Given a ray r(t) = o + t d (d normalized) and a Gaussian N(μ, Σ), consider the quadratic form
E(t) = (r(t) - μ)^T Σ^{-1} (r(t) - μ).
The t that minimizes E(t) (maximizes Gaussian density along the ray) is:
t* = (d^T Σ^{-1} (μ - o)) / (d^T Σ^{-1} d).
Use t* > 0, then evaluate E(t*) as a score; the smallest E(t*) (or largest exp(-0.5 E)) wins.

<a id="practical-gpu-picking-strategy"></a>
#### Practical GPU picking strategy:

Reuse the renderer:
• Render an ID buffer (integer splat IDs) using the same depth order and alpha logic.
• Or render a 'max contribution' pass that stores the ID with maximal a_i per pixel.
This avoids duplicating projection math and automatically respects splat footprints.

<a id="3-2-rigid-and-affine-transforms"></a>
### 3.2 Rigid and affine transforms

For an affine transform x' = A x + t:
• Mean: μ' = A μ + t
• Covariance: Σ' = A Σ A^T
Translation affects only μ. Rotation and scale affect both μ and Σ.

<a id="editing-with-q-s-parameterization"></a>
#### Editing with (q, s) parameterization

If you store orientation q and scales s:
• For a pure rotation R_g, update q as q_new = q_g ⊗ q_old (or the opposite order depending on your convention), and μ_new = R_g μ_old + t.
• For uniform scale s_g, multiply scales: s_new = s_g * s_old, and μ_new = s_g μ_old + t.
• For non-uniform scale or general A, the safe route is:
  (1) reconstruct Σ, (2) compute Σ' = A Σ A^T, (3) decompose Σ' back into q and s.
This avoids subtle shear/reflection issues.

<a id="decomposing-into-rotation-and-scales-spd-case"></a>
#### Decomposing Σ' into rotation and scales (SPD case)

For a symmetric positive definite Σ', compute eigendecomposition:
Σ' = U diag(λ) U^T.
Then s = sqrt(λ), and R = U (with a determinant fix so det(R)=+1). Convert R to a quaternion (a stable method such as Shepperd's algorithm is recommended).

```
import numpy as np

def decompose_covariance(Sigma, eps=1e-8):
    # Sigma: 3x3 symmetric PSD
    # returns R (3x3), s (3,)
    lam, U = np.linalg.eigh(Sigma)
    lam = np.maximum(lam, eps)
    s = np.sqrt(lam)
    R = U
    # ensure a proper rotation (det=+1)
    if np.linalg.det(R) < 0:
        R[:, 0] *= -1
    return R, s
```

<a id="3-3-object-transforms-instancing-and-baking"></a>
### 3.3 Object transforms, instancing, and baking

For an editor, it is convenient to treat a selection or imported asset as an 'object' with a single transform gizmo. For correct rendering and processing you still need per-splat world-space values.
Two robust design patterns:
A) Bake on commit: during interactive manipulation, keep an object transform; when the user commits, apply A,t to all splats and reset the object transform to identity.
B) GPU transform stage: keep immutable object-local splats and per-instance transforms, and each frame run a compute kernel that writes transformed μ and Σ (and optionally rotated SH) into a unified world-space buffer used for sorting and rendering.
Pattern B enables instancing (many copies of an asset) while preserving global sorting.

<a id="3-4-rotating-spherical-harmonics-sh-correctly"></a>
### 3.4 Rotating spherical harmonics (SH) correctly

If your color model uses spherical harmonics, rotating an object requires rotating the SH coefficient vectors. SH coefficients transform under SO(3) rotations via Wigner D matrices. Coefficients from different degrees l do not mix; each band is rotated independently.

<a id="band-wise-rotation-rule"></a>
#### Band-wise rotation rule:

Let f_l be the coefficient vector for degree l (length 2l+1). Under rotation R:
f'_l = D^l(R) f_l,
where D^l(R) is the (2l+1)×(2l+1) Wigner D matrix (in the chosen real-SH basis). Degree 0 is invariant.

<a id="practical-implementation-strategies"></a>
#### Practical implementation strategies:

• Baking strategy (recommended for export/merge): apply SH rotation once when committing a transform.
• Runtime strategy (fast for single objects): evaluate SH using an inverse-rotated view direction in the object's local space. This avoids per-splat coefficient updates but complicates unified sorting when multiple differently rotated objects share one draw.
• Hybrid: bake on demand when an object is about to be merged into the global splat set.

<a id="reference-implementation-python-using-an-sh-rotation-library"></a>
#### Reference implementation (Python, using an SH rotation library):

```
# Conceptual example: rotate real SH up to L=3.
# NOTE: Different pipelines store SH coefficients in different orders (and different real-SH bases).
# Always confirm your coefficient ordering with a unit test.

import torch
from e3nn import o3

def rotate_sh(sh, R, L=3):
    # sh: [N, 3, (L+1)^2]
    # R: 3x3 rotation matrix
    angles = o3.matrix_to_angles(torch.tensor(R, dtype=torch.float32))
    out = sh.clone()
    idx = 0
    for l in range(0, L+1):
        dim = 2*l + 1
        band = sh[:, :, idx:idx+dim]
        if l == 0:
            out[:, :, idx:idx+dim] = band  # invariant
        else:
            D = o3.wigner_D(l, *angles)     # D shape: [dim, dim]
            out[:, :, idx:idx+dim] = torch.einsum("ij,ncj->nci", D, band)
        idx += dim
    return out
```

Coefficient ordering warning: file formats and renderers may store coefficients in a permuted order relative to math libraries. Treat ordering as part of your format spec and implement explicit permute/unpermute steps around rotation.

<a id="3-5-editing-opacity-and-color"></a>
### 3.5 Editing opacity and color

Opacity is often stored as a logit 'a' and converted by α = sigmoid(a) at render time. For editing, prefer operating in the stored domain to avoid saturation.
Common operations:
• Multiply opacity: a ← logit( clamp( k * sigmoid(a) ) )
• Fade selection in/out with a spatial falloff
• Recolor: edit DC term for diffuse changes; edit higher SH bands for view-dependent effects.

<a id="3-6-deletion-cutouts-and-boolean-style-operations"></a>
### 3.6 Deletion, cutouts, and boolean-style operations

Because splats are unstructured, many 'boolean' edits can be implemented as pruning or masking:
• Delete splats inside a selection volume.
• Soft cut: reduce opacity by a smooth signed-distance falloff.
• Slice/clip planes: drop splats on one side of a plane.
• Hollowing: keep only splats near a surface estimated by density gradients or a proxy mesh.

<a id="3-7-spatial-filters-and-procedural-modifiers"></a>
### 3.7 Spatial filters and procedural modifiers

Procedural editing becomes practical once you can compute per-splat local frames and neighborhoods. Useful core operators:
• Neighborhood smoothing of positions or scales (edge-preserving variants help).
• Denoising: prune tiny low-opacity splats; merge nearly-identical splats.
• Resampling: split large splats along principal axes; clone small splats for detail.
• Attribute painting: brush-based modifications in screen space using projected ellipse weights.
• Spatially varying transforms: twist, bend, noise displacement, taper, lattice/FFD.