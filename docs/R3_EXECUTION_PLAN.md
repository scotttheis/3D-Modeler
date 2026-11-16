# 3D Modeler Rust rewrite: R3 execution plan

This document is the persistent execution checklist for completing the R3
mesh-operation stage in `E:\3D-Modeler`. It does not replace or override
the workspace rules, project `AGENTS.md` files, `E:\RustDeveloper.md`, or the
authoritative order in section R of `L:\Builder3D\TODO.md`.

## Starting state

R0, R1, and all R2 file-I/O formats are complete in the current Rust rewrite:

- R2a: STL, OBJ, and PLY
- R2b: 3MF
- R2c: GLTF and GLB
- R2d: WRL

Read `WORK_LOG.md` before starting. Preserve its existing history and use it
for the current continuation point. The Python application at
`L:\Builder3D` remains the temporary behavioral reference and must not be
modified.

## Required preparation

Before changing code:

1. Recheck `git status` from `E:\3D-Modeler`.
2. Do not reset, checkout, delete, overwrite unrelated work, or commit.
3. Read in full:
   - `E:\AGENTS.md`
   - `E:\3D-Modeler\AGENTS.md`
   - `L:\Builder3D\AGENTS.md`, if applicable
   - `E:\RustDeveloper.md`
   - `E:\3D-Modeler\WORK_LOG.md`
   - `L:\Builder3D\docs\rust-rewrite.md`
   - section R of `L:\Builder3D\TODO.md`
4. Identify the exact R3 order from section R and do not invent a different
   order. Inspect each complete Python implementation and its relevant tests
   before implementing that operation in Rust.
5. Inspect the existing Rust mesh, scene, command, history, resource-budget,
   and error architecture before adding new APIs or dependencies.
6. Use `apply_patch` for file edits.

## Authoritative R3 scope

Complete every R3 task in the exact order specified by section R. The scope
currently includes:

- booleans through `manifold-csg`;
- mesh repair;
- simplification;
- subdivision;
- smoothing;
- all five authoritative brush operations; and
- density normalization.

Do not assume the brush names, parameters, ordering, or result contracts.
Derive them from section R, the Python source, and the Python tests.

Do not stop after booleans, repair, simplification, or the first brush. R3 is
complete only after every listed operation and its integration tests are done.

## Implementation requirements

For every R3 operation:

- Implement real Rust behavior, not a stub, placeholder, or test-only path.
- Integrate with the existing `modeler3d-core` mesh, scene, command, history,
  and resource-budget architecture.
- Preserve Python behavior for object identity, names, colors, metadata,
  transforms, visibility, selection, no-op detection, failure behavior, and
  undo/redo wherever those behaviors exist.
- Use typed `CoreError` results and leave the scene unchanged when an
  operation fails validation or resource checks.
- Enforce bounded vertex/face counts, memory growth, boolean complexity,
  recursion/iteration counts, and result sizes before expensive work.
- Reject non-finite coordinates, invalid indices, degenerate or otherwise
  unsafe input with recoverable errors.
- Handle topology-changing results deliberately; preserve or document winding,
  manifoldness, disconnected shells, and component behavior.
- Avoid `unwrap`, `expect`, `panic`, `dbg`, `todo`, and `unimplemented` in
  library code.
- Avoid unnecessary dependencies. If `manifold-csg` or another dependency is
  required, verify compatibility with the existing Rust toolchain and target
  platforms before adding it.
- Keep changes scoped to R3 and do not modify `L:\Builder3D`.

Where Python uses a third-party geometry engine, document the Rust equivalent,
its numerical behavior, and any deliberate tolerance or capability boundary.
Do not silently substitute a materially different algorithm.

## Required test coverage

Add focused Rust fixtures and tests for each operation. Cover, as applicable:

- valid simple and compound meshes;
- transformed target and tool objects;
- empty, malformed, non-finite, degenerate, non-manifold, open, disconnected,
  duplicate-face, reversed-winding, and self-intersecting inputs;
- face and vertex boundary limits;
- memory and operation-budget limits;
- topology and component changes;
- bounds, extents, volume, face/vertex counts, winding, manifoldness,
  watertightness, and operation reports;
- colors, metadata, object identity, selection, and transforms;
- no-op behavior and typed failure behavior;
- atomic failure behavior with no partial scene mutation;
- undo and redo after success and after failed operations;
- round trips or serialization behavior where relevant; and
- golden comparisons with the Python reference using explicit tolerances.

Run the relevant Python reference tests from `L:\Builder3D` for every
subtask. Tests must compare meaningful geometry and state results, not merely
successful return codes.

## Sequential subtask protocol

Work through R3 sequentially without waiting for another prompt. After each
subtask is genuinely complete:

1. Run its focused Rust tests.
2. Run its relevant Python reference tests.
3. Review the diff and check the library source for forbidden panic-style
   calls and accidental debugging paths.
4. Update `WORK_LOG.md` with the implementation, changed files, behavioral
   evidence, tests and results, limitations, remaining R3 work, and the next
   exact action.
5. Create `evidence/r3*/report.md` and a concise continuation checkpoint with:
   - completed subtask;
   - changed files;
   - tests/checks passed;
   - remaining R3 subtasks; and
   - next exact action.
6. Create a new additive, timestamped snapshot on `G:\`.
7. Exclude `E:\3D-Modeler\target`.
8. Do not modify existing snapshots.
9. Verify the snapshot with source/backup file counts, byte totals, relative
   paths, and hashes; record missing, extra, and mismatched files.
10. Compact or summarize the working context before continuing if it becomes
    large, preserving all decisions, failures, test results, and paths in the
    checkpoint and `WORK_LOG.md`.

Do not mark a subtask complete merely because it compiles. Finish behavior,
tests, documentation, reference comparison, and focused verification first.
If blocked by an external condition, exhaust safe in-scope alternatives and
record the blocker clearly before asking for direction.

## Final R3 verification

After every R3 operation is complete, run all of the following from
`E:\3D-Modeler`:

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --all-targets -- -D warnings
cargo deny check
cargo audit
```

If `cargo audit` is unavailable, report that explicitly rather than installing
tools without checking compatibility. Run all relevant Python reference tests
from `L:\Builder3D` again and record warnings separately from failures.

## Final backup and handoff

Only after all R3 implementation and final verification work is complete:

1. Create a new timestamped backup of `E:\3D-Modeler` on `G:\`.
2. Exclude `E:\3D-Modeler\target`.
3. Do not modify existing snapshots.
4. Verify that the backup exists.
5. Compare source and backup file counts and byte totals.
6. Compare relative paths, lengths, and hashes; report missing, extra, and
   mismatched files.
7. Update `WORK_LOG.md` with all R3 implementations, behavioral evidence,
   focused and full test results, the final backup path and verification,
   unavailable checks, limitations, remaining project work, and the next
   authoritative task after R3.

Do not commit.

The final handoff must report:

- changed files;
- every R3 task completed;
- behavior implemented for each operation;
- Rust tests and checks;
- Python reference results;
- final backup path and verification;
- limitations or unavailable checks;
- remaining project work; and
- the next authoritative plan task, which should be R4 MCP server work once
  section R3 is complete.
