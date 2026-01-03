# Lobedo Backlog (Issue List)

## Epic G - Geometry primitives (architecture change)
- **G1**: Introduce `Geometry` container with primitive list (mesh/point/spline/splat)
- **G2**: Replace `Splats` pin type with unified `Geometry` pin type
- **G3**: Node execution: apply per-primitive ops + pass-through for unsupported primitives
- **G4**: Define splat deformation policy (SH rotation, scale handling, optional resampling)
- **G5**: Migrate project format + graphs to new geometry pin type

## Epic S - Core splat data type
- **S1**: Define `Splat` schema + typed channel storage
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
- **Q4**: Spreadsheet splat mode (show first 100 splats by default)
