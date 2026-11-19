# Robot-claw grab cursors
Goal: simple open/closed mechanical claw feedback for ready/held grabs.
Files: grab_cursor.rs, lib.rs, shell.rs, review_capture.rs, Invoke-Gate.ps1.
Use one final egui pass adapter for Grab/Grabbing across all widgets;
retain existing hit testing and add held feedback for orbit, pan and Grab brush.
Use vector drawing with a dark outline and light interior at display scale.
Verify: cursor pass state transitions, non-grab/focus/leave behavior, existing
Resize/Move/Rotate hit/drag tests, shortcut DnD, both-theme capture states,
Full gate, Release build and smoke. Independent review; owner approval pending.
Follow E:\RustDeveloper.md and AGENTS.md. Baseline and Settle recovery are
recorded in resize-settle.md and logs/resize-settle-recovery.txt.

Verified: Full gate logs/gate/20261118-120020-full passed: 521 workspace
tests, fmt/Clippy, deny/audit, MCP parity, smoke, 104 captures. Both new
cursor tests pass; Move/Rotate/Resize picking and cancellation tests pass.
Independent code and both-theme capture review found no blockers.
Capture pack: evidence/rebuild/captures/2026-11-18-settle-claw/README.md.
Release build, smoke and both-theme captures passed; EXE 23,396,352 bytes.
Recovery: `G:\3D-Modeler_snapshot_20261118-122146_settle-claw` (copy and file/byte counts verified).

Update 2026-11-19: artwork replaced to follow erros/claw.png; see STATUS.md "Now". Owner: "much better!".
