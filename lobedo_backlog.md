# Lobedo Backlog (Issue List)
Status: done | in progress | not started

## Epic G - Geometry primitives (architecture change)
- **G1**: Introduce `Geometry` container with primitive list (mesh/point/spline/splat) (done)
- **G2**: Replace `Splats` pin type with unified `Geometry` pin type (done)
- **G3**: Node execution: apply per-primitive ops + pass-through for unsupported primitives (done)
- **G4**: Define splat deformation policy (SH rotation, scale handling, optional resampling) (done: L1-L3 SH rotation + scale handling in transform)
- **G5**: Migrate project format + graphs to new geometry pin type (done)
- **G6**: Add group system (named point/vertex/primitive groups; splats in primitive groups) (done)

## Epic S - Core splat data type
- **S1**: Define `Splat` schema + typed channel storage (done)
- **S3**: Expand `SceneSnapshot` to multiple drawables (done)
- **S4**: Add validation + unit tests (sizes, NaN checks) (in progress: validation + tests; NaN checks pending)

## Epic R - Splat rendering
- **R1**: Baseline splat rasterization (SH0) (done)
- **R2**: Sorting strategy v1 (CPU) + web fallback (in progress)
- **R3**: Debug modes (opacity/scale/depth/overdraw) (not started)
- **R4**: SH evaluation degree 2/3 (not started)
- **R5**: Sorting strategy v2 (GPU/tile binning) (not started)

## Epic N - Nodes
- **N1**: Read/Write Splats (PLY) (done)
- **N2**: Transform (splat-aware) (done)
- **N3**: Delete / Prune / Regularize (in progress: Delete done, Prune/Regularize pending)
- **N6**: Group node (box/sphere/plane; group by existing group) (done)
- **N7**: Group selection parameter on applicable nodes (Auto/Point/Vertex/Primitive) (done)
- **N4**: LOD/Decimate (voxel clustering) (not started)
- **N5**: Conversions (splats->points, mesh->splats) (not started)

## Epic M - ML jobs
- **M1**: Job framework (async, progress, cancel, cache) (not started)
- **M2**: Depth estimation sidecar (PyTorch) (not started)
- **M3**: Backproject -> points (not started)
- **M4**: Fit splats (not started)

## Epic Q - Quality and UX
- **Q1**: Golden test assets + screenshot tests (native) (not started)
- **Q2**: Project format migration + versioning (done)
- **Q3**: Performance profiling harness (splat count scaling) (not started)
- **Q4**: Spreadsheet splat mode (show first 100 splats by default) (not started)

## Epic P - Polish
- **P1**: Transient node info panel on middle mouse (not started)
- **P2**: Open app fullscreen by default (not started)
- **P3**: Pin hot area 2x radius of visual pin circle (not started)
- **P4**: Viewport top-left icons for normals + stats toggles (not started)
- **P5**: Parameter sliders are 2x wider (not started)
- **P6**: Show splat info text in Read Splats parameter pane (not started)
- **P7**: File requester buttons for Read/Write OBJ + Read/Write Splats (not started)
