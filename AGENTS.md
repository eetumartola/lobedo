Maintain the project directory index via `scripts/update_project_directory.ps1`. The script regenerates `project_directory.md` with a list of source files (scoped to Rust sources under `crates/**/src/**`), a short description per file, and the function names in order. Descriptions can be overridden in `project_directory_map.toml`. Prefer running the script after code changes instead of manual edits.

There is reference documentation on gaussian splat editing in `reference/` folder, beginning with `reference/index.md`

Run tests/clippy after extensive changes