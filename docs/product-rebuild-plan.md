# Builder3D product rebuild and parity plan

Date: 2026-11-03. Status: active replacement plan; RB-02 is complete and
owner-accepted. RB-03 through RB-07 implementation and engineering evidence are
recorded; their visual review and owner acceptance remain pending. RB-08
implementation and automated checks are recorded, but its paired Rust native
matrix and completion snapshot remain pending. RB-09 Shared AI and Regression
implementation and automated verification are recorded; the live-client/native
gate is still unverified and the owner paused work on 2026-11-03. When work
resumes, finish that RB-09 gate and stop before RB-10. This does not close RB-08
or accept the product.

The active Rust source/build/run workspace is `E:\Builder3D-Rust`. Use `G:\`
as the primary additive backup destination. Removable USB copies are occasional
one-offs unless the owner requests otherwise. `L:\Builder3D` remains the
separate Python reference/shipping build and is not the Rust working copy.

Owner direction: reproduce the original functionality and aesthetic, reuse the
existing Rust application wherever that is the fastest dependable route.

## 1. Authority and intended result

This is the execution authority for repairing and completing the Rust product.
It supersedes `docs/r8-execution-brief.md` and the R8 product-completion claims.
Use the slice names RB-00 through RB-11 below; do not invent another numbered
R stage or treat R1-R7 conversion coverage as proof of desktop readiness.

The result is the owner's Builder3D: the recognizable layout, supplied artwork,
interaction habits, modelling capabilities, file behavior, and AI control of
the original Python app, implemented as a dependable native Rust application.
Visual fidelity and working behavior are equally mandatory. Improvements follow
an accepted baseline and must preserve the owner's design unless approved.
An aspiration to exceptional quality is not a measurable completion claim.

The owner's 2026-10-31 condition to complete matched live RB-01 Light/Dark
captures at 1280 x 840 logical pixels and receive explicit approval was
satisfied before RB-02. RB-02 has since been completed, and the owner accepted
its current Light/Dark visual baseline, with fine-tuning deferred. RB-03's
implementation and engineering work are recorded in
[`evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md`](../evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md);
owner visual acceptance remains pending. RB-04 implementation/native evidence is
complete; see its report/checkpoint under `evidence/rebuild/rb-04-scene-and-precise-transforms/`.
The original RB-04 scope and historical RB-05 through RB-11 route are in
[`rb-04-through-rb-11-completion-handoff.md`](rb-04-through-rb-11-completion-handoff.md).

Read `E:\AGENTS.md`, `E:\RustDeveloper.md`, this workspace's `AGENTS.md`,
and the relevant reference instructions before implementation. The original
rewrite's file, unit, undo, architecture, resource, size, and portability
contracts still apply. This plan replaces its inadequate product acceptance
process; it does not waive those contracts.

## 2. What failed, and what is actually reusable

The original rewrite correctly called for a gradual replacement and identified
UI fidelity as a decision gate. Conversion-first work was later mistaken for
an integrated product. The R8 brief allowed behavior to be described as bounded
and owner visual acceptance to be deferred past completion. Its final report
called the app complete while acknowledging that interactive checks were
blocked. That conclusion is withdrawn.

Concrete findings from the current code and records:

- `crates/builder3d-app/src/shell.rs::draw_preview_objects` paints object-bound
  rectangles and picks through `preview_object_rects`. This is not an integrated
  3D editing viewport. `builder3d-viewport::ViewportRenderer` exists separately.
- `artwork.rs` paints simplified symbols instead of reproducing the original
  supplied artwork. The Python reference has eight Insert icons and 44 action
  files per theme, plus `Builder3D.png`, `Builder3D.ico`, `pin.jpg`, and `snap.png`.
- Rust command, format, operation, platform, and viewport modules contain useful
  implementations and tests. Their individual validation does not establish
  correct live integration, complete desktop reachability, or visual parity.
- `evidence/r8/00-baseline/python-reference.png` shows a blank client area.
  It cannot establish the original visual baseline. Captures must be inspected.
- Prior snapshots excluded evidence and the work log. New recovery points must
  include the material needed to understand and verify the recovered source.

Do not throw away the Rust workspace or automatically rewrite the whole shell.
The previous recommendation to rebuild the front end wholesale was too broad.
Use this disposition, then revise it only with evidence from RB-00/RB-02:

| Existing area | Default disposition | Required proof before relying on it |
|---|---|---|
| Core scene, geometry, formats, fixtures | Keep; repair specific demonstrated defects | Original result, units, identity, undo, round trips |
| Commands, transforms, preferences, jobs, print services | Reuse and connect; audit semantics | UI result and error/cancel paths match service contracts |
| R5 renderer, camera, picking, gizmos, snapping, resource planner | Reuse behind a real native viewport adapter | Onscreen geometry, exact picking, real drag, telemetry |
| eframe app startup and useful UI-state helpers | Keep initially | Native window, scale, fonts, persistence, clean shutdown |
| Ribbon, welcome, panel, dialog code | Repair or replace component by component | Same appearance, placement, gestures, and successful workflow |
| Painted rectangle viewport and substitute artwork | Retire from accepted product paths | Real renderer and original visual resources take their place |
| Existing tests and catalog | Keep as regression evidence with scoped claims | Each test's scope distinguished from product acceptance |

## 3. Reference hierarchy and historical requirements

The reference product is `L:\Builder3D`, not Microsoft's 3D Builder, a newly
invented design, or the rejected Rust screenshots. Keep that entire tree
read-only. Preserve existing snapshots and all unrelated user work. No commits
or pull requests are authorized.

Resolve requirements in this order: current owner direction; accepted original
Builder3D visuals and behavior; its current contracts and tests; historical
notes that explain those choices. Where records disagree, record the conflict
and use a concrete comparison; do not silently select the easier behavior.
Do not reproduce a known defect as a requirement or implement an old wishlist
as though it were an accepted feature. Owner decisions are needed only for
material unresolved design or scope choices, not routine repairs.

| Reference in `L:\Builder3D` | Use |
|---|---|
| `docs/ui-style-guide.md`, `src/style.py` | Visual contract, exact theme tokens, scale, states, gestures |
| `src/icon_assets.py`, `src/branding.py`, `assets/` | Owner artwork mappings, theme selection, brand, transparency |
| `src/main_window.py`, `src/toolbar.py`, `src/numeric_controls.py` | Full ribbon/menu/panel/dialog/transform behavior |
| `src/viewport.py`, `src/gl_renderer.py`, `src/gizmo.py`, `src/snapping.py` | Rendering, camera, input, drag, floor, snap behavior |
| `src/application_services.py`, `src/model_actions.py`, `src/scene.py` | Shared workflows, command boundaries, history |
| `docs/project-format.md`, format implementations and tests | Project and model compatibility |
| `docs/architecture.md`, `docs/ai-mcp.md`, `docs/mcp-modern-design.md` | Shared desktop/AI state and public contracts |
| `TODO.md`, `issues.md`, `COMPLETED.md`, `WORK_LOG.md`, `docs/testing.md` | Accepted decisions, open limitations, dated evidence |
| `docs/rust-rewrite.md`, Rust catalog, R1-R8 evidence and log | Reuse candidates, original gates, historical implementation claims |
| `docs/3d_builder_reference/README.md` | Historical layout/behavior research only; not a source of proprietary art |

RB-00 must reconcile dated owner decisions in the Python notes with current
source and captures, including theme artwork, 2026-10-02 floor acceptance,
navigation modifiers, brush popovers, gizmo sizes, numeric units, snapping,
high-poly behavior, and file/AI workflows. Store requirement-to-evidence links;
do not copy years of notes into another diary. Missing records stay unknown.

## 4. Required evidence and truthful status

Maintain `evidence/rebuild/parity-matrix.toml` during execution. It is a feature
ledger, separate from the module-level `docs/component-catalog.toml`. Create one
stable requirement ID per distinct behavior, including context menu entries,
shortcuts, drag effects, settings, and error states; a module name is too broad.

Each requirement records: owner/source reference; Python symbol/test; Rust
component ID/symbol; UI entry points; expected result and tolerances; fixture;
automated test; actual interactive steps/result; visual capture pair when
applicable; host/build/DPI/theme; open defects; and acceptance evidence.
Use separate fields for implementation, automated verification, interactive
verification, visual review, and owner acceptance. Allowed field states include
pending, pass, fail, and not-applicable with a reason. The current plan creates
no passing feature records by inference.

Catalog `validated` means the named component contracts have evidence; it is
not a product seal. UI surfaces awaiting parity remain `polish` or `ported`.
Never require a ledger to contain no incomplete statuses as a substitute for
finishing the work. Missing, blocked, or untested rows remain visible and keep
the applicable gate open. A scope exclusion requires explicit owner agreement.

Store new evidence under `evidence/rebuild/<slice>/`: report, verification,
checkpoint, captures, and bounded diagnostics. Preserve historical R1-R8
artifacts; attach supersession notices to misleading current completion records.
Record actual commands and exit results, failures, skipped work, source hashes,
and build identity. Headless tests, a startup smoke, and screenshots prove
different things; none can silently substitute for another.

## 5. Original visual system and asset requirements

Use the original assets directly where practical. The owner explicitly wants
their artwork reproduced. Copy approved product resources into the Rust tree
with a manifest containing source path, SHA-256, theme, logical action,
dimensions, format, alpha treatment, and packaged destination. No runtime
dependency on `L:\Builder3D` is permitted. Never alter source assets.

Required sets: `insert_icons/`, `insert_icons_light/`, `action_icons/`,
`action_icons_light/`, and the original branding/pin/snap resources actually
used by the reference. Preserve the real mapping, including `ExRay.png`,
`Subtract.png`, and the existing Subdivide/Simplify mapping. Do not generate
replacement art or silently fall back to a generic icon when a supplied asset
exists. Preserve the original generated icons where those are intentional,
including filled curved undo/redo arrows; the manifest alone does not dictate
which art the running reference chooses.

Preserve transparent edges and antialiased fringes, optical size, aspect ratio,
theme variants, and pin matte removal. No icon-sized background rectangles.
Test missing resources, theme switching, alpha, scaling, and packaging. The
existing Rust 4 KiB artwork limit must not exclude valid owner artwork.
Deduplicate or resize derived packaging resources only with visual evidence;
retain originals and provenance. Report any conflict with size targets instead
of saving space by substituting different designs.

The design contract includes:

- Calm neutral cards, restrained indigo accent, original light and dark
  palettes. Light tokens include accent `#5B6FEA`, background `#F4F6FA`, panel
  `#FFFFFF`, text `#1D2433`, muted text `#667085`, divider `#DDE2EA`.
  Read dark tokens from `style.py`; do not invent a dark palette.
