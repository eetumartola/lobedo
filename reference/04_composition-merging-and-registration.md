<a id="4-composition-merging-and-registration"></a>

## 4. Composition, Merging, and Registration

Combining splat assets is fundamentally a data processing task: bring all splats into a common world frame, manage overlap and density, then render/operate on the merged set as one.

<a id="4-1-coordinate-alignment-and-registration"></a>
### 4.1 Coordinate alignment and registration

If the relative transform between two splat sets is known, apply it directly. If unknown, estimate it with registration.
Common registration options:
• ICP (Iterative Closest Point): fast and simple; works well when overlap is significant and an initial guess is reasonable.
• Generalized ICP (GICP): uses local covariances; often more robust.
• Feature-based coarse-to-fine: compute coarse alignment from downsampled geometry, then refine with image-guided or photometric alignment by rendering both sets.

<a id="practical-icp-pipeline-for-splats"></a>
#### Practical ICP pipeline for splats:

1) Extract a proxy point set (e.g., means of high-opacity splats) and optionally normals (from local PCA).
2) Downsample (voxel grid) for speed.
3) Run ICP/GICP to estimate A,t.
4) Validate by rendering overlap views and measuring photometric error; optionally refine by gradient-based optimization on A,t.

<a id="4-2-overlap-handling-avoid-double-density"></a>
### 4.2 Overlap handling: avoid 'double density'

Naively concatenating splat lists in overlapping regions can produce over-darkening or over-opaque areas because both sets explain the same surface. Overlap handling strategies:
• Importance-based pruning: compute a per-splat contribution score and drop redundant splats.
• Spatial thinning: in overlap voxels, keep only the best-K splats by score.
• Short re-optimization: after merging, run a brief optimization pass to re-balance opacity and color.
• Local blending seams: insert small 'connector' splats or adjust colors/opacities near boundaries.

<a id="a-practical-redundancy-score-heuristic"></a>
#### A practical redundancy score heuristic:

For each splat, combine:
• opacity (higher often more important),
• projected area (prevents keeping huge blurry splats),
• view coverage (how many cameras see it),
• local uniqueness (distance to nearest neighbors in feature space).
This is not the only choice; if you have access to training views, second-order sensitivity scores can be used to prune aggressively while preserving fidelity.

<a id="4-3-unified-buffer-and-global-sorting"></a>
### 4.3 Unified buffer and global sorting

After alignment and overlap handling, concatenate all splats into a single superset for rendering.
Key requirement: sort all splats together per camera view so alpha compositing respects true depth ordering across formerly separate assets.

<a id="4-4-hybrid-scenes-meshes-splats"></a>
### 4.4 Hybrid scenes: meshes + splats

To integrate splats with traditional geometry:
• Render opaque meshes first and write depth.
• During splat rendering, depth-test against the mesh depth buffer and reject splat fragments that are behind.
This provides correct 'mesh occludes splat' behavior. The inverse (splats occluding meshes) is harder unless splats are effectively opaque or you use a more complex transparency solution.

<a id="4-5-asset-level-composition-operations"></a>
### 4.5 Asset-level composition operations

Beyond merge, a toolchain usually needs:
• Extract sub-objects: cluster splats into parts (connected components, k-means in feature space).
• Duplicate/instance: create many copies with different transforms (GPU transform stage).
• Scatter: place assets procedurally on surfaces; align with normals; randomize scale/rotation.
• Bake: apply all transforms and output a single portable splat file.