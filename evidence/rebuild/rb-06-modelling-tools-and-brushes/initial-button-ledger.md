# RB-06 initial button ledger

Date: 2026-11-01  
Reference: `L:\Builder3D` (read-only)  
Rust project: `E:\3D-Modeler`

This is the pre-implementation inventory required by the RB-06 handoff. The
reference routes were read from `src/main_window.py`, `src/viewport.py`, and
the related mesh-operation tests. Rust routes were inspected before edits.

| Surface / label | Reference route | Rust route before RB-06 | Defect or acceptance condition |
|---|---|---|---|
| Insert / Cube, Cylinder, Pyramid, Cone, Sphere, Torus, Hexagon, Wedge | `MainWindow._on_add_primitive` | `CommandState::add_primitive` | Reachable; Torus must open a parameter editor and persist metadata. |
| Object / Edit Torus | `_on_edit_torus` | unconditional `OpenDialog(Torus)` | Must reject a non-parametric selection instead of silently loading defaults. |
| Object / Repair and file-menu Repair | `_on_repair` | typed Repair dialog and core repair | Must commit only a real result and report failure/cancel without a success-looking status. |
| Edit / Simplify, Subdivide, Smooth | mesh-operation workers | synchronous `Scene::apply_*` calls | Must preserve one history entry and return typed failure/no-op results. |
| Edit / Grab, Crease, Smooth Brush, Blob | persistent tool popover + Shift-left stroke | `apply_center_brush` placeholder | Must activate a persistent tool, use picked path samples, preview without history, commit once, and cancel without history. |
| Edit / Normalize | persistent settings + selected-face stroke | center sample placeholder | Must select a picked face region, expose radius/target/passes, and commit once. |
| Edit / Merge | boolean union of two objects | `group_selected` | Wrong route; must consume tool geometry and retain target semantics. |
| Edit / Intersect, Subtract | boolean picker / exact two selection | direct `boolean_selected` only | One-selection picker, two-selection operation, invalid pick retry, Escape cancel. |
| Edit / Emboss, Hollow | typed editor and worker | typed dialogs and core path | Apply/Cancel/error path must be observable and non-destructive on failure. |
| Paint / Material, Paint, Color | material/color editor | shared Material dialog | Basic reachability and explicit error handling are required at the RB-06 boundary. |
| View / Center, Frame, Reset, Top, Side, Grid, Wireframe, X-Ray | camera/display commands | implemented command paths | Must remain reachable while brush/picker modes are active and must not create mesh history. |
| Favorites / overflow command buttons | persisted command IDs | calls `Modeler3dApp::dispatch` | Must use the same enablement and mode routes as ribbon buttons. |
| Items / health `!` and action buttons | health report, Repair, Copy/Cut/Paste/Duplicate/Delete | health row repair plus actions | Every visible action must either mutate through a real route or show a typed error. |
| Dialog / Apply, Cancel, close, Escape | modal accept/reject | egui dialog actions | Torus Cancel must restore its render preview; every Apply must report errors in-place. |

## Pre-edit defects carried into implementation

1. `CommandId::Merge` grouped objects instead of performing Boolean Union.
2. All five brush buttons applied one mesh-center sample from the shell.
3. Normalize did not use the picked face region.
4. Edit Torus accepted arbitrary selected meshes and loaded default parameters.
5. Torus insertion/editing had no Solid/Hole control or live render preview.
6. Boolean commands had no viewport target/tool picker or picker cancellation.

The existing Rust core already contains bounded primitives, repair, simplify,
subdivide, smooth, brush, density, boolean, emboss, and hollow operations.
RB-06 work therefore repairs the application interaction seams and adds only
the missing adapters/tests; the Python tree remains untouched.

## Post-implementation disposition

The final audit was performed after the Rust source and native replay were
complete. `pass` means the route has an implementation and automated contract;
the native column identifies the accepted OS-input evidence where it was
appropriate. Visual review and owner acceptance are still pending.

| Surface | Implementation / automated | Native evidence | Disposition |
|---|---|---|---|
| Insert primitives and Torus | pass | Torus dialog, preview, apply | pass |
| Object Edit Torus | pass; typed rejection for non-parametric mesh | Torus route exercised | pass |
| Repair | pass | Repair dialog reachability | pass |
| Simplify/Subdivide/Smooth | pass | commit records in both themes | pass |
| Grab/Crease/Smooth Brush/Blob | pass; persistent gesture, one commit | active/before/after Shift stroke per brush | pass |
| Normalize | pass; picked-region options | active/before/after Shift stroke | pass |
| Merge | pass; Boolean Union | picker, cancel, exact-two, commit | pass |
| Intersect/Subtract | pass; exact-two route | committed results in both themes | pass |
| Emboss/Hollow | pass; typed apply/error/cancel core path | dialogs and cancellation | pass |
| Paint / Material | pass for RB-06 reachability/error boundary | Paint tab and Material dialog | pass |
| View commands | pass; no mesh history | View toggle sequence | pass |
| Items actions | pass; typed selection/action routes | All/None/Invert action sequence | pass |
| Dialog Apply/Cancel/Escape | pass; preview restore and typed actions | Torus apply; Boolean/Emboss/Hollow cancel | pass |

The accepted native operation journal is `rust-native-captures.ndjson`; the
compact run index is `accepted-journal.ndjson`. Emboss/Hollow successful native
geometry applies and corrupted-mesh Repair are deliberately listed as
automated-only limits rather than inferred from dialog screenshots.
