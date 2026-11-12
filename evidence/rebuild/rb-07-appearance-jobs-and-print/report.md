# RB-07 completion report

**Date:** 2026-11-01  
**Status:** implementation, automated verification, and native interaction complete at the RB-07 boundary; visual review and owner acceptance remain pending.

## Delivered

- Connected the existing Rust appearance handlers to the native Paint surface. Material, Paint, and Color reach the same selected-object solid RGBA editor. Applying to a multi-selection is one undoable command group; cancel/close/reopen preserves the prior authored values. The average-image-color route uses a native picker or entered path, reports bounded input errors, and applies an opaque solid color. It does not create texture coordinates or a texture, matching the Python reference contract.
- Preserved authored per-face/material color through mesh replacement and color undo, rendered the per-face palette in the viewport, and verified 3MF color export/re-import. A sampled color remains a solid color rather than a texture.
- Reused OperationRegistry and connected native shell work to background workers, progress polling, cooperative cancel, terminal error reporting, retry, and revision/target checks before commit. Print readiness and staging also run off the UI thread. Duplicate work and stale worker results are rejected by the shell guards.
- Completed native print preparation, staged-output validation and cleanup, supported slicer selection, Bambu Studio STL fallback, and the constrained launch path. The installed Bambu Studio process opened the staged 12-triangle STL and was closed after capture.
- Audited the affected native command and dialog actions in initial-button-ledger.md. Automated-only edge cases and unconfirmed themed capture behavior are marked explicitly.

Existing image_color, operations, print_prep, print_handoff, and slicer work was reused and verified at the native shell boundary. No broad legacy R7 rewrite and no MCP interface change was made.

## Verification and evidence

- Rust workspace: **301 passed**, zero failures. Focused runs: app 150, core 88, platform 15, viewport 34.
- Python focused reference suite: **103 passed**. Full reference suite: **785 passed, 1 failed**, with two existing trimesh runtime warnings. The one failure is a stale K:\builder3D.png assertion in L:\Builder3D\docs\private-use.md; that tree is read-only and its 116 pre-existing Git status records and the ten recorded behavior-source hashes are unchanged.
- All **10 documented Rust quality gates passed**. cargo audit is not installed and was not installed.
- Accepted native evidence: **117 PNGs**: Python 34 (17 Light, 17 Dark), Builder3D Rust 82 (40 Light, 42 Dark), and one actual Bambu Studio frame. Builder3D captures are 1280x840 at 96 DPI / 100%; each frame has a SHA-256, theme, route, build identity, and PNG modification timestamp in capture-metadata.json and accepted-journal.ndjson.
- Final Rust debug executable SHA-256: 0BF7753FE460A6A854906457F86363DDC905B455FA790760B6D9122F6F9CB0D6. Release executable SHA-256: E26C37DE652B6434727AE7F6396012CDF4AA911299857B621D336AFB4DA38448 (15,261,184 bytes).
- Bambu Studio at C:\Program Files\Bambu Studio\bambu-studio.exe opened the staged STL. The captured slicer toast reports 40 x 40 x 40 mm, 64,000 mm3, and 12 triangles. The staged fixture hash is EC1B527705B8045D5748A532519BF9EAE4AD3B7DDAD6081B85C3386669A8C15C.

## Limits kept visible

- Native Rust cancellation is confirmed in Dark; Rust Light shows running progress, a successful retry, Subdivide completion, and the resource-limit failure, but repeated Light Cancel attempts did not confirm a cancelled terminal frame. Python native cancellation is confirmed in both themes; Rust cancellation/no-partial-commit is also covered by shell tests.
- Corrupt-image failure was shown natively. Missing, unreadable, oversized-byte, oversized-pixel, and unsupported-image failures are covered by automated checks; those inputs were not all displayed in a native dialog.
- Python native Print Prep menu dispatch did not open its report through the runner's QAction route. The clean report was captured manually in both themes. Python native external slicer launch was not captured; the reference adapter tests cover chooser/fallback behavior and the Rust native handoff was launched in the actual installed slicer.
- Per-frame event timestamps were not emitted by the native runners; the metadata records each accepted PNG's filesystem last-write time in UTC and identifies that basis. Python native result JSON is retained for its 32 runner captures; two clean-report frames are manually recorded.
- Only Windows 11 at 96 DPI / 100% display scale is represented. Visual comparison and owner acceptance remain pending.

## Recovery and next slice

The additive RB-07 baseline is G:\Builder3D-Rust_snapshot_20261101-132912_RB-07-baseline. The final verified snapshot is G:\Builder3D-Rust_snapshot_20261101-162515_RB-07-complete; see snapshot-receipt.md. L:\Builder3D remained read-only. No files were staged or committed. **Next slice: RB-08.** Stop here; no RB-08 work is included.

