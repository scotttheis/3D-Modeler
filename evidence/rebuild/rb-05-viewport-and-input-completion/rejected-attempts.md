# RB-05 rejected evidence

Rejected material is retained for audit and is not linked as accepted proof.

| Material | Reason rejected | Replacement/interpretation |
|---|---|---|
| `python-native-rejected-run-1.txt` | First Python harness stopped on a NumPy truth-value error while inspecting a primitive block. | Corrected Python replay produced 30 accepted records. |
| `rust-native-rejected-run-1.ndjson` through `rust-native-rejected-run-4.ndjson` | Partial or pre-final Rust harness runs; they contain incomplete sequences or an older binary/render pass. | `rust-native-results.json` is the final 52-record replay tied to `4a42ec…69232`. |
| `rust-native-rejected-run-4-error.txt` | Error journal from the pre-final Rust attempt. | Final replay completed with no current error journal. |
| `r5-diagnostic/report.json` and related diagnostic images | Renderer diagnostics before the final floor-line draw-order fix; useful for implementation debugging, not native acceptance. | Final native Rust captures show the corrected floor/grid ordering. |
| `rust-display-rejected-grid-run-1.json` and `rust-display-rejected-grid-run-2.json` | Supplemental direct View-ribbon replay captured an identical Grid-off frame and later hit a flaky native fixture-dialog precondition. | Do not count these as Grid-toggle proof. Rust grid geometry/display state remains covered by focused renderer/command tests and accepted resting floor/grid captures. |
| `rust-view-tab-test.png` | Coordinate smoke check with an empty welcome surface, not a paired fixture state. | Retained as diagnostic only. |

No rejected output is used to claim visual parity, owner acceptance, or a successful Grid-off native Rust capture.