- A dominant viewport, original top navigation/ribbon, right Items/actions
  panel, and contextual bottom Move/Rotate/Resize strip. Match measured
  proportions rather than retaining the current Rust layout by convenience.
- Hamburger alone at the left, tabs Insert/Object/Edit/Paint/View, compact
  favorites with overflow, and top-right Save/Save As/Undo/Redo utilities plus
  the original 3D Print placement. About stays in Settings.
- Original labels, groups, tooltips, icon sizing (46 logical px ribbon),
  spacing, typography, radii, pinning/collapse, drag ordering and reset.
  Use the platform font contract rather than the default developer UI font.
- Original welcome cards and up to ten recent entries, responsive columns,
  thumbnails and documented fallback states. Match actual panel thumbnails
  and health/visibility/selection indicators.
- Visible hover, press, selected, focus, disabled, error and loading states.
  No clipped text, overlapping controls, duplicate controls, or unreadable art.
- Native drag/resize/snap/minimize/maximize/close and appropriate title/icon
  treatment; platform behavior belongs in platform adapters.

### Capture protocol

Before implementation, capture the functioning original in light and dark
themes: welcome with/without recent files; each ribbon tab; menu/settings;
empty/selected/multiple/grouped/hidden objects; each transform mode; brush
popover; import units; operation progress/error; and print-prep/handoff.

