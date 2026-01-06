<a id="5-non-rigid-deformation-and-animation"></a>

## 5. Non-Rigid Deformation and Animation

Non-rigid deformation warps space. For splats, the correct update rule follows directly from how covariances transform under a local linear map. The core idea: approximate the deformation locally around each splat center with its Jacobian.

<a id="5-1-general-deformation-rule-jacobian-based-covariance-transform"></a>
### 5.1 General deformation rule (Jacobian-based covariance transform)

Let f(x) be a deformation field mapping original space to deformed space.
• Mean: μ' = f(μ)
• Covariance: Σ' = J Σ J^T, where J = ∂f/∂x evaluated at x=μ.
This is the same rule as affine transforms, applied locally using the first-order Taylor approximation.

<a id="5-2-updating-orientation-and-scales-from-the-jacobian"></a>
### 5.2 Updating orientation and scales from the Jacobian

If you store (q, s), you may want to update them directly rather than storing Σ'.
Given J, compute its polar decomposition:
J = R S, where R is a rotation (orthogonal, det=+1) and S is symmetric positive definite.
Then:
• Update orientation: q_new = quat(R) ⊗ q_old
• Update scales: s_new ≈ diag(S) ⊙ s_old in the local frame, or recompute Σ' and decompose for accuracy.

<a id="polar-decomposition-stable-numeric-template"></a>
#### Polar decomposition (stable numeric template):

```
import numpy as np

def polar_decomposition(J):
    # J = R S, with R orthogonal, S symmetric PSD
    U, s, Vt = np.linalg.svd(J)
    R = U @ Vt
    if np.linalg.det(R) < 0:
        U[:, -1] *= -1
        R = U @ Vt
        s[-1] *= -1
    S = Vt.T @ np.diag(s) @ Vt
    return R, S
```

<a id="5-3-example-twist-deformation-about-the-z-axis"></a>
### 5.3 Example: twist deformation about the Z axis

Twist is a canonical stress-test because it introduces shear terms in the Jacobian.
Define θ = k z and:
x' = x cosθ - y sinθ
y' = x sinθ + y cosθ
z' = z
Compute J(μ) and apply Σ' = J Σ J^T. If you update positions only, the surface often exhibits a 'venetian blind' artifact (ellipsoids do not align with the twisted surface).

<a id="5-4-deformation-drivers-how-to-obtain-f-and-j"></a>
### 5.4 Deformation drivers (how to obtain f and J)

Your application can support many deformation paradigms. The key is producing f(μ) and J(μ).

| Driver | How μ' is computed | How J is computed |
| --- | --- | --- |
| Analytic procedural field (bend/twist/taper/noise) | Apply closed-form mapping f(μ) | Closed-form Jacobian ∂f/∂x |
| FFD / lattice / cage | Interpolate from cage vertex transforms | Differentiate interpolation (or estimate via finite differences) |
| Skeleton skinning (LBS/DQS) | Blend bone transforms with weights | Blend bone linear parts; optionally derive local rotation via polar decomposition |
| Control points + RBF / MLS | Interpolate transform from sparse controls | Analytic Jacobian of the interpolant |
| Proxy mesh binding (barycentric) | Move with deformed mesh triangle/tet | Use triangle/tet deformation gradient |
| Physics simulation (MPM/FEM/cloth) | Follow simulated particle position | Use deformation gradient F from the simulator |
| Proxy-free splat graph methods | Solve deformation on a splat neighborhood graph | Implicitly defined by graph Laplacian / local fits |

<a id="5-5-handling-extreme-deformation-anisotropy-control-and-splitting"></a>
### 5.5 Handling extreme deformation: anisotropy control and splitting

Large J can create extremely stretched ellipsoids, causing holes, banding, or instability.
Recommended monitoring:
• Compute eigenvalues of Σ' (or singular values of J). Track ratio r = sqrt(λ_max/λ_min).
• If r exceeds a threshold (e.g., 10–20), trigger corrective action.
Corrective actions:
• Clamp singular values of J (limits stretch).
• Split the splat along the principal stretch direction into multiple children with reduced scales.
• Re-distribute opacity among children to preserve approximate density.

<a id="long-axis-splitting-template"></a>
#### Long-axis splitting template:

```
# Given Sigma (or L), find principal axis and split into K children
# 1) principal axis v = eigenvector of Sigma with largest eigenvalue
# 2) create K centers mu_i = mu + offsets along v (e.g., evenly spaced within +/- a*r)
# 3) reduce the largest scale by ~K, keep others, adjust opacity so total coverage stays similar
```

<a id="5-6-sh-handling-under-deformation"></a>
### 5.6 SH handling under deformation

Under non-rigid deformation, view-dependent appearance can change in complex ways.
Practical options:
• Ignore SH rotation for small deformations (fast, often acceptable).
• Use the rotational part R from polar decomposition of J and rotate SH coefficients by R.
• If you use runtime view-direction tricks for rigid objects, you can analogously update the local view direction per splat, but this is more complex and rarely worth it.

<a id="5-7-animation-and-time-varying-splats"></a>
### 5.7 Animation and time-varying splats

For dynamic content, you can represent time in several ways:
• Keyframed splats: store μ(t), q(t), s(t), SH(t) at discrete frames; interpolate.
• Deformation field over time: store a canonical splat set plus a learned or procedural deformation field f(x,t).
• Full 4D Gaussians (x,y,z,t): treat each primitive as a spatiotemporal Gaussian and render with time as an additional dimension. This is used in 4D Gaussian splatting methods for real-time dynamic scenes.