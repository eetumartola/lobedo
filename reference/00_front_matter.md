# Procedural 3D Gaussian Splat Editing and Processing

_A ground-truth reference for building an interactive and procedural 3D application_

Version 1.0 • 2026-01-06


This document is a self-contained engineering and math reference for editing, transforming, deforming, merging, optimizing, compressing, and rendering 3D Gaussian splats. It is designed for developers building a production-grade 3D application with a procedural editing stack (modifiers/nodes), not just a viewer. It focuses on: (1) correct parameter transforms (means, covariances, view-dependent color), (2) predictable compositing behavior, and (3) practical GPU/CPU architectures for real-time interaction.


<a id="table-of-contents"></a>
## Table of Contents

Tip: In Word, insert or update an automatic Table of Contents if desired. This section lists the major headings for quick navigation.

- 1. Core Representation
- 2. Rendering and Rasterization Pipeline
- 3. Editing Operations
- 4. Composition, Merging, and Registration
- 5. Non-Rigid Deformation and Animation
- 6. Optimization, Densification, Pruning, and LOD
- 7. Compression, Storage, and Streaming
- 8. Application Architecture
- 9. Numerical Stability and Validation
- Appendices (file formats, SH rotation, code templates)