Capture Rust with identical fixture, camera, selection, theme, client size,
display scale, font setup, and UI state. Record logical and physical dimensions.
Use 1280x840 logical px as the initial comparison size; audit the existing
980x640 minimum against the original before confirming it. Check 100%, 125%,
150%, and 200% scaling plus ordinary/maximized window layouts and monitor moves.
Record unavailable display configurations as untested, never passed.

Open every capture. Reject blank, occluded, lock-screen, stale, partial-paint,
or wrong-window images. If the reference cannot be observed, use provenance-
checked historical captures temporarily and keep live acceptance pending.
Compare side by side and with overlays/difference images. Mask only justified
OS decoration/font rasterization differences; never mask product regions.
Define geometric/color/image tolerances per region before verification using
repeatability measurements. The owner judges visual fidelity; a global image
similarity score cannot excuse the wrong layout, icons, or viewport.

## 6. Functional parity inventory

RB-00 expands every row below into specific requirements by enumerating the
original ribbon specification, file menu, context menus, handlers, toolbar,
viewport input bindings, settings, and tests. Cross-check against Rust dispatch
and all 60 current MCP tools. This table is a minimum, not a coverage shortcut.

| Area | Required workflows and observable result |
|---|---|
| Lifecycle | Launch; welcome; New/Open/Save/Save As; dirty title; save/discard/cancel before replace/exit; failed save retains state |
| Recent files | Ten-entry ordering/deduplication/persistence; project/model dispatch; valid thumbnail; missing/unsupported/corrupt fallback without mutation |
| Import | Add/Open; native file selection; all supported import formats; unitless unit choice and scaling once; authoritative 3MF/glTF units; names/colors/transforms/multiple objects |
| Export | Supported STL/OBJ/PLY/3MF/glTF/GLB/WRL outputs; selection/visible rules; transforms/colors; overwrite decisions; reopen independent outputs |
| Primitives | Cube/Cylinder/Pyramid/Cone/Sphere/Torus/Hexagon/Wedge; original dimensions/palette; torus creation/edit/role, bounds and undo |
| Selection | Viewport and Items sync; empty click; sticky and modifier selection; all/none/invert; visibility; naming; stable identity after history |
| Organization | Group/ungroup; copy/cut/paste/duplicate/delete; context menus; selection order and one grouped history entry |
| Numeric transforms | Move/Rotate/Resize modes; absolute vs delta semantics; mm/in and degree display; finite input, suffix selection, tab/enter/escape, no-op history |
| Gizmo transforms | Real axis/plane/rotation/resize handles; hover/cursor; world/object resize basis; opposite anchor; proportional corners; drag cancel; one undo per gesture |
| Placement | Settle, Mirror, Measure, Align, Arrange; group-relative layout, anchors and gap bounds |
| Snapping | Shared object/floor landmark solver, drag-start snapshot, view-scaled tolerance, capture/release feedback, Shift bypass and persistent setting |
| Camera/input | Actual orbit/pan/zoom/pinch and high-resolution wheel; Reset/Frame all/selected/Center/Top/Front/Side; smooth view transitions as reference |
| View modes | Actual shaded meshes, Y=0 floor, metric grid labels, authored colors, readable selection; opaque wireframe with black depth-tested edges; separate X-Ray |
| Repair/mesh tools | Repair, Simplify, Subdivide, Smooth; correct selected targets, controls, progress, mesh-health reports, identities and one-step undo |
| Boolean tools | Merge/Subtract/Intersect; operand order/pick/cancel; correct solid result and cut-surface color provenance |
| Emboss/Hollow | All original parameters/previews/limits, font behavior, raised/engraved placement; bounded concave limitation and safe failure |
| Persistent editing | Grab/Crease/Smooth Brush/Blob/Normalize; defaults, remembered non-modal popovers, real surface stroke/picking, region preview/cancel/commit and undo |
| Appearance | Material/Paint/Color/Quick Color and image average sampling as actually implemented; solid-color semantics; persistence/export and undo |
| Background work | Responsive shell; queued/running/progress/cancelling/completed/failed; revision conflict; isolated compute and one commit; no mutation on failure/cancel |
| Print | Actionable prep findings, bed settings, known slicer selection/discovery, staging 3MF or Bambu STL fallback, validated launch and clear unavailable state |
| Preferences | Theme/units/ribbon/favorites/pin/order/snap/slicer/recent/window state where original supports it; restart, malformed-settings recovery, safe migration |
| Keyboard/accessibility | Original shortcuts, text/modal precedence, keyboard focus/order, accessible labels, contrast, cursor feedback; platform modifier normalization |
| AI integration | Same live scene as GUI; all public tools/schemas/profiles; IDs/revisions/replay/root policy; visible camera/model/file changes and clean shutdown |
| Reliability/distribution | Offline use and MCP-disabled startup; malformed inputs, device failure, large models, repeated project replacement, package launch without Python |

