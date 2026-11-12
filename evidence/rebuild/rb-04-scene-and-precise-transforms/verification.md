# RB-04 verification

Current-source gates: [final-native-gate-results.json](final-native-gate-results.json), [build artifacts and source SHA-256](final-native-build-artifacts.json). Earlier gate records and failures remain historical.

| Check | Result |
|---|---|
| Workspace tests | **261 passed, 0 failed, 0 ignored**, logs/modal-complete-tests.txt |
| fmt / check / metadata | Pass; locked workspace configuration |
| Strict Clippy | Pass, workspace/all targets/all features, warnings denied |
| cargo deny | Pass; clipboard Boost exceptions version-scoped and notices packaged |
| Debug / ordinary release smoke | Both pass, separate from native interaction |
| Release | 14,972,928 bytes; SHA-256 `fb5c4d3427b73607378af8c74ccd83dc6428d01d1e6b934765971aaa855d1c87` |
| Explicit app bloat | 11.6 MiB .text / 14.3 MiB file; measured 14,963,712 bytes |
| Workspace bloat | Exit 0; tool ignores --workspace and analyzes a default binary, not a combined package total |
| cargo audit | Unavailable; no audit pass claimed |
| Read-only Python tests | 52 gizmo +25 scene/selection +11 service +7 controls +4 adapter, all pass |
| Real-source independent oracles | 122 gesture +32 multi-numeric +42 placement +9 Euler/polar |
| Native scenes | 74/74 cases verified, IDs/meshes/matrices/selection/revisions, exact mutation undo/redo |
| Native viewport | 22/22, committed/no-op/cancel/focus cases, grouped/hidden selection, both bases and uniform corners |
| Native Settle | 4/4; matrices match source within 9.32e-9 per entry, one action plus undo/redo |
| Native viewport pick | Both themes: empty -> object0 -> Ctrl-empty preserved -> ordinary empty cleared; exact geometry/history unchanged |
| Native modal control | Settings blocks Duplicate and numeric Apply; positive Duplicate after close works |
| Captures | 386 paired action states / 812 capture PNGs; PNG SHA-256 and linked paths validated |
| Preservation | 108 asset SHA pairs unchanged, Python Git status unchanged, 32 RB-03 status rows unchanged |

Native evidence details: [scene](native-scene-verification.json), [viewport](native-viewport-verification.json), [Settle](native-settle-verification.json), [modal](native-modal-verification.json), [Python scenes](python-scene-verification.json), [walkthrough](walkthrough.md). Each capture keeps its actual build hash. Final debug SHA-256 `82fd07e7691d76cb6f348ba6aafd86d91c9f9b94b56ee72e7de398a70477dfa1`. Preserved binaries are under ignored target/ and excluded from recovery copies; source/manifests/tests/notices/evidence are included.

Selection/no-op cases correctly have no undo/redo requirement; an `exact_undo_redo: false` flag on those records means not applicable, not a failed mutation test. Repair verifies equal bounds/area/four faces plus exact native undo/redo while retaining the recorded fan-point difference. Shift-range and Escape/focus outcomes are intentionally different from source; see report.md. Visual/owner acceptance remains pending. Performance, installed size and cross-host coverage are not accepted by this slice.
