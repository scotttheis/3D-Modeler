# RB-06 modelling tools and brush completion

Date: 2026-11-01  
Reference: `L:\Builder3D` (read-only)  
Rust project: `E:\Builder3D-Rust`

RB-06 is complete at its implementation, automated-test, and native-interaction
boundary. The work stops before RB-07. Visual review and owner acceptance remain
pending; the captures are evidence for review, not an acceptance claim.

## Completed surfaces

- Primitive insertion and Torus editing now use bounded typed parameters,
  Solid/Hole state, metadata persistence, and a renderable live preview.
- Edit Torus rejects a selected mesh without parametric Torus metadata instead
  of silently opening a default editor.
- Repair, Simplify, Subdivide, Smooth, Normalize, Emboss, and Hollow use the
  typed core operations and preserve the one-command/error/cancel boundary.
- Grab, Crease, Smooth Brush, Blob, and Normalize are persistent shell tools.
  A picked Shift-primary path previews without history and commits one bounded
  replacement on release; Escape cancels without a history entry.
- Merge is Boolean Union. Intersect and Subtract use the exact-two selection
  route, while a one-selection request opens a picker and Escape cancels it.
- View, Object, Paint, Items, and dialog buttons were audited for enablement,
  route, cancellation, and typed failure behavior. The native run exercised the
  visible command surfaces and the focused Rust tests cover the mutation paths.

## Evidence boundary

The Python reference suite ran read-only first: 272 tests passed with two
pre-existing trimesh divide warnings. Its native reference replay produced six
baseline captures (empty, primitive Cube, and a modelling fixture in Light and
Dark). The Rust replay then used actual Windows pointer, keyboard, and mouse
input at a 1280 x 840 client size and 96 DPI. It produced 72 accepted records,
36 per theme, against debug executable SHA-256
`82284b6fd85221df3a804790cafb1171c469c5cb614460cb73199068ac551d14`.

The Rust native set includes Torus preview/apply, all five persistent brushes,
Simplify/Subdivide/Smooth, Boolean picker/cancel/Union/Intersect/Subtract,
Emboss and Hollow dialogs, Repair, Paint/Material, View controls, and Items
actions. Both final Subtract frames report `Subtract: 10496 face(s)`; the
earlier empty-result harness attempt is rejected and documented separately.

## Status

| Dimension | Status |
|---|---|
| Implementation | pass |
| Automated Rust verification | pass |
| Read-only Python reference verification | pass |
| Native OS-input evidence | pass |
| Visual review | pending |
| Owner acceptance | pending |
| RB-03/RB-04/RB-05 review statuses | preserved; pending where previously pending |
| Next slice | RB-07; not started |

See [verification.md](verification.md), [requirement-test-map.md](requirement-test-map.md),
[walkthrough.md](walkthrough.md), [paired-comparison-board.md](paired-comparison-board.md),
[capture-metadata.json](capture-metadata.json), and [checkpoint.md](checkpoint.md).
