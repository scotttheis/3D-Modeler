# RB-07 Appearance, Jobs, and Print Completion Handoff

Status: ready for the next implementation session  
Written: 2026-11-01  
Scope boundary: complete the current RB-07 slice, then stop before RB-08

## 1. Objective

Complete the current product rebuild plan's RB-07 slice in the Rust rewrite:

1. appearance, material, color, and image workflows;
2. background progress, failure, and cancellation behavior;
3. print-readiness checks, staging, slicer selection, and handoff;
4. every affected native button and entry point, including failure and cancel
   paths;
5. focused automated tests, paired native evidence, and a verified additive
   snapshot.

RB-07 is complete only when the behavior is usable from the native desktop
surface and is supported by tests and evidence. Do not call the whole product
complete: RB-08 through RB-11 remain later slices. When RB-07 is complete,
record the next slice as RB-08 and stop.

This is an execution handoff, not a new product policy. Make normal
implementation decisions without stopping for routine approvals. Keep changes
inside the existing architecture and reuse working code before adding new
abstractions.

## 2. Verified starting point

The preceding slice, RB-06, is complete and has a verified snapshot:

    G:\Builder3D-Rust_snapshot_20261101-095500_RB-06-complete

The RB-06 snapshot receipt is:

    E:\3D-Modeler\evidence\rebuild\rb-06-modelling-tools-and-brushes\snapshot-receipt.md

The final RB-06 verification recorded:

- source and snapshot: 2,532 included files and 396,942,394 bytes;
- missing files: 0;
- extra files: 0;
- size differences: 0;
- SHA-256 differences: 0;
- reparse entries: 0;
- target, cache, and log directories excluded;
- Rust workspace: 271 tests passed, with no failures or ignored tests;
- Python reference: 272 tests passed, with two pre-existing trimesh
  RuntimeWarnings;
- cargo fmt check, cargo check, clippy with warnings denied, cargo deny,
  cargo metadata, debug/release builds, and smoke checks passed;
- final Rust native evidence: 72 records and 72 Rust PNGs, paired with six
  Python baseline records and six Python PNGs;
- final debug executable SHA-256:
  82284b6fd85221df3a804790cafb1171c469c5cb614460cb73199068ac551d14;
- final release executable SHA-256:
  874b2076060a29cdadcee2497c2da2ba3a178048623508f9dddd99c72de2879a.

The final RB-06 snapshot is an immutable reference. Do not edit it or replace
it. Start from the current E:\3D-Modeler worktree and take a new additive
RB-07 baseline before implementation. The Rust repository currently has no
commits and its files are untracked; preserve the worktree and do not use
reset, checkout, or other commands that discard it. Do not stage or commit.

The Python reference at L:\Builder3D is read-only for this project. Inspect it
and run its tests as needed, but do not edit, format, install into, or clean
that tree. Existing Python changes and existing warnings are not RB-07
changes; preserve them and report them accurately.

Visual and owner acceptance was still pending at the end of RB-06. Keep that
status honest in RB-07 reports. It does not prevent technical implementation,
testing, or evidence collection.

## 3. Authoritative reading order

Before changing code, read the current versions of:

1. E:\AGENTS.md and E:\RustDeveloper.md;
2. E:\3D-Modeler\AGENTS.md;
3. E:\3D-Modeler\docs\product-rebuild-plan.md, especially sections 4,
   6, 8, 9, 10, and 11;
4. the RB-06 report, verification, checkpoint, walkthrough, button ledger,
   and snapshot receipt under
   E:\3D-Modeler\evidence\rebuild\rb-06-modelling-tools-and-brushes;
5. E:\3D-Modeler\WORK_LOG.md;
6. the current component catalog and parity matrix;
7. the current Rust sources listed in section 5 below;
8. the relevant Python handlers, services, tests, and native evidence under
   L:\Builder3D;
9. legacy technical evidence under evidence\r7\10-image-color,
   evidence\r7\11-print-handoff, evidence\r7\12-runtime, and
   evidence\r7\13-desktop-adapters.

The product plan is authoritative for the current slice names. Legacy R7
folders and their technical reports are useful implementation evidence, but
they are not proof that the current RB-07 native desktop workflow is complete.
Do not redo already-working core behavior merely because it has an old R7
folder; verify its current desktop reachability and fill the actual gap.

