# Owner feedback — 2026-11-12 (second batch)

Raw owner requests, in the order given. Reference for "how it should look/feel"
is Microsoft 3D Builder unless an item names something else (Blender, the
Python build at `L:\Builder3D`). Each item becomes a task card before work
starts; record approved deviations from Python parity in `STATUS.md`.

| # | Area | What the owner wants | Reference | State |
|---|---|---|---|---|
| OF-1 | Viewport measurements | Remove the floor ruler numbers ("50 mm … 1600 mm" crowding into the distance, `erros/numbers.png`) — not wanted even when objects are out there. Show measurements the way 3D Builder does: premium-looking dimension tags on the selected object only. | 3D Builder | **Redo in progress.** `e4c62b1` removed the floor labels and added badge tags — both wrong per the 3D Builder captures. (a) Floor numbers: back, painted flat on the floor plane at 50 mm steps, foreshortened and depth-tested (2026-11-12, awaiting owner review; captures `evidence/rebuild/captures/2026-11-12-of1-floor-numbers/`). (b) Resize view: open. |
| OF-2 | Contact shadow | The shadow under objects looks generic. Replace it with a convincing, grounded shadow. | 3D Builder | Open |
| OF-3 | Resize arrows | The arrows in Resize mode must look and behave like 3D Builder's scale handles. (Extends the 2026-11-11 "arrows feel cheap" item, S1 task 7.) | 3D Builder | Open |
| OF-4 | Top-bar hover | Hovering over a command or shortcut in the top bar must show a visible hover state. | 3D Builder | Open |
| OF-5 | X-ray | X-ray is too extreme and makes objects disappear. It should work like 3D Builder's: objects stay readable and you can see through them. | 3D Builder | Open |
| OF-6 | Wireframe | Wireframe should be a true wireframe like Blender's (clean edges over the model, not a shaded approximation). | Blender | Open |
| OF-7 | Paint / Color / Material | Paint, Color and Material currently do nothing. They must apply to the selection (and paint to faces/regions). | 3D Builder | Open |
| OF-8 | Edit-mode sculpt tools | Grab, Crease and the other newer tools under Edit no longer work. Restore them. The owner liked them in the Python build and likes Blender's versions even more. | Python Builder3D, Blender | Open |
| OF-9 | Merge color | Merging objects should give the result one color: that of the top-most selected object in the right-hand object list. | — | Open |
