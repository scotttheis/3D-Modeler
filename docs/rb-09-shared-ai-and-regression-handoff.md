# RB-09 Shared AI and Regression Handoff

**Status:** the historical Python/Rust paired gate passed on its recorded build. The current Rust Release GUI/MCP workflow and optional-MCP failure check ran on 2026-11-06. The RB-09 schema-parity repair completed on 2026-11-07 against a verified read-only current `L:\Builder3D` catalog capture. The schema criterion passes; full RB-09 acceptance remains open.
**Written:** 2026-11-07; owner path/backup direction updated 2026-11-03; current Rust Release recheck added 2026-11-06; schema-parity repair checklist added 2026-11-07
**Boundary:** keep schema disposition, visual review, owner acceptance, and overall acceptance pending; stop before RB-10

## Current continuation (2026-11-06)

Run `currpair-20261106-072716` used app SHA-256
`0bc4f1a0fd0327e51ecd8acdb75a01120b3c53325b11738a7b3fb6ce198cc75a` and
gateway SHA-256 `aff0af548a3f8055e2ebe1d218135a774cc9b88f210a94f19627a9f9b22e1c1e`.
The official Python MCP SDK 2.2.0 was used only as a client to the Rust stdio
gateway; the Python 3D Modeler product was not launched and the saved Python
catalog snapshots were not refreshed. The Rust desktop reopened the saved
project at revision 25 with the stable Cube/Sphere IDs visible in the viewport
and Items panel. A separate optional-MCP failure run kept the Rust GUI usable,
returned retryable `desktop_unavailable` to the client, and removed owned
runtime state at shutdown.

All four profile counts and ordered names match the saved catalogs. Exact Rust
input/output schema matches are zero in all four profiles. The differences
are listed in `currpair-20261106-072716/schema-comparison.json`; they are not
approved deviations. Keep that RB-09 contract gap open. The saved-project
run, schema comparison, logs, captures, and final snapshot receipt are the
current evidence.

