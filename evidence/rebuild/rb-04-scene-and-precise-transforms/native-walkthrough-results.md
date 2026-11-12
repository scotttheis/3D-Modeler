# RB-04 live comparison record (in progress)

Windows is unlocked. Every action/capture checks the Default input desktop and exact owned target; Python and Rust target windows are opened sequentially. Python source and environment remain read-only. Captures are 1280 x 840 client pixels at 96 DPI; each Rust journal entry records HWND, origin, fixture and executable SHA256. Origin is queried for every event.

## Verified pointer outcomes

The 11 Light viewport cases now pass saved-state verification: Move single/hidden/group; Rotate single/group; Resize Grid face/Object face/uniform corner; no-motion; Escape; focus loss. Dark repeats are running. Each committed drag changes all selected roots, preserves IDs/metadata/visibility/meshes and unselected poses, and advances scene revision once. Undo/redo advance once each and restore exact saved matrices. Face resizing fixes every vertex of the opposite face; uniform corners keep the selection center fixed, matching Python. The initial verifier incorrectly assumed a corner fixes its opposite corner; source gizmo inspection and saved native matrices establish center scaling, and the verifier was corrected without changing the application.

Python has 76 valid visible viewport states plus an orphan edge-before absence probe. Both visible pickers expose six faces and eight corners; internal edge mathematics remain separately tested by the 122-case oracle. The orphan is not a successful edge action or pair.

Python Escape after pointer motion leaves a changed authoritative transform without a history entry and clears selection. Python minimize/restore finishes the drag with one history entry. Rust discards its immutable preview for Escape/focus loss and leaves exact input matrices/history unchanged. This is an observed behavioral variance, not claimed parity or owner acceptance.

## Precision repair discovered natively

On the earlier F05 debug build, editing only X to `1 in` reapplied rounded Y `0.591 in`, changing 15 mm to 15.01140022277832 mm. The saved probe is retained. Current C5 debug tracks edited axes: the same native X edit saves [25.399999618530273,15,0] mm. Focused tests also preserve untouched Resize dimensions and non-round Roll/Pitch angles, and no-edit Apply creates no history. Full workspace tests259/fmt/strict all-feature Clippy pass.

## Full-size visible observations

Inspected Python-left/Rust-right Light hidden-Move and Object-face Resize pairs at full size, plus Move controls and prior Settle pairs. Python displays flatter teal selected faces with blue mesh/bounds outlines, a coarser grey grid, large wireframe Items tiles, near-face arrow controls, circular white corner grips and a compact icon/numeric strip. Rust displays softly shaded green faces, a fine grid, shaded Items rows, centered arrows, small square corner/face grips and a larger text-mode strip/side inspector. Both keep the hidden selected item in Items while showing only visible geometry. Python displays mm in viewport captures; Rust displays inches in the current run. Drag world deltas differ because camera framing and projected handle geometry differ; the independently checked matrix contracts and exact units are recorded separately from these visible differences.

Visual and owner acceptance remain pending. Wider viewport styling and camera/gizmo placement are RB-05 work. RB-03's 32 review rows remain unchanged.

## Remaining native work

Complete Dark viewport replay; current-build Settle recap; shared-fixture scene/clipboard/selection/numeric/focus and actual placement/measure dialogs in both themes; naming/visibility prepared-reference comparison; unhealthy Items to Repair reachability. Then regenerate paired metadata/board, inspect all affected action surfaces, run final current-source build/size/evidence checks and verify the completed additive G snapshot. No RB-05, staging or commit.

## Native clipboard correction after C5

The real Ctrl+C/change-selection/Ctrl+V sequence found a defect that semantic/synthetic Key-only tests missed. Pinned egui-winit0.37 translates these shortcuts and may suppress Paste altogether for an empty text clipboard. NativeOptions now disables that translation, delivering object clipboard keys independently. The raw-input hook forwards focused text Copy/Cut and platform RequestPaste; eframe clipboard is enabled for actual OS text editing. A real TextEdit regression verifies model paste, IDs/offset, text Cut/Paste and unchanged geometry history. The 4129 debug native replay produced the exact expected cloned object with one action and exact undo/redo; failed C5 state remains explicitly superseded.

The clipboard feature adds six Linux-target lockfile entries and activates the existing Windows clipboard backend. The Boost1.0 texts for pinned clipboard-win5.4.3/error-code3.4.0 were reviewed against upstream; version-specific cargo-deny exceptions and staged notices were added. The previous deny rejection is retained; current checks are running after this correction. Clipboard routing does not change transform/picking math; C5 native viewport/Settle records retain their actual build identity.

Evidence path typing uses guarded Win32 Unicode SendInput, preserving the OS clipboard. Verified native Open/Save results establish correct path entry. Official API references: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput and https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-keybdinput. License references: https://www.boost.org/LICENSE_1_0.txt and https://embarkstudios.github.io/cargo-deny/checks/licenses/cfg.html.
