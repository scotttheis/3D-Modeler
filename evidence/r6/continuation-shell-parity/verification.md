# R6 continuation verification

## Source and dependency gates

- `cargo fmt --all -- --check`: pass.
- `cargo check --workspace --locked`: pass.
- `cargo test --workspace --locked`: pass, 133 tests; all doctest runs passed
  with zero tests.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D
  warnings`: pass.
- `cargo deny check`: pass for advisories, bans, licenses, and sources. It
  reports the existing duplicate-version warnings and uses the documented
  `RUSTSEC-2026-0192` exception for the pinned named-font rasterizer.
- `cargo metadata --locked --format-version 1 --no-deps`: pass.
- `cargo audit`: unavailable because this Cargo installation has no audit
  subcommand.

## Runtime and size checks

- Native smoke: `Builder3D shell smoke: objects=2 selected=1 tabs=5`.
- Owner visual acceptance: light and dark native shell accepted on
  2026-10-30.
- Official Python MCP client stdio probe: protocol `2025-11-25`, 60 tools,
  Emboss completed with 72 occupied cells, Hollow completed through the
  `convex-exact` branch at `9128.000000000015` mm³, and MCP undo returned
  `changed=true`.
- `cargo bloat --release --package builder3d-app --bin builder3d-app -n 20`:
  9.1 MiB `.text` section and 11.4 MiB executable file.
- Size-profile package: 13 files, 9,621,111 installed bytes (9.175 MiB),
  4,282,705 ZIP bytes (4.084 MiB), both under the 40 MiB installed and 15 MiB
  compressed gates.
- Package manifest: 12 member entries, zero missing/extra/byte/hash
  mismatches.
- ZIP entries: 13, including `PACKAGE-MANIFEST.json`, with no extra or missing
  entries.
- Archive SHA-256:
  `a9754d7c8af8c0de0d97609c6aa50cc50d52c6d3358bf65cb88c412d84ac3c3f`.

## Recovery snapshots

The pre-change additive baseline was copied and verified before implementation:
356 files, 5,550,395 bytes, zero missing/extra/byte/hash mismatches, and no
`target\\` content. The final snapshot listed in `checkpoint.md` is created
after this evidence and the final source checks; it excludes `target\\` and is
verified with the same relative-path, byte-length, and SHA-256 comparison.
Excluding this self-referential verification artifact, both manifests contain
360 files and 5,638,063 bytes, with zero missing paths, extra paths,
byte-length mismatches, or SHA-256 mismatches. Hidden files in both manifests:
0. The aggregate SHA-256 over sorted UTF-8 `path|bytes|sha256` lines is
`8622afdb6a2d5ff2edc035c3a6c351759b494cf300f0025a8d9e0219fcbfead9`.
`target\\` is absent from the snapshot.

The pre-existing final R6 snapshot
`G:\Builder3D-Rust_snapshot_20261029-214752_R6-final` was not modified.

