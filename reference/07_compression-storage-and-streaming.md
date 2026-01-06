<a id="7-compression-storage-and-streaming"></a>

## 7. Compression, Storage, and Streaming

Gaussian splat scenes can contain millions of primitives. A production application typically needs:
• an interchange format (commonly PLY with custom properties),
• one or more compressed on-disk formats for distribution,
• a GPU-friendly in-memory layout,
• streaming and LOD for large scenes.

<a id="7-1-ply-schema-common-interchange"></a>
### 7.1 PLY schema (common interchange)

PLY is widely used because it is simple and extensible. A typical per-vertex property list includes:
• position: x y z
• scale: sx sy sz (often stored in log space)
• rotation quaternion: qw qx qy qz
• opacity: a (logit)
• color DC: f_dc_0 f_dc_1 f_dc_2
• higher SH coefficients: f_rest_0 ...
The exact naming/order varies; treat it as part of your I/O contract.

<a id="minimal-recommended-internal-schema-format-agnostic"></a>
#### Minimal recommended internal schema (format-agnostic):

| Field | Type | Space | Comments |
| --- | --- | --- | --- |
| mu | float32[3] | object or world | center |
| log_s | float32[3] | object | log scale; clamp in render |
| quat | float32[4] | object | normalize before use |
| opacity_logit | float32 | object | alpha=sigmoid |
| sh | float32[3, (L+1)^2] | object | canonical ordering |
| id | uint32 | n/a | stable identity for selection |
| group | uint16 | n/a | layer/collection |

<a id="7-2-quantization-and-chunked-compression"></a>
### 7.2 Quantization and chunked compression

A common high-quality compression strategy is chunk-based quantization:
• Partition splats into spatial chunks (e.g., 256–4096 splats per chunk).
• Store chunk bounds (min/max) in float.
• Store per-splat values normalized to bounds and quantized to 8–16 bits.
• Compress chunks with entropy coding (gzip/zstd) for disk.
This yields large size reductions with minimal visual loss and enables streaming by chunk.

<a id="7-3-specialized-compressed-formats"></a>
### 7.3 Specialized compressed formats

Several compressed container formats exist in the ecosystem. If you design your own, consider:
• Fixed-point quantization for position/scale.
• Compact quaternion encoding (e.g., 3 components + sign, or 8-bit per component).
• SH coefficient quantization (often the largest payload).
• Spatial ordering (Morton/Z-order) to improve cache locality and compression.
• Optional vector quantization (VQ) of SH blocks or other attributes.

<a id="7-4-gpu-memory-layout"></a>
### 7.4 GPU memory layout

For real-time rendering/editing on GPU:
• Use a Structure-of-Arrays (SoA) layout for coalesced reads.
• Keep frequently accessed fields (μ, log_s, quat, opacity, DC color) in separate contiguous buffers.
• Consider quantized GPU formats: 16-bit floats for scales, packed 10:10:10 or 8:8:8 for normals/IDs.
• Store SH in a separate stream; many passes only need DC or low-degree bands.

<a id="7-5-streaming-and-out-of-core-editing"></a>
### 7.5 Streaming and out-of-core editing

For large datasets:
• Chunk by space (octree nodes or fixed grid).
• Keep an on-disk index and load only chunks intersecting the camera frustum.
• Maintain a coarse LOD resident for all space; refine locally.
• For editing, lock chunks being modified to avoid streaming races; commit edits back to the chunk store.