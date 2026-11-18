# Resize by floor or imported object front

Owner request 2026-11-18: add to TODO and try a two-position icon-only switch
above Apply in the bottom Resize strip. Explain the modes in tooltips.
Floor uses fixed floor axes. Object uses imported mesh axes following its
current pose; a multiple selection uses the first selected object's axes.
Reuse the existing frame and atomic command for drag, live dimensions and
numeric Apply. Switching is presentation state with no history or dirtying.
Read shell.rs, chrome.rs, commands.rs, transform_gesture.rs, review_capture.rs.
Verify Fast/Full gates, typed-size/drag/units/undo and switch UI regression,
MCP basis-setting parity, both-theme captures for resize-floor/resize-object,
Release build and smoke. Owner visual acceptance follows the trial.

Baseline: G:\3D-Modeler_snapshot_20261118-110420_resize-basis-baseline | 13844 files | 2005805430 bytes verified.
Verified: Full gate `logs/gate/20261118-112226-full` passed (274 app,
57 viewport tests, MCP parity, 96 captures). Imported-model numeric sizing,
proportions, units, drag cancellation/commit, reverse-order multi-selection,
no-op/invalid requests, undo/redo and the clean-scene switch are covered.
Release build and smoke passed; both-theme Release captures inspected.
Release EXE: 23,402,496 bytes. Awaiting the owner's trial review.
Recovery: `G:\3D-Modeler_snapshot_20261118-112934_resize-basis-toggle` (counts in logs/resize-basis-recovery.txt).