Original navigation remains left-drag orbit and middle/right-drag pan.
Persistent editing uses Shift-left-drag; camera navigation stays available.
Modifier precedence with snapping must be verified in each interaction mode.
Preserve exact full-resolution model/picking/export authority; interaction LOD
is a temporary render choice. Default floor has no reflection-like colored
footprint, following the accepted 2026-10-02 treatment.

Do not add speculative UV texturing, unimplemented historical commands, new
geometry tools, or protocol features as part of parity. If an original action
has a documented limitation, reproduce its honest messaging and record that
boundary rather than turning the control into a false success.

## 7. Architecture and reuse gate

Keep the existing Cargo workspace. Use one authoritative live document with
shared commands, selection, history, view state, and operation scheduler.
The desktop and MCP are adapters to it, not independent scenes kept in
approximate sync. Widgets never mutate meshes or authoritative collections.
Stable object IDs cross adapters; a filtered render-list index must never be
treated as a scene index. Explicitly test hidden/reordered objects.

Split presentation responsibilities as they change: shell composition,
ribbon/menu, welcome/recent, Items, transform strip, dialogs/popovers,
viewport adapter, input router, artwork/theme, and feedback. Reuse existing
helpers and avoid a giant `shell.rs` rewrite merely to reorganize files.
Keep domain contracts free of eframe, GPU, platform, or MCP dependencies.