If a change touches the MCP server, tool schemas, or protocol behavior, first
read E:\MCP_DESIGN_GUIDE.md, then the owning MCP and architecture documents
and the current official MCP sources required by the workspace instructions.
Avoid MCP changes when the existing interface is sufficient.

## 4. RB-07 scope and completion contract

### 4.1 Appearance, material, color, and image

Trace the Python behavior and current Rust implementation before changing the
contract. The completed native workflow must cover:

- the Material, Paint, and Color entry points, including duplicate menu,
  toolbar, item, and command routes that are supposed to reach the same
  behavior;
- choosing and applying an authored solid color to the selected object;
- sampling an image color through a file chooser or equivalent native route;
- missing, unreadable, corrupt, oversized, and unsupported image failures;
- a clear distinction between average-color sampling and true texture mapping.
  Do not describe a sampled average color as texture mapping. If the Python
  contract actually requires texture coordinates or an image texture, trace
  and implement that real behavior or record the precise remaining gap;
- preservation of authored color through selection changes, boolean
  operations, save/export, reopen, and re-import where the reference promises
  that behavior;
- cancel, close, retry, and reopen behavior for the dialog without stale
  values or silent no-ops;
- visible success and failure status, with the selected object and scene
  remaining consistent after errors.

The existing image-color implementation is in
crates\modeler3d-app\src\image_color.rs. It is bounded and currently samples
an average color. Confirm its limits and error contract against the reference
before changing it.

### 4.2 Background jobs, progress, errors, and cancellation

The existing operation model in
crates\modeler3d-app\src\operations.rs has a bounded single-active-operation
lifecycle, progress, messages, cooperative cancellation, retention, and
terminal states. Use it as the starting seam. Do not create a competing job
registry unless the current design cannot satisfy a demonstrated requirement.

Verify and complete its actual desktop integration:

- a long-running operation does not freeze the native window;
- the user can see meaningful queued/running/progress/committing and terminal
  status as appropriate;
- cancel is available only when it can act, reaches a terminal cancelled
  state, and does not commit partial results;
- failures preserve the previous scene and history and show a useful reason;
- retry or a later successful operation works after cancellation or failure;
- duplicate requests, busy-state input, dialog close/reopen, new-scene
  actions, and selection changes do not create stale worker updates;
- a worker result is applied only when the scene revision/object identity it
  was created for is still current;
- all visible progress, error, and cancel buttons have real handlers and
  non-silent disabled states.

Test the desktop path, not only the registry's unit tests. Preserve RB-06
brush, boolean, selection, undo, and history behavior while wiring jobs into
the shell.

### 4.3 Print readiness, staging, and slicer handoff

The completed print workflow must provide:

- a native print-preparation dialog for the visible/selected printable
  objects;
- bed-size and scale input with clear valid, warning, and failure results;
- checks for fit, below-bed geometry, height, scale, mesh health,
  non-manifold or disconnected shells, hidden objects, and empty selections
  where the current contract calls for them;
- a report that does not mutate the scene and remains understandable after a
  failed check;
- independently checked staged output: expected files exist, have nonzero
  content, use the intended format, and can be reopened or otherwise
  validated;
- known-slicer discovery and selection with no arbitrary executable launch;
- the Bambu Studio STL fallback where the existing handoff contract requires
  it, and the normal 3MF route for other supported slicers;
- safe behavior for no discovered slicer, a missing executable, failed
  staging, invalid scene data, and failed process launch;
- an actual installed known slicer opening the staged test scene, when one is
  available on the machine. Capture the process and scene result, then close
  the test process cleanly.

The current implementation seams are:

- crates\modeler3d-app\src\print_prep.rs;
- crates\modeler3d-app\src\print_handoff.rs;
- crates\modeler3d-platform\src\slicer.rs;
- dialog and command handling in
  crates\modeler3d-app\src\dialogs.rs,
  crates\modeler3d-app\src\commands.rs, and
  crates\modeler3d-app\src\shell.rs.

Use an installed slicer found on this machine if one exists. Do not silently
invent a fake launch or claim the installed-slicer exit evidence from a unit
test. If no supported slicer is installed, record the exact unavailable
external prerequisite in the report while completing all code, tests, staging
checks, and safe failure paths that are available locally.

