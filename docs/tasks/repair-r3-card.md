# Task card R3: Windows engine route for Repair

Read first: `docs/tasks/repair-plan.md` (phase R3), `docs/tasks/repair-spec.md`
sections 8-9 and "Rules for the R3 Windows route", `docs/tasks/repair-r2-card.md`
(what R2 built: `RepairScore`, `gate`, `REPAIR_REFUSED_MESSAGE`).
Probe showing a working WinRT call sequence:
`evidence/repair-eval-2026-11-22/scripts/windows_repair_probe.rs`.

Edit only the files named here. Do not commit. Do not run the Full gate.
Before cargo, run `scripts/Initialize-BuildClang.ps1` in the same PowerShell
session. No `unwrap`/`expect` in library code. Never call the engine through
anything but its public WinRT API (`Printing3DModel.RepairAsync`); no DLL
internals. Run `cargo fmt`, then
`cargo clippy -p modeler3d-core -p modeler3d-platform -p modeler3d-app -p modeler3d-mcp --all-targets -- -D warnings`,
then `cargo test -p modeler3d-core -p modeler3d-platform -p modeler3d-app -p modeler3d-mcp`.

## 1. Dependencies

Root `Cargo.toml`, workspace `windows` entry: add features `Foundation`,
`Foundation_Collections`, `Graphics_Printing3D`, `Storage_Streams`,
`Win32_System_WinRT` (keep the rest). Pin unchanged. No new crates.

## 2. Windows engine (`crates/modeler3d-core/src/windows_repair.rs`, new)

`#[cfg(windows)]` implementation, plus a non-Windows stub with the same
signature that returns `Err(WindowsRepairError::Unavailable)`.

```rust
pub(crate) enum WindowsRepairError { Unavailable, TimedOut, Cancelled, Failed(String) }
pub(crate) fn run_windows_repair(
    mesh: &Mesh,
    time_limit: Duration,
    cancelled: &dyn Fn() -> bool,
    elapsed: &mut dyn FnMut(Duration),   // called about every 100 ms while waiting
) -> Result<Mesh, WindowsRepairError>
```

- Build `Printing3DModel` + one `Printing3DMesh` straight from `mesh.vertices`
  (R32G32B32Float, stride 12) and `mesh.faces` (R32UInt, stride 12): set
  counts and descriptions, create the buffers, copy bytes (use
  `IBufferByteAccess`), add the mesh to `model.Meshes()`. No temp files.
- Start `model.RepairAsync()`; do NOT `.join()`. Poll its status every 20 ms:
  completed, error, cancelled; on `cancelled()` or time limit call `.Cancel()`
  and return `Cancelled` / `TimedOut`. Call `elapsed` while waiting.
- Read the mesh back (the model may hold one or more meshes; concatenate with
  index offsets). Return a `Mesh` that passes `Mesh::validate`. Any WinRT
  error, empty result or invalid mesh is `Failed` (never panic, never unwrap).
- Windows build only: `unsafe` is allowed only for `IBufferByteAccess`;
  comment why.

## 3. Routing (`crates/modeler3d-core/src/repair.rs`, `lib.rs`)

Add and export:

```rust
pub enum RepairEngine { Windows, Native }   // as_str(): "windows" / "native"
pub struct RepairRouteControl<'a> {
    pub allow_windows: bool,
    pub cancelled: &'a dyn Fn() -> bool,
    pub progress: Option<&'a mut dyn FnMut(Duration, Duration)>, // (elapsed, estimate)
}
pub fn repair_time_estimate(faces: usize) -> Duration; // 0.1 s * (F/4000)^1.28
pub fn repair_time_limit(faces: usize) -> Duration;    // 3x estimate, clamp 30 s..600 s
pub fn repair_mesh_routed(mesh: &Mesh, control: RepairRouteControl<'_>) -> Result<RepairResult, CoreError>;
```

`RepairReport` gains `pub engine: RepairEngine` and
`pub engine_note: Option<String>` (set by native repair to `Native`/`None`).
`repair_mesh`, `repair_mesh_with_profile`, `repair_mesh_with_options` stay
native-only and unchanged in behaviour (boolean operands keep using them).

`repair_mesh_routed`:
1. `before = inspect_mesh`. If `before.valid`, return the native result (nothing
   to fix; same as today).
