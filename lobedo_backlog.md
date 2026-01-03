# Lobedo Backlog (Issue List)

## Epic S - Core splat data type
- **S1**: Define `SplatGeo` schema + typed channel storage
- **S2**: Add `Splats` pin type and graph typing rules
- **S3**: Expand `SceneSnapshot` to multiple drawables
- **S4**: Add validation + unit tests (sizes, NaN checks)

## Epic R - Splat rendering
- **R1**: Baseline splat rasterization (SH0)
- **R2**: Sorting strategy v1 (CPU) + web fallback
- **R3**: Debug modes (opacity/scale/depth/overdraw)
- **R4**: SH evaluation degree 2/3
- **R5**: Sorting strategy v2 (GPU/tile binning)

## Epic N - Nodes
- **N1**: Read/Write Splats (PLY)
- **N2**: Transform (splat-aware)
- **N3**: Crop / Prune / Regularize
- **N4**: LOD/Decimate (voxel clustering)
- **N5**: Conversions (splats->points, mesh->splats)

## Epic M - ML jobs
- **M1**: Job framework (async, progress, cancel, cache)
- **M2**: Depth estimation sidecar (PyTorch)
- **M3**: Backproject -> points
- **M4**: Fit splats

## Epic Q - Quality and UX
- **Q1**: Golden test assets + screenshot tests (native)
- **Q2**: Project format migration + versioning
- **Q3**: Performance profiling harness (splat count scaling)