### 4.4 Affected-button completion

Maintain a focused ledger for every button and command touched by RB-07. At
minimum audit:

- Material, Paint, and Color open/apply/cancel paths;
- image path selection and Sample image color;
- print preparation open, Run print prep check, bed/scale fields, slicer
  selection, Prepare visible objects, and Launch slicer;
- progress, cancel, retry, close, and error acknowledgement controls exposed
  by the job integration;
- duplicate entry points in menus, toolbars, scene/item actions, and command
  dispatch where they are part of the current RB-07 surface.

Each ledger row should identify the route, expected result, native result,
automated coverage, and evidence frame or an exact reason it is not
applicable. A visible button that does nothing, only changes a label, or
silently swallows an error is not complete.

## 5. Existing implementation to reuse and verify

These modules already contain substantial RB-07-related work. Treat them as
starting points, not as proof of end-to-end completion:

| Area | Current Rust seam | Verification still needed |
| --- | --- | --- |
| Image color | modeler3d-app/src/image_color.rs | Reference contract, native chooser/error route, authored-color persistence |
| Material dialog | modeler3d-app/src/dialogs.rs | All entry points, apply/cancel/reopen behavior, visible errors |
| Desktop dispatch | modeler3d-app/src/commands.rs and shell.rs | Real reachability, state lifetime, no synchronous UI freeze |
| Operations | modeler3d-app/src/operations.rs | Shell worker/poll/cancel integration and stale-result guards |
| Print checks | modeler3d-app/src/print_prep.rs | Native reports, invalid/empty/hidden cases, no scene mutation |
| Print staging | modeler3d-app/src/print_handoff.rs | Filesystem validation, format fallback, cleanup, failure safety |
| Slicer discovery | modeler3d-platform/src/slicer.rs | Installed-slicer discovery and actual safe launch |
| Existing technical adapters | modeler3d-mcp/src/r4/server.rs | Reuse only where needed; not a substitute for native proof |

Use ripgrep to locate the exact Python handlers and tests rather than
assuming that a similarly named Python module exists. The reference source
and its tests decide whether a Rust behavior is a parity gap.

## 6. Explicit non-goals for this handoff

Do not pull later work into RB-07:

- RB-08 preferences, settings migration, favorites/order/pinning/reset,
  shell-wide menu redesign, accessibility, or welcome/onboarding work;
- RB-09 shared live-document MCP behavior or a broad MCP redesign;
- RB-10 performance, packaging, host validation, and distribution work except
  the focused build checks needed to verify RB-07;
- RB-11 final owner acceptance and release-readiness sign-off;
- rewriting working legacy R7 modules without a current contract or
  integration failure;
- replacing an average solid-color sample with a pretend texture workflow;
- edits to L:\Builder3D, staging, commits, or broad repository cleanup.

Do not add new gates, approval pauses, configuration systems, or process rules
unless RB-07 behavior actually requires them. Keep the implementation and
handoff focused on the contract above.

## 7. Recommended execution sequence

This sequence is a practical route, not a reason to stop between every small
change:

1. Confirm the current worktree, active processes, available disk space, and
   the RB-06 receipt. Create an additive baseline such as
   G:\Builder3D-Rust_snapshot_YYYYMMDD-HHMMSS_RB-07-baseline. Exclude
   target, cache, logs, and reparse entries.
2. Read the reference paths and current Rust seams. Create
   evidence\rebuild\rb-07-appearance-jobs-and-print\ and start the
   requirement map, affected-button ledger, and source-preservation note.
3. Run focused reference and Rust tests before changing behavior. Record
   existing failures separately from RB-07 failures.
4. Close appearance/material/image parity gaps and add focused tests for
   success, invalid input, cancel, persistence, and the solid-color versus
   texture distinction.
5. Connect the existing operation model to the native shell. Exercise a
   real progress, cancellation, failure, retry, and stale-result path. Keep
   the normal scene/history path intact.
6. Complete the native print-prep, staging, slicer-selection, and launch
   paths. Validate staged output independently and exercise safe failures.
7. Perform the complete affected-button audit, then run the full relevant
   test and quality gates.
8. Capture paired Python/Rust native evidence in light and dark themes,
   update the evidence and parity documents, and create the verified
   RB-07-complete snapshot only after the last source or evidence change.
