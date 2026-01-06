<a id="appendix-a-formula-cheat-sheet"></a>

## Appendix A. Formula Cheat Sheet

<a id="a-1-affine-transform-of-a-gaussian"></a>
### A.1 Affine transform of a Gaussian

```math
Given x' = A x + t:
μ' = A μ + t
Σ' = A Σ A^T
```

<a id="a-2-deformation-field-linearization"></a>
### A.2 Deformation field linearization

```math
Given x' = f(x), J = ∂f/∂x at x=μ:
μ' = f(μ)
Σ' = J Σ J^T
```

<a id="a-3-projection-jacobian-pinhole"></a>
### A.3 Projection Jacobian (pinhole)

```math
x_img = fx*(x/z) + cx
y_img = fy*(y/z) + cy
J_proj = [[fx/z, 0, -fx*x/z^2], [0, fy/z, -fy*y/z^2]]
```

<a id="a-4-screen-space-gaussian-evaluation"></a>
### A.4 Screen-space Gaussian evaluation

```math
Given μ_2D and Q = Σ_2D^{-1}:
w(p) = exp(-1/2 (p-μ_2D)^T Q (p-μ_2D))
a(p) = clamp( α * w(p), 0, a_max )
```

<a id="appendix-b-ply-interchange-template"></a>
## Appendix B. PLY Interchange Template

Below is an illustrative (not universal) PLY header template. Treat property names and ordering as an interchange contract; your importer should handle common variants.

```
ply
format binary_little_endian 1.0
element vertex N
property float x
property float y
property float z
property float nx
property float ny
property float nz
property float f_dc_0
property float f_dc_1
property float f_dc_2
property float f_rest_0
...
property float f_rest_44    # if L=3 (45 'rest' coeffs over RGB depends on packing)
property float opacity
property float scale_0
property float scale_1
property float scale_2
property float rot_0        # qw
property float rot_1        # qx
property float rot_2        # qy
property float rot_3        # qz
end_header
```

<a id="appendix-c-spherical-harmonics-rotation-notes"></a>
## Appendix C. Spherical Harmonics Rotation Notes

C.1 Real SH basis and ordering
There are multiple real-SH conventions in common use (signs, axis mapping, coefficient ordering). Your application should define one canonical internal ordering and explicitly convert at import/export.

C.2 Minimal correctness test
A high-leverage test is to construct a synthetic l=1 field whose color is proportional to the x-direction, rotate the splats by a known rotation, rotate SH coefficients, and verify the rendered lobe rotates accordingly. This catches basis/order mistakes quickly.

<a id="appendix-d-procedural-deformations-and-jacobians"></a>
## Appendix D. Procedural Deformations and Jacobians

For procedural modifiers, implement closed-form Jacobians where possible. This enables correct Σ' updates and stable results.

<a id="d-1-twist-about-z"></a>
### D.1 Twist (about Z)

```
# theta = k*z
x' = x*cos(theta) - y*sin(theta)
y' = x*sin(theta) + y*cos(theta)
z' = z

# J = d(x',y',z')/d(x,y,z) includes terms with dtheta/dz = k:
# dx'/dz = -k*(x*sin(theta) + y*cos(theta))
# dy'/dz =  k*(x*cos(theta) - y*sin(theta))
```

<a id="d-2-bend-simple-model"></a>
### D.2 Bend (simple model)

Bend can be implemented as a rotation whose angle depends on position along an axis. Derive J similarly to twist: differentiate both the rotation terms and the angle dependence.

<a id="d-3-noise-displacement"></a>
### D.3 Noise displacement

If f(x) = x + n(x) where n is a vector noise field, then J = I + ∂n/∂x. Finite differences can approximate ∂n/∂x if an analytic derivative is inconvenient. Use small step sizes and clamp resulting singular values to avoid extreme anisotropy.

<a id="appendix-e-further-reading-selected-topics"></a>
## Appendix E. Further Reading (selected topics)

The field is moving quickly. Below are representative topics you may want to consult:
• Core splatting and rasterization (original 3D Gaussian splatting)
• Anti-aliasing and scale robustness (Mip-Splatting; Analytic-Splatting)
• Sort-free / approximate order-independent rendering (Weighted Sum Rendering)
• Registration and multi-capture merging (GaussReg; GICP/ICP variants)
• Deformation (cage-based GSDeformer; proxy-free graph/Laplacian methods; mesh-attached frameworks)
• Physics-driven dynamics (PhysGaussian)
• Dynamic/4D representations and editing (4D Gaussian splatting; instructive 4D editors)
• Compression and streaming (SPZ; spatially ordered/quantized formats; self-organizing Gaussian grids)
• LOD hierarchies for large scenes (hierarchical/Octree-based Gaussian representations)


<a id="bibliography"></a>
## Bibliography

1. Kerbl, Kopanas, Leimkühler, Drettakis. "3D Gaussian Splatting for Real-Time Radiance Field Rendering." ACM TOG (SIGGRAPH), 2023. arXiv:2308.04079.
1. Yu et al. "Mip-Splatting: Alias-free 3D Gaussian Splatting." CVPR, 2024. arXiv:2311.16493.
1. Liang et al. "Analytic-Splatting: Anti-Aliased 3D Gaussian Splatting via Analytic Integration." ECCV, 2024. arXiv:2403.11056.
1. Hou et al. "Sort-free Gaussian Splatting via Weighted Sum Rendering." arXiv:2410.18931 (2024); appeared in ICLR 2025 workshop/materials.
1. Chang et al. "GaussReg: Fast 3D Registration with Gaussian Splatting." ECCV, 2024. arXiv:2407.05254.
1. Hanson et al. "PUP 3D-GS: Principled Uncertainty Pruning for 3D Gaussian Splatting." CVPR, 2025.
1. Morgenstern et al. "Compact 3D Scene Representation via Self-Organizing Gaussian Grids." ECCV, 2024. arXiv:2312.13299.
1. Ren et al. "Octree-GS: Towards Consistent Real-time Rendering with Level-of-Detail 3D Gaussian Splatting." arXiv:2403.17898.
1. Kerbl et al. "A Hierarchical 3D Gaussian Representation for Real-Time Rendering of Very Large Datasets." SIGGRAPH, 2024.
1. Huang et al. "GSDeformer: Direct, Real-time and Extensible Cage-based Deformation for 3D Gaussian Splatting." arXiv:2405.15491.
1. Xie et al. "PhysGaussian: Physics-Integrated 3D Gaussians for Generative Dynamics." arXiv:2311.12198.
1. Wu et al. "4D Gaussian Splatting for Real-Time Dynamic Scene Rendering." CVPR, 2024. arXiv:2310.08528.
1. Kim, Yoo, Sung. "Proxy-Free Gaussian Splats Deformation with Splat-Based Surface Estimation." arXiv:2511.19542 (2025).
1. Zhou et al. "DeMapGS: Simultaneous Mesh Deformation and Surface Attribute Mapping via Gaussian Splatting." arXiv:2512.10572 (2025).
1. Niantic Labs. "SPZ: File format for compressed 3D Gaussian splats." GitHub repository (nianticlabs/spz), 2024.
1. PlayCanvas. "SOG (Spatially Ordered Gaussians) format specification" and related Gaussian splat compression notes (developer.playcanvas.com), 2024–2025.
1. Nerfstudio Project. "gsplat" differentiable rasterization library documentation (docs.gsplat.studio), 2024–2025.