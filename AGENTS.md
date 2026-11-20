# 3D Modeler Rust Workspace Rules

## Mission

Port 3D Modeler to Rust without changing its product behavior. The intended
end state is a Rust-only product. During the migration, the Python build at
`L:\Builder3D` remains the temporary shipping fallback and behavioral
reference until a stage gate in `L:\Builder3D\docs\rust-rewrite.md` replaces
it; it is not a permanent second implementation.

This workspace also follows the shared Rust development practices in
`E:\RustDeveloper.md`. The project-specific rules below adapt those practices
to 3D Modeler's cross-platform, contract-preserving rewrite.

## Name

The product is **3D Modeler** (renamed from Builder3D on 2026-11-16, owner
decision). Use "3D Modeler" wherever a person reads the name: window titles,
dialogs, the installer, shortcuts and documents. Names a person does not read
cannot start with a digit, so they use `Modeler3D` / `modeler3d`: the program
file `Modeler3D.exe`, the crates `modeler3d-*`, the settings folder
`%LOCALAPPDATA%\Modeler3D`, environment variables `MODELER3D_*` and the MCP
server `modeler3d-rust-mcp`. This mirrors Microsoft's 3D Builder, whose
program file is `Builder3D.exe`.

The old name stays in exactly these places, on purpose:

- `L:\Builder3D`, the Python reference build, which is a real folder and is
  still called Builder3D.