9. Write the checkpoint with next slice RB-08, report any honest external or
   owner-acceptance limitation, and stop before implementing RB-08.

Use incremental local checkpoints if they help recovery, but do not create
administrative snapshots for every small edit. The final snapshot and its
verification receipt are required.

## 8. Test and quality gates

Run focused tests while implementing, then run the complete project gates from
E:\3D-Modeler. Use the existing Rust toolchain and lockfile.

Python reference tests must use the existing environment and remain
read-only. A typical command is:

~~~powershell
$env:PYTHONDONTWRITEBYTECODE = "1"
& "L:\Builder3D\venv\Scripts\python.exe" -B -m pytest -q -p no:cacheprovider <focused-tests>
~~~

Choose the actual focused test paths after locating the relevant reference
tests. At minimum cover image/material, operation manager or application
services, print behavior, and native action routes that are part of the
reference contract. Run the complete reference suite when practical and
record its exact count and pre-existing warnings.

Rust focused coverage should include the changed tests in the app and
platform crates, especially image_color, dialogs/commands/shell,
operations, print_prep, print_handoff, and slicer discovery. Then run:

~~~powershell
Set-Location E:\3D-Modeler
. .\scripts\Initialize-BuildClang.ps1
cargo fmt --all -- --check
cargo check --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo deny check
cargo metadata --locked --format-version 1 --no-deps
cargo run -p modeler3d-app --bin modeler3d-app -- --smoke
cargo build --release --locked -p modeler3d-app --bin modeler3d-app
cargo bloat --release --workspace
cargo bloat --release -p modeler3d-app --bin modeler3d-app
~~~

Run cargo audit if it is installed and record the result. If it is not
installed, say so; do not turn an unavailable optional tool into an
unrelated installation task. Record exact test counts, warnings, executable
hashes, and any test not run.

The smoke command is only a sanity check. It does not replace launching the
native application and exercising the RB-07 workflows with real OS input.

## 9. Paired native evidence

Create the new evidence set under:

    E:\3D-Modeler\evidence\rebuild\rb-07-appearance-jobs-and-print

Use the established RB-06 paired-evidence discipline:

- run the Python reference first, close it, then run the Rust application;
- capture light and dark themes for each applicable route;
- use the same 1280x840 window and 96 DPI/100% scale unless the reference
  requires another documented size;
- use real OS input and the real native window; do not substitute headless
  widget screenshots for native proof;
- run captures serially and confirm no stale 3D Modeler process remains;
- record build SHA, route, theme, window size, DPI, fixture, action sequence,
  result, and timestamp for every frame;
- keep accepted and rejected attempts separate and explain rejected captures;
- compare the Python and Rust result, not merely the presence of a button.

The minimum scenario set is:

### Appearance and image

- open each supported appearance entry point;
- apply a solid authored color to a selected object;
- sample a valid image color and show the resulting state;
- exercise missing, corrupt, unreadable, oversized, and cancelled image input;
- save/export, reopen or re-import, and verify the authored color;
- show the distinction between average solid-color sampling and any true
  texture-mapping behavior required by the reference.

### Jobs

- start a long enough operation to show progress without freezing the window;
- cancel it and verify no partial scene/history commit;
- trigger a controlled failure and show its message;
- retry successfully;
- exercise busy/duplicate input and a stale selection or scene revision if
  the native route exposes it;
- verify dialog close/reopen and new-scene behavior after terminal states.

### Print

- run a clean print-prep report;
- run fit, height, below-bed, scale, health, hidden, and empty/error cases
  covered by the contract;
- prepare staged output and independently verify its files and format;
- exercise known-slicer selection, including the Bambu fallback where
  applicable;
- exercise missing executable and failed staging safely;
- open the staged test scene in an actually installed known slicer and
  capture the visible result when available.

If the Python native runner cannot complete a particular modal or external
application route, record the exact limitation and use focused automated
reference coverage plus Rust native evidence. Do not label an unobserved
pair as passed.

## 10. Required RB-07 deliverables

Populate the RB-07 evidence directory with the same concise, auditable
structure used for RB-06:

