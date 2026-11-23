# Repair plan: make Repair as good as 3D Builder's (2026-11-22)

Findings and test numbers: [`fixrepair.md`](fixrepair.md) (read only its
"Results", "Root causes" and "Fix plan" sections; the rest is history).
Evidence and reusable scripts: `evidence/repair-eval-2026-11-22/`.

## Owner answers (2026-11-22)

1. **Copy how Windows does it.** Wanted: "ideally we would find out how
   Windows does it and copy that." See "How we learn what Windows does" below.
2. **Small shape changes are fine.** "Adding faces is a great way to fix it."
   Repair may move points by a few thousandths of a mm and add triangles.
3. **Merge overlapping parts into one solid.** "Yes again. Love that."
4. **Refuse a repair that does not help.** "That's a good idea." Show
   "Repair couldn't improve this model. Nothing was changed." and add no undo
   entry.
5. **3D Builder side-by-side is not possible.** 3D Builder shows no Repair
   option for these models: it ignores the errors and only repairs on its own
   when an STL is badly broken. So the Windows engine (same engine, called
   directly) is our reference, not the 3D Builder app.
6. **Keep correct cavities** (asked after R1). A hollow model stays hollow:
   Repair keeps an inward-facing closed part fully inside another part, unlike
   Windows, which fills it. The Windows route (R3) skips such models.

## How we learn what Windows does

The repair engine is in `C:\Windows\System32\Windows.Graphics.Printing.3D.dll`
(Netfabb's engine, Microsoft's licence). We **learn its behaviour, we do not
copy its code**:

- Allowed: call its public API (`Printing3DModel.RepairAsync`) on small test
  models we make, measure exactly what comes back (which points moved and by
  how much, which faces were added, how a hole was filled, how overlaps were
  merged), and write our own code that produces the same results.
- Not allowed: decompiling that DLL, or copying any of its code or tables into
  this Apache-2.0 project. Copying Microsoft's code would put the public
  GitHub release at legal risk. A behaviour spec written from experiments is
  the safe way to "copy" it.
- On Windows we also simply **call** the engine (phase R3), so Windows users
  get the exact 3D Builder result even before our own engine catches up.

## How the sessions run (cost)

- One phase per fresh session. Each session reads only: `AGENTS.md`,
  `docs/STATUS.md` (top of "Now"), this file, and the files its phase names.
  Do not read `WORK_LOG.md`, old handoffs, or all of `fixrepair.md`.
- The session model is the **overseer**. It writes a short task card, hands
  the coding to a worker, reads the diff and gate output, fixes or sends back,
  commits. Overseers do not hand-write large code unless a worker failed twice.
