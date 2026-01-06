<a id="2-rendering-and-rasterization-pipeline"></a>

## 2. Rendering and Rasterization Pipeline

A real-time renderer for Gaussians typically follows a 'project → bound → sort → tile → blend' pipeline. Even if your application focuses on editing and processing, a correct renderer is essential for feedback and for validating transforms.

<a id="2-1-camera-transform-and-3d-2d-covariance-projection"></a>
### 2.1 Camera transform and 3D → 2D covariance projection

Let the world-to-camera transform be a 4×4 matrix V. In camera space, a mean transforms as μ_c = V μ (homogeneous). The covariance transforms with the linear part of the transform:
Σ_c = R_v Σ R_v^T where R_v is the 3×3 rotation part of V.

To project a 3D Gaussian into screen space, linearize the perspective projection around μ_c. Let π: R^3 → R^2 be the projection (including intrinsics). The 2×3 Jacobian at μ_c is J_proj = ∂π/∂x.

<a id="pinhole-camera-jacobian-at-camera-space-point-x-y-z"></a>
#### Pinhole camera Jacobian (at camera-space point (x, y, z)):

```
# Intrinsics: fx, fy, cx, cy; assuming x' = fx*(x/z)+cx, y' = fy*(y/z)+cy
J_proj = [[ fx/z,     0.0, -fx*x/(z*z) ],
          [   0.0,  fy/z,  -fy*y/(z*z) ]]
```

The resulting 2D covariance in pixel space is:
Σ_2D = J_proj Σ_c J_proj^T.
Many implementations add a small diagonal term to Σ_2D (or to the conic) to avoid numerical instability for very small or near-degenerate Gaussians.

<a id="2-2-screen-space-ellipse-conic-form-and-bounding-box"></a>
### 2.2 Screen-space ellipse, conic form, and bounding box

Given Σ_2D (2×2), the Gaussian footprint in screen space is an ellipse. Rendering often uses the conic matrix Q = Σ_2D^{-1} (or a scaled variant) and evaluates weights via:
w(p) = exp(-1/2 (p - μ_2D)^T Q (p - μ_2D)).
To accelerate, compute a conservative axis-aligned bounding box that encloses k-sigma of the ellipse (k≈2 to 3 is common), then only evaluate pixels in that box.

<a id="stable-bounding-radius-from-eigenvalues"></a>
#### Stable bounding radius from eigenvalues:

If Σ_2D has eigenvalues λ1, λ2, the ellipse radii for k-sigma are r1 = k*sqrt(λ1), r2 = k*sqrt(λ2). A conservative AABB half-extent is simply max(r1, r2) along both axes, or you can rotate the axes to get tighter bounds.

<a id="2-3-depth-sorting-and-alpha-compositing"></a>
### 2.3 Depth sorting and alpha compositing

Gaussians are typically semi-transparent, so standard z-buffer compositing is insufficient. The usual approach is alpha compositing in a strict depth order (front-to-back is common for early-out).

Front-to-back accumulation per pixel:
T_0 = 1
C = 0
For splats i sorted nearest→farthest:
  a_i = α_i * w_i  (possibly clamped)
  C += T * a_i * c_i
  T *= (1 - a_i)
Stop when T is below a threshold (e.g., T < 1e-3).

<a id="2-4-tiling-and-gpu-execution-model"></a>
### 2.4 Tiling and GPU execution model

A practical GPU architecture uses:
• A preprocessing kernel to compute μ_2D, Σ_2D (or Q), and a tile AABB per splat.
• A binning step that appends each splat index into all tiles it overlaps.
• A per-tile sort (or a global sort plus segmented lists) by depth.
• A raster kernel: one thread block per tile; each block processes its splat list and accumulates pixels.

<a id="2-5-multiple-objects-and-the-unified-sorting-requirement"></a>
### 2.5 Multiple objects and the unified-sorting requirement

If you render splat objects in separate draw calls, their internal sorting is correct but the combined result is not: one whole object may appear in front of another regardless of true depth ordering. For correct compositing between objects, treat the entire scene as one splat set for the purposes of sorting and blending (a unified buffer / single-pass design).

<a id="2-6-anti-aliasing-and-scale-generalization"></a>
### 2.6 Anti-aliasing and scale generalization

Vanilla splatting can exhibit aliasing, over-blurring, or 'dilation' artifacts when rendered at a different scale from training. Two widely used improvements are:
• Mip-Splatting: constrains 3D Gaussian sizes based on sampling frequency and introduces a 2D MIP filter.
• Analytic-Splatting: analytically approximates the pixel-area integral of the Gaussian footprint for better anti-aliasing.
If your application targets high-quality offline previews or arbitrary zoom, consider supporting one of these approaches in your renderer.

<a id="2-7-reducing-or-eliminating-sorting-cost"></a>
### 2.7 Reducing or eliminating sorting cost

Sorting can dominate frame time for millions of splats. Options:
• Faster GPU radix sort, depth key quantization, and sorting only when the camera changes.
• Hierarchical / LOD representations that reduce splat count at distance.
• Approximate order-independent rendering (OIT) or sort-free formulations such as Weighted Sum Rendering (WSR) that replace strict alpha compositing with a commutative approximation. These can be attractive on mobile or for massive scenes, but they can change the look of semi-transparent regions.

<a id="2-8-practical-guard-rails-for-a-stable-renderer"></a>
### 2.8 Practical guard rails for a stable renderer

Common numerical pitfalls and mitigations:
• Clamp z (depth) to a minimum near-plane value before using 1/z or 1/z^2 in J_proj.
• Add a small diagonal term to Σ_2D before inversion to avoid singular conics.
• Clamp exponent arguments when evaluating exp(-0.5*...) to avoid underflow/overflow.
• Clamp per-pixel alpha contribution a_i to a maximum (e.g., 0.99) to keep transmittance well-behaved.
• Use a consistent coordinate convention for view direction d used by SH evaluation (camera-to-splat vs splat-to-camera) and document it.