The first approach is eframe with original resources and custom styling, plus
the existing R5 renderer adapted to native presentation. RB-02 must prove an
actual shaded object, orbit, exact pick, gizmo edit, original ribbon art, theme,
and persistent project state in the same window. Check graphics device/context
ownership, surface/texture presentation, resize, DPI/input coordinates, event
capture, GPU lifetime, and shared budgets. An offscreen renderer alone is not
the proof. Avoid a permanent CPU readback per displayed GPU frame.

If eframe cannot meet the reference design or renderer integration within a
bounded representative implementation, document the specific limitation,
completed experiment, repair cost, alternative cost, size/runtime constraints,
and recommendation. Prototype only the affected frontend seam. Recheck current
official toolkit documentation and licenses at that point. A materially
different toolkit needs an owner decision after a concrete comparison, not
speculative migration. No Qt/Python/WebView runtime is introduced by default.

## 8. Ordered slices and exit gates

Each slice can contain smaller independently tested and snapshotted patches.
The order below is the default; independent contract repairs may proceed while
owner review is pending, but dependent visual work cannot assume approval.

| Slice | Deliverable | Exit evidence |
|---|---|---|
| RB-00 Reference and reuse audit | Complete feature ledger, dated design decisions, known limitations, real capture baseline, asset manifest, UI-to-service map and reuse decision | Every exposed original action inventoried; unknowns explicit; valid reference captures; risk-ranked backlog and baseline snapshot |
| RB-01 Visual foundations | Original artwork/branding, tokens/fonts/states, ribbon and panel proportions, welcome and transform-strip component designs in light/dark | Matched capture board and live original-art rendering; owner approves representative design direction; no placeholder art |
| RB-02 First working application slice | In the existing app: New, add Cube, native import, real shaded viewport, orbit/pan/zoom, exact pick, numeric/gizmo move, undo, save/reopen | One real unassisted task changes geometry and survives reopen; original-looking shell and real renderer together; owner review; toolkit decision recorded |
| RB-03 Lifecycle and file completion | All file formats/units/export, save prompts, recents/thumbnails, failure/cancel handling | Round trips plus live multi-object import/edit/export; failed operations preserve project/path/dirty state |
| RB-04 Scene and precise transforms | Items/thumbnails, selection/sticky/visibility/naming/groups/clipboard, Move/Rotate/Resize, settle/mirror/measure/align/arrange | Live grouped and hidden-object cases, stable IDs, exact values and one history entry per action/drag |
| RB-05 Viewport and input completion | Full camera gestures/presets, gizmo styles, snapping, floor/grid, lighting/colors/wireframe/X-Ray, scale/resize | Matched rest/interaction captures; real pick/drag/cancel tests; hidden/grouped scenes; no rectangle preview in product path |
| RB-06 Modelling tools | All primitives/torus controls; repair/simplify/subdivide/smooth; booleans, emboss/hollow; persistent brush/density workflows | Each tool demonstrated on meaningful geometry, valid undo/export, real surface stroke and bounded failure/cancellation |
| RB-07 Appearance, jobs and print | Original color/material/image workflows; background progress/errors/cancel; print checks/slicer staging/handoff | Live success/failure/cancel paths; no frozen UI; independently checked export/staging; actual installed slicer opens test scene |
| RB-08 Preferences and complete shell | Favorites/overflow/order/pin/reset, all menus/dialogs/settings, persistence/migration, shortcuts/accessibility, welcome fallbacks | Restart/keyboard/text-field/modal matrix and both themes across sizes/scales; all functional ledger rows integrated |
| RB-09 Shared AI and regression | Desktop/MCP same live document, existing public catalog, revisions/replay/files/cancel and optional-MCP reliability | Real client edits appear immediately in GUI; GUI edits inspectable by client; matching saved geometry and history; no parallel dummy scene |
| RB-10 Performance and packaging | Representative large models, live resource routing, memory/cache lifetime, clean install/launch, size and host matrix | Actual app measurements and complete package totals; platform/profile results explicitly distinguished from simulations |
| RB-11 Acceptance and release readiness | Repair remaining parity defects; complete side-by-side review and task walkthrough; final gates and recovery | Owner aesthetic/workflow acceptance, full required evidence, no unresolved required parity gaps, verified complete snapshot |

