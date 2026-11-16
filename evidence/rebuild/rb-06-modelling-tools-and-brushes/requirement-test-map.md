# RB-06 requirement-to-test map

Implementation, automated verification, and native interaction are complete.
Visual review and owner acceptance remain pending.

| Requirement | Rust implementation / automated evidence | Native evidence |
|---|---|---|
| Primitive insertion and editable Torus | `CommandState::add_primitive`, `edit_torus_selected`, Torus metadata tests in `crates/modeler3d-app/src/commands.rs`; bounded dialog tests in `dialogs.rs` | `torus-dialog`, `torus-hole-preview`, `torus-applied` in both themes |
| Repair and typed failure | `Scene::repair_selected`, repair integration tests, command error/status paths | `object-tab`, `repair-dialog` in both themes; corrupted-mesh repair result remains automated-only |
| Simplify/Subdivide/Smooth | core mesh-operation tests and command history tests | `simplify-result`, `subdivide-result`, `smooth-result` in both themes |
| Persistent Grab/Crease/Smooth Brush/Blob | `BrushStroke`, `apply_brush_stroke`, shell `ActiveBrush`/`BrushGesture`, persistent-mode tests | active, before, and Shift-stroke-after records for each brush in both themes |
| Normalize picked region | `NormalizeDensityOptions`, shell Normalize gesture, density tests, one-history/no-op tests | `brush-normalize-active`, before, and after records in both themes |
| Merge Boolean Union | `apply_boolean_pair`, `CommandId::Merge`, `merge_dispatches_boolean_union_and_consumes_the_tool_object` | `merge-picker`, cancel, exact-two precondition, and `merge-complete` |
| Intersect and Subtract | `apply_boolean_pair` operation dispatch and Boolean integration tests | `intersect-complete` and `subtract-complete`; both show commit status |
| Emboss and Hollow | `text_emboss`, `hollow_mesh`, `mesh_ops_shell.rs`, command/dialog apply/error tests | typed `emboss-dialog` and `hollow-dialog` reachability/cancel; successful geometry paths are automated |
| Paint/Material and visible command buttons | command descriptors, enablement tests, shell route/error tests | `paint-tab`, `material-dialog`, View controls, Items actions |
| Dialog Apply/Cancel/Escape and non-destructive preview | typed `DialogAction` routes, Torus preview scene replacement, cancel tests | Torus preview/apply and Boolean picker cancel; Emboss/Hollow cancel frames |
| Paired reference contract | read-only Python focused suite and baseline native journal | Python six-record baseline paired with Rust empty/primitive/modelling states |

The complete capture index is in [rust-native-captures.ndjson](rust-native-captures.ndjson);
the compact accepted run index is [accepted-journal.ndjson](accepted-journal.ndjson).
