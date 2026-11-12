# RB-08 Preferences and Complete Shell Handoff

Status: historical RB-08 handoff; implementation and automated checks are
recorded, but native Rust evidence and the completion snapshot are pending  
Written: 2026-11-02  
Original scope boundary: complete RB-08, then stop before RB-09 (superseded by
the owner's later 2026-11-02 instruction; see the RB-09 handoff)

## 1. Objective and owner direction

Complete the current RB-08 slice in the Rust rewrite, using the RB-08 row in
[product-rebuild-plan.md](product-rebuild-plan.md) as the scope:

- settings and preference workflows, persistence and migration;
- ribbon favorites, fit-sensitive overflow, ordering, pinning, collapse and reset;
- all menus, dialogs and visible shell controls in the RB-08 surface;
- original keyboard shortcuts, text-field and modal precedence, and accessibility
  behavior in the product plan;
- welcome-screen empty, recent and preview fallback states;
- focused and workspace verification, paired native evidence, updated ledgers,
  and an additive verified completion snapshot.

The owner said on 2026-11-02: “the interface is still needing work, so are the
functionalities. but we're getting closer.” Carry that as current product
direction. It is not visual approval or product acceptance. The owner was shown
the Rust Release application in the preceding session; that preview is not an
RB-08 build or acceptance.

This is one slice of the rebuild, not whole-product completion. Its original
stop-before-RB-09 instruction has been superseded by the owner's later
direction to continue. RB-08 remains open because the native Rust matrix and
completion snapshot are pending; the later direction does not retrospectively
close or accept RB-08. Continue under
[`rb-09-shared-ai-and-regression-handoff.md`](rb-09-shared-ai-and-regression-handoff.md).

## 2. Verified starting point

### Worktree and recovery state

Start from the current worktree at:

    E:\Builder3D-Rust

The preceding slice has a verified immutable completion snapshot:

    G:\Builder3D-Rust_snapshot_20261101-162515_RB-07-complete

Receipt:

    E:\Builder3D-Rust\evidence\rebuild\rb-07-appearance-jobs-and-print\snapshot-receipt.md

Final receipt-inclusive verification measured 2,874 files and 574,294,626
bytes in both source and snapshot, with zero missing or extra paths, size
differences, SHA-256 differences, or reparse entries. The RB-07 baseline is:

    G:\Builder3D-Rust_snapshot_20261101-132912_RB-07-baseline

The current worktree is the working source. Do not restore it from or overwrite
it with the RB-07 snapshot. This handoff and the current status-pointer updates
postdate the RB-07 snapshot; that expected documentation difference does not
invalidate the verified RB-07 recovery point. Before RB-08 implementation,
make a new uniquely named, additive RB-08 baseline on G: from the current
worktree. After completing RB-08, make and verify a separate RB-08 completion
snapshot. Do not overwrite or clean existing snapshots.

The project Git worktree currently shows its project files as untracked, with
nothing staged. Do not stage or commit, reset the worktree, or discard its
contents. Keep L:\Builder3D read-only: it is the behavioral reference and may
be inspected or tested, but do not write, format, install into, or clean it.

The Rust Release executable shown in the previous session had SHA-256
E26C37DE652B6434727AE7F6396012CDF4AA911299857B621D336AFB4DA38448. The
RB-07 Debug executable had SHA-256
0BF7753FE460A6A854906457F86363DDC905B455FA790760B6D9122F6F9CB0D6. These
identify the prior build only; rebuild and record the RB-08 executable hash
for RB-08 evidence. If the owner’s preview window is still open, leave it
available for the owner and do not present it as the RB-08 result.

### RB-07 results to preserve

The RB-07 checkpoint is
[evidence/rebuild/rb-07-appearance-jobs-and-print/checkpoint.md](../evidence/rebuild/rb-07-appearance-jobs-and-print/checkpoint.md);
the full report, verification, button ledger, requirement map, capture metadata,
walkthrough and paired-comparison board are beside it.

- Rust workspace: 301 tests passed. Focused app/core/platform/viewport counts:
  150 / 88 / 15 / 34.
- Python focused reference tests: 103 passed.
- Python full suite: 785 passed, with one existing read-only machine-path
  failure in
  tests/test_setup_contract.py::test_private_use_guide_covers_launcher_icon_and_recovery
  because the unchanged reference document expects K:\builder3D.png; two
  existing trimesh warnings were reported.
- All ten documented Rust quality gates passed. cargo audit was not installed.
- Accepted native evidence: 117 PNGs: Python 34 (17 per theme), Rust 82
  (40 Light / 42 Dark), and one Bambu Studio frame. The installed Bambu Studio
  opened the independently validated staged 684-byte, 12-triangle STL.
- The RB-07 Debug and Release hashes are above. The final snapshot receipt has
  exact per-file verification details.

Known RB-07 evidence limits remain recorded in its report: Rust Light-theme
Cancel did not reach a confirmed native cancelled frame; Python external
slicer launch was not captured natively; several image and failure cases have
automated-only coverage. Do not claim those cases were natively proven. They
are not RB-08 tasks by default; reopen one only if an RB-08 change exposes a
specific, reproducible defect.

Visual review and owner acceptance remain pending for RB-07 and earlier
unaccepted slices. Preserve those statuses. They do not stop technical RB-08
work, and this handoff does not grant acceptance.

## 3. Read first and resolve stale pointers

Read the current versions of:

1. E:\AGENTS.md and E:\RustDeveloper.md.
2. E:\Builder3D-Rust\AGENTS.md.
3. E:\Builder3D-Rust\docs\product-rebuild-plan.md, especially the RB slice
   table, design/asset contract, functional inventory, evidence and verification
   requirements.
4. This handoff and the RB-07 report, verification, checkpoint, initial button
   ledger, requirement-test map and snapshot receipt.
5. E:\Builder3D-Rust\WORK_LOG.md,
   docs\component-catalog.toml and
   evidence\rebuild\parity-matrix.toml.
6. The current Rust settings, shell, dialog, command, UI-state, theme and
   recent-preview implementations listed in section 5.
7. The matching read-only Python handlers, settings keys, ribbon/menu
   definitions, tests and current native behavior under L:\Builder3D.

At the time this RB-08 handoff was prepared, the former RB-05 continuation
text in the project instructions and old Section 12 of the product plan was
stale after RB-07; those pointers were refreshed to RB-08. The RB-07
checkpoint records its then-current status, before RB-08 implementation. The
owner has since authorized RB-09, while this RB-08 slice remains open for its
native matrix and completion snapshot. The current pointers are now
`AGENTS.md`, Section 12 of the product plan, and
[`rb-09-shared-ai-and-regression-handoff.md`](rb-09-shared-ai-and-regression-handoff.md).

Use the current Rust implementation and native behavior as evidence before
editing. Historical Python/R7 reports or module names are not proof that a
Rust control is reachable or that it works. Do not change MCP or public tool
contracts as part of this slice. If a real RB-08 issue requires an MCP change,
read E:\MCP_DESIGN_GUIDE.md and the owning MCP documents before making it.

## 4. RB-08 scope and current ledger targets

Trace each requirement to its current Python behavior, Rust component, native
entry point, tests and expected result. The product plan is authoritative; a
stale next_slice value in the parity matrix does not remove a still-open
requirement from RB-08.

| Area | RB-08 behavior to complete |
| --- | --- |
| Preferences | Theme selection applies immediately and persists; first-run behavior matches the recorded Dark default. Default import units apply to ambiguous STL/OBJ/PLY inputs without overriding authoritative 3MF/glTF units. Display units persist while internal geometry remains in millimetres. Preferred slicer can be selected, browsed for and cleared, and the 3D Print workflow can remember a choice. Clear Recents removes stored paths without deleting user files. About is reachable from Settings as in the reference. |
| Settings persistence | Preserve valid preferences through restart. Compare the Rust JSON settings boundary with reference keys and any supported legacy settings migration. Test missing, partial, malformed and unsupported values; recover safely with an understandable message and usable defaults. Do not lose unrelated valid settings during recovery or migration. Keep snapping/window settings where they are already part of the current product contract. |
| Ribbon and shell | Verify the hamburger, tab labels and order, command routes, compact favorites, fit-sensitive overflow, favorite add/remove/reorder, tab command order, pin/collapse behavior, per-tab/default reset, Save/Save As/Undo/Redo utilities and 3D Print placement against the original. Persist settings that the reference persists. Confirm narrow and wide layouts instead of assuming overflow or ordering from source alone. |
| Menus and dialogs | Inventory every RB-08 menu item, settings action, dialog action, close/cancel path and duplicate entry point. Every visible enabled control must perform the expected action or expose an honest disabled reason. Failures must be visible and recoverable; no silent no-ops. Keep multiple entry points on the same semantic command when the current architecture already supports that. |
| Shortcuts and focus | Verify New, Open, Save, Save As, Duplicate, Delete, Exit, Copy, Cut, Paste, Undo and Redo against the reference. Preserve the dirty-document guards. Scene commands must yield to text editors and modal dialogs when they own focus. Test platform modifier behavior, text selection/editing, Escape and modal cancellation using actual keyboard input. |
| Accessibility | Cover the product-plan requirements for keyboard navigation and focus order, visible focus, accessible control names/labels, contrast and cursor feedback. Check actual native controls and the current toolkit’s exposed accessibility behavior; report unsupported or unverified behavior honestly. Do not invent a new accessibility feature outside the product contract. |
| Welcome and recents | Verify empty welcome, persisted recent ordering, project/model dispatch, responsive recent tiles, valid previews, missing/corrupt/unsupported preview fallbacks and stale background preview protection. Keep the shell responsive and do not mutate the live scene when a recent entry fails. |
| Responsive appearance | Compare Light and Dark themes at the documented baseline and across available window sizes/scales. Check text, icon contrast, focus/disabled/error states, clipping and control placement. Use the actual original assets and palettes; do not substitute generic art for an available product asset. |

### Open parity records to carry into RB-08

The current parity matrix marks these areas partial or pending:

- Settings: theme, default import unit, display unit, preferred slicer, Clear
  Recents and About.
- Ribbon: collapse/pin, favorites, tab order and utility layout.
- Shortcuts: New, Open, Save, Save As, Duplicate, Exit, Copy, Cut, Paste,
  Undo and Redo. Delete is already marked implementation/automated/interactive
  pass; retain that status and check for regression. Save and Undo, plus theme,
  collapse/pin and utility layout, have older next-slice labels but remain
  incomplete or partial in the matrix and fall under the RB-08 plan scope.

Reconcile these rows with observed behavior; do not reset a passing field or
reuse a broad module-level row as proof. Add stable feature IDs for distinct
menu/dialog/shortcut/fallback behavior that is missing from the ledger. Keep
implementation, automated, interactive, visual and owner-acceptance statuses
separate. Visual review and owner acceptance remain pending unless the owner
explicitly reviews and accepts the actual RB-08 result.

The welcome recent-files, dispatch and preview rows currently record
implementation, automated and interactive pass with visual/owner review
pending. Preserve those facts, verify the fallback and layout states needed
for RB-08, and update only from new evidence.

## 5. Rust implementation to reuse and inspect

These modules already contain substantial behavior. Verify their current
native reachability and persisted results before changing them.

| Rust seam | Existing behavior to inspect |
| --- | --- |
| crates/builder3d-app/src/preferences.rs | JSON load/save and recovery boundary; defaults; recent paths; theme; default import/display units; favorites, pin and tab order; normalization. Existing fields do not include a preferred-slicer setting. |
| crates/builder3d-app/src/ui_state.rs | Favorite normalization/movement, command ordering and recent entry semantics. |
| crates/builder3d-app/src/dialogs.rs | Settings currently exposes Theme, Default units for ambiguous imports, Transform display units and Done. Confirm native behavior and compare missing preferred-slicer, Clear Recents and About routes before implementation. |
| crates/builder3d-app/src/shell.rs | Settings initialization and warning recovery; preference persistence; ribbon, overflow and customizer; shortcut dispatch; dialogs/menus; recent preview worker and welcome surface. |
| crates/builder3d-app/src/commands.rs | Semantic command dispatch and reference ribbon specification/order. |
| crates/builder3d-app/src/recent_preview.rs | Bounded preview loading and safe fallback for unsupported, missing or corrupt recents. |
| crates/builder3d-app/src/theme.rs and artwork.rs | Theme palette, original artwork mapping and light/dark rendering. |
| crates/builder3d-app/src/numeric_controls.rs | Unit parsing and display conversion; preserve mm as the internal unit. |

The current Settings source already has theme and unit controls. The preferences
type already persists theme, units, favorites, pin/order, recents and snapping.
Use those seams. Settings source currently has no preferred-slicer, Clear
Recents or About control, but verify the running build and reference handlers
first. Do not create a second settings store, parallel shortcut router or broad
replacement shell without a demonstrated architectural gap.

## 6. Suggested implementation sequence

1. Inspect git status, current processes/build identity, existing RB-07
   checkpoint and current source. Preserve the worktree. Create a new additive
   RB-08 baseline snapshot on G: before implementation; verify its relative
   paths and contents. Do not copy the RB-07 snapshot over the worktree.
2. Capture/inspect the original behavior from L:\Builder3D and enumerate the
   exact RB-08 menu, dialog, settings, ribbon, shortcut, accessibility and
   welcome routes. Build the initial button ledger and requirement-to-test map
   before changing code. Identify real Rust gaps and already-working routes.
3. Implement the smallest set of changes that closes those gaps. Reuse current
   Preferences, shell, dialogs, UI state and preview logic. Add focused tests
   for normalization, persistence/migration, command dispatch, keyboard
   precedence, settings actions and fallbacks as appropriate.
4. Exercise the affected native routes after the code change. Check the full
   restart, keyboard, text-field, modal and both-theme/window/scale matrix in
   section 7. Pair reference and Rust states and record failures as failures.
5. Run the integrated Rust gates listed below, focused reference tests that
   cover the matched behavior, and the relevant build/smoke/native launch.
   Keep the Python reference read-only and use its existing environment. Follow
   the product plan's Windows reliability constraints for Python GUI tests.
6. Update the RB-08 report, verification, checkpoint, button ledger,
   requirement-test map, walkthrough, paired-comparison board and capture
   metadata. Update component-catalog.toml, parity-matrix.toml and WORK_LOG.md.
7. Create and verify an additive RB-08 completion snapshot on G: after the
   receipt and documentation are final. Record exact file/byte counts,
   comparison results, executable hash, test counts, capture counts, remaining
   limits and next slice RB-09. Stop before RB-09.

These are the project’s existing recovery and evidence expectations, not a
request for a new approval process. Make normal implementation choices
autonomously; do not stop for routine design or test decisions.

## 7. Verification and native evidence

### Focused and integrated checks

Add tests for the actual behavior being changed, then run the current workspace
suite. At the integrated milestone, run the product-plan commands from
E:\Builder3D-Rust:

    cargo fmt --all -- --check
    cargo check --workspace --locked
    cargo test --workspace --locked
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
    cargo deny check
    cargo metadata --locked --format-version 1 --no-deps
    cargo run -p builder3d-app --bin builder3d-app -- --smoke
    cargo build --release --locked -p builder3d-app --bin builder3d-app
    cargo bloat --release --workspace
    cargo bloat --release -p builder3d-app --bin builder3d-app

Run cargo audit --locked when it is installed; record when it is unavailable.
Record the exact command, exit result, test counts, warnings and skipped checks.
A smoke result is not native interaction evidence. Continue to run the actual
desktop application.

### Paired native matrix

Use the reference first, capture it, close that target window, then open and
capture the Rust app. Use real keyboard and pointer input for interaction
claims. Keep each comparison aligned by theme, fixture, route, action, window
size, display scale and state. Start with the plan's 1280x840 logical-pixel,
96-DPI/100% baseline; cover both Light and Dark, other window sizes, and the
documented 100%, 125%, 150% and 200% scales when available. Record unavailable
host/display combinations as untested rather than passed.

At minimum capture and walk through:

- first run and restart in both themes; change each setting, close, relaunch,
  and confirm the persisted result;
- valid, partial, missing and malformed settings recovery where the adapter
  allows safe fixture setup;
- import/display units and the unchanged internal-mm geometry;
- preferred-slicer select, browse, clear and remembered choice, including
  unavailable/invalid choices;
- Clear Recents with files still present on disk; About from Settings;
- each menu and dialog action, including close, cancel, disabled and error
  states;
- favorites added, removed, reordered and reset; overflow when the top row
  fits and when it is constrained; pin/collapse and per-tab/default order reset;
- all shortcuts with a scene focused, a text field focused and a modal open;
  verify dirty guards and native text editing;
- keyboard-only navigation, focus order/indication, labels and contrast;
- welcome with no recents and with supported, missing, corrupt and unsupported
  recents; wait for preview loading and verify its fallback without a freeze;
- representative narrow/wide and maximized layouts in both themes.

For every accepted frame, retain the source/build identity, executable
SHA-256, timestamp, theme, route/action, logical and physical dimensions,
DPI/scale, fixture, and screenshot hash. Open and inspect captures; reject
wrong-window, blank, occluded, stale or partial-paint images with a reason.
Keep paired capture counts and rejected attempts exact in the evidence report.

The plan restricts broad Python desktop suites and offscreen OpenGL testing on
Windows. Use focused Python reference tests for this slice and native Python
captures for comparison; do not use a headless UI run as a substitute for
native behavior.

## 8. RB-08 evidence and documentation

Use this evidence directory:

    E:\Builder3D-Rust\evidence\rebuild\rb-08-preferences-and-complete-shell\

Keep it focused and parallel to the RB-07 evidence. Include:

- report.md, verification.md and checkpoint.md;
- initial-button-ledger.md covering every touched visible button, menu item,
  duplicate route, disabled/error/cancel state and native outcome;
- requirement-test-map.md with stable IDs, source/test symbols, Rust component,
  fixture, expected result and evidence route;
- walkthrough.md with real steps and outcomes for restart, keyboard, text,
  modal, favorite/overflow and welcome cases;
- paired-comparison-board.md plus capture-metadata.json and the accepted
  capture journal; retain rejected attempts and reasons;
- source-preservation.md and snapshot-receipt.md with the exact additive
  baseline and final snapshot verification.

Update docs/component-catalog.toml for the real settings, shell, UI-state,
dialog and persistence responsibilities. Add a distinct component only if
the code now has a distinct module/contract that merits one. Update
evidence/rebuild/parity-matrix.toml per feature, not per module. Update
WORK_LOG.md with the RB-08 work, exact verification results, snapshot path,
known limits and next slice. Validate the TOML, stable IDs and referenced
paths. Do not rewrite unrelated historical evidence.

## 9. Completion boundary

RB-08 is complete when:

- each RB-08 preference, shell, shortcut, accessibility and welcome behavior
  has a working native route or a precise reference-backed not-applicable/
  unsupported explanation;
- no in-scope visible control is inert or silently fails;
- automated checks and actual native interaction support the separate ledger
  statuses;
- paired Light/Dark and layout/scale evidence is recorded honestly;
- all documented gates have exact results or a clearly recorded unavailable
  prerequisite;
- the component catalog, parity matrix, work log, checkpoint and final
  additive snapshot agree;
- visual review and owner acceptance are reported according to the evidence
  actually received; do not claim them by inference.

Record exact Rust and Python test counts, focus-test counts, native evidence
counts by target and theme, accepted/rejected frames, current executable
hashes, snapshot path, relative-path/file-count/byte/SHA-256 verification, and
the next slice. This was the RB-08 exit boundary when this handoff was written.
The owner later authorized RB-09; its own handoff governs the next session.
Do not backfill RB-08 native evidence or a completion snapshot with RB-09
evidence, and do not call RB-08 complete by inference.

## 10. Current continuation

The owner later authorized work through RB-09. Start from the current
worktree and use
[`rb-09-shared-ai-and-regression-handoff.md`](rb-09-shared-ai-and-regression-handoff.md).
RB-08 native Rust evidence and its completion snapshot remain pending; retain
that status and do not treat the RB-09 snapshot as an RB-08 completion receipt.