Early owner checkpoints are RB-01 (appearance direction) and RB-02 (real usable
slice). RB-11 is final approval. Subsequent changes receive focused review
when they materially affect accepted appearance or interaction. Keep pending
reviews pending; silence, successful launch, or a model-generated judgment is
not owner approval. Do not build the entire shell before discovering whether
the owner accepts its first representative screen and real interaction.

## 9. Verification strategy and mandatory walkthroughs

Use three independent gates: behavior, visual fidelity, and integration/runtime.
Add meaningful contract and regression tests for changed behavior. Keep pure
tests for geometry/commands/input state, and real native checks for rendering,
dialogs, pointer capture, focus, scale and presentation. Tests must assert user
outcomes rather than only matching implementation structures.

Minimum acceptance journeys, using identical reference/Rust fixtures:

1. Fresh launch, New, create several distinct primitives, orbit and select,
   move/rotate/resize by mouse and fields, undo/redo, save/reopen. Verify IDs,
   dimensions, colors, transforms, groups, selection and documented view state.
2. Import a unitless model in mm and inches; import a multi-object colored 3MF
   and glTF/GLB. Check dimensions, Y=0 placement, names, colors and framing.
   Export each supported output and independently reopen/check results.
3. Multi-select, group, hide, reorder where supported, rename, copy/cut/paste,
   duplicate, align/arrange, settle and delete. Verify correct object targets
   and history after each step, including a hidden first scene object.
