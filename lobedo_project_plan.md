# Splat-Native Node Editor "Lobedo" Project Plan

## Vision
A node-based 3D editor where **Geometry is a container of primitives** (mesh, point, spline, splat). The tool supports:
- Shared editing operations that work on multiple primitive types (where meaningful)
- Splat-specific operations that correctly handle **covariances/orientation** and **spherical harmonics (SH)**
- Conversion between representations (mesh -> points -> splats)
- Optional ML-powered nodes (e.g., image + depth -> points -> splats; generative splats later)

Lobedo is not trying to compete with full DCC suites; it is a **splat asset pipeline + procedural editing graph**.

## Goals
- Unified `Geometry` container with primitive variants and robust import/export per primitive
- Real-time splat renderer in the viewport (native + web)
- Correct transforms for splats, including **SH rotation** when the splat frame rotates
- Practical editing pipeline nodes: crop, splat prune, splat regularize, LOD/decimate, pack/export
- Conversion nodes between mesh/points/splats
- Job-style ML nodes (async + cached) that do not block UI and run PyTorch-backed operators
- Spreadsheet supports a splat view (default to first 100 splats)

## Non-goals (initially)
- Full 3DGS training/optimization suite
- Rich sculpt/paint UI comparable to dedicated splat editors
- Perfect physically based relighting of captured splats
- Unreal integration or `.uasset` authoring

## Epic P - Polish (done)
- Transient node info panel on middle mouse (dismiss on release)
- Open app fullscreen by default
- Pin hit area 2x radius of visual pin circle
- Viewport top-left icons for normals + stats overlay toggles
- Parameter sliders are 2x wider
- Viewport/node editor split is draggable
- Parameter pane help strings for nodes + parameters (hover)
- Screen-space MMB pan in viewport
- File menu About entry
- Delete node reconnects upstream/downstream links
- README refresh + GPL license switch

## Epic V - Viewport Editing (done)
- Shared viewport tool framework (gizmos + draw tools)
- Transform gizmo (move/rotate/scale) for Transform + Copy/Transform
- Curve node with viewport draw + edit tools (workplane y=0)
- Box/bounds gizmo for Box/Group/Delete/Splat Heal
- Group selection mode (click/box, modifiers, backface toggle)

## Epic C - Architecture cleanup (done)
- Centralize built-in node registry metadata (definitions/defaults/compute/input policy)
- Drive geometry eval inputs from the node input policy
- Wrap mesh/splat eval around geometry eval to remove duplicate input logic     
- Reduce duplicate node lists in UI menu/state helpers

## Epic T - Parallelism
- Add optional CPU parallelism (rayon) for heavy geometry loops with a single-thread fallback for web (done)
- Parallelize expensive per-element ops (wrangle, attribute ops, prune/regularize, LOD clustering) (in progress: LOD pending)
- Target high-cost loops (Attribute Transfer, Smooth, Ray, Copy to Points) + CPU splat depth sorting (in progress: Ray/Copy to Points pending)
- Parallelize per-primitive evaluation inside a node (mesh list, splat list, point list) (not started)
- Explore parallel node evaluation for independent subgraphs (graph scheduling) (not started)
- Add profiling-driven thresholds (only parallelize above a size/complexity cutoff) (in progress: size threshold, no profiling)

## Epic A - Automation & Scripting (future)
- Embedded scripting console (REPL) with the same API usable in headless mode
- Scripted graph creation + parameter edits + cook + export for automated testing

## Epic U - Materials & Textures (done)
- Add string attributes with shared value tables (Houdini-style) for per-primitive material assignment
- Ensure meshes carry UVs end-to-end (reader, nodes, renderer)
- `UV Texture` node for basic projections (planar/box/cyl/sphere)
- `UV Unwrap` node for basic unwrapping
- `Material` node with named PBR parameters + diffuse texture path
- Renderer support for UVs + diffuse texture sampling (MVP: diffuse only)       

