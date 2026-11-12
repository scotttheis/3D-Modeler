# RB-08 Initial Button and Route Ledger

Initial native observation was performed against Python first, then Rust, at
1280×840 client size / 96 DPI. All entries below are the pre-implementation
state; the final ledger in `report.md` records closure and remaining gaps.

| Surface / control | Python reference observation | Rust pre-edit observation | Required RB-08 closure |
|---|---|---|---|
| Main menu | Builder3D, New, Open, Save, Save As, Export, Repair, Print Prep Check, Settings, Exit | Same main routes plus Customize ribbon | Preserve routes and accessible names; cancel/disabled/error paths stay visible |
| Theme | Dark/Light applies immediately and persists | Dark/Light applies immediately and persists | Keep JSON restart behavior and test theme migration |
| Import units | mm, µm, cm, m, in, ft; only STL/OBJ/PLY use default | Same bounded import policy | Preserve aliases and authoritative 3MF/glTF units |
| Transform display units | No equivalent Python preference | mm/in | Keep display conversion separate from stored millimetres |
| Preferred slicer | Change Slicer, Clear Preference; chooser has browse and “Always use this slicer” | Settings has no preferred-slicer controls; Print Prep lists discovered slicers only | Add browse/select/clear and remembered print choice with path revalidation |
| Recent files | Settings can clear the non-destructive recent list | No Clear Recent Files List control | Clear list/cache without deleting source files |
| About | Settings opens native About message | No About route in Settings | Add reachable About dialog and correct Escape nesting |
| Ribbon | Tab route, favorites, overflow, customization, pin/collapse | Existing shell routes work; check first-run pin default, narrow overflow, keyboard names/focus | Preserve favorites/order/reset and improve keyboard/accessibility |
| Shortcut coverage | Ctrl/Cmd+N, O, S, Shift+S, D, Q; edit shortcuts; text controls keep normal editing | Scene shortcuts include S/C/X/V/D/A, undo/redo/delete/Escape; N/O/Save As/Q absent | Add missing file/exit shortcuts; modal/text focus precedence; dirty guards |
| Welcome | Empty/recent tiles, preview fallback | Empty and recent surface exist; preview fallback has bounded worker | Verify valid/missing/corrupt/unsupported and stale-result fallback |
| Dialog cancellation | Native file pickers and dialogs cancel without side effects | Typed dialogs and picker seams exist | Verify cancel/error/close; no silent no-op routes |
| Accessibility | Qt native labels/focus semantics | Custom icon-only controls have weak accessible naming | Label custom widgets, preserve visible focus and pointer feedback |

Pre-edit Rust settings recovery discarded all settings on one decode error and
did not read the Python QSettings migration boundary. Existing working shell,
dialog, preview, command, and snapping code will be reused and extended.
