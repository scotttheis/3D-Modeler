# R6 shell and packaging report

R6 was completed in the authoritative order from section R of
`L:\Builder3D\TODO.md`:

1. ribbon
2. commands
3. dialogs
4. theme-scoped artwork
5. launchers
6. installers

The Python reference tree was not modified. The Rust shell keeps the core,
platform, viewport, app, and shell boundaries intact. Ribbon and dialog events
cross typed command boundaries; geometry and file mutations remain in the
core-backed application state. Theme artwork is compile-time scoped and
validated before startup. Launchers and installers resolve the native sibling
executable without a Python runtime dependency.

## Verification

- `cargo fmt --all -- --check`: pass.
- `cargo check --workspace --locked`: pass.
- `cargo test --workspace --locked`: pass; 109 Rust tests plus doctest runs.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`: pass.
- `cargo deny check`: pass for advisories, bans, licenses, and sources. The
  existing transitive duplicate-version warnings remain informational.
- `cargo metadata --locked --format-version 1 --no-deps`: pass.
- `cargo bloat --release --package modeler3d-app --bin modeler3d-app -n 20`:
  8.9 MiB `.text`, 11.2 MiB file; the 40 MB installed gate is measured on the
  staged package below.
- Production Rust scan: zero forbidden `unwrap`, `expect`, `panic!`,
  `todo!`, `unimplemented!`, or `dbg!` markers outside test code.
- Launcher/installer script policy scan: zero Python, `cmd /c`, or
  `shell=true` matches. Three PowerShell files parsed with zero errors.
- `cargo audit` was checked but is not installed in this toolchain; cargo-deny
  advisories passed.
- Native `--smoke`: `objects=2 selected=1 tabs=5`.
- Python reference selectors: 183 passed. One pre-existing setup-contract
  failure still expects `K:\\builder3D.png` in the dirty L-drive private-use
  guide; the reference tree was not changed. The Windows host has no `sh`
  executable, so POSIX shell syntax was statically checked but not executed.

## Packaging measurement

The size-profile binary staged `target\r6-package-final` and its deterministic
ZIP. The package contains 13 files, including 12 manifest members plus the
manifest. Installed bytes are 9,425,015 (8.988 MiB), and the archive is
4,190,185 bytes (3.996 MiB). Both the 40 MiB installed and 15 MiB compressed
limits pass. All manifest SHA-256 values and all 13 ZIP entries verified;
manifest mismatches: 0. Archive SHA-256:

`9f441174880d8041f8ee03143d9474e51b7ee9cb936d0f1d70479a5e5f3de8ce`

## Per-subtask recovery snapshots

Each snapshot was additive, excluded `target\`, and was verified by sorted
relative-path, byte-length, and SHA-256 comparison with zero missing, extra,
byte, and hash mismatches:

- R6-01 ribbon: `G:\Builder3D-Rust_snapshot_20261029-211137_R6-01-ribbon`
- R6-02 commands: `G:\Builder3D-Rust_snapshot_20261029-211441_R6-02-commands`
- R6-03 dialogs: `G:\Builder3D-Rust_snapshot_20261029-211751_R6-03-dialogs`
- R6-04 artwork: `G:\Builder3D-Rust_snapshot_20261029-212104_R6-04-artwork`
- R6-05 launchers: `G:\Builder3D-Rust_snapshot_20261029-212252_R6-05-launchers`
- R6-06 installers: `G:\Builder3D-Rust_snapshot_20261029-214251_R6-06-installers`

The final additive snapshot is recorded in `evidence/r6/verification.md`.

## Limitations and next authority

The GUI still needs owner visual acceptance in both light and dark themes and
the owner’s MCP-parity promotion decision. Installer execution was not run as
a destructive system install; staging, guards, manifests, hashes, and package
limits were verified. Section R contains no R7 item. After owner acceptance,
the next authoritative action is to promote the R6 shell/MCP parity result or
write a new owner-approved plan.
