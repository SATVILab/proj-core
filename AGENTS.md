# Agent Instructions

## 1. Strict Read-Only Guardrails (The _reference Directory)
- The `_reference` directory and its contents are strictly for reference purposes. 
- You MUST NOT modify, manipulate, delete, or rename any files or folders inside the `_reference` directory. All new implementation code must live in the root package structure (e.g., `src/`, `Cargo.toml`).

## 2. Compilation Compatibility & Crate Selection
- We ultimately want this project wrapped as native Python and R language modules (via tools like `extendr` and `PyO3`). 
- CRAN and enterprise environments (like Debian Stable or RHEL) enforce rigid, older compiler architectures. 
- Use stable, mature, and widely compatible crates (e.g., standard `serde`, `serde_yaml`, `clap`, `regex`) rather than bleeding-edge alternatives. Avoid unnecessary external dependencies to minimize compilation friction on older toolchains.

## 3. Library-First Structural Constraint
- This project must be designed as a core shared library first (`src/lib.rs`), with a thin CLI binary wrapper (`src/main.rs`). 
- Internal logic (parsing configuration, compiling ignores, computing paths) must execute through exposed, public module functions within the library layer. The binary entry point should only handle command-line shell argument routing.

## 4. Documentation Strategy
- For every struct, enum, and public function implemented, you must immediately author inline Rust documentation comments (`///`).
- Include a descriptive overview, details on expected parameters, edge cases/errors, and an executable markdown ````rust ... ```` code example block within the comments. This ensures that CLI help screens (`--help`) and API definitions stay perfectly synchronized.