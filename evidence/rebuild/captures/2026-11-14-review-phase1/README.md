# Review pack: owner batch 7, Phase 1 (2026-11-14)

Every image here was taken from one release build
(`target\release\modeler3d-app.exe`) with the review harness, in both themes
(`-light.png`, `-dark.png`). Older images for comparison are in `before\`.
Images show what was painted; they are not approval and do not test the
mouse. Open the same exe to try things by hand.

| Item | Files | What to look at |
|---|---|---|
| OF-4 (from last time) | `of4-hover-tab`, `of4-hover-button` | Buttons and tabs light up when the pointer is over them. |
| OF-28 No console window | (none) | Start the exe: no black command window behind the app. |
| OF-29 App icon | `of29-exe-icon.png` (before: `before\of29-exe-icon.png`) | The exe file shows the blue 3D Modeler cube instead of the plain program icon. |
| OF-30 Light Cube icon | `of30-insert-tab` | New glass cube on the Insert tab in light mode; dark mode unchanged. |
| OF-31 Floor numbers | `of31-floor-framed`, `of31-floor-top` | Each number starts at the corner where its 50 mm line meets the axis. |
| OF-32 Resize arrows | `of32-resize`, `of32-resize-drag` | Arrows twice as big; they are also twice as easy to grab. |
| OF-33 Grab cursor | (none) | Hover any arrow or handle: open hand. Press and drag: closed hand. |
| OF-34 Selection colour | `of34-selection-colors` | Red and blue cubes are selected: same colour, a bit darker (red no longer turns purple). No outline yet; it comes with X-ray (OF-47). |
| OF-35 Darker dark-mode viewport | `of35-viewport-shade-a-dark` (built in), `of35-viewport-shade-b-dark` (darker) | Pick A or B. Light mode unchanged. |
| OF-36 Movable dialogs | `of36-paint-dialog`, `of36-41-mirror-dialog` | Drag any dialog by its title bar; it stays on screen. Settings stays docked. |
| OF-37 Ungroup after Merge | `of37-ungroup-after-merge` | After a Merge, Ungroup is grey. |
| OF-38 Merge of parts that don't touch | `of38-merge-not-touching` | They become a Group and a note shows for about 4 seconds. |
| OF-39 Repair result readable | `of39-40-repair-clean-model` | Result text in the normal text colour; the button says Close. |
| OF-40 Repair with nothing to fix | `of39-40-repair-clean-model` | "No problems found — your model was already clean. Nothing was changed." No undo step is added. |
| OF-41 Mirror axis colours | `of36-41-mirror-dialog` | X red, Y green, Z blue, like the arrows. |
| OF-42 Align and Arrange | `of42-align-one-object`, `of42-align-two-objects` | Grey with one object (hover for the reason); work with two or more. |
| OF-43 Paste and Duplicate | `of43-paste-twice` | Copies sit 10 mm beside the original, then beside each other. |
| OF-45 Color button removed | `of45-paint-tab` | Paint tab shows Material and Paint only. |