4. Repair a damaged mesh; simplify/subdivide/smooth; perform each boolean with
   distinct operand colors; emboss and hollow. Check geometric results,
   limitations, progress and one-step undo. Fail/cancel without mutation.
5. Use each persistent editing tool with a genuine surface stroke. Navigate
   while active, edit settings, toggle off/on, preview/cancel density work,
   and verify remembered settings and history boundaries.
6. Set color/material, sample an image, save/export/reopen. Distinguish average
   solid-color application from texture mapping. Preserve authored color during
   selection and through boolean cut surfaces.
7. Check print readiness, fix a reported issue, select a known slicer, stage
   and open the visible scene; verify Bambu fallback separately when available.
   Missing executable and failed staging must explain the problem safely.
8. Switch themes and window sizes, customize ribbon/favorites/pinning, change
   units/preferences, restart, and verify every restored state. Exercise
   keyboard-only routes, focused numeric/text edits and modal cancellation.
9. Use a real MCP client to inspect the running GUI, create/transform a model,
   observe it on screen, undo in GUI, inspect through MCP, save/export, and
   cancel a long operation. Repeat with MCP unavailable and after restart.
10. Open representative dense controls, orbit/select/edit/save, replace with
    an empty/new scene repeatedly, and measure memory/cache release. Exercise
    device/budget pressure through supported fault injection and real-host
    tests where available; identify which evidence is simulated.

For each journey record expected and observed state before/after, actual test
steps, native window/build identity, artifacts and defects. A scripted semantic
journey complements real input testing; it cannot prove pointer behavior.
If Windows is locked or capture/input is intercepted, mark the native check
unverified, diagnose it, and continue independent work. Never close that gate
with headless tests. Run foreground targets serially. Honor the reference's
Windows reliability containment; do not launch broad Python desktop suites or
use `QT_QPA_PLATFORM=offscreen` with its OpenGL widget.

Before any MCP interface change, reread `E:\MCP_DESIGN_GUIDE.md` and the
project MCP/architecture/testing documents, verify assumptions against current
official protocol/SDK sources, and retain the current catalog wire contracts.
This plan does not select a new protocol version, transport, or tool schema.

### Engineering gates

For each implementation patch: format, focused checks/tests, changed-crate
Clippy, native exercise of the affected workflow, catalog and evidence update.
At integrated milestones and RB-11 run from `E:\Builder3D-Rust`:

```powershell
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
```

Run `cargo audit --locked` when installed; record unavailable tools and their
effect on verification. Keep the workspace bloat invocation required by local
rules, but use the explicit package result if the tool ignores `--workspace`.
Validate catalog/ledger TOML, unique IDs, referenced paths, stable symbols,
asset hashes, feature coverage and evidence freshness. Launch the real app;
`--smoke` reports semantic startup checks, not interactive success.

### Performance, packaging and platform gates

Measure the integrated release application on the actual Quadro RTX 4000,
including the 30-million-face control and available owner model corpus. Record
parse/commit/first useful frame, orbit paint p50/p95 and total input-to-display
latency, source/display face counts, upload count/bytes, CPU/RAM/VRAM peaks,
cache release, adapter/backend and fallback reason. Preserve the original
2-9 ms orbit-paint target and full resolution at rest; name the statistic and
workload rather than hiding outliers behind a mean or measuring a separate
renderer executable. Resolve unmet budgets with measured repairs, not claims.

Maintain a single system/GPU resource ledger, bounded granular fallback and
hardware-based routing. Test Intel integrated, AMD unified memory, dedicated
GPU and no-accelerator profiles, budget pressure and adapter failure. Simulated
profiles establish policy only; real hardware results have separate status.

Measure the entire installed package (artwork/fonts/libraries included) against
40 MB and the compressed installer against 15 MB; measure idle working set
against the original 120 MB target. Use a clean staging directory, package
manifest, hashes, and an ordinary launch independent of the workspace/reference.
No Python/Qt/WebView runtime dependency. Never use a small executable size as
proof of total installed size. A target conflict remains open for repair or
an explicit owner decision; it cannot justify discarding the original look.

