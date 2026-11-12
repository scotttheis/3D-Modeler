# RB-07 requirement and test map

| Requirement | Rust surface | Automated evidence | Native evidence / disposition |
|---|---|---|---|
| AP-01 Material, Paint, Color routes | commands.rs, dialogs.rs, shell.rs | App command/dialog/shell tests; Python tests/test_paint.py and desktop routes | Each Paint-tab entry opens the shared editor in both themes; cancel/apply/reopen frames in capture metadata. |
| AP-02 Authored solid RGBA color | Selected-object color command group, viewport palette | App command and viewport tests; Python test_paint.py | Multi-selection color apply and cancel in Light/Dark; authored red 3MF export/re-import captured. |
| AP-03 Average image color | image_color.rs, image picker and command action | Rust image/command tests; Python test_paint.py and test_ai_commands.py | Native picker, successful sample, corrupt-image error, and retry in both themes. Average sampling applies a solid color; it is not texture mapping. |
| AP-04 Bounded image errors | image_color.rs, dialogs.rs | Missing, unreadable, corrupt, unsupported, byte-size, pixel-size tests | Corrupt path shown natively in both themes. Other invalid inputs are automated-only. |
| JOB-01 Native background lifecycle | operations.rs, shell.rs worker/poll/commit path | Shell off-thread, cancel, retry, duplicate/busy, failure, close/reopen tests | Python Light/Dark and Rust Dark show progress/cancel/terminal/retry. Rust Light shows progress and successful retry; native cancellation was not confirmed in Light. |
| JOB-02 Stale-result safety | Shell target identity/revision gate | Selection-change and stale print/job shell tests | Automated-only for deliberate stale target/revision and busy duplicate; see operation-manifest.json. |
| PRINT-01 Read-only readiness report | print_prep.rs, dialogs.rs, shell.rs | Report, failure/retry, no mutation, fit/height/health/hidden/empty cases | Clean Python report both themes and Rust clean/warning reports; Python runner modal route needed manual report capture. |
| PRINT-02 Validated staging | print_handoff.rs, export.rs, shell job | Output reopen, format validation, stale cleanup, failure safety tests | Rust progress/ready in Light/Dark; staged Bambu STL independently verified as 684 bytes / 12 triangles. |
| PRINT-03 Known slicer handoff | Platform slicer.rs, app print dialog and worker | Chooser, path validation, Bambu fallback, missing executable and launch errors | Rust Light/Dark chooser/stage/launch flow and actual installed Bambu Studio load. Python external launch has automated reference coverage only. |
| BTN-01 Affected control audit | initial-button-ledger.md | All RB-07 command/dialog/shell/platform tests | Visible routes and unavailable native paired cases are enumerated per control in the ledger. |

## Baseline and result

Pre-change focused baseline: Python reference 103 passed; Rust app 125 passed; Rust platform slicer selection 2 passed. Final counts and complete suite result are in verification.md. The Python full-suite machine-path assertion remains an existing read-only reference failure. Visual review and owner acceptance remain pending.

