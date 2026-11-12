# R8-02 presentation parity polish

Date: 2026-10-30

This slice closes the small presentation gaps found during the R8-01 live
inspection and Python-oracle comparison.

- Object-panel actions now use the same typed enablement policy as ribbon and
  context actions, with an explanatory disabled-state tooltip.
- The native viewport presents the authoritative command camera intent as a
  visible Perspective/Top/Side/Framed cue while retaining the existing
  renderer-neutral bed, grid, contact, color, selection, wireframe, and X-ray
  cues.
- The status strip transform summary is compact at the supported minimum
  layout width.
- The recent-file fallback test now uses the existing project manifest to
  exercise a real unsupported extension, in addition to the missing-path
  case.

The relevant Python oracle comparison used the ribbon customization and
camera/viewport command definitions in `L:/Builder3D/src/main_window.py`.
Python and existing snapshots remained read-only.

The final native launch capture is
[`native-final-shell-r8.png`](../01-shell-foundation/native-final-shell-r8.png).
The existing scene capture remains
[`native-scene-status-fixed.bmp`](../01-shell-foundation/native-scene-status-fixed.bmp);
the final camera cue is also covered by the command/view-state tests because
the desktop lock-screen foreground prevented a reliable synthetic scene click
during this pass.