Windows product acceptance and all-platform acceptance are separate records.
Keep portability in every seam now. Before claiming Windows/Linux/macOS support
complete, run native launch, desktop workflows, real rendering/input, package,
and live MCP checks on each real host. Missing hosts keep those gates pending;
Windows unit tests and simulated platform values cannot close them.

## 10. Slice workflow, recovery and handoff

Before the first implementation slice, inspect current Git/process state and
create a new verified baseline snapshot. Preserve existing user changes;
do not reset the all-untracked working tree or remove the rejected app code
until a tested replacement for that path exists. Inspect running test instances
before controlled launch; never terminate an unrelated process.

For every bounded patch:

1. State the objective and requirement/component IDs. Read the relevant original
   source/tests and inspect the matching appearance and behavior.
2. Identify the preceding verified recovery point and define the expected
   observable result, test, capture and acceptance check before implementation.
3. Repair/reuse the narrowest useful seam; add focused behavioral regressions.
4. Run appropriate engineering checks and real native workflow checks. Review
   new captures; update separate ledger dimensions honestly.
5. Update catalog paths/symbols/contracts and the slice evidence/checkpoint;
   append concise history to `WORK_LOG.md` with exact next work and open risks.
6. Create and verify the post-slice snapshot before another slice or compaction.
   A failed check means the slice remains open; record failures and recovery.

Snapshots use new `G:\Builder3D-Rust_snapshot_<timestamp>_RB-<slice>-<purpose>`
directories. Never overwrite or modify an existing snapshot. Include source,
assets, manifests/lockfiles, tests/fixtures, docs, catalog, parity ledger,
evidence, work log and hidden Git metadata. Exclude `target`, caches and only
reproducible outputs not needed as acceptance evidence. Do not exclude the
entire evidence tree or work log. Avoid junction traversal. Verify copy result,
relative path set, file count, byte count, and recovery-critical file hashes;
record exclusions, absolute path, timestamp and results. A receipt appended
after copying must say the snapshot predates that receipt. Recovery materials
must make both the source and its verification understandable.

On context resume, read the latest checkpoint, instructions, plan and Git
status. Continue the exact next unfinished slice; do not restart conversion.
Do not compress unknowns into a statement that the product is finished.

## 11. Final definition of done

RB-11 can close only when every required feature ledger row has automated and
interactive evidence appropriate to its contract, every required visual state
has a valid matched comparison, the owner has explicitly accepted the look and
task walkthrough, and all applicable engineering/runtime/resource/package
gates pass. No broken/inert controls, substitute artwork, fake viewport,
separate AI scene, lost edits, or required untested workflows remain.

The final handoff names the exact accepted build, completed and pending platform
gates, measured budgets, remaining original limitations, approval evidence,
verification commands/results and the verified final recovery snapshot. A
Windows-only milestone may be named honestly while broader host work remains;
it must not be described as whole-project completion. No promotion of the Rust
app over the Python fallback occurs before the owner replacement gate passes.

## 12. Next execution session

RB-07 Appearance, Jobs and Print is complete. Its report, verification,
checkpoint, native evidence and snapshot receipt are under
`evidence/rebuild/rb-07-appearance-jobs-and-print/`. The verified immutable
completion snapshot is
`G:\Builder3D-Rust_snapshot_20261101-162515_RB-07-complete`.

RB-08 implementation and automated verification are recorded in
`evidence/rebuild/rb-08-preferences-and-complete-shell/`; its native Rust
interaction matrix and completion snapshot are still pending. The owner
directed on 2026-11-02 to continue to RB-09. Use
[`rb-09-shared-ai-and-regression-handoff.md`](rb-09-shared-ai-and-regression-handoff.md)
for the current slice. Do not represent RB-08 as closed or treat RB-09 as
resolving its open evidence. The owner reports that the interface and floor are
still missing; retain them as open product parity gaps. Visual review and owner
acceptance remain pending across the earlier slices. Keep `L:\Builder3D`
read-only and do not stage or commit.
