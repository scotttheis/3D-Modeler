# Review pack: owner batch 7, Phase 3 — sculpt and mesh tools (2026-11-15)

Every image here was taken from one release build
(`target\release\modeler3d-app.exe`) with the review harness, in both themes
(`-light.png`, `-dark.png`). Older images for comparison are in `before\`.
Images show what was painted; they are not approval and do not test the
mouse. Brushes are Shift+drag on a selected object. Open the same exe to try
things by hand.

| Item | Files | What to look at |
|---|---|---|
| OF-48 Grab | `of48-grab` | Shift+drag on the surface: the spot you grabbed stays under the mouse and pulls the surface out like pinching clay, even when you drag past the edge of the object. |
| OF-49 Crease | `of49-crease` (before: `before\of49-crease`) | Seen along the stroke: a soft, clean groove instead of a lumpy, pulled-in edge. Going back and forth over the same line does not dig deeper. |
| OF-50 Smooth Brush | `of50-smooth-brush` (before: `before\of50-smooth-brush`) | A strong stroke round the top of the stock cylinder: before, the rim melted into waves; now the rim and corners hold. Bumpy, rounded surfaces still smooth out. |
| OF-51 Simplify | `of51-simplify` | Simplify opens a small window under the ribbon: a slider in 10 % notches, the face count before → after, a Simplify button. The window stays open for another notch. The sphere here is at half its faces and still round (wireframe on). On a cube it says it can't simplify without changing the shape. |
| OF-52 Subdivide | `of52-subdivide` | Wireframe, both cubes 48 faces: left is the old Subdivide, uneven slivers; right is the new one, even triangles. The window offers Whole object, or Area I brush over (Shift+drag over the part you want finer). |
| OF-53 Normalize | — | Not changed; a question for you (see the message). |
| OF-54 Hollow | `of54-hollow` | A 40 mm torus hollowed with a 1.5 mm wall, seen in X-ray: the inner shell shows through. This used to fail with the "resource limit" error. No orange repair mark in Items. |