The active Rust source/build/run copy is `E:\3D-Modeler`. Use `G:\` as
the primary additive backup destination. Removable USB copies are occasional
one-offs unless the owner requests otherwise; the current one-off copy is
recorded in
`evidence/rebuild/rb-09-shared-ai-and-regression/usb-backup-receipt.md` and is
not a working tree. `L:\Builder3D` remains the separate read-only Python
reference/shipping build.

## Current schema-parity repair checklist (2026-11-07)

At the start of this repair, RB-09 schema compatibility was open. The
2026-11-06 comparison in
`currpair-20261106-072716/schema-comparison.json` compared current Rust catalogs
with saved Python catalog snapshots; it did not refresh those snapshots or
launch the Python product. Treat those saved catalogs as evidence, not as proof
of the current Python reference contract. Before changing Rust schemas, verify
the identity of the currently designated `L:\Builder3D` reference and capture
its current schemas read-only. Do not launch or test an older Python build. If
the current reference identity cannot be confirmed, stop before schema changes
and report the missing evidence.

The ordered tool names already match in all profiles: observe 7, model 52,
files 59, and full 60. Exact input- and output-schema matches are zero in all
four profiles. Resolve these items against the refreshed current reference:

1. Reconcile the count in the evidence: the summary says Rust adds
   `return_detail` to 19 tools, while the detailed full-profile list contains
   20. The listed Rust-only fields are on `set_torus_parameters`,
   `elastic_grab`, `crease_polish`, `smooth_brush`, `blob`,
   `normalize_face_density`, `snap_translate`, `settle`, `mirror`, `align`,
   `arrange`, `repair`, `simplify`, `subdivide`, `smooth`, `boolean_merge`,
   `boolean_subtract`, `boolean_intersect`, `emboss_text`, and `hollow`. The
   current Rust handler does not consume this field; remove it if the refreshed
   current reference still omits it.
2. Make `orbit_view` requiredness match the reference. Rust currently requires
   `yaw_delta_deg` and `pitch_delta_deg`, although its handler supplies defaults
   when they are omitted.
3. Resolve the input-schema strictness and constraint differences. Rust
   currently sets `additionalProperties: false` on every tool; the saved
   Python schemas declare no such restriction. The MCP design guide says input
   schemas should reject unexpected fields unless a deliberate extension point
   is documented, so do not remove Rust's strict validation silently. First
   capture the current Python runtime's behavior for unknown fields; then make
   an explicit, owner-approved compatibility decision: match that behavior, or
   preserve strict Rust validation as a documented and tested deviation. The
   comparison also reports field-bound and nullability differences. Generate a
   property-by-property diff from the refreshed current reference before
   editing those constraints.
4. Replace the single Rust output-schema superset (104 properties, all
   unconstrained) with the tool-specific typed output schemas from the current
   reference. The saved full profile has 18 distinct Python output schemas.
5. Match observed top-level results to their output schemas. The saved
   comparison reports extra Rust fields: `add_primitive`, `undo`, and `redo`
   add `action`, `affected_object_ids`, `changed`, and `history_state_id`;
   `get_status` adds `approved_roots`, `operation_id`, and `profile`;
   `new_project` adds `action` and `changed`; `scene_summary` adds `dirty`,
   `history_state_id`, and `selection`. `set_color` and `set_transform` are
   missing the Python schema's required `result` field. Recheck these against
   current-reference schemas and live Rust responses.
6. Match the tool annotations: saved Python has
   `emboss_text.destructiveHint: false` and
   `launch_slicer.openWorldHint: false`, while Rust currently reports `true`
   for both.
7. Extend parity verification to validate full nested JSON Schemas, including
   required fields, types, bounds, nullability, and additional properties. Run
   it across every profile and against real successful result envelopes, not
   only names, exact top-level equality, or top-level output keys.

No schema deviation has owner approval. Do not mark the schema criterion or
RB-09 complete until the refreshed reference comparison and tests pass, or an
intentional deviation is explicitly approved, documented, and tested.

### Repair results (2026-11-07)

The designated L:\Builder3D identity was confirmed read-only: Git HEAD
3a0160a95c55f68ce9f1e89d17c792816ac001df, Python 3.12.10, MCP SDK 2.2.0.
The official SDK captured all four catalogs with tools/list only. The Python
product was not launched, no handler was invoked, and the reference tree was
not modified. Current catalog captures are under
evidence/rebuild/rb-09-shared-ai-and-regression/reference-current-20261107-084248/.

The mismatch was 20 Rust return_detail fields before repair, not 19 as the
prior summary stated. The refreshed current reference has five:
apply_actions, set_color, set_transform, transform_selection, and translate.
Rust now exposes exactly those five in model/files/full, and none in observe.
Rust Release tool names/order, every input/output JSON Schema, and official MCP
annotations match recursively in observe/model/files/full (7/52/59/60 tools),
with zero recursive differences. The current Python runtime accepts and drops
unknown root fields, while rejecting unknown fields inside apply_actions.args;
Rust matches both behaviors.

jsonschema 0.60.2 validates complete Draft 2020-12 schemas, including nested
inputs and outputs, in all four profiles. Automated examples compile and
validate each tool schema. The rebuilt current Release app (SHA-256
0e874b6fb06d9b6cb5e70517ca35856d3a8304975e2a0842cd971f20df2c45ba) was
opened with the matching Release gateway (SHA-256
636171b58800915de87fc8ad26814498279919ea80341076a344c208757da370). The
official Python MCP SDK received 25 successful structured results; all 25
passed the focused Rust output-schema validation test. This includes a
successful new_project from a clean empty document and a completed
operation_status. The deliberately invalid nested apply_actions.args field
returned invalid_input. The app closed normally and its runtime descriptor
was removed.

The full recursive comparison and final Release result evidence are indexed in
schema-parity-run-manifest.json. The official Tools specification and Rust SDK
were checked; the protocol uses Draft 2020-12. One Rust custom
_meta.com.modeler3d/longRunning value for emboss_text remains true while the
reference reports false. The comparator records this in three profiles,
separately from schemas and official ToolAnnotations; no owner approval is
claimed.

The schema criterion is complete, but this scoped repair does not close overall
RB-09 acceptance. RB-08 native evidence, the interface/floor gaps, visual
review, owner acceptance, and overall product acceptance remain pending. No
RB-10 work was started. The only Git changes here are the preserved working-tree
changes; nothing was staged, committed, pushed, reset, or restored.

## Current RB-09 endpoint (2026-11-04)

The actual Python and Rust desktop/MCP sequence passed after a scoped fix for
Rust MCP project presentation from Welcome. Its historical full-gate app and
gateway hashes remain d680cf0ba8e4c8bf7d5ccd1536bf7d5720987d676a1a0946c6c3505b613b0cb9
and a1dba8751c5a15cafbf6e26fac8db0b8a96f8464bf750ac62336f573b729545b.
Subsequent cargo-bloat and app-restore builds replaced the target outputs; the
original stderr logs are copied under
evidence/rebuild/rb-09-shared-ai-and-regression/logs/.

The current Release app/gateway hashes,
0bc4f1a0fd0327e51ecd8acdb75a01120b3c53325b11738a7b3fb6ce198cc75a and
aff0af548a3f8055e2ebe1d218135a774cc9b88f210a94f19627a9f9b22e1c1e, were
verified before and after a separate launch. Host preflight confirmed the
active Default desktop and 3D Modeler foreground. An official Python MCP SDK
2.2.0 client listed 60 tools and its read-only get_status call returned
app_state=ready, dirty=false, and scene revision 0. The app/gateway processes
and runtime descriptor were absent after shutdown. This was an identity check,
not a repeat of the full paired workflow or 50-capture matrix.

The original gate-time validator passed. The later wrap-up validator failure
was caused by checking current target paths against historical live-build
records. The updated validator now checks historical hashes against the paired
capture metadata and current target hashes against the identity-only recheck;
it passes. Details and hashes are in
evidence/rebuild/rb-09-shared-ai-and-regression/artifact-identity-recheck.json
and verification.md. The source manifest is unchanged.

The additive review snapshot
G:/Builder3D-Rust_snapshot_20261104-215904_RB-09-artifact-identity-review was
independently verified with 3,904 matching files / 701,101,053 bytes and zero
path, size/hash, or reparse differences. It is not a completion snapshot; see
the snapshot receipt.

Use the current report, verification, walkthrough, paired comparison board,
and checkpoint as the endpoint record. The dated pause and starting-point notes
below are historical. Keep the owner-reported interface/floor gaps, visual
review, owner acceptance, and RB-08 native matrix/completion snapshot open.
RB-09 verification does not mean overall product acceptance. Do not begin
RB-10 as part of this handoff.

## 1. Objective and owner direction

Complete the RB-09 row in [the product rebuild plan](product-rebuild-plan.md):
the desktop and MCP must inspect and edit the same live 3D Modeler document,
using the existing public MCP catalog and preserving revision, replay, file,
cancellation, history, and optional-MCP reliability contracts.

The owner said on 2026-11-02: “interface and floor are still not there, but
let's keep going.” Proceed with RB-09. Record the interface and floor as open
product parity gaps; RB-09 is not visual approval or overall acceptance. The
floor belongs to the existing viewport parity inventory. Do not silently mark
it fixed or broaden RB-09 into unrelated shell redesign.

RB-08 implementation and automated verification are recorded, but its paired
Rust native interaction matrix and completion snapshot are still pending
because the prior Windows session was locked. The owner has authorized moving
on; this does not close or accept RB-08. Do not fabricate its missing native
evidence, and do not use the RB-09 snapshot as an RB-08 completion receipt.

## 2. Required reading and authority

Before changing code, read:

1. `E:\AGENTS.md`, `E:\RustDeveloper.md`, this project's `AGENTS.md`,
   `docs\product-rebuild-plan.md`, and this handoff.
2. `WORK_LOG.md`, the RB-08 in-progress checkpoint/report/verification,
   `docs\component-catalog.toml`, and `evidence\rebuild\parity-matrix.toml`.
3. The RB-07 report, checkpoint, verification, walkthrough, and final snapshot
   receipt. Preserve their complete/pending status as written.
4. The read-only reference documents `L:\Builder3D\docs\ai-mcp.md`,
   `mcp-modern-design.md`, `architecture.md`, `testing.md`, `rust-rewrite.md`,
   and `project-format.md`, plus the matching Python catalog, bridge, shared
   command, history, file, and operation sources/tests. Keep `L:\Builder3D`
   read-only.
5. `E:\MCP_DESIGN_GUIDE.md`, the Rust MCP/app source and tests, and the current
   official protocol/SDK sources below. Recheck them when implementation
   begins; a protocol release or SDK change can make this reading stale.

The workspace guide is the MCP safety/design baseline. Use the primary
specification and official Rust SDK references: [MCP 2026-07-28 overview](https://modelcontextprotocol.io/specification/2026-07-28),
[tools](https://modelcontextprotocol.io/specification/2026-07-28/server/tools),
[transports](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports),
[authorization](https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization),
[official Rust SDK](https://github.com/modelcontextprotocol/rust-sdk), and
[rmcp 3.8.0 documentation](https://docs.rs/crate/rmcp/3.8.0). The workspace's
local MCP guide also links the current authorization rules. Preserve the
current product-specific operation-status/cancel contract unless a tested
client and SDK support justify a deliberate change; do not adopt an extension
just because it exists.

## 3. Verified starting point (2026-11-03 historical state)

Work from the current `E:\3D-Modeler` tree. Inspect `git status`, process
state, and current source again at session start. The tree is intentionally
uncommitted and currently appears as untracked project files; preserve it.
Do not stage, commit, reset, discard, or restore/overwrite it from a snapshot.

The verified RB-07 completion snapshot is
`G:\Builder3D-Rust_snapshot_20261101-162515_RB-07-complete`; its receipt
records 2,874 files / 574,294,626 bytes and zero relative-path, size, SHA-256,
or reparse-point differences. The additive RB-08 baseline is
`G:\Builder3D-Rust_snapshot_20261102-085830_RB-08-baseline`; its receipt
records 3,082 files / 577,354,485 bytes and zero path, size, SHA-256, or
reparse-point differences. Neither snapshot is the working tree.

RB-08 evidence is under
`evidence/rebuild/rb-08-preferences-and-complete-shell/`. Its current
checkpoint records 314 workspace tests passed, 178 focused Rust app/platform
tests, and 12 focused Python reference tests. The accepted Python live run has
50 captures at
`evidence/rebuild/rb-08-preferences-and-complete-shell/native/python-live-baseline-20261102-094708/captures/`.
There are zero accepted Rust native captures from the locked
session; its black diagnostic image is rejected. The prior Release binary was
15,653,888 bytes with SHA-256
`db406708cf19efe9ed62c3c53d02684dc49c4458d44c59bd940e6bc45ca81f75`. This
identifies only that earlier build; rebuild and record the RB-09 binaries.
The ten documented Rust gates exited successfully; `cargo audit` was
unavailable and the `cargo bloat --workspace` CLI warning is recorded in the
RB-08 verification. The RB-08 final snapshot does not exist yet.

The previous RB-08 session used a Release window in
`evidence/rebuild/rb-08-preferences-and-complete-shell/native/rust-live-final-20261102-101724/`;
its checkpoint recorded PID 27176 and Windows LockApp PID 9808 at the time.
These are historical observations, not current process state. Do not send
input to or close a window until its current identity and owner are verified.

Owner direction on 2026-11-03: treat `E:\3D-Modeler` as the only active
Rust project tree, `G:\` as primary backup, and USB as an occasional one-off
backup destination. The current paused checkpoint and latest G: backup are
listed in `evidence/rebuild/rb-09-shared-ai-and-regression/checkpoint.md`.

## 4. Existing seams and the demonstrated gap

Reuse the current typed Rust catalog, server safety, and desktop command
behavior. Do not assume the current MCP tests prove app integration:

- `modeler3d-app` owns `CommandState` and `Modeler3dApp`; the command state
  contains the desktop's scene, dirty/project state, clipboard, operation
  registry, preferences, and display state. The app crate does not currently
  depend on `modeler3d-mcp`.
- `modeler3d-mcp` has an ordered 60-tool `TOOL_CATALOG`, strict schemas,
  profiles, approved-root checks, expected project/revision validation,
  replay retention, and operation status/cancellation. Existing MCP tests
  verify the server/catalog behavior but do not prove the running desktop and
  server share a live document.
- `R4Server::new` constructs a default `R4State` with a new independent
  `modeler3d_core::Scene`. The `r4_mcp` binary serves that R4 instance over
  stdio. This proves the server can edit its own scene; it does not connect
  that scene to the running desktop. A second MCP-owned scene or cloned scene
  without a shared command/history boundary fails RB-09.
- The Python product contract uses a stdio gateway, a per-user runtime
  descriptor, an authenticated loopback bridge, shared application commands,
  isolated long work, revision-checked single commits, and a clear unavailable
  response when the desktop is closed. The Rust platform crate already has
  runtime-path/descriptor seams, but the exact live app bridge must be
  inspected before choosing an implementation.

Choose and document the smallest dependency-safe integration seam before
editing. Keep core geometry independent of UI and MCP protocol types. Route
all desktop and MCP mutations through one semantic application command/history
boundary. Use a typed app command queue or shared service as the architecture
requires; never mutate widgets from an MCP worker thread or let an MCP commit
bypass the UI's dirty state, undo stack, render invalidation, or Items panel.
Long operations must work on isolated snapshots and commit once on the UI
owner's thread after checking project identity, document generation, and
revision.

## 5. Contracts RB-09 must preserve

- Keep the 60 public tool names, deterministic catalog order, JSON schemas,
  capability profiles, result envelopes, and supported protocol compatibility
  unless the read-only Python reference proves a specific contract mismatch.
  Do not add a generic endpoint-discovery or arbitrary action tool.
- Reads must describe the currently open desktop project and current scene
  revision. A client edit must be visible in the viewport and desktop panels
  without restarting the app; a GUI edit, undo, or redo must be inspectable by
  MCP immediately.
- Preserve stable object IDs, project identity, expected-revision conflict
  checks, compact scene summaries/deltas, and structured recoverable errors.
  A single successful model operation creates the same one-step undo entry for
  both adapters; a no-op creates none.
- Preserve idempotency replay: same key plus canonical arguments replays the
  result; a reused key with different arguments fails. Current Rust bounds
  include 600-second/256-entry replay retention, one-hour/128 operation-record
  retention, and bounded payloads. Verify these against the product reference
  before changing constants.
- Preserve file policy: configured approved roots, safe canonical paths,
  bounded inputs/outputs, explicit overwrite, and no live-document mutation on
  failed open/import/export. Project new/open/save tools target the same open
  desktop document and path/dirty state.
- Long-running work reports a stable operation ID and bounded progress,
  honors cooperative cancellation, rejects stale revisions, commits once, and
  leaves scene and history unchanged after failure/cancel/conflict. Keep the
  UI responsive; test resource limits and the busy path.
- MCP remains optional. The desktop launches and remains fully usable if the
  bridge is disabled, absent, unavailable, or fails to start. Report a safe
  unavailable result to the client, and clean up owned runtime descriptors,
  listeners, workers, and secrets at shutdown. Keep the bridge private and
  authenticated; do not add remote exposure or log credentials.

## 6. Work sequence

1. Inspect current processes, environment, Rust tests, current source, Python
   catalog/bridge behavior, and any earlier probe. Verify the current Python
   app can expose a live project to an actual MCP client. Record its exact
   starting state and use a small paired fixture with distinct objects.
2. Before code changes, create a uniquely named, additive RB-09 baseline on
   `G:\` from the current worktree. Never overwrite an existing RB-07/RB-08
   snapshot. Verify relative paths, file count, bytes, per-file SHA-256, and
   reparse exclusions; record the receipt. Keep `target` and reproducible
   caches excluded as permitted by the plan.
3. Run a current Rust pre-change probe: list all catalog names/schemas by
   profile, exercise representative direct server tools, and prove in a live
   client whether the desktop sees a mutation. Record the separate-scene
   baseline honestly. Compare Python reference behavior before changing code.
4. Write a small architecture decision in the RB-09 evidence: where the live
   document and shared semantic command boundary live, how the MCP server
   reaches them, how UI-thread dispatch works, how disconnect/shutdown works,
   and which existing module is reused. Then implement the narrow seam.
5. Integrate query, model edit, project/file, history, and operation paths with
   the running app. Ensure both the viewport and panels invalidate on MCP
   commits, and that GUI changes advance the same visible revision. Guard the
   live project against stale concurrent edits.
6. Add focused contract, negative, concurrency, replay, bridge, and regression
   tests. Use fake transports/state for deterministic tests and a real MCP
   client for integration; a direct `R4Server::call_value` test alone cannot
   close the live-app row.

## 7. Required verification

Run focused MCP/app/shared-boundary tests first, then the workspace tests and
all documented engineering gates in `docs/product-rebuild-plan.md`. Run the
focused Python catalog/gateway tests from the existing read-only environment;
do not install or write into `L:\Builder3D`, run broad Python desktop suites,
or use an offscreen OpenGL UI. Record exact command, exit code, test count,
warnings and unavailable optional tools. Build the final Release app and MCP
entry point from the same source revision and record both executable hashes.

With the apps exercised sequentially and matching fixtures, use an actual MCP
client for each supported server path:

1. Open a Python project and Rust project in their respective live GUIs. Read
   project ID, scene revision, stable object IDs and geometry through MCP.
2. Add and transform a model through MCP. Capture the visible Rust/Python
   viewport and panel update. Edit/rename/hide or transform through the GUI;
   confirm MCP reads the immediate new values and revision.
3. Undo an MCP edit in the GUI, then redo it; also undo a GUI edit through
   MCP. Confirm identical target IDs, geometry, revision progression, dirty
   feedback, and one history entry per actual edit.
4. Save through MCP, inspect the same path/dirty state in the desktop, close
   and reopen through the GUI, and compare project ID/object IDs/geometry and
   independently reopened output. Also test New/Open through MCP updating the
   visible desktop and honoring dirty guards.
5. Exercise approved and denied file paths, overwrite rejection, malformed or
   unsupported input, and failures. Verify a failed request changes no scene,
   path, revision, dirty flag, or history.
6. Test the same idempotency key twice, key conflict with different arguments,
   expected-revision conflict, two clients racing a mutation, a real long
   operation with status/cancel, cancellation/failure/stale-result rejection,
   and no partial model or history change.
7. Start the desktop with MCP disabled/unavailable, and with the client present
   while the desktop is closed. Confirm ordinary GUI work stays available,
   clients get a clear bounded error, startup remains responsive, and owned
   runtime state is cleaned up after normal and failed shutdown.
8. Capture matching app states before/after representative cross-adapter
   workflows in Light and Dark at 1280×840 logical pixels / 100% when
   available, plus one available alternate size/scale. Capture the owner-
   reported interface/floor gaps as still open if visible; do not claim RB-09
   fixed them. Preserve source/build identity and PNG hashes.

Native capture and real-client checks are mandatory. If Windows is locked or
input/capture is intercepted, label the affected cases unverified and continue
independent automated work; do not close RB-09 from headless tests alone.

## 8. Evidence, docs, and recovery

Use `evidence/rebuild/rb-09-shared-ai-and-regression/`. Keep the existing
RB-08 evidence untouched except for factual status corrections authorized by
the owner. Add:

- `report.md`, `verification.md`, `checkpoint.md`, and the small integration
  architecture decision;
- `requirement-test-map.md` mapping stable IDs to Python/Rust source,
  commands, tests, fixtures, outcomes, and artifacts;
- `walkthrough.md` with the paired GUI/MCP steps and before/after semantic
  state; `paired-comparison-board.md` and `capture-metadata.json`;
- accepted/rejected evidence journals, client/server logs with secrets
  redacted, and build/fixture manifests;
- `source-preservation.md` and a final additive G: snapshot receipt with
  independently verified path/count/byte/SHA-256/reparse totals.

Update the existing MCP catalog/server/desktop-bridge components in
`docs/component-catalog.toml`; add a component only for a real new module and
update stable symbols, tests, contracts, and evidence paths. Update the
`modeler3d.mcp.catalog_and_parity` row and any directly affected parity rows in
`evidence/rebuild/parity-matrix.toml`; distinguish implementation,
automated, interactive, visual, and owner-acceptance states. Append exact
results and next slice to `WORK_LOG.md`. If a Rust-facing MCP contract document
is needed, add it under this Rust project. Never edit the Python reference
contract on `L:`.

Create a new `G:\Builder3D-Rust_snapshot_<timestamp>_RB-09-complete` after the
integrated work and evidence are final. Verify it independently, without
mirroring/deleting old snapshots. Include the current Rust source, tests,
fixtures, catalog, parity matrix, work log, and required evidence. Report
exclusions and any expected receipt post-copy difference exactly.

## 9. Completion boundary

RB-09 is complete only when:

- the real Rust MCP client and native desktop observe and edit one live
  document; no parallel MCP-only scene remains on the product route;
- cross-adapter queries, edits, undo/redo, save/reopen, revisions, replay,
  conflicts, file policy, cancellation, and failure atomicity pass automated
  and live-client/native checks;
- the 60-tool catalog/profile/schema contract remains compatible, or each
  reference-backed intentional deviation is documented and tested;
- MCP can be unavailable without blocking ordinary desktop use;
- evidence and ledgers are updated, all documented gates have exact outcomes,
  and the additive RB-09 completion snapshot is independently verified.

Report RB-08's still-pending native matrix/completion snapshot separately.
Keep the owner's interface/floor feedback and all earlier visual/owner review
statuses open. Finish RB-09 and stop before RB-10: no performance/packaging
slice or RB-11 acceptance closure.

## 10. Fresh-session prompt

    Continue only the RB-09 schema-parity repair in E:\3D-Modeler; stop
    before RB-10. Read E:\AGENTS.md, E:\RustDeveloper.md,
    E:\MCP_DESIGN_GUIDE.md, this workspace's AGENTS.md,
    docs/product-rebuild-plan.md, and this handoff before changing an MCP
    interface or schema. Read the project's MCP contract, architecture, and
    testing documents listed in handoff section 2, and verify protocol/SDK
    assumptions against current official MCP sources. Inspect Git status and
    3D Modeler processes first, and preserve all existing changes and untracked
    files. Do not stage, commit,
    push, reset, or restore from a snapshot unless I explicitly ask.

    First verify the identity of the currently designated L:\Builder3D
    read-only reference and capture its current catalog schemas without
    modifying it. Do not launch or test an older Python build, and do not use
    saved catalog snapshots as the final oracle. If the current reference
    identity cannot be confirmed, stop before schema edits and report that
    blocker. Then follow the “Current schema-parity repair checklist” above:
    reconcile the 19-versus-20 return_detail count, align input and output
    schemas, requiredness, bounds, nullability, annotations, and actual result
    shapes, and add full nested JSON Schema validation across all four profiles.
    Do not remove Rust's additionalProperties rejection without first checking
    current Python behavior and making an explicit, documented compatibility
    decision consistent with the MCP design guide.

    Before Rust source edits, create a new additive G:\ recovery snapshot of
    the current E:\ worktree; keep G:\B3D_R09W_20261106 and all older snapshots
    read-only. Verify paths, file/byte counts, and critical SHA-256 hashes.
    Run focused and workspace tests, the documented engineering checks, and
    the current Rust Release GUI/MCP checks needed to prove schema parity.
    Do not treat name/order parity as schema parity or claim an intentional
    deviation without owner approval. Update the plan, WORK_LOG, and evidence
    only for work actually completed. Keep RB-08 native evidence, the
    interface/floor gaps, visual review, owner acceptance, and overall product
    acceptance pending. Do not begin RB-10.

The latest additive recovery snapshot is `G:/B3D_R09W_20261106`; its
independent verification and critical hashes are in
`evidence/rebuild/rb-09-shared-ai-and-regression/snapshot-receipt.md`. It is a
review/recovery copy, not a completion snapshot, and its receipt postdates the
copy. The older `G:/B3D_R09C_20261106` copy now differs in the two regenerated
current-run JSON files; it was left untouched.