2. Use the Windows engine only when `allow_windows`, env var
   `MODELER3D_REPAIR_ENGINE` is not `native`, and `before.cavity_shells == 0`
   (owner answer 6: models with a correct cavity go native; note
   "kept the hollow inside, so the native engine was used").
3. Run it with `repair_time_limit(faces)`. Build a candidate `RepairResult`:
   `after = inspect_mesh(candidate)`, scores, `gate`, `candidate_safe`,
   one `RepairStageReport` named `"Windows repair engine"`,
   `engine = Windows`, `can_apply` as in native. Accept it only if verdict is
   `Improved` AND it passes the collapse check below.
4. **Collapse check** (new `fn collapsed(before_mesh, before_health, after_mesh) -> Option<String>`,
   unit tested on every platform): reject if any axis of the bounding box
   shrinks by more than 0.1 mm (min rises or max falls), or, when the input is
   a single closed manifold shell with positive volume, if the volume drops by
   more than 3 %. Skip the volume part for open, multi-shell or non-manifold
   input (overlaps and open sheets legitimately change volume).
5. On `Unavailable`, `TimedOut`, `Failed`, `Cancelled`-not-requested, a refused
   candidate or a collapse: run the native pipeline and return its result with
   `engine = Native` and `engine_note = Some("<plain reason>")`, e.g.
   "Windows repair took too long, so the built-in engine was used." If the user
   cancelled, return `Err(CoreError::MeshOperation("Repair cancelled".into()))`.

Capability check: in `crates/modeler3d-platform` add
`pub fn windows_repair_supported() -> bool` (true on Windows when
`%SystemRoot%\System32\Windows.Graphics.Printing.3D.dll` exists, false
elsewhere; doc comment states the cross-platform contract) with a unit test.

## 4. Adapters

- Desktop `crates/modeler3d-app/src/desktop_jobs.rs`: use
  `repair_mesh_routed` with `allow_windows = windows_repair_supported()`,
  `cancelled` from the job handle, and `progress` mapped to
  `report(percent, "Repairing with the Windows engine, Ns of about Ms")`
  (percent 10..90 from elapsed/estimate, capped at 90). Result detail: if
  `engine == Windows` use `"Windows repair engine"`, else the existing stage
  text. Keep nothing-to-fix and refused branches. If `engine_note` is set,
  append it to the detail text.
- `commands.rs` `repair_selected`: same routed call (no progress, never cancels),
  same message rule.
- MCP `server.rs` repair branch and `repair_report`: use the routed call
  (`allow_windows = windows_repair_supported()`), add `"engine"` and
  `"engine_note"` keys to the report JSON; keep every existing key. Update the
  catalog/golden only if a test requires it (MCP parity: desktop and MCP must
  match).

## 5. Tests (generated in code, no binary files)

Core, `#[cfg(windows)]` tests in `crates/modeler3d-core/tests/repair_windows.rs`
(skip with a printed note if `windows_repair_supported`-style activation fails):
- f3-like sphere with several single-triangle holes: `engine == Windows`,
  `can_apply`, 0 open edges after.
- f4-like flipped sphere: Windows, fixed, volume within 1 % of the original.
- Cube with one duplicated face and a thin sliver: engine Windows or a refused
  fallback with a note; never worse.
- Model with a correct cavity (outer box, inner inward-facing box, plus a
  deliberate 3-edge hole in the outer box): `engine == Native`, note mentions
  the hollow inside, inner cavity still present after repair.
- Unwelded soup with +/-0.01 mm jitter on a cylinder: the Windows result is
  rejected by the collapse check (or fixed correctly), and the final result
  is never a near-empty mesh.
- `allow_windows = false` and `MODELER3D_REPAIR_ENGINE=native` give `Native`.
Cross-platform unit tests: `collapsed()` cases; `repair_time_estimate` /
`repair_time_limit` numbers (4,000 faces about 0.1 s; limit clamps to 30 s and
600 s); non-Windows stub returns `Unavailable` and routed repair equals native.
Also an `#[ignore]` Windows test `repair_fixture_dir` that reads the directory
in env `MODELER3D_REPAIR_FIXTURES` (files `<name>.v` = f64 xyz triples,
`<name>.t` = u32 index triples, as written by
`evidence/repair-eval-2026-11-22/scripts/genfx.py`) and prints one line per
fixture: name, engine, verdict, note, score before/after, volume before/after,
seconds. Existing tests must still pass unweakened.

Report at the end: files changed, test counts, size/deny/audit notes if you ran
them, anything you could not do.
