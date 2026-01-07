# Lobedo - A Node Editor with Native Gaussian Splats

Lobedo is a node-based 3D editor focused on fast iteration for procedural geometry and Gaussian splats, with a path toward ML-powered operators.

## Overview
Lobedo currently implements a Houdini-lite / GeometryNodes-lite workflow for meshes, and is evolving into a splat-native editor with a job-based ML pipeline.

## Features (current)
- Node-based geometry graph and 3D viewport.
- Project persistence (JSON) with native and web save/load.
- Headless CLI for automated validation (testing only).
- Evaluation engine with topo sort, caching, and per-node error reporting.

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
