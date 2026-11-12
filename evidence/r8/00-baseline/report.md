# R8 baseline report

Date: 2026-10-30

The R7 native shell launches and is responsive, but it is still a conversion
shell rather than a day-to-day Builder3D workspace. The baseline native window
was 1296 x 879 pixels. The captured view is in
[`native-printwindow.png`](../baseline/native-printwindow.png).

Observed gaps carried into R8:

- the ribbon has the five tabs and core command labels, but no compact
  favorites strip, command grouping/overflow treatment, or persistent ribbon
  customization surface;
- the application opens directly into an empty grid instead of a welcome/recent
  surface and has no visible recent-entry fallback state;
- the object panel is a sparse list with basic selection buttons and no
  visibility, context actions, object metadata, or transform editing;
- dialogs are not yet reachable as a coherent native file/settings workflow;
- the viewport is a grid-only presentation in the empty state, without the
  build-bed/contact/selection cues needed for normal scene interaction;
- status, dirty-state, keyboard focus, disabled states, and operation feedback
  are not yet presented as a complete workflow.

The Rust command layer already supplies the R7 semantic seams. R8 changes are
therefore scoped to the eframe adapter, presentation state, and narrow typed
UI-facing helpers; Python remains a read-only oracle.
