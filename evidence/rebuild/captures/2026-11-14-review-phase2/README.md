# Review pack: owner batch 7, Phase 2 (2026-11-14)

Every image here was taken from one release build
(`target\release\builder3d-app.exe`) with the review harness, in both themes
(`-light.png`, `-dark.png`). Older images for comparison are in `before\`.
Images show what was painted; they are not approval and do not test the
mouse. Open the same exe to try things by hand.

| Item | Files | What to look at |
|---|---|---|
| OF-44 Paint | `of44-paint`, `of44-paint-colour-dropdown` (before: `before\of44-paint-dialog`) | Paint opens a bar like 3D Builder's: Cancel, Done, the "Selected" colour with a drop-down, and Coverage. The pointer is a spray can in the chosen colour. Click an object to paint it. Coverage 100 % paints the whole object; lower paints just the area you click (0 % = one flat side). Each click is one Undo. Cancel removes this session's paint. |
| OF-47 X-ray | `of47-xray`, `of47-xray-four-cubes` (before: `before\of47-xray-four-cubes`) | The selected object stays solid with a thin light-blue outline; the others turn pale and see-through instead of vanishing. |
| OF-46 Material textures | (none) | Not built yet: it needs an owner decision on how textures are saved (see the stop message). |
