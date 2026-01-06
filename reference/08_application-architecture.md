<a id="8-application-architecture"></a>

## 8. Application Architecture

A procedural splat editor benefits from a clean separation between:
• Data model (authoring space)
• Evaluation (procedural graph / modifier stack)
• Render/preview (GPU pipeline)
• Import/export (formats)
• Compute/acceleration structures (spatial index, LOD, streaming)

<a id="8-1-recommended-core-subsystems"></a>
### 8.1 Recommended core subsystems

1) Scene graph / collections: groups of splats with transforms and metadata.
2) Procedural graph: nodes that read/write splat sets; supports caching and dependency tracking.
3) Acceleration structures: BVH/octree/grid for selection, culling, neighborhood queries.
4) GPU backend: transform kernel, projection kernel, binning, sorting, raster.
5) Tool layer: gizmos, brushes, selection sets, constraints.
6) Persistence: chunk store, undo/redo, versioned assets.

<a id="8-2-non-destructive-editing-model"></a>
### 8.2 Non-destructive editing model

To support procedural workflows:
• Treat splat sets as immutable inputs; each node produces a new splat set or a view (mask/index list).
• Use copy-on-write or delta storage for edits to avoid duplicating millions of splats.
• Provide 'bake' nodes that materialize results into a concrete splat set for export/performance.

<a id="8-3-gpu-pipeline-integration-for-interactive-editing"></a>
### 8.3 GPU pipeline integration for interactive editing

Interactive editing often needs two representations:
• Authoring representation (CPU): editable, supports random access and complex operations.
• Render representation (GPU): packed, streamable, optimized for coherent reads.
Typical frame:
1) Evaluate procedural graph changes → produce a set of dirty ranges.
2) Upload only changed chunks/ranges to GPU.
3) Run per-frame GPU kernels: apply instance transforms → project → bin → sort → raster.
4) Optionally render auxiliary buffers: ID buffer, selection mask, error heatmaps.

<a id="8-4-undo-redo-at-scale"></a>
### 8.4 Undo/redo at scale

Full copies are too expensive. Practical approaches:
• Store edits as reversible operations on stable IDs (e.g., 'delete these IDs', 'add these new splats').
• For continuous edits (brush strokes), store sparse deltas in a per-chunk log.
• Snapshot only metadata and a random seed for procedural nodes; re-evaluate deterministically.