# Product plan reset - 2026-10-30

Owner requested a comprehensive replacement plan that reproduces the original
functionality and aesthetic and reuses the Rust app where that is faster.
This is a documentation/status slice; no application implementation is claimed.

## Findings and corrections

- Reviewed workspace instructions, Rust practices, original rewrite/architecture,
  UI style guide, artwork manifest, source workflows, testing/MCP contracts,
  historical TODO decisions, Rust catalog/source and R8 final records.
- Confirmed original owner artwork is present: eight Insert files and 44 action
  files in each theme, plus four root asset files. No assets were modified.
- Current shell paints object rectangles and uses rectangular hit targets;
  a separate R5 renderer implementation exists. Core and backend tests remain
  useful but cannot prove a working integrated frontend.
- Inspected archived native and original captures. The Python baseline capture
  `evidence/r8/00-baseline/python-reference.png` is blank, so it cannot be the
  accepted visual baseline. The Rust capture shows substitute icons/layout.
- The R8 final report admits blocked interactive checks while claiming product
  completion. That readiness claim is withdrawn, without erasing test history.
- Corrected the earlier blanket suggestion to replace the frontend: use the
  existing Rust workspace/eframe first, repairing or replacing components based
  on actual integration and original-design evidence.

## Changes

New authority: `docs/product-rebuild-plan.md`. Retired the old R8 brief; added
supersession notices to the final report/checkpoint and verification scope.
Linked the new authority from `AGENTS.md`. Reopened catalog shell/adapters as
`polish` and commands as `ported`; retained individual contract test records.
No Python/source/runtime behavior, existing snapshots or commits changed.

The new plan includes feature-level evidence, original asset provenance,
matched captures, early owner review, real renderer/shared-state integration,
full workflow inventory, failure/cancel/history checks, resource/package/host
gates, and complete recovery snapshots. Product implementation starts at RB-00
when the owner directs execution; it is not claimed complete by this plan.

## Recovery baseline

`G:\Builder3D-Rust_snapshot_20261030-132411_rebuild-plan-baseline`

Robocopy exit 1; source and snapshot: 443 files, 50,471,141 bytes each.
Included evidence, work log and hidden Git metadata; excluded `target` and
`.cache`, with junction traversal disabled. Final verification is recorded in
`WORK_LOG.md` and the companion checkpoint after document checks.