- report.md;
- verification.md;
- requirement-test-map.md;
- walkthrough.md;
- paired-comparison-board.md;
- initial-button-ledger.md with final dispositions;
- capture-metadata.json;
- fixture-manifest.json and operation-manifest.json as applicable;
- accepted-journal.ndjson and rejected-attempts.md;
- source-preservation.md;
- checkpoint.md;
- snapshot-receipt.md.

Update, without rewriting prior history:

- E:\3D-Modeler\WORK_LOG.md;
- E:\3D-Modeler\docs\component-catalog.toml;
- E:\3D-Modeler\evidence\rebuild\parity-matrix.toml.

Add current RB-07 rows and an RB-07 audit entry. Keep legacy R7 rows
distinct. Do not claim owner acceptance, overall product acceptance, or
later-slice completion from RB-07 evidence.

The report should state:

- what was implemented and where;
- which existing R7 work was reused;
- the exact automated test and quality-gate results;
- the exact native capture counts and paired routes;
- slicer availability and actual launch result;
- any honest limitation, warning, or pending owner review;
- the final executable hashes;
- the final snapshot path and verification result;
- next slice: RB-08.

## 11. Final additive snapshot and stop condition

After all code, tests, native captures, and documentation are final, create a
new snapshot with a unique timestamp, for example:

    G:\Builder3D-Rust_snapshot_YYYYMMDD-HHMMSS_RB-07-complete

Never overwrite the RB-06 snapshot or another existing snapshot. Exclude
target, cache, log, and reparse entries. Verify the source and snapshot by
path, file count, byte count, missing/extra paths, size differences, and
SHA-256 differences. Write the receipt only after the final copy. If the
receipt or any evidence changes after copying, resynchronize and verify
again.

Before stopping:

- confirm no test or application process is left running;
- confirm the Python tree was not modified;
- confirm git status shows no staged or committed changes;
- confirm the final snapshot receipt and RB-07 checkpoint exist;
- set the checkpoint's next slice to RB-08;
- stop. Do not begin RB-08 implementation in the same session.

## 12. Known risks and useful decisions

- The product plan's current name is RB-07. Legacy R7 evidence is a
  technical reference, not current-slice completion proof.
- The existing image sampler produces an average color. Preserve that
  distinction unless the reference explicitly requires texture mapping.
- The operation registry and print modules already exist; the likely risk is
  native shell reachability and lifecycle correctness, not a need to replace
  every core module.
- A slicer launch must be real evidence when an installed supported slicer is
  available. A mocked process or unit test is not equivalent.
- Existing Python worktree dirt and the two known trimesh warnings are
  pre-existing context and must not be “cleaned up” as part of RB-07.
- Windows-native evidence is the target environment. Document any external
  application or display limitation rather than hiding it.

## 13. Prompt for a fresh context window

Paste the following as the first user message in the next context:

~~~text
Read and follow E:\3D-Modeler\docs\rb-07-appearance-jobs-and-print-handoff.md.

Continue the 3D Modeler Rust rewrite from the current E:\3D-Modeler
worktree. Complete the entire current RB-07 slice: appearance/material/image
workflows, native background progress/errors/cancel, print checks, staging,
known-slicer handoff, and every affected non-working button. Reuse the
existing Rust image_color, operations, print_prep, print_handoff, slicer,
dialog, command, and shell work after verifying its current desktop behavior;
do not redo working legacy R7 implementation without a demonstrated gap.

Use L:\Builder3D only as a read-only Python reference and test target. Do not
edit it. Do not stage or commit any changes. Start from the current worktree
and the verified RB-06 snapshot
G:\Builder3D-Rust_snapshot_20261101-095500_RB-06-complete; create a new
additive RB-07 baseline and, after all work is finished, a verified
RB-07-complete snapshot on G:\. Work autonomously and do not stop for routine
approval questions or add process rules that are not needed for this slice.

Run focused and full Rust/Python tests, the documented quality gates, paired
native Python/Rust evidence in light and dark themes, and the real installed
slicer launch check when available. Keep all statuses honest, especially
average-color sampling versus texture mapping, unavailable slicers, and
owner/visual acceptance. Update the RB-07 evidence directory, button ledger,
component catalog, parity matrix, WORK_LOG, and checkpoint. End with exact
test counts, native evidence counts, executable hashes, snapshot verification,
and next slice RB-08. Then stop before implementing RB-08.
~~~
