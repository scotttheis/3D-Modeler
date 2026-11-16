# Review pack: OF-65 transform strip clean-up (2026-11-16)

`owner-cleanup.png` is the owner's marked-up screenshot. Every other image was
taken with the review harness from a `review`-profile build, in both themes
(`-light.png`, `-dark.png`): `before-*` from the code before the change,
`after-*` with it. States `move`, `rotate`, `resize` (one cube selected).
Images show what was painted; they are not approval.

| Item | Files | What to look at |
|---|---|---|
| Count and unit note removed | `before-move-*` vs `after-move-*` | The "1 selected  mm" text and the line before it are gone. The count is still in the Items header ("1 of 1 selected") and the status bar ("Selected: 1"); every box already shows its unit ("20.000 mm", "0.000 deg"). |
| Tabs centred over the boxes | `after-move-*`, `after-rotate-*`, `after-resize-*` | Owner follow-up 2026-11-16: each label sits straight above its box, centred on it. Each tab is exactly as wide as its box, with the same left and right edges; the X / Y / Z letters and Apply have nothing above them. Move over the X box, Rotate over Y, Resize over Z. Test `transform_strip_has_no_count_and_each_tab_is_centred_over_its_box` renders the real strip in all three modes and checks the edges. |
| Rotate shows "0.000", not "-0.000" | `after-rotate-*` | Owner yes 2026-11-16. An unrotated cube's Y box used to read "-0.000 deg" (a tiny negative rounding leftover). Angles now round like distances already did: anything that would show as zero shows "0.000 deg". Test `angles_near_zero_show_without_a_minus_sign`. |

MCP: nothing changes (display only).
