# Review pack: owner batch 7, Phase 4 — Measure (2026-11-15)

Every image here was taken from one release build
(`target\release\modeler3d-app.exe`) with the review harness, in both themes
(`-light.png`, `-dark.png`). Images show what was painted; they are not
approval and do not test the mouse. Open the same exe to try it by hand.

| Item | Files | What to look at |
|---|---|---|
| OF-55 Measure ruler | `of55-measure-ruler`, `of55-measure-live` (before: the old Dimensions window, `evidence/rebuild/rb-04-scene-and-precise-transforms/captures/rust-light-scene-measure-after.png`) | **Measure** opens a Measure bar instead of a window. `of55-measure-ruler`: a finished ruler from the cube's bottom corner (square = snapped to a corner) to the cylinder's top rim (diamond = snapped to an edge). The label and the bar show the length and its X, Y, Z parts; the dashed red/green/blue lines show those parts in the model. `of55-measure-live`: only the first point placed; the ruler follows the pointer (round dot = plain surface, no snap). |
