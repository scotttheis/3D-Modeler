# R8 final acceptance

> SUPERSEDED 2026-10-30. The product acceptance claim below is withdrawn after
> owner rejection and source/evidence review. This is a historical report, not
> current readiness evidence. See [the replacement plan](../../../docs/product-rebuild-plan.md)
> and [the correction record](../../rebuild/00-plan-reset/report.md).

Date: 2026-10-30

R8 is complete at the documented native eframe boundary. The Rust shell now
presents the major 3D Modeler workflows coherently over the existing typed
command, core, viewport, platform, and print seams:

- welcome/recent/new/open/save/save-as/import/export lifecycle with bounded
  missing and unsupported fallback states;
- ribbon tabs, grouped commands, artwork, favorites, overflow, pinning,
  per-tab order/reset, file menu, theme toggle, dirty title/status, and
  keyboard shortcuts;
- object panel selection/visibility/names/grouping/cut-copy-paste/duplicate/
  delete/context actions with clear disabled states and grouped transforms;
- settings, persisted theme/units/ribbon preferences, malformed-settings
  recovery, import-unit messaging, numeric suffixes, bounded ranges, and
  accessible labels/tooltips;
- viewport bed/grid/contact/object-color/selection/camera-mode/wireframe/X-ray
  cues with the existing R5 renderer contracts preserved in the viewport crate;
- operation status/progress/cancellation presentation, print-preparation
  warnings, known-slicer discovery/selection, 3MF staging, Bambu STL fallback,
  image-color sampling, and material/color controls.

Focused UI-state/routing coverage is GPU-free. The native executable was
launched for manual inspection and captured under
`evidence/r8/01-shell-foundation/`. Synthetic scene clicks were not claimed
because Windows' lock-screen foreground intercepted them during the final
pass; the live scene capture and all semantic interaction seams are recorded
in the slice evidence.

Final recovery snapshot: `G:\Builder3D-Rust_snapshot_20261030-123828_R8-final`.
Robocopy returned exit code 1 (success range); the source and snapshot each
contain 169 files and 1,730,290 bytes after excluding `target`, `evidence`,
and `WORK_LOG.md`.
