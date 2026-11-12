# Product plan reset checkpoint

- Objective: replace the rejected R8 execution/acceptance plan with an original-
  design and functional-parity rebuild plan, using existing Rust where useful.
- Completed: source/notes/evidence review, replacement plan, retirement notices,
  instruction routing, catalog status correction and correction report.
- Application implementation: not started. No owner visual approval inferred.
- Baseline: `G:\Builder3D-Rust_snapshot_20261030-132411_rebuild-plan-baseline`;
  verified 443 files / 50,471,141 bytes, robocopy exit 1.
- Verification and final recovery receipt: see the latest plan-reset entry in
  `WORK_LOG.md`; this checkpoint does not certify a working rebuilt app.
- Risks: live original capture baseline still needed; real native renderer and
  same-scene AI integration require proof; existing tests cover narrower seams.
- Exact next work on execution authorization: RB-00 in
  `docs/product-rebuild-plan.md`, beginning with original source/UI inventory,
  provenance-checked real captures, and the feature-level parity ledger. Reuse
  eframe and backend by default; do not blindly restart or trust old R8 claims.
