# Builder3D Rust Workspace Rules

## Mission

Port Builder3D to Rust without changing its product behavior. The intended
end state is a Rust-only product. During the migration, the Python build at
`L:\Builder3D` remains the temporary shipping fallback and behavioral
reference until a stage gate in `L:\Builder3D\docs\rust-rewrite.md` replaces
it; it is not a permanent second implementation.

This workspace also follows the shared Rust development practices in
`E:\RustDeveloper.md`. The project-specific rules below adapt those practices
to Builder3D's cross-platform, contract-preserving rewrite.

## Active project path and backups

- The active Rust source, build, and run copy is `E:\Builder3D-Rust`. Start
  project commands there and launch the Release app from
  `target\release\builder3d-app.exe` under this workspace. Do not work from a
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

## Sources of truth

Owner direction on 2026-10-30 replaces the R8 execution/acceptance process with
[`docs/product-rebuild-plan.md`](docs/product-rebuild-plan.md). Read it before
product work. It owns the current RB slice order, original-design/asset parity,
interactive and visual gates, and recovery evidence. The R8 brief is retired;
historical R8 completion claims are withdrawn. Keep the original contracts
below, reuse proven Rust components, and keep `L:\Builder3D` read-only. No
commits or pull requests are authorized. Do not call a UI complete from module
coverage, smoke output or unreviewed captures; required owner approval must
actually occur for acceptance and promotion.
RB-02 is complete. The owner accepted its current Light/Dark visual baseline
on 2026-10-31 and said fine-tuning can happen later. RB-03 through RB-07
implementation and engineering evidence are recorded; visual review and owner
acceptance remain pending where their checkpoints say so. RB-07 is complete,
with its report, verification, checkpoint and snapshot receipt under
`evidence/rebuild/rb-07-appearance-jobs-and-print/`. RB-08 implementation and
automated checks are recorded, but its paired Rust native matrix and completion
snapshot are still pending; do not mark RB-08 complete. The owner has now
authorized continuing to RB-09. Read
[`docs/rb-09-shared-ai-and-regression-handoff.md`](docs/rb-09-shared-ai-and-regression-handoff.md)
and finish that slice, then stop before RB-10. The owner's 2026-11-02 feedback
is that the interface and floor are still missing. Record those as open product
gaps; RB-09 does not provide visual approval or overall acceptance. Preserve
all earlier pending statuses. RB-08's original “stop before RB-09” boundary is
superseded by this later owner direction, not by a claim that RB-08 is closed.
As of 2026-11-03, RB-09's implementation and automated checks are recorded,
but its live-client/native gate is unverified and paused by the owner. On
resumption, work from `E:\Builder3D-Rust`, use `G:\` for the primary recovery
snapshot, finish the RB-09 native gate, and stop before RB-10. USB copies are
occasional one-offs only.
The RB-01 and RB-02 capture records and acceptance are in
[`evidence/rebuild/rb-01-visual-foundations/capture-board.md`](evidence/rebuild/rb-01-visual-foundations/capture-board.md)
and
[`evidence/rebuild/rb-02-first-working-application/checkpoint.md`](evidence/rebuild/rb-02-first-working-application/checkpoint.md).
For comparison captures, open one target app, capture it, close it, then open
the other. A background MCP gateway without a target UI window is not itself
a blocker.

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
- `builder3d-platform` owns capability discovery and platform-specific GPU
  budget/power/display queries. `builder3d-viewport` and long-running core
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
semantics do not change. Each needs a golden-file test compared against the
Python build's current output. Shared contracts live as test fixtures, never
as shared source.

## Git and recovery

This tree is its own Git repository. Commit in the style of the Python tree:
short, imperative subjects. Do not commit build output, downloaded models, or
machine-specific paths.

## Subtask checkpoints, backups, and context compaction

Work in named, bounded subtasks rather than one uncheckpointed conversion
pass. A subtask is complete only after its code or documentation is verified,
its recovery snapshot exists, and its checkpoint is written.

- Before the first implementation subtask, create a baseline snapshot of
  `E:\Builder3D-Rust` under a new, timestamped directory on `G:\`, using a
  name such as `G:\Builder3D-Rust_snapshot_20261029-143000_R0b-baseline`.
  Never overwrite an existing snapshot and never modify the existing
  `Builder3D_snapshot_*` backup folders.
- After every completed subtask, create another timestamped snapshot on `G:\`
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
