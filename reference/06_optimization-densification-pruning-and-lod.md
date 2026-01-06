<a id="6-optimization-densification-pruning-and-lod"></a>

## 6. Optimization, Densification, Pruning, and LOD

Even if your editor does not train splats from scratch, optimization-style refinement is useful after large edits (merge, heavy deformation, aggressive pruning). This section summarizes practical refinement and LOD strategies.

<a id="6-1-short-re-optimization-after-edits"></a>
### 6.1 Short re-optimization after edits

When you have access to source images/cameras, a short differentiable optimization can significantly improve quality after editing. Typical pattern:
• Freeze geometry-heavy parameters for stability (e.g., keep topology and only adjust opacity/color).
• Or do a staged refinement: adjust μ and opacity first, then covariance and SH.
• Add regularizers to prevent runaway scales and opacity spikes.
If you do not have training images, you can still run internal regularization passes (neighbor smoothing, density equalization) to stabilize results.

<a id="6-2-densification-splitting-and-cloning"></a>
### 6.2 Densification: splitting and cloning

Densification increases detail where needed. Two common operations:
• Clone: duplicate a splat (same Σ) and jitter μ slightly; used to add capacity without changing footprint.
• Split: replace one splat with multiple smaller ones; used when a splat is too large or covers multiple surfaces.
Heuristics for triggering densification:
• Large projected footprint (Σ_2D eigenvalues too large).
• High residual error or high gradient (if optimizing).
• High anisotropy ratio (stretch), especially after deformation.

<a id="6-3-pruning-and-decimation"></a>
### 6.3 Pruning and decimation

Pruning reduces memory and improves performance. Common pruning rules:
• Opacity threshold: drop splats with α below a small cutoff.
• Size threshold: drop splats whose contribution is always subpixel.
• Redundancy pruning in dense regions: keep only top-scoring splats per voxel.
Advanced pruning uses sensitivity / uncertainty scores computed from training-view residuals to drop many splats while preserving perceptual quality.

<a id="6-4-level-of-detail-lod-for-very-large-scenes"></a>
### 6.4 Level-of-detail (LOD) for very large scenes

A scalable application needs LOD to render and edit large captures.
Common approaches:
• Multi-resolution Gaussian hierarchies: train or build a pyramid of splats at different scales.
• Octree partitioning: store splats in spatial nodes; select nodes based on screen-space error.
• Chunk streaming: load only visible chunks; keep a coarse fallback for far regions.

<a id="practical-screen-space-lod-criterion"></a>
#### Practical screen-space LOD criterion:

Estimate each splat's projected footprint size (e.g., sqrt(max eigenvalue of Σ_2D)). If below a pixel threshold, you can skip it or replace with a coarser representation. Conversely, if a coarse node becomes too large on screen, refine to its children.