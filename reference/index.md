# Index

This package is a mechanical split of the DOCX reference into Markdown files with explicit anchors for stable linking.

## Files

- [Front matter](00_front_matter.md#table-of-contents)
- [1. Core Representation](01_core-representation.md#1-core-representation)
- [2. Rendering and Rasterization Pipeline](02_rendering-and-rasterization-pipeline.md#2-rendering-and-rasterization-pipeline)
- [3. Editing Operations](03_editing-operations.md#3-editing-operations)
- [4. Composition, Merging, and Registration](04_composition-merging-and-registration.md#4-composition-merging-and-registration)
- [5. Non-Rigid Deformation and Animation](05_non-rigid-deformation-and-animation.md#5-non-rigid-deformation-and-animation)
- [6. Optimization, Densification, Pruning, and LOD](06_optimization-densification-pruning-and-lod.md#6-optimization-densification-pruning-and-lod)
- [7. Compression, Storage, and Streaming](07_compression-storage-and-streaming.md#7-compression-storage-and-streaming)
- [8. Application Architecture](08_application-architecture.md#8-application-architecture)
- [9. Numerical Stability and Validation](09_numerical-stability-and-validation.md#9-numerical-stability-and-validation)
- [Appendices](10_appendices.md#appendix-a-formula-cheat-sheet)

## Detailed contents

### 00_front_matter.md

- [Table of Contents](00_front_matter.md#table-of-contents)

### 01_core-representation.md

- [1. Core Representation](01_core-representation.md#1-core-representation)
  - [1.1 Notation and coordinate conventions](01_core-representation.md#1-1-notation-and-coordinate-conventions)
  - [1.2 The Gaussian primitive used for splatting](01_core-representation.md#1-2-the-gaussian-primitive-used-for-splatting)
  - [1.3 Covariance parameterization (SPD by construction)](01_core-representation.md#1-3-covariance-parameterization-spd-by-construction)
    - [Recommended stored parameters per splat:](01_core-representation.md#recommended-stored-parameters-per-splat)
    - [Reconstruction of Σ from (q, s):](01_core-representation.md#reconstruction-of-from-q-s)
    - [Implementation template (CPU-side, Python-like):](01_core-representation.md#implementation-template-cpu-side-python-like)
  - [1.4 View-dependent color via spherical harmonics](01_core-representation.md#1-4-view-dependent-color-via-spherical-harmonics)
  - [1.5 Practical invariants and sanity checks](01_core-representation.md#1-5-practical-invariants-and-sanity-checks)

### 02_rendering-and-rasterization-pipeline.md

- [2. Rendering and Rasterization Pipeline](02_rendering-and-rasterization-pipeline.md#2-rendering-and-rasterization-pipeline)
  - [2.1 Camera transform and 3D → 2D covariance projection](02_rendering-and-rasterization-pipeline.md#2-1-camera-transform-and-3d-2d-covariance-projection)
    - [Pinhole camera Jacobian (at camera-space point (x, y, z)):](02_rendering-and-rasterization-pipeline.md#pinhole-camera-jacobian-at-camera-space-point-x-y-z)
  - [2.2 Screen-space ellipse, conic form, and bounding box](02_rendering-and-rasterization-pipeline.md#2-2-screen-space-ellipse-conic-form-and-bounding-box)
    - [Stable bounding radius from eigenvalues:](02_rendering-and-rasterization-pipeline.md#stable-bounding-radius-from-eigenvalues)
  - [2.3 Depth sorting and alpha compositing](02_rendering-and-rasterization-pipeline.md#2-3-depth-sorting-and-alpha-compositing)
  - [2.4 Tiling and GPU execution model](02_rendering-and-rasterization-pipeline.md#2-4-tiling-and-gpu-execution-model)
  - [2.5 Multiple objects and the unified-sorting requirement](02_rendering-and-rasterization-pipeline.md#2-5-multiple-objects-and-the-unified-sorting-requirement)
  - [2.6 Anti-aliasing and scale generalization](02_rendering-and-rasterization-pipeline.md#2-6-anti-aliasing-and-scale-generalization)
  - [2.7 Reducing or eliminating sorting cost](02_rendering-and-rasterization-pipeline.md#2-7-reducing-or-eliminating-sorting-cost)
  - [2.8 Practical guard rails for a stable renderer](02_rendering-and-rasterization-pipeline.md#2-8-practical-guard-rails-for-a-stable-renderer)

### 03_editing-operations.md

- [3. Editing Operations](03_editing-operations.md#3-editing-operations)
  - [3.1 Selection, picking, and grouping](03_editing-operations.md#3-1-selection-picking-and-grouping)
    - [Ray–Gaussian picking (analytic best-t along the ray):](03_editing-operations.md#ray-gaussian-picking-analytic-best-t-along-the-ray)
    - [Practical GPU picking strategy:](03_editing-operations.md#practical-gpu-picking-strategy)
  - [3.2 Rigid and affine transforms](03_editing-operations.md#3-2-rigid-and-affine-transforms)
    - [Editing with (q, s) parameterization](03_editing-operations.md#editing-with-q-s-parameterization)
    - [Decomposing Σ' into rotation and scales (SPD case)](03_editing-operations.md#decomposing-into-rotation-and-scales-spd-case)
  - [3.3 Object transforms, instancing, and baking](03_editing-operations.md#3-3-object-transforms-instancing-and-baking)
  - [3.4 Rotating spherical harmonics (SH) correctly](03_editing-operations.md#3-4-rotating-spherical-harmonics-sh-correctly)
    - [Band-wise rotation rule:](03_editing-operations.md#band-wise-rotation-rule)
    - [Practical implementation strategies:](03_editing-operations.md#practical-implementation-strategies)
    - [Reference implementation (Python, using an SH rotation library):](03_editing-operations.md#reference-implementation-python-using-an-sh-rotation-library)
  - [3.5 Editing opacity and color](03_editing-operations.md#3-5-editing-opacity-and-color)
  - [3.6 Deletion, cutouts, and boolean-style operations](03_editing-operations.md#3-6-deletion-cutouts-and-boolean-style-operations)
  - [3.7 Spatial filters and procedural modifiers](03_editing-operations.md#3-7-spatial-filters-and-procedural-modifiers)

### 04_composition-merging-and-registration.md

- [4. Composition, Merging, and Registration](04_composition-merging-and-registration.md#4-composition-merging-and-registration)
  - [4.1 Coordinate alignment and registration](04_composition-merging-and-registration.md#4-1-coordinate-alignment-and-registration)
    - [Practical ICP pipeline for splats:](04_composition-merging-and-registration.md#practical-icp-pipeline-for-splats)
  - [4.2 Overlap handling: avoid 'double density'](04_composition-merging-and-registration.md#4-2-overlap-handling-avoid-double-density)
    - [A practical redundancy score heuristic:](04_composition-merging-and-registration.md#a-practical-redundancy-score-heuristic)
  - [4.3 Unified buffer and global sorting](04_composition-merging-and-registration.md#4-3-unified-buffer-and-global-sorting)
  - [4.4 Hybrid scenes: meshes + splats](04_composition-merging-and-registration.md#4-4-hybrid-scenes-meshes-splats)
  - [4.5 Asset-level composition operations](04_composition-merging-and-registration.md#4-5-asset-level-composition-operations)

### 05_non-rigid-deformation-and-animation.md

- [5. Non-Rigid Deformation and Animation](05_non-rigid-deformation-and-animation.md#5-non-rigid-deformation-and-animation)
  - [5.1 General deformation rule (Jacobian-based covariance transform)](05_non-rigid-deformation-and-animation.md#5-1-general-deformation-rule-jacobian-based-covariance-transform)
  - [5.2 Updating orientation and scales from the Jacobian](05_non-rigid-deformation-and-animation.md#5-2-updating-orientation-and-scales-from-the-jacobian)
    - [Polar decomposition (stable numeric template):](05_non-rigid-deformation-and-animation.md#polar-decomposition-stable-numeric-template)
  - [5.3 Example: twist deformation about the Z axis](05_non-rigid-deformation-and-animation.md#5-3-example-twist-deformation-about-the-z-axis)
  - [5.4 Deformation drivers (how to obtain f and J)](05_non-rigid-deformation-and-animation.md#5-4-deformation-drivers-how-to-obtain-f-and-j)
  - [5.5 Handling extreme deformation: anisotropy control and splitting](05_non-rigid-deformation-and-animation.md#5-5-handling-extreme-deformation-anisotropy-control-and-splitting)
    - [Long-axis splitting template:](05_non-rigid-deformation-and-animation.md#long-axis-splitting-template)
  - [5.6 SH handling under deformation](05_non-rigid-deformation-and-animation.md#5-6-sh-handling-under-deformation)
  - [5.7 Animation and time-varying splats](05_non-rigid-deformation-and-animation.md#5-7-animation-and-time-varying-splats)

### 06_optimization-densification-pruning-and-lod.md

- [6. Optimization, Densification, Pruning, and LOD](06_optimization-densification-pruning-and-lod.md#6-optimization-densification-pruning-and-lod)
  - [6.1 Short re-optimization after edits](06_optimization-densification-pruning-and-lod.md#6-1-short-re-optimization-after-edits)
  - [6.2 Densification: splitting and cloning](06_optimization-densification-pruning-and-lod.md#6-2-densification-splitting-and-cloning)
  - [6.3 Pruning and decimation](06_optimization-densification-pruning-and-lod.md#6-3-pruning-and-decimation)
  - [6.4 Level-of-detail (LOD) for very large scenes](06_optimization-densification-pruning-and-lod.md#6-4-level-of-detail-lod-for-very-large-scenes)
    - [Practical screen-space LOD criterion:](06_optimization-densification-pruning-and-lod.md#practical-screen-space-lod-criterion)

### 07_compression-storage-and-streaming.md

- [7. Compression, Storage, and Streaming](07_compression-storage-and-streaming.md#7-compression-storage-and-streaming)
  - [7.1 PLY schema (common interchange)](07_compression-storage-and-streaming.md#7-1-ply-schema-common-interchange)
    - [Minimal recommended internal schema (format-agnostic):](07_compression-storage-and-streaming.md#minimal-recommended-internal-schema-format-agnostic)
  - [7.2 Quantization and chunked compression](07_compression-storage-and-streaming.md#7-2-quantization-and-chunked-compression)
  - [7.3 Specialized compressed formats](07_compression-storage-and-streaming.md#7-3-specialized-compressed-formats)
  - [7.4 GPU memory layout](07_compression-storage-and-streaming.md#7-4-gpu-memory-layout)
  - [7.5 Streaming and out-of-core editing](07_compression-storage-and-streaming.md#7-5-streaming-and-out-of-core-editing)

### 08_application-architecture.md

- [8. Application Architecture](08_application-architecture.md#8-application-architecture)
  - [8.1 Recommended core subsystems](08_application-architecture.md#8-1-recommended-core-subsystems)
  - [8.2 Non-destructive editing model](08_application-architecture.md#8-2-non-destructive-editing-model)
  - [8.3 GPU pipeline integration for interactive editing](08_application-architecture.md#8-3-gpu-pipeline-integration-for-interactive-editing)
  - [8.4 Undo/redo at scale](08_application-architecture.md#8-4-undo-redo-at-scale)

### 09_numerical-stability-and-validation.md

- [9. Numerical Stability and Validation](09_numerical-stability-and-validation.md#9-numerical-stability-and-validation)
  - [9.1 Common numerical pitfalls](09_numerical-stability-and-validation.md#9-1-common-numerical-pitfalls)
  - [9.2 Recommended clamping and epsilon choices](09_numerical-stability-and-validation.md#9-2-recommended-clamping-and-epsilon-choices)
  - [9.3 Validation test suite (high leverage)](09_numerical-stability-and-validation.md#9-3-validation-test-suite-high-leverage)
  - [9.4 Debug visualization buffers](09_numerical-stability-and-validation.md#9-4-debug-visualization-buffers)

### 10_appendices.md

- [Appendix A. Formula Cheat Sheet](10_appendices.md#appendix-a-formula-cheat-sheet)
  - [A.1 Affine transform of a Gaussian](10_appendices.md#a-1-affine-transform-of-a-gaussian)
  - [A.2 Deformation field linearization](10_appendices.md#a-2-deformation-field-linearization)
  - [A.3 Projection Jacobian (pinhole)](10_appendices.md#a-3-projection-jacobian-pinhole)
  - [A.4 Screen-space Gaussian evaluation](10_appendices.md#a-4-screen-space-gaussian-evaluation)
- [Appendix B. PLY Interchange Template](10_appendices.md#appendix-b-ply-interchange-template)
- [Appendix C. Spherical Harmonics Rotation Notes](10_appendices.md#appendix-c-spherical-harmonics-rotation-notes)
- [Appendix D. Procedural Deformations and Jacobians](10_appendices.md#appendix-d-procedural-deformations-and-jacobians)
  - [D.1 Twist (about Z)](10_appendices.md#d-1-twist-about-z)
  - [D.2 Bend (simple model)](10_appendices.md#d-2-bend-simple-model)
  - [D.3 Noise displacement](10_appendices.md#d-3-noise-displacement)
- [Appendix E. Further Reading (selected topics)](10_appendices.md#appendix-e-further-reading-selected-topics)
- [Bibliography](10_appendices.md#bibliography)