- Workers run **one at a time** in `E:\3D-Modeler` (shared `target\`, no
  worktrees, no extra target folders on `E:`):
  - Codex: `codex exec -C E:\3D-Modeler -s workspace-write -c model_reasoning_effort="<low|medium|high>" "<prompt naming the task card>"`
    run in the background; the card says what to edit and which gate to run.
  - Or a Claude subagent (Haiku for mechanical, Sonnet for features).
- **The work must be right, not just done.** No worker marks its own work
  finished. The overseer checks: the diff against the card, the Fast gate
  output, and the phase's acceptance test numbers (run the evaluation
  scripts). Code Codex wrote is reviewed by Claude, and the reverse
  (cross-family rule, plan 3.2). Two failed attempts at one tier: move up a
  tier with the failure attached, never a third try.
- Throwaway harnesses (like the probes in the evidence folder) build outside
  `E:` with `CARGO_TARGET_DIR=C:\Users\josiph\AppData\Local\Temp\rrt`
  (a short path; the Manifold C++ build fails on long paths).
- End of every session, in this order:
  1. Gate for the phase (Fast; Full where the phase says so).
  2. One local commit on `rebuild/work` (no push).
  3. Tick the phase below and update the one Repair line in `STATUS.md`.
  4. After R2, R3, R6 and R7 (each ships something): `Update-CurrentBuild.ps1`,
     and a `G:\3D-Modeler_snapshot_<timestamp>_repair-<phase>` snapshot.
  5. **Finish the reply to the owner with the prompt for the next session,
     and on the very last line the model and effort to use for it.** Use the
     template at the end of this file. This is required every session.

## Phases

Tick a phase when its acceptance is met. Each phase starts with the owner
pasting the previous session's prompt.

### [x] R1: Windows engine behaviour spec (no product code)

- Session: **Claude Opus 5.5, medium.** Investigation needs judgment; no
  worker.
- Reuse `evidence/repair-eval-2026-11-22/scripts/` (Windows probe, Blender
  measurement, fixture generator). Make many tiny, targeted test models and
  run each through the Windows engine. Answer, with numbers:
  - Point snapping: is there a grid (results suggest 0.002 mm)? Which points
    move, by how much?
  - Weld/stitch: how far apart can two points or two open edges be and still
    be joined? T-junctions?
  - Thin triangles: which are collapsed, by what rule (edge length, angle,
    area)? Which way does the collapse go?
  - Holes: how is a 3, 4, 10, 64, 500-edge hole filled; flat vs curved; are
    new interior points added; triangle sizes?
  - Flat faces re-cut (12-face box → 96): what pattern and why?
  - Flipped faces, inside-out shells, multiple shells: what decides "outside"?
  - Overlaps ("Wrap"): union of shells? Internal cavities kept or removed?
    Loose fins and non-manifold edges: removed, split or kept?
  - Tiny shells: size cut-off.
  - Time vs face count (for the R3 progress dialog and time limit).
- Output: `docs/tasks/repair-spec.md`, one section per step above, each with
  the test model, the measured result, and the rule we will implement. Keep
  any new fixture generator next to the old ones in the evidence folder.
- Acceptance: every question answered or marked "not observable" with the
  reason. No product source changed.
- Done 2026-11-22: [`repair-spec.md`](repair-spec.md). Where it differs from
  `fixrepair.md` Step 3, the spec wins: weld radius 5 grid steps (0.01 mm),
  no zipping of open edges, min-area hole fill with no refine or smooth, no
  re-cut. Owner answered: keep correct cavities (answer 6).

### [x] R2: stop the damage (fix plan Step 1)

- Session: **Claude Sonnet 5.5, medium** (overseer). Worker: **Codex,
  medium.** Review: the Sonnet session (cross-family).
- Card items (from `fixrepair.md` Step 1): before/after score and gate in
  core used by desktop and MCP; refuse message (owner answer 4); stop
  deleting slivers (only repeated-index faces); one degenerate threshold
  shared with `inspect_mesh`; `boundary_loops` survives a vertex shared by two
  holes; health flags inside-out shells.
- Tests: f10 sealed sliver stays sealed; f3's single-triangle holes close;
  f9 inside-out is flagged and fixed; refused repair adds no history (desktop
  and MCP); book-like sealed sliver mesh stays sealed.
- Gate: Fast for `modeler3d-core`, `modeler3d-app`, `modeler3d-mcp`, plus the
  MCP test. Re-run `compare.py` on all fixtures: **nothing may get worse**.
- Ships: Release rebuilt. Owner can retry the book: Repair now refuses or
  fixes, never damages.
- Done 2026-11-22: [`repair-r2-card.md`](repair-r2-card.md). Fast gate passed
  (fmt, clippy, core/app/mcp tests). Fixtures (generated, `genfx.py`): f3, f4,
  f8, f9 fixed; f10 stays sealed; f2, f5, f6, f7 refused and unchanged (R4-R6
  fix them); none worse. Gate allows float32 shifts: it counts defects only,
  so the R3 Windows result still needs the volume/bounding-box collapse check
  from the spec added in R3. Boolean operand repair left as it was.

### [x] R3: Windows engine route (fix plan Step 2)

- Session: **Claude Sonnet 5.5, medium** (overseer). Worker: **Codex, high**
  (WinRT interop). Review: the Sonnet session; escalate to Opus if review
  fails twice.
- Build the `Printing3DModel`/`Printing3DMesh` straight from our buffers (the
  probe `windows_repair_probe.rs` shows a working call sequence), worker
  thread, progress, cancel, time limit from R1, fall back to native on any
  failure, gate from R2 on the result, `engine` field in the report and MCP
  result. Models with a correct cavity go to the native engine (owner
  answer 6). Capability check in `modeler3d-platform`.
- `Cargo.toml` changes: run `cargo deny check` and `cargo audit`; record the
  size change.
- Gate: **Full.** All 10 fixtures + book: Windows route matches the R0 table
  (all fixed).
- Ships: Release rebuilt; owner tries Repair on the book in the app.
- Done 2026-11-23: [`repair-r3-card.md`](repair-r3-card.md). Full gate passed
  (capture board not run: it stalls at the Save picker). Fixtures (`repair_fixture_dir`
  test): Windows route fixed f1 book (12.7 s), f2-f9; f10 is already valid and
  untouched; none worse. Findings: the engine's input must be doubles with
  stride 3 (float32 is rejected with "parameter is incorrect"); the collapse check
  also rejects any result losing over half the volume (the +/-0.01 mm soup case
  came back at 4.7 of 14,114 mm3); only closed shells count as a correct cavity
  (unwelded triangles are not cavities); open input may lose a loose fin.
  Release exe +45 KB. Snapshot `G:D-Modeler_snapshot_20261123-054045_repair-R3`.

### [ ] R4: native engine A: weld, stitch, non-manifold, thin triangles

- Session: **Claude Opus 5.5, medium** (overseer; algorithm review is the
  hard part). Worker: **Codex, high.**
- Fix plan Step 3 items 1-5, following the rules written in
  `repair-spec.md` sections 1-3 (snap grid, weld open-edge points within
  5 steps, T-junctions, collapse only what snapping makes degenerate). Native path only; the Windows route stays the default on
  Windows.
- Acceptance: native fixes f1 book, f5, f7, f8, f10 with no holes opened;
  distance from the original within the spec's bound. Fast gate.

### [ ] R5: native engine B: fill holes of any size

- Session: **Claude Opus 5.5, medium.** Worker: **Codex, high.**
- Fix plan Step 3 item 6, as measured in `repair-spec.md` section 4:
  minimum-total-area fill, no new points, no refine or smooth, flat holes
  flat; cheaper fill above 300 edges. Remove
  `regularize_planar_regions` from Repair.
- Acceptance: native fixes f2 and f3 and a 500-edge hole; book under 5 s.
  Fast gate.

### [ ] R6: native engine C: Wrap (merge overlaps) and self-intersection check

- Session: **Claude Opus 5.5, medium.** Worker: **Codex, high.**
- Fix plan Step 3 item 8: union closed shells with Manifold (already a
  dependency), keeping correct cavities (owner answer 6, spec section 7); add the BVH self-intersection check to `inspect_mesh` so
  health stops saying "not checked" and the needs-repair marker shows for
  self-intersecting models.
- Acceptance: native fixes f6 (one solid, volume 15,000) and all of
  f1-f10; Windows-only test that both engines pass the same fixtures.
  **Full** gate. Ships: Release rebuilt.

### [ ] R7: what the owner sees, and finish

- Session: **Claude Sonnet 5.5, low** (overseer). Worker: **Codex, medium**,
  or a Haiku subagent for the test/ledger updates.
- Plain-words result message ("Repaired: closed 87 holes, fixed 1,858 flipped
  faces, merged 2 overlapping parts."), refused message, needs-repair marker
  for inside-out and self-intersecting models. Captures of the Repair dialog
  in both themes (review capture state `repair`). Fixtures turned into core
  tests generated in code (no binary files). MCP golden test.
- **Full** gate, `Update-CurrentBuild.ps1`, snapshot. Owner tries Repair on
  the book and on one downloaded broken model and approves (approval is never
  inferred).

## Next-session prompt template (end every session with this)

```text
Work in E:\3D-Modeler. Read AGENTS.md, the top of docs/STATUS.md "Now", and
docs/tasks/repair-plan.md. Do phase <ID>: <one line goal>. Follow that
phase's session, worker, gate and acceptance exactly, and the "How the
sessions run" rules. Use Codex as the worker where the phase says so, but
check its work yourself before committing. Finish by ticking the phase,
updating STATUS, committing, and giving me the prompt for the next session
with the model to use on the last line.
```

Last line of the reply, always: `Model for next session: <model>, <effort>`.