## Epic W - Volume Support (done)
- Add a new Volume primitive type (sparse-friendly core representation)
- `Volume from Geometry` node (density + SDF modes)
- Volume rendering in viewport (raymarching MVP)
- Volume bounds + node info reporting
- `Volume Combine` node (binary ops + resolution mode)
- `Volume to Mesh` node (density/SDF surface extraction)
- Web-compatible volume texture upload/render path

## Epic N - N-gon Support (done)
- Extend mesh data model with per-face vertex counts (n-gon primitives)
- Add triangulation helper + per-triangle-to-face mapping for render + triangle-only ops
- Update scene/render conversion to use triangulated indices + corner mapping
- Update UI counts, groups, and selection to target n-gon primitives
- Update IO: OBJ should write n-gons; GLTF should triangulate
- Update mesh ops to use triangulation when needed (scatter/smooth/ray/uv)
- Update Boolean Geo (SDF mode) to preserve n-gon topology + cleaner cut edges

---

## Core Data Model

### Architecture change: unified primitives
All geometry flows through the graph as a **single geometry pin type**. Geometry carries a list of primitives (mesh, point, spline, splat). Nodes must:
- Apply edits to each primitive they support
- Preserve primitives they do not handle (pass-through)
- Explicitly convert between primitive types when the user requests it

Splat-specific behaviors (ex: SH rotation under deformation) live inside primitive-aware node implementations.

### Geometry primitives
- `Mesh` (tri/quad surface primitives)
- `Point` (unordered points)
- `Spline` (future)
- **`Splat` (Gaussian splats)**

### Proposed `Splat` schema (internal)
Minimum viable per-splat attributes:
- `P: Vec3` (position)
- `R: Quat` (orientation)
- `S: Vec3` (log-scale or axis radii) **or** covariance representation
- `opacity: f32`
- `sh: SHCoeffs` (at least SH0/DC per channel; optionally higher bands)

Optional but useful:
- `id: u32` (stable ID)
- `confidence: f32`
- `feature masks` / user tags

### Attribute system integration
Each primitive type has its own attribute storage with a shared accessor interface for common semantics
(`P`, `Cd`, etc). Splat attributes remain typed channels with required semantics (P/R/S/opacity/SH).
Splat core channels are exposed as Houdini-style point attributes (`P`, `Cd`, `orient`, `scale`, `opacity`)
and mapped back to PLY properties on export.

### Groups (Houdini-style)
Geometry carries **named groups** for point/vertex/primitive membership. Groups can be populated via
Delete-style rules (box/sphere/plane) or by copying existing groups. Nodes may optionally restrict
their operation to a group or exclude a group; unsupported primitive types pass through.
Group expressions use Houdini-style matching (wildcards) and exclusion tokens (e.g. `grp* ^tmp`).
Splats always expose an intrinsic `splats` group to target splat primitives explicitly.

### Pin types (expanded)
Add pin types:
- `Geometry` (unified mesh/point/spline/splat container)
- `Image` (for ML nodes later)
- `Cameras` (for multi-view later)

### SceneSnapshot (expanded)
Move from "single display mesh" toward:
- `SceneSnapshot { drawables: Vec<Drawable> }`
- `Drawable` variants: `MeshDrawable`, `SplatDrawable`, (later `PointDrawable`)
- Viewport display/template flags remain, but now apply per drawable output.

---

## Rendering Plan (wgpu)

### Splat renderer MVP
- Project each splat to screen; rasterize an ellipse/quad with Gaussian falloff
- Depth test + blending

Sorting approach (phased):
- **MVP-1:** CPU sort for smaller splat counts; fallback approximate sorting on web
- **MVP-2:** GPU sort (radix/bitonic) or tile-binning with per-tile depth buckets

SH evaluation:
- **MVP-1:** SH0 (constant color) for speed + correctness baseline
- **MVP-2:** SH up to degree 2 or 3 for view-dependent appearance

### Debug views (splat-specific)
- SH0 color
- Opacity heatmap
- Scale/radius heatmap
- Depth
- Overdraw / contribution heatmap
- Optional: visualize covariance axes (screen-space ellipse outlines)

### Shared viewport features
- Keep existing camera + lighting UI
- For splats, "lighting" is typically baked; the viewer is primarily for inspection and debug.

---

## Node Library (current)

### IO
- `File` (OBJ/GLTF)
- `Splat Read` (PLY)
- `Splat Write` (PLY)
- `OBJ Output`
- `GLTF Output`

### Core geometry ops
- `Transform`, `Merge`, `Delete`, `Group`, `Group Expand`, `FFD`, `Boolean SDF`, `Boolean Geo`  
- `Copy/Transform`, `Copy to Points`, `Scatter`, `Resample`
- `Normal`, `Color`, `Noise/Mountain`, `Erosion Noise`, `Smooth`
- `Ray`, `Sweep`, `Tube`, `Circle`, `Curve`, `Sphere`, `Grid`, `Box`
- `Attribute Noise`, `Attribute Expand`, `Attribute from Feature`, `Attribute from Volume`, `Attribute Transfer`, `Attribute Math`, `Wrangle`

### Materials & UV
- `UV Texture`, `UV Unwrap`, `UV View`
- `Material`

### Volume
- `Volume from Geometry`
- `Volume Combine`
- `Volume Blur`
- `Volume to Mesh`

### Splat-native ops
- `Splat Prune`
- `Splat Regularize`
- `Splat LOD`
- `Splat to Mesh` (mesh or SDF volume)
- `Splat Deform`
- `Splat Delight`
- `Splat Integrate`
- `Splat Heal`
- `Splat Outlier`
- `Splat Cluster`
- `Splat Merge`

### Planned conversions
- `Splats -> Points`
- `Points -> Splats (Fit)`
- `Mesh -> Splats (Sample Surface)`

### ML (future)
- `Depth Estimation (Job)`
- `Backproject to Points`
- (Optional) `Points -> Splats` downstream

---
## ML Integration Architecture
- Treat ML nodes as **jobs**:
  - async execution, progress, cancel
  - content-hash caching on disk
  - deterministic "model version + parameters" tracking
- MVP implementation: **sidecar Python** worker (PyTorch) invoked by the node
- Later options: ONNX Runtime native inference; WebGPU inference experiments

---

## Milestones

### Milestone G0 - Geometry primitive unification
- Introduce `Geometry` container with primitive list
- Replace `Splats` pin type with unified `Geometry` pin type
- Update node evaluation to operate per primitive with pass-through semantics
- Add deformation policies for splats (SH rotation rules, scale handling)

### Milestone S0 - Baseline + splat scaffolding
- Add feature flag `splats` to stage integration
- Extend project format versioning for new pin types and drawable outputs
- Add placeholder `Splat` primitive type

### Milestone S1 - Splat import + viewer baseline
- Implement `Splat Read (PLY)` and `SplatGeo` internal representation
- Add `SplatDrawable` to `SceneSnapshot`
- Implement splat rendering with SH0 color
- Add splat debug modes (opacity/scale/depth)
- Validate on native + web with a few known test assets

### Milestone S2 - Correct transforms (incl. SH rotation)
- Implement `Transform` node for splats
- Implement SH rotation for chosen max degree (start with degree 2)
- Add regression tests:
  - rotate splats by 90 deg and verify SH-consistent shading change
  - verify covariance/orientation remains valid

### Milestone S3 - Editing pipeline nodes
- Crop, Splat Prune, Splat Regularize
- Merge splat sets
- Export PLY (round-trip)
- Add "pipeline preset" example graph for cleanup

### Milestone S4 - LOD/Decimate + Packing
- Voxel clustering decimator
- Chunking (optional) + export naming
- Optional SPZ read/write

### Milestone S5 - ML depth -> splats
- Depth Estimation job node + caching
- Backproject node
- Fit splats node
- End-to-end example: image + depth + points + splats + export


