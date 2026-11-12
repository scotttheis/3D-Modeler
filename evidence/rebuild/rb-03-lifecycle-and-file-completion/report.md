# RB-03 lifecycle and file completion report

**Status: RB-03 implementation and engineering evidence recorded; owner visual review pending.** Focused automated coverage, integrated Rust gates, required Rust native workflows, independent format verification, and sequential Python/Rust operation captures are recorded. This report does not claim visual pass. The owner has directed continuation into RB-04 without waiting on the pending RB-03 review; no RB-04 implementation has started.

## Completed

- Finished `.3dp` lifecycle, atomic replacement, schema validation/migration, savepoint dirty tracking, project/recent dispatch, and failure preservation.
- Completed STL/OBJ/PLY/3MF/glTF/GLB imports, the six reference unit choices, authoritative units, multi-object fields, and grouped undo behavior.
- Completed STL/OBJ/PLY/3MF/glTF/GLB/WRL exports, selected-visible rules, overwrite cancellation, and independent readers.
- Completed recent ordering/persistence, model previews and fallback states, and missing/corrupt/unsupported handling.
- Passed the integrated Rust gates with 228 workspace tests. See [verification.md](verification.md).
- Captured the Python reference while performing old-app lifecycle, imports, export, cancellation, and recent-file actions. The side-by-side board is [visual-comparison.html](visual-comparison.html); observed differences are described in [visual-comparison.md](visual-comparison.md).

## Acceptance state

| Area | Implementation / automated | Native Rust | Python-reference UI | Visual / owner |
|---|---|---|---|---|
| Project lifecycle | Pass | Save/Open/New/Exit and dirty guards exercised. | Native Save Project and dirty prompts captured during operation. | Captured; owner review pending. |
| Imports and units | Pass | All six import formats, mm/in, authoritative units, and multi-object cases exercised. | Old picker, mm/in dimensions, 3MF/glTF/GLB, cancel, and malformed input captured. | Captured; owner review pending. |
| Exports | Pass for all seven formats; independent verification passed. | Seven formats and selection/visibility matrix exercised. | Shared Export Mesh picker, successful 3MF, overwrite prompt/cancel captured; no per-format Python replay. | Captured; owner review pending. |
| Failure/cancellation | Pass in tests and native cancellation/error flows. | New/import/export cancel, overwrite, corrupt project and unsupported recent errors exercised. | Native prompt/error appearance captured while performing each represented action. | Captured; owner review pending. |
| Recents and previews | Pass in tests. | Restart, dispatch, previews/fallbacks, missing/corrupt/WRL cases exercised. | Two-column wireframe cards, dispatch, and missing/corrupt/WRL dialog appearance captured. | Captured; owner review pending. |
| Engineering gates | Pass, with `cargo audit` unavailable and bloat CLI caveat recorded. | Smoke and release build passed. | Focused reference tests passed; broad desktop suite not run. | Not applicable to build gates. |

## Recorded visual differences

- Python uses native Windows Open/Save dialogs and message boxes; Rust uses in-app file forms and error/status surfaces.
- Python recent tiles use a two-column scrolling layout with large wireframe thumbnails. Rust uses a four-column layout with compact shaded previews, file status, and actions.
- Python malformed and recent-file errors use native modal dialogs. Rust shows detailed import errors in the import form and recent errors in the Welcome status area.
- Some screenshots have different accumulated scene counts or recent ordering; each pair marks the difference. The board is not presented as a pixel-matched score.

## Remaining review and exact next slice

The comparison package is ready for the owner to inspect. The Python GUI did not replay all seven export filters individually after the desktop locked; the Rust app did complete and independently verify all seven formats. Two interactive ledger rows remain partial: the full live saved-checkpoint undo/redo/new-branch sequence, and every filesystem-level read/write failure. Their automated coverage passes. These pending review items do not block RB-04 under the owner's latest direction.

The exact next slice is **RB-04 — Scene and precise transforms**. No RB-04 work is included here; continue using [`rb-04-through-rb-11-completion-handoff.md`](../../../docs/rb-04-through-rb-11-completion-handoff.md).
