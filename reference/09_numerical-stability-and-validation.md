<a id="9-numerical-stability-and-validation"></a>

## 9. Numerical Stability and Validation

Gaussian splatting is sensitive to small numerical errors because it relies on matrix inversions, exponentials, and sorted alpha compositing. Build validation and test harnesses early.

<a id="9-1-common-numerical-pitfalls"></a>
### 9.1 Common numerical pitfalls

• Unnormalized quaternions → invalid rotations and exploding covariances.
• Negative eigenvalues from numerical error → NaNs when taking sqrt.
• Nearly singular Σ_2D → unstable conic inversion and large footprints.
• Very small z → 1/z^2 blowups in projection Jacobian.
• Float32 depth sorting collisions → flicker or popping.
• Mixed coordinate conventions for SH evaluation → lighting rotates incorrectly.

<a id="9-2-recommended-clamping-and-epsilon-choices"></a>
### 9.2 Recommended clamping and epsilon choices

Typical safe values (tune to your scale):
• eps_eig = 1e-8 (for eigenvalue clamp)
• near_plane = 0.01 to 0.1 (scene-dependent)
• alpha_min = 1e-4, alpha_max = 1 - 1e-4
• scale_min = 1e-4, scale_max = large but bounded (prevents single splat covering the whole screen)

<a id="9-3-validation-test-suite-high-leverage"></a>
### 9.3 Validation test suite (high leverage)

Add unit tests and integration tests:
• Transform round-trip: apply (A,t) then (A^{-1}, -A^{-1}t) → recover μ and Σ (within tolerance).
• Covariance SPD: eigenvalues always ≥ 0 after every operator.
• SH rotation: rotate a synthetic directional lobe and verify it rotates as expected.
• Rendering determinism: fixed camera and fixed dataset → identical images across runs (within floating-point noise).
• Stress tests: splats near the camera, extreme scales, extreme anisotropy, degenerate cases.

<a id="9-4-debug-visualization-buffers"></a>
### 9.4 Debug visualization buffers

In addition to color rendering, provide:
• Splat ID buffer (for picking)
• Depth buffer (splat depth key)
• Alpha/transmittance buffer (where saturation occurs)
• Footprint size heatmap (projected eigenvalues)
• Anisotropy heatmap (scale ratio)
These dramatically reduce iteration time when building editing tools.