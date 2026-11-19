# 3D Modeler

A free, open-source 3D modeling app for 3D printing, inspired by Microsoft 3D
Builder.

3D Modeler is written in Rust. It runs on Windows, Linux and macOS, opens and
saves `.3dp` projects, imports and exports common 3D formats, and includes an
MCP server so AI assistants can drive the same tools you use on the desktop.

## Build and run

See [`docs/building.md`](docs/building.md) for the prerequisites (a Rust
toolchain as pinned in `rust-toolchain.toml`, a C toolchain and libclang). Then:

```
cargo build --release
target/release/modeler3d-app
```

On Windows, run `scripts/Initialize-BuildClang.ps1` in the same PowerShell
session before Cargo. Run the tests with `cargo test --workspace`.

## Licence

Licensed under the Apache License, Version 2.0. See [`LICENSE`](LICENSE).

## Disclaimer

3D Modeler is an independent project. It is not affiliated with, endorsed by
or sponsored by Microsoft. 3D Builder is a trademark of Microsoft Corporation.
