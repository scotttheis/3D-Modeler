# Rotation controls follow the object

Goal: Rotate rings and arrows follow the selected object's orientation, live
during a drag and after commit, with matching picking and drag axes.
Files: shell.rs, shell/rotate_controls.rs, transform_gesture.rs,
review_capture.rs, STATUS, this card and review captures.
Do: reuse the existing signed rotation basis; use the first selected object's
orientation for a multi-selection and rotate that frame during its preview. Preserve history and
the shared arbitrary-axis transform commit path.
Verify: frame/geometry/picking/preview regression tests, Fast and Full gates,
both-theme captures, independent diff review, Release build and smoke.
Out: Move/Resize behavior, numeric rotation semantics, MCP interface changes.
Baseline: G:\3D-Modeler_snapshot_20261118-070332_rotate-follow-baseline
(12,395 files, 1,805,413,409 bytes; count/bytes verified).

Review: independent diff review found no remaining blockers after fixing the
multi-selection release jump and adding both states to the gate capture list.
Captures: `evidence/rebuild/captures/2026-11-18-rotate-follow/` (both themes;
upright, tilted and live local-axis drag). Captures inspect rendering only;
the regression exercises gesture mathematics and the shared commit/history path.

Release: locked build completed in 4m38s (23,378,944 bytes); `--smoke` exit 0.
Both-theme Release captures inspected. Interactive Release opened on a tilted
cube with isolated review settings for the owner to try.

Full gate: logs/gate/20261118-071412-full (all checks passed; 92 captures).
Recovery snapshot: G:\3D-Modeler_snapshot_20261118-072157_rotate-follow (source, evidence and Git metadata).
