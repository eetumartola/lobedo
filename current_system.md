# Current System Summary (Lobedo Geometry Graph Editor)

## Purpose
A lightweight Houdini-lite / GeometryNodes-lite node-based geometry editor aimed at fast iteration and web viability. It serves as the foundation for Lobedo's Gaussian splat editor and PyTorch-backed ML operators. Primary target is Windows 11 native; secondary target is web (wasm).

## Core Features (as implemented)
- **3D viewport + node graph** (egui-snarl) with fast interactive iteration.
- **Project persistence** (serde JSON) with working native + web save/load flows.
- **Headless CLI mode** to build/validate projects from a plan JSON and emit project JSON.
- **Evaluation engine** with topo sort, dirty propagation, caching, and per-node error reporting.
- **Viewport** with simple-lit shading (plus three-point lighting), optional key shadows, point rendering mode.
- **Debug tooling**: grid/axes/normals/bounds overlays, lit/normals/depth visualization, perf stats, dirty-node reasons, evaluation report.
- **Attribute inspection**: spreadsheet panel split from viewport; domain filtering.
- **Houdini-ish UX**: display/template flags (display drives viewport; template overlays wireframe), node info panel, add-node search, wire insertion.

## Technology
- **Rust**
- **egui + eframe** UI shell (native + wasm)
- **egui-snarl** node UI
- **wgpu** rendering
- **glam** math
- **serde + JSON** persistence

## Architecture

### `core`
- `Project`, `Graph` (nodes/pins/links)
- Pin type system (`Mesh`, scalar/vector types)
- Evaluation engine: topo sort, dirty propagation, caching, `EvalReport`
- Geometry kernel: `Mesh` + helpers
- Attribute system with Houdini-style **domains** (point/vertex/primitive/detail) and precedence rules
- `SceneSnapshot` output boundary (currently single display mesh + params)

### `render`
- WGPU setup, camera controls
- Pipelines: lit, normals, depth, point rendering, optional wireframe/lines
- Debug overlays (grid/axes/normals/bounds), viewport edge toggles, stats overlay

### `app`
- UI panels (Viewport, Node Graph, Inspector, Spreadsheet, Debug, Console)
- Graph UI adapter, add-node menu, node info/menus
- Orchestration: evaluation triggers + debounce; error surfacing

## Implemented Node Set (current)
- Sources: Box, Grid, Sphere, File (OBJ)
- Ops: Transform, Merge, Copy to Points, Scatter, Normal, Color, Noise/Mountain, Attribute Math, Copy/Transform
- IO: OBJ Output
- Output: Output node exists; viewport primarily driven by Display flag

## Known Gaps / In-Progress
- **Wrangle node** (mini attribute language) planned.
- **Undo/redo** system in progress (graph edits, param changes, layout moves).
- **No splat geometry type or renderer yet** (mesh-only output).
- **No ML job system yet** (PyTorch sidecar work is planned).
- Potential future expansions: offline render view, more nodes, better graph UX polish
