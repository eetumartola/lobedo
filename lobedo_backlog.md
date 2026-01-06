# Lobedo Backlog (Issue List)
Status: done | in progress | not started

## Epic G - Geometry primitives (architecture change)
- **G1**: Introduce `Geometry` container with primitive list (mesh/point/spline/splat) (done)
- **G2**: Replace `Splats` pin type with unified `Geometry` pin type (done)
- **G3**: Node execution: apply per-primitive ops + pass-through for unsupported primitives (done)
- **G4**: Define splat deformation policy (SH rotation, scale handling, optional resampling) (done: L1-L3 SH rotation + scale handling in transform)
- **G5**: Migrate project format + graphs to new geometry pin type (done)
- **G6**: Add group system (named point/vertex/primitive groups; splats in primitive groups) (done)
- **G7**: Treat splat centers as points (point ops apply), add point vs primitive groups, map splat attributes to Houdini names (done)

## Epic C - Architecture cleanup
- **C1**: Centralize built-in node registry metadata (definitions/defaults/compute/input policy) (done)
- **C2**: Drive geometry eval inputs from the node input policy (done)
- **C3**: Mesh eval wraps geometry eval and derives mesh outputs (done)
- **C4**: Splat eval wraps geometry eval and derives splat outputs (done)
- **C5**: Reduce duplicate node lists in the UI menu/state helpers (done)

## Epic T - Parallelism (future)
- **T1**: Add optional CPU parallelism (rayon) with single-thread fallback for web/wasm (not started)
- **T2**: Parallelize attribute-heavy nodes (Wrangle, Attribute Noise/From Feature/Transfer/Math, Smooth) (not started)
- **T3**: Parallelize splat ops (Prune, Regularize, LOD/Decimate) (not started)
- **T4**: Parallelize mesh ops (Delete/Group, Copy to Points/Transform, Ray) (not started)
- **T5**: Parallelize per-primitive evaluation inside Geometry nodes (mesh/splat lists) (not started)
- **T6**: Explore parallel node evaluation for independent subgraphs (scheduler) (not started)
- **T7**: Add profiling + size thresholds to guard parallel overhead (not started)

## Epic S - Core splat data type
- **S1**: Define `Splat` schema + typed channel storage (done)
- **S3**: Expand `SceneSnapshot` to multiple drawables (done)
- **S4**: Add validation + unit tests (sizes, NaN checks) (done)

## Epic R - Splat rendering
- **R1**: Baseline splat rasterization (SH0) (done)
- **R2**: Sorting strategy v1 (CPU) + web fallback (done)
- **R3**: Debug modes (opacity/scale/depth/overdraw) (not started)
- **R4**: SH evaluation degree 2/3 (not started)
- **R5**: Sorting strategy v2 (GPU/tile binning) (not started)

## Epic N - Nodes
- **N1**: Splat Read/Splat Write (PLY) (done)
- **N2**: Transform (splat-aware) (done)
- **N3**: Delete / Splat Prune / Splat Regularize (done)
- **N6**: Group node (box/sphere/plane; group by existing group) (done)
- **N7**: Group selection parameter on applicable nodes (Auto/Point/Vertex/Primitive) (done)
- **N8**: Tube (support splats where applicable) (done)
- **N9**: Attribute Noise (named attribute, default P; float/vec2/vec3 + point/vertex/prim; noise type; shared noise library) (done)
- **N10**: Attribute from Feature (area + gradient; Measure SOP + Heightfield Mask by Feature hybrid; support splats where applicable) (done)
- **N11**: Attribute Transfer (space-delimited attribute names; domain selector; splats as source at minimum) (done)
- **N12**: Smooth (space-delimited attribute names, default P; support splats where applicable) (done)
- **N13**: Ray (normal/direction/closest; max distance; hit group; import hit attributes; optional no-transform) (done)
- **N4**: LOD/Decimate (voxel clustering) (done)
- **N5**: Conversions (splats->points, mesh->splats) (not started)
- **N5a**: Splat to Mesh (density/ellipsoid implicit) (done)
- **N5b**: Splat to Mesh transfer color to mesh Cd attribute (done)
- **N14**: Wrangle: @ptnum/@vtxnum/@primnum + point/vertex/prim/splat queries (inputs 0/1) (done)
- **N15**: Splat Deform (deform source splats from edited point cloud; optional new splats) (done)

## Epic M - ML jobs
- **M1**: Job framework (async, progress, cancel, cache) (not started)
- **M2**: Depth estimation sidecar (PyTorch) (not started)
- **M3**: Backproject -> points (not started)
- **M4**: Fit splats (not started)

## Epic Q - Quality and UX
- **Q1**: Golden test assets + screenshot tests (native) (not started)
- **Q2**: Project format migration + versioning (done)
- **Q3**: Performance profiling harness (splat count scaling) (not started)
- **Q4**: Spreadsheet splat mode (show first 100 splats by default) (done)

## Epic P - Polish
- **P1**: Transient node info panel on middle mouse (done)
- **P2**: Open app fullscreen by default (done)
- **P3**: Pin hot area 2x radius of visual pin circle (done)
- **P4**: Viewport top-left icons for normals + stats toggles (done)
- **P5**: Parameter sliders are 2x wider (done)
- **P6**: Show splat info text in Splat Read parameter pane (done)
- **P7**: File requester buttons for Read/Write OBJ + Splat Read/Splat Write (done)
- **P8**: Viewport/node editor split is draggable (done)
