# S1-3b: Inspector fixes after capture review

Fix in the uncommitted S1-3 work (`chrome.rs` section/grid helpers and the
Inspector in `shell.rs`). Review captures at
`evidence/rebuild/captures/2026-11-19-polish-m3/` (move-dark.png, resize-light.png)
show:
1. Numeric fields clip their text: Position X reads "0.000 r" (the " mm" suffix is
   cut). Make the three axis fields fit the panel width: shrink the label column
   and gaps (tokens), and let fields share the rest equally; the value must never
   clip at the Normal size or Big size. If a suffix cannot fit, drop the suffix from
   the field display only when the unit selector already shows the unit
   (keep the parser accepting suffixes). Do not change parsing or behaviour.
2. In the Resize view "Apply scale" is cut off at the bottom edge and looks
   shorter than "Apply size"; the Items header is squeezed. Give the transform
   section its natural height (wrap in a ScrollArea only if needed) so both Apply
   buttons are complete, same height/width; Items takes the remainder.
3. The "Transform" title and the explanatory line below it are very similar in
   weight; make the title clearly a section title (FONT_SMALL, strong colour) and the note
   FONT_CAPTION muted.
Same rules as the S1-3 card: fmt + clippy only, no tests, no commit.