- Backups on `G:\` made before the rename (`G:\Builder3D-Rust_snapshot_*`).
  New snapshots are named `G:\3D-Modeler_snapshot_*`.
- The shipped RC1 files (`Builder3D-RC1.msi`, `Builder3D-RC1.zip`).
- The `.3dp` format marker (`PROJECT_FORMAT = "Builder3D"`), so every existing
  project file still opens and the Python build can still read new ones.
- The Python build's own settings location read by
  `modeler3d-platform/src/legacy_settings.rs`.

Settings were not carried over from `%LOCALAPPDATA%\Builder3D` (owner
decision: start fresh).

## Active project path and backups

- The active Rust source, build, and run copy is `E:\3D-Modeler`. Start
  project commands there and launch the Release app from
  `target\release\modeler3d-app.exe` under this workspace. Do not work from a
  G: snapshot or removable-media copy.
- Use `G:\` as the primary destination for additive Rust recovery snapshots.
  Never overwrite or rotate existing snapshots.
- Removable USB storage is for occasional one-off backups only unless the
  owner explicitly requests otherwise. The current USB snapshot is recorded in
  `evidence/rebuild/rb-09-shared-ai-and-regression/usb-backup-receipt.md`; it
  is not a working copy.
- `L:\Builder3D` and its rewrite-plan documents are the separate Python
  reference/shipping build. Keep that tree read-only when using it as a
  reference.

## One current build (owner rule, 2026-11-19)

There is exactly one build of 3D Modeler the owner runs:
`E:\3D-Modeler\target\release\modeler3d-app.exe`, and it must always be built
from the current source. An old build must never be mistakable for the current
one.

- **Before launching the app for the owner, or saying a visible change is
  done,** run `.\scripts\Update-CurrentBuild.ps1` (add `-Launch` to open it).
  If the exe is older than any source file it stops a running 3D Modeler,
  moves the old exe to `G:\3D-Modeler-old-builds\<timestamp>_OLD-BUILD\`
  (renamed `modeler3d-app.OLD-BUILD.exe`), rebuilds Release, and confirms.
  `-Check` only reports CURRENT or STALE. Never launch an exe that reports STALE.
- **Old builds live only in that backup folder,** always labeled `OLD-BUILD`.
  Never leave an old exe in `target\release`, beside the current one, or
  anywhere the owner might open it. New backups only; never overwrite one.
- **Debug and review binaries are test tooling.** Never launch
  `target\debug` or `target\review` for the owner and never describe them as
  "the app". If the owner asks to see the app, it is the Release build.
- **A slice is not done with a Debug-only pass.** After the gate, run the
  script so the Release exe matches what was tested. Never write "Debug build
  only" in `docs/STATUS.md` as a resting state.
- RC installers in `E:\3D-Modeler-RC` are versioned shipping packages, not the
  working app.

## Sources of truth

[`docs/product-rebuild-plan.md`](docs/product-rebuild-plan.md) (v2, adopted
2026-11-12) is the delivery plan: operating model, model routing, gates, work
order and the open owner decisions. [`docs/STATUS.md`](docs/STATUS.md) is the
only place current status is written. Do not restate status here, in the plan,
or in a handoff document.

A session reads this file, `docs/STATUS.md`, its task card and the files the
card names. `WORK_LOG.md`, the v1 plan under `docs/archive/`, and the
`docs/rb-*-handoff.md` files are history; open them only when a task card
says to.

Standing boundaries:

- Keep `L:\Builder3D` read-only. Keep the original contracts below.
- Local commits are authorized (owner decision D1, 2026-11-12): work on the
  local branch `rebuild/work`, one commit per verified task. No push, no pull
  request, no history rewriting, no commits to `main`. Plus one `G:\` tree
  snapshot per completed slice (plan section 3.6).
- Do not bloat `E:`. Build only into `target/debug`, `target/review` or
  `target/release`; no ad hoc `--target-dir` folders and no scratch files in
  `target/`.
- Do not call a UI complete from module coverage, smoke output or unreviewed
  captures. Owner approval must actually occur; it is never inferred.
- For anything visible, produce review captures with the harness described in
  the plan (section 3.5) instead of photographing the desktop or adding a
  temporary startup hook. For reference comparisons, open one target app,
  capture it, close it, then open the other.
- Run the gate tier the plan assigns to the task, and report a gate that could
  not run as not run.

| File | Purpose |
|---|---|
| `L:\Builder3D\docs\rust-rewrite.md` | Scope, targets, stage plan, risks, decision gates |
| `L:\Builder3D\TODO.md` section R | The ordered work |
| `L:\Builder3D\docs\ai-mcp.md` | The MCP contract this port must keep |
| `L:\Builder3D\docs\project-format.md` | The `.3dp` contract this port must keep |
| `L:\Builder3D\docs\architecture.md` | The layering this port must keep |

## Rules carried over

- Geometry, scene state, commands, and project formats stay independent of the
  UI toolkit, the GPU API, and MCP.
- Every model mutation is one command or one grouped command with undo/redo.
- The desktop and MCP are equal adapters over the same services.
- MCP never simulates physical input; it exposes the semantic result.
- Long operations work on isolated state and commit once.
- Platform paths, settings, process checks, and rendering stay behind
  explicit adapters.
- Millimetres everywhere in the model and the MCP contract.
- One undo entry per operation, and no history entry for a no-op.

## Rust-specific rules

- No `unwrap()` or `expect()` in library code; return a typed error.
- No panics across a crate boundary.
- Public seams carry doc comments that state the cross-platform contract and
  any intentional platform difference.
- Keep implementation details private; expose only the smallest necessary
  `pub` API, using `pub(crate)` for intra-crate seams.
- Smart resource usage is a product requirement, not an optional optimization.
  The Rust application must inspect the actual CPU, system-memory, graphics
  adapter, memory-topology, driver, display, and OS budget available at
  runtime, then choose a workload route from that profile. Do not use a
  vendor-name heuristic as the resource policy.
- `modeler3d-platform` owns capability discovery and platform-specific GPU
  budget/power/display queries. `modeler3d-viewport` and long-running core
  jobs consume a capability profile and a workload plan; they do not each
  invent their own CPU/GPU rules. A central resource ledger accounts for
  authoritative meshes, staging buffers, render targets, caches, undo data,
  and in-flight work across system RAM and GPU/shared memory.
- The planner must support CPU, GPU, and hybrid routes. Intel integrated
  graphics should default to CPU/RAM-heavy geometry work and use the iGPU for
  presentation or measured wins; AMD unified-memory systems must avoid
  duplicating large buffers and may route parallel work to the GPU when the
  calibration and budget say it wins; systems with a healthy dedicated GPU
  should keep viewport/render/compute data resident there when that avoids
  transfers. These are starting policies, never hard-coded vendor guarantees.
- Every GPU feature needs a bounded CPU reference or fallback at the smallest
  useful unit (tile, page, mesh batch, or operation). Over-budget work must
  stream, tile, evict, or split before falling back; an all-or-nothing device
  switch is not acceptable. Adapter selection must respect the display-driving
  device and the operating-system power preference, with an explicit user
  override and a safe startup fallback.
- Record the selected adapter/backend, memory budgets, route, transfer costs,
  frame-time percentiles, peak allocations, and fallback reason in diagnostics.
  Resource routing is not complete until it is tested on Intel integrated,
  AMD unified-memory, dedicated-GPU, and no-accelerator/software-fallback
  profiles, including budget pressure and adapter failure.
- Validate data in parsing/constructor functions so validated domain types
  cannot be created in an invalid state. Use type-state distinctions when they
  make illegal transitions impossible without obscuring the design.
- Mark important return values and domain types with `#[must_use]` when
  ignoring them could lose an error, mutation, command, or state transition.
- Prefer focused modules and traits with clear responsibilities. Use generic
  code for compile-time behavior, trait objects only where runtime selection
  is required, and macros only to remove genuinely repeated boilerplate.
- Keep any binary `main.rs` limited to startup and dependency wiring; put
  reusable behavior in testable library modules.
