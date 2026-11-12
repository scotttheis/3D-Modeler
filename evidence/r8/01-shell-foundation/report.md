# R8-01 native shell and workflow foundation

Date: 2026-10-30

This slice replaces the R7 conversion shell with a usable native eframe
workspace surface. It is scoped to the R8 shell, navigation, object-panel,
presentation-state, and workflow seams; R1-R7 conversion coverage was not
repeated.

Implemented behavior:

- welcome surface with New/Open/Import actions and bounded recent-entry
  Ready/Missing/Unsupported fallback states;
- compact ribbon favorites, pinning, per-tab command ordering, reset/default
  behavior, overflow access, file menu, dirty title/status state, and light /
  dark presentation modes;
- object panel selection modifiers, visibility, group/ungroup, cut/copy/paste,
  duplicate/delete, context actions, and transform editing with grouped core
  history commands;
- numeric position/size/rotation/scale/arrange controls with explicit mm/in,
  degree, and scale suffix handling;
- native settings, import-unit/display-unit, material/color, and print-prep
  dialog paths, including slicer discovery, bed-fit checks, staged 3MF/STL
  output selection, and typed no-slicer/error feedback;
- build-bed/grid/contact/selection viewport cues, artwork-backed primitive
  ribbon controls, keyboard shortcuts, tooltip labels, and a foreground status
  strip showing dirty state, selection, transform, and active operation status.

The Python application remains a read-only behavioral oracle. The relevant
oracle surfaces were `L:/Builder3D/src/main_window.py` and
`L:/Builder3D/src/toolbar.py`; no Python source or existing snapshot was
changed.

Manual evidence from the native executable:

- [`native-final-shell.bmp`](native-final-shell.bmp) records the welcome,
  favorites, ribbon, pin/theme controls, and primary actions.
- [`native-status-area.png`](native-status-area.png) records the rebuilt
  full-width status strip and the empty-state presentation after the live
  launch.
- [`native-scene-status-fixed.bmp`](native-scene-status-fixed.bmp) records the
  scene viewport, build bed, grid, object cue, and object panel from the live
  scene inspection before the final status-area anchor adjustment.

The current native process was closed after inspection. Windows' lock-screen
foreground prevented reliable synthetic pointer clicks during the final live
pass, so interaction coverage is backed by the focused Rust tests and the
native smoke gate rather than by claiming an automated click path.
