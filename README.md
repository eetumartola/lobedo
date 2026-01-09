# Lobedo - A Node Editor with Native Gaussian Splats

Lobedo is a Houdini-style node editor for procedural geometry, Gaussian splats, and volumes. It targets fast iteration in a native desktop app and a Web build; headless mode exists for testing only.

## Current features
- Node-based graph + 3D viewport for meshes, curves, splats, and volumes.
- Gaussian splat pipeline: read/write PLY, prune/regularize, LOD, deform, and splat-to-mesh.
- Geometry operators: transform/copy, delete/group, noise/erosion, smooth, attribute tools, and wrangle scripting.
- Curves: draw and edit curves in the viewport; curve primitives share the main point pool.
- Volumes: volume-from-geometry (density or SDF), combine, volume-to-mesh, and attribute-from-volume.
- UVs & materials: UV projection/unwrap, UV view, material assignment with texture-backed base color.
- Project persistence (JSON) with native and web save/load; headless CLI for validation/testing.

## Requirements
- Rust stable toolchain.
- For web builds: wasm32 target and wasm-bindgen CLI.

## Run (native)
```powershell
cargo run -p lobedo
```

## Build (web)
```powershell
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
.\build_web.ps1
```

The web build outputs to `dist/`.

## Headless mode (testing only)
```powershell
cargo run -p lobedo -- --headless --plan headless_plan.json --print
```

Optional outputs:
```powershell
cargo run -p lobedo -- --headless --plan headless_plan.json --save output.json
```

## Docs
- `current_system.md`
- `lobedo_project_plan.md`
- `lobedo_backlog.md`

## License
GPL-3.0-only
