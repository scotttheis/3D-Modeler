# R4 final - MCP server

R4 is complete through the ordered R4a, R4b, and R4c checkpoints. The Rust
adapter now serves the Python MCP contract over a private, client-launched
stdio transport and keeps core scene behavior, platform process/path seams,
and MCP protocol handling in separate crates.

## Delivered

- The catalog is exactly 60 tools in Python reference order. The profile
  counts are observe 7, model 52, files 59, and full 60.
- Tool metadata, annotations, strict bounded input schemas, structured-output
  allow lists, revision/project context, idempotency replay, and the
  `operation_status`/`cancel_operation` lifecycle are live Rust behavior.
- Core-backed scene mutations include grouped actions, transforms, colors,
  selection/history, project lifecycle, clipboard, mesh operations, and
  bounded PPM color sampling.
- Approved-root reads/writes canonicalize paths, reject reparse/symlink
  components, refuse overwrite unless requested, and default to deny-all when
  no trusted roots are configured.
- Slicer discovery and handoff are platform-owned. Handoff stages only the
  visible scene, bounds object count and staging retention, writes 3MF or the
  Bambu STL fallback, rechecks project/revision state, and launches only a
  discovered known slicer without a shell or arbitrary command endpoint.
- Operation records, replay entries, clipboard data, image input, result
  payloads, and slicer staging have explicit bounds and expiry/retention.

The protocol implementation follows the official MCP tools, stdio transport,
authorization, and Rust SDK contracts:

- https://modelcontextprotocol.io/specification/2026-07-28/server/tools
- https://modelcontextprotocol.io/specification/2026-07-28/basic/transports
- https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization
- https://github.com/modelcontextprotocol/rust-sdk

## Verification

- `cargo fmt --all -- --check` passed.
- `cargo check --workspace` passed.
- `cargo test --workspace` passed: 93 Rust tests plus doc-test targets; the
  MCP crate has 13 tests and the platform crate has 9 tests.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  passed.
- `cargo deny check` passed. It retains the existing unmatched BSD-3-Clause
  allowance and duplicate transitive `hashbrown`/`syn` warnings.
- `cargo audit` could not run because the installed Cargo has no `audit`
  subcommand; it was not installed or changed as part of R4.
- `cargo bloat --release --package builder3d-mcp --bin r4_mcp` passed: 3.6 MiB
  `.text`, 4.3 MiB executable.
- The official Python MCP client discovered all 60 tools and returned
  structured results. It negotiated protocol `2025-11-25`; a raw discovery
  probe confirmed server compatibility through `2026-07-28`, with tools
  capability and exit code 0.
- The behavior probe exercised status, scene creation, view state, print
  checks, mesh health, asynchronous smoothing, operation polling, grouped
  actions, clipboard, and slicer discovery.
- Python reference tests `test_mcp_catalog.py` and `test_mcp_sdk.py` passed: 9
  tests. The Python project was not modified.
- Forbidden panic/debug scan found only `expect` calls inside Rust test
  modules. No production `unwrap(`, `expect(`, `panic!`, `todo!`,
  `unimplemented!`, or `dbg!` paths were found. The only process launch is
  the constrained known-slicer platform seam.

## Explicit limitations

`emboss_text` and `hollow` are advertised with the reference schemas and
return typed `Unsupported` results because their geometry kernels were not in
the completed R3 Rust core. They do not silently mutate, fall back to an
unsafe approximation, or expose arbitrary execution. `sample_image_color`
currently accepts the bounded portable PPM subset. Slicer availability and
the discovery count are host-dependent; this Windows host reported no
installed slicer to the official probe.

## Final backup

The additive final snapshot is:

`G:\Builder3D-Rust_snapshot_20261029-155958_R4-final`

It excludes every `target\` directory. Exact source/snapshot counts, bytes,
relative paths, and SHA-256 results are in `verification.md`. No commit was
created, and no existing snapshot or `L:\Builder3D` file was changed.