- `dbg!` is temporary diagnostic code and must be removed before handoff.
  `todo!` is permitted only for explicit local scaffolding; it must not remain
  reachable from accepted application behavior or a public crate boundary.
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` must pass
  before handoff.
- `cargo deny check` must pass; the license set is reviewed, not assumed.
- Measure the installed size with `cargo bloat` against the 40 MB target.
- The viewport must hold the measured Windows budget: full resolution at rest,
  an adaptive interaction tier, orbit paint in the 2-9 ms range on the Quadro
  RTX 4000, including the 30-million-face control.

## Contract carry-over

The `.3dp` project format, the 60-tool MCP catalog and its schemas, the
imported and exported file formats, the millimetre contract, and undo
semantics do not change. Exception (owner decisions D6/D7, 2026-11-12): MCP
must reach 100% desktop parity, so the catalog may gain tools for desktop
actions it lacks, and an MCP result changes when the owner changes the
product behaviour it mirrors. See plan section 3.7. Each needs a golden-file test compared against the
Python build's current output. Shared contracts live as test fixtures, never
as shared source.

## GitHub upload

The owner plans a public GitHub repository: first a private v0.99 for
invited testers, then a public v1.00. When they say they are ready for
either, follow [`docs/github-upload.md`](docs/github-upload.md) ("Release
plan"); until then do not push.

## Test releases and packaging

When the owner asks for a release candidate, test build, installer, MSI or
"something a friend can install", the session does the whole job itself by
following [`docs/packaging.md`](docs/packaging.md). Do not hand the owner a
command to run. Take the next RC number from the "Test releases" line in
`docs/STATUS.md`, build with `-Build -Verify`, update that line, commit, and
give the owner the MSI path and the tester instructions from the doc. Stopping
a running 3D Modeler to unlock the build is always allowed.

For Linux and Mac packages (AppImage and `.dmg`), follow
[`docs/packaging-linux-mac.md`](docs/packaging-linux-mac.md) the same way.

## Git and recovery

This tree is its own Git repository. Commit in the style of the Python tree:
short, imperative subjects. Do not commit build output, downloaded models, or
machine-specific paths.

## Subtask checkpoints, backups, and context compaction

Work in named, bounded subtasks rather than one uncheckpointed conversion
pass. A subtask is complete only after its code or documentation is verified,
its recovery snapshot exists, and its checkpoint is written.

- Before the first implementation subtask, create a baseline snapshot of
  `E:\3D-Modeler` under a new, timestamped directory on `G:\`, using a
  name such as `G:\3D-Modeler_snapshot_20261029-143000_R0b-baseline`.
  Never overwrite an existing snapshot and never modify the existing
  `Builder3D_snapshot_*` backup folders.
- After every completed slice (not every patch; see plan section 3.6), create
  another timestamped snapshot on `G:\`
  before starting the next subtask or compacting context. Include source,
  manifests, tests, documentation, and the Git metadata needed for recovery;
  exclude `target\`, caches, logs, and reproducible generated output unless
  the subtask specifically requires them. If files in `L:\Builder3D` are
  changed, include those changed files in the checkpoint manifest and snapshot
  them without replacing the working copy.
- Verify each snapshot with the copy tool's result plus a file-count and byte-
  count check. Record the absolute snapshot path, subtask name, timestamp, and
  verification result in `WORK_LOG.md` or the current Rust progress log.
- Before context compaction, write a concise checkpoint containing: objective,
  completed work, changed files, tests/checks, measured evidence, snapshot
  path, unresolved risks, and the exact next subtask. Do not begin the next
  subtask until this checkpoint and backup are complete.
- After the checkpoint, use the session's context-compaction mechanism when it
  is available. On resumption, reread the checkpoint, current `git status`,
  and the relevant project instructions instead of relying on conversational
  memory. If the product compacts automatically, the checkpoint is the source
  of truth for the continuation.
- Backups are additive and recoverable. Do not delete, rotate, overwrite, or
  recursively modify a `G:\` snapshot without explicit owner approval.

## Verification

The QHull placement dependency needs build-time libclang. Read
[`docs/building.md`](docs/building.md); on Windows run
`scripts/Initialize-BuildClang.ps1` in the same PowerShell session before Cargo.
This is a project-local build tool, not a product runtime dependency.

- Use the pinned toolchain in `rust-toolchain.toml`; do not silently develop
  against a different compiler or omit rustfmt/Clippy components.
- Treat compiler diagnostics as design feedback. Read the ownership/type
  explanation before reaching for a clone or changing an API.
- During development, `cargo check` is the fast loop. Bacon and Cargo Nextest
  are optional accelerators when installed; they are not required project
  dependencies.
- Unit and integration tests: `cargo test --workspace`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Format: `cargo fmt --check`
- Supply chain: `cargo deny check`
- Security audit: `cargo audit` when the tool is installed in the development
  environment.
- Coverage: use Cargo Tarpaulin for coverage reports once the core has real
  behavior and fixtures; do not treat starter-code coverage as evidence.
- Size: `cargo bloat --release --workspace`

Run `cargo clippy --fix` only as a local reviewed edit; never accept its
changes blindly. Cargo Chef may be used for CI dependency caching, but it is
not part of the application or runtime contract.
