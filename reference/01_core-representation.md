<a id="1-core-representation"></a>

## 1. Core Representation

A 3D Gaussian splat is a compact, explicit primitive used to approximate radiance and opacity in a scene. A scene is represented as an unordered set of Gaussians; rendering turns them into screen-space ellipses and alpha-composites their contributions. Editing requires transforming coupled parameters consistently.

<a id="1-1-notation-and-coordinate-conventions"></a>
### 1.1 Notation and coordinate conventions

• Vectors are column vectors. A 3D point is x ∈ R^3.
• A Gaussian has mean μ ∈ R^3 and covariance Σ ∈ R^{3×3} (symmetric positive semi-definite).
• World space: W. Object-local space: O. Camera/view space: C.
• A (world) affine transform is x' = A x + t with A ∈ R^{3×3} and t ∈ R^3.
• A deformation field is f: R^3 → R^3 with Jacobian J(x) = ∂f/∂x.

<a id="1-2-the-gaussian-primitive-used-for-splatting"></a>
### 1.2 The Gaussian primitive used for splatting

A Gaussian density (used as a soft footprint, not as a probability distribution) is commonly written:
G(x) = exp( -1/2 (x - μ)^T Σ^{-1} (x - μ) ).
Rendering attaches additional learnable attributes:
• Opacity α ∈ (0, 1)
• View-dependent color C(d) parameterized by spherical harmonics (SH) coefficients, where d is a view direction.

<a id="1-3-covariance-parameterization-spd-by-construction"></a>
### 1.3 Covariance parameterization (SPD by construction)

Directly storing Σ is error-prone because editing or optimization can produce invalid (non-PSD) matrices. A robust parameterization stores a rotation and axis-aligned scales, then reconstructs Σ.

<a id="recommended-stored-parameters-per-splat"></a>
#### Recommended stored parameters per splat:

| Attribute | Typical storage | Notes |
| --- | --- | --- |
| Mean μ | float32[3] | World-space or object-space center |
| Scale s | float32[3] (often log-scale) | Axis scales; store log(s) for unconstrained optimization |
| Orientation q | float32[4] quaternion (w,x,y,z) | Must be normalized before use |
| Opacity a | float32 (logit) | α = sigmoid(a) during rendering |
| SH coeffs | float32[3×(L+1)^2] | Typically L=3 (16 coeffs per channel) |
| Optional extras | normals/features/IDs | Selection, materials, semantic tags, etc. |

<a id="reconstruction-of-from-q-s"></a>
#### Reconstruction of Σ from (q, s):

Let R(q) be the 3×3 rotation matrix from the normalized quaternion q, and S = diag(sx, sy, sz). Two common equivalent forms:
• Σ = R S^2 R^T  (where S^2 = diag(sx^2, sy^2, sz^2))
• L = R S,  Σ = L L^T  (Cholesky-like factor used for stability and speed)

<a id="implementation-template-cpu-side-python-like"></a>
#### Implementation template (CPU-side, Python-like):

```
def quat_normalize(q):
    q = q / max(1e-12, (q*q).sum()**0.5)
    return q

def quat_to_rotmat(q_wxyz):
    # q = (w, x, y, z)
    w, x, y, z = quat_normalize(q_wxyz)
    return [[1-2*(y*y+z*z), 2*(x*y - w*z), 2*(x*z + w*y)],
            [2*(x*y + w*z), 1-2*(x*x+z*z), 2*(y*z - w*x)],
            [2*(x*z - w*y), 2*(y*z + w*x), 1-2*(x*x+y*y)]]

def build_covariance(q_wxyz, s_xyz):
    R = quat_to_rotmat(q_wxyz)
    sx, sy, sz = s_xyz
    S = [[sx,0,0],[0,sy,0],[0,0,sz]]
    # L = R @ S, Sigma = L @ L^T
    L = matmul(R, S)
    Sigma = matmul(L, transpose(L))
    return Sigma
```

<a id="1-4-view-dependent-color-via-spherical-harmonics"></a>
### 1.4 View-dependent color via spherical harmonics

Many splat pipelines encode view-dependent appearance using real spherical harmonics (SH). For maximum SH degree L, each color channel stores (L+1)^2 coefficients. A common choice is L=3 → 16 coefficients per channel (48 total).

The rendered color for view direction d (unit vector) is:
C(d) = Σ_{l=0..L} Σ_{m=-l..l} f_{l,m} Y_{l,m}(d)
where Y_{l,m} are SH basis functions and f_{l,m} are learned coefficients.

Important: when a Gaussian is rotated in world space, its SH coefficients must be rotated consistently or specular-like effects will appear 'stuck' in the old orientation.

<a id="1-5-practical-invariants-and-sanity-checks"></a>
### 1.5 Practical invariants and sanity checks

You can treat these as always-on assertions in debug builds:
• q is finite and normalized (||q|| ≈ 1).
• s is finite and clamped to [s_min, s_max] in render units.
• Σ is symmetric, and eigenvalues are ≥ 0 (or ≥ eps for numerical robustness).
• α = sigmoid(a) is clamped to [α_min, α_max] to avoid NaNs and sorting instability.
• SH coefficients are finite; DC term is within a reasonable range for your color space.