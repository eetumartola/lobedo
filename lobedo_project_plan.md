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
- Practical editing pipeline nodes: crop, prune, regularize, LOD/decimate, pack/export
- Conversion nodes between mesh/points/splats
- Job-style ML nodes (async + cached) that do not block UI and run PyTorch-backed operators
- Spreadsheet supports a splat view (default to first 100 splats)

## Non-goals (initially)
- Full 3DGS training/optimization suite
- Rich sculpt/paint UI comparable to dedicated splat editors
- Perfect physically based relighting of captured splats
- Unreal integration or `.uasset` authoring

## Epic P - Polish (future)
- Transient node info panel on middle mouse (dismiss on release)
- Open app fullscreen by default
- Pin hit area 2x radius of visual pin circle
- Viewport top-left icons for normals + stats overlay toggles
- Parameter sliders are 2x wider

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

### Groups (Houdini-style)
Geometry carries **named groups** for point/vertex/primitive membership. Groups can be populated via
Delete-style rules (box/sphere/plane) or by copying existing groups. Nodes may optionally restrict
their operation to a group or exclude a group; unsupported primitive types pass through.
Group expressions use Houdini-style matching (wildcards) and exclusion tokens (e.g. `grp* ^tmp`).

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

## Node Library (MVP)

### IO
1. `Read Splats` (PLY + optional SPZ)
2. `Write Splats` (PLY + optional SPZ)
3. `Read Image` (for ML path)

### Shared transform / selection-like ops
4. `Transform` (works on all supported primitives)
   - Splat-aware: transforms `P/R/S` and rotates SH coefficients when rotation is applied
5. `Merge` (Mesh merge; Splat merge)
6. `Group` (create named groups by box/sphere/plane or from existing groups)
7. `Filter by Mask` (for points/splats; mask source could be box/sphere/plane)

### Splat-native ops
8. `Crop` (box/sphere/plane)
9. `Prune` (by opacity/scale/confidence/outlier heuristics)
10. `Regularize` (clamp scales, normalize opacity, remove invalid values)
11. `LOD / Decimate` (voxel clustering or k-means-ish; preserve appearance)
12. `SH Tools` (utility)
   - `Rotate SH` (explicit)
   - `Reduce SH Order` (e.g., L3->L1)

### Conversion
13. `Splats -> Points`
14. `Points -> Splats (Fit)`
15. `Mesh -> Splats (Sample Surface)` (optional in MVP, but high leverage)

### ML (MVP)
16. `Depth Estimation (Job)`
17. `Backproject to Points`
18. (Optional) `Points -> Splats` downstream

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
- Implement `Read Splats (PLY)` and `SplatGeo` internal representation
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
- Crop, Prune, Regularize
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
