# Review pack: OF-65 transform strip clean-up (2026-11-16)

`owner-cleanup.png` is the owner's marked-up screenshot. Every other image was
taken with the review harness from a `review`-profile build, in both themes
(`-light.png`, `-dark.png`): `before-*` from the code before the change,
`after-*` with it. States `move`, `rotate`, `resize` (one cube selected).
Images show what was painted; they are not approval.

| Item | Files | What to look at |
|---|---|---|
| Count and unit note removed | `before-move-*` vs `after-move-*` | The "1 selected  mm" text and the line before it are gone. The count is still in the Items header ("1 of 1 selected") and the status bar ("Selected: 1"); every box already shows its unit ("20.000 mm", "0.000 deg"). |
| Tabs line up with the boxes | `after-move-*`, `after-rotate-*`, `after-resize-*` | Each tab is exactly as wide as the column below it (axis letter + box): Move over X, Rotate over Y, Resize over Z, same left and right edges. Apply has no tab above it. Test `transform_strip_has_no_count_and_each_tab_sits_over_its_axis_box` renders the real strip in all three modes and checks the edges. |

MCP: nothing changes (display only).

Seen while capturing, not changed: the Rotate Y box shows "-0.000 deg" for an
unrotated cube (`after-rotate-*`).
