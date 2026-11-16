# Review pack: owner batch 7, Phase 3 follow-ups (2026-11-15)

Every image here was taken from one release build
(`target\release\modeler3d-app.exe`) with the review harness, in both themes
(`-light.png`, `-dark.png`). Older images for comparison are in `before\`.
Images show what was painted; they are not approval and do not test the
mouse. Brushes are Shift+drag on a selected object. Open the same exe to try
things by hand.

| Item | Files | What to look at |
|---|---|---|
| OF-59 Crease sharpness | `of59-crease-sharpness1` (before: `before\of59-crease-sharpness0`) | The Crease settings window has a new **Sharpness** slider. Same stroke, same sphere: at 0 (before) the soft groove you approved; at 1 a narrower, deeper, crisper groove. Try values in between. |
| OF-60 Simplify where I choose | `of60-simplify-area` (before: `before\of51-simplify`) | The Simplify window now offers **Whole object** or **Area I brush over**. Here a 50 % stroke went across the top of the sphere (wireframe on): bigger triangles along the stroke, the rest untouched and still joined. **Normalize is gone** from the Edit tab. |
| OF-61 Hollow | `of61-hollow-dialog`, `of61-hollow-error` | The Hollow window starts at **2.0 mm**. When a hollow can't be done (here 1 mm on the stock sphere) the message has a **Copy** button, in the window and in the status bar at the bottom; the text can also be selected with the mouse. The stock sphere now hollows at 2 mm (it failed at every thickness before). |